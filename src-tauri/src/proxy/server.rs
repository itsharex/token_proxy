use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::Response,
    routing::any,
    Router,
};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::net::TcpListener;

use super::log::LogWriter;
use super::sqlite;
use super::{
    config::ProxyConfig,
    http,
    openai_compat::{
        inbound_format, transform_request_body, ApiFormat, FormatTransform, CHAT_PATH,
        PROVIDER_CHAT, PROVIDER_RESPONSES, RESPONSES_PATH,
    },
    request_body::ReplayableBody,
    upstream::forward_upstream_request,
    ProxyState, RequestMeta,
};

const PROVIDER_ANTHROPIC: &str = "anthropic";
const PROVIDER_GEMINI: &str = "gemini";
const ANTHROPIC_MESSAGES_PREFIX: &str = "/v1/messages";
const ANTHROPIC_COMPLETE_PATH: &str = "/v1/complete";
const GEMINI_MODELS_PREFIX: &str = "/v1beta/models/";
const GEMINI_GENERATE_SUFFIX: &str = ":generateContent";
const GEMINI_STREAM_SUFFIX: &str = ":streamGenerateContent";
const REQUEST_META_LIMIT_BYTES: usize = 2 * 1024 * 1024;
const REQUEST_TRANSFORM_LIMIT_BYTES: usize = 4 * 1024 * 1024;

struct DispatchPlan {
    provider: &'static str,
    outbound_path: Option<&'static str>,
    request_transform: FormatTransform,
    response_transform: FormatTransform,
}

fn resolve_dispatch_plan(config: &ProxyConfig, path: &str) -> Result<DispatchPlan, Response> {
    let has_chat = config.provider_upstreams(PROVIDER_CHAT).is_some();
    let has_responses = config.provider_upstreams(PROVIDER_RESPONSES).is_some();
    let has_anthropic = config.provider_upstreams(PROVIDER_ANTHROPIC).is_some();
    let has_gemini = config.provider_upstreams(PROVIDER_GEMINI).is_some();
    let allow_format_conversion = config.enable_api_format_conversion;

    if is_gemini_path(path) {
        if has_gemini {
            return Ok(DispatchPlan {
                provider: PROVIDER_GEMINI,
                outbound_path: None,
                request_transform: FormatTransform::None,
                response_transform: FormatTransform::None,
            });
        }
        return Err(http::error_response(
            StatusCode::BAD_GATEWAY,
            "No available upstream configured.",
        ));
    }

    if is_anthropic_path(path) {
        if has_anthropic {
            return Ok(DispatchPlan {
                provider: PROVIDER_ANTHROPIC,
                outbound_path: None,
                request_transform: FormatTransform::None,
                response_transform: FormatTransform::None,
            });
        }
        return Err(http::error_response(
            StatusCode::BAD_GATEWAY,
            "No available upstream configured.",
        ));
    }

    let Some(format) = inbound_format(path) else {
        if has_chat {
            return Ok(DispatchPlan {
                provider: PROVIDER_CHAT,
                outbound_path: None,
                request_transform: FormatTransform::None,
                response_transform: FormatTransform::None,
            });
        }
        if has_responses {
            return Ok(DispatchPlan {
                provider: PROVIDER_RESPONSES,
                outbound_path: None,
                request_transform: FormatTransform::None,
                response_transform: FormatTransform::None,
            });
        }
        if has_anthropic {
            return Ok(DispatchPlan {
                provider: PROVIDER_ANTHROPIC,
                outbound_path: None,
                request_transform: FormatTransform::None,
                response_transform: FormatTransform::None,
            });
        }
        return Err(http::error_response(
            StatusCode::BAD_GATEWAY,
            "No available upstream configured.",
        ));
    };

    match format {
        ApiFormat::ChatCompletions => {
            if has_chat {
                return Ok(DispatchPlan {
                    provider: PROVIDER_CHAT,
                    outbound_path: None,
                    request_transform: FormatTransform::None,
                    response_transform: FormatTransform::None,
                });
            }
            if has_responses {
                if !allow_format_conversion {
                    return Err(http::error_response(
                        StatusCode::BAD_GATEWAY,
                        "OpenAI format conversion is disabled (enable_api_format_conversion=false). Configure provider \"openai\" for /v1/chat/completions or enable conversion.",
                    ));
                }
                return Ok(DispatchPlan {
                    provider: PROVIDER_RESPONSES,
                    outbound_path: Some(RESPONSES_PATH),
                    request_transform: FormatTransform::ChatToResponses,
                    response_transform: FormatTransform::ResponsesToChat,
                });
            }
        }
        ApiFormat::Responses => {
            if has_responses {
                return Ok(DispatchPlan {
                    provider: PROVIDER_RESPONSES,
                    outbound_path: None,
                    request_transform: FormatTransform::None,
                    response_transform: FormatTransform::None,
                });
            }
            if has_chat {
                if !allow_format_conversion {
                    return Err(http::error_response(
                        StatusCode::BAD_GATEWAY,
                        "OpenAI format conversion is disabled (enable_api_format_conversion=false). Configure provider \"openai-response\" for /v1/responses or enable conversion.",
                    ));
                }
                return Ok(DispatchPlan {
                    provider: PROVIDER_CHAT,
                    outbound_path: Some(CHAT_PATH),
                    request_transform: FormatTransform::ResponsesToChat,
                    response_transform: FormatTransform::ChatToResponses,
                });
            }
        }
    }

    Err(http::error_response(
        StatusCode::BAD_GATEWAY,
        "No available upstream configured.",
    ))
}

fn build_upstream_cursors(config: &ProxyConfig) -> HashMap<String, Vec<AtomicUsize>> {
    let mut cursors = HashMap::new();
    for (provider, upstreams) in &config.upstreams {
        let group_cursors = upstreams
            .groups
            .iter()
            .map(|_| AtomicUsize::new(0))
            .collect();
        cursors.insert(provider.clone(), group_cursors);
    }
    cursors
}

pub(crate) fn spawn(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_proxy(app).await {
            eprintln!("proxy server failed: {err}");
        }
    });
}

async fn run_proxy(app: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ProxyConfig::load(&app)
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let addr = config.addr();
    let sqlite_pool = match sqlite::open_pool(&app).await {
        Ok(pool) => Some(pool),
        Err(err) => {
            eprintln!("sqlite init failed: {err}");
            None
        }
    };
    let log = Arc::new(LogWriter::new(&config.log_path, sqlite_pool).await?);
    let client = reqwest::Client::new();
    let cursors = build_upstream_cursors(&config);
    let state = Arc::new(ProxyState {
        config,
        client,
        log,
        cursors,
    });

    let app = Router::new()
        .route("/*path", any(proxy_request))
        .layer(DefaultBodyLimit::disable())
        .with_state(state);

    let listener = TcpListener::bind(&addr).await?;
    println!("proxy listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy_request(
    State(state): State<Arc<ProxyState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let path = uri.path();
    tracing::info!(method = %method, path = %path, "incoming request");
    tracing::debug!(headers = ?headers.keys().collect::<Vec<_>>(), "request headers");

    if let Err(response) = http::ensure_local_auth(&state.config, &headers) {
        tracing::warn!("local auth failed");
        return response;
    }
    let (path, _) = extract_request_path(&uri);
    let plan = match resolve_dispatch_plan(&state.config, &path) {
        Ok(plan) => {
            tracing::debug!(provider = %plan.provider, "dispatch plan resolved");
            plan
        }
        Err(response) => {
            tracing::warn!("no dispatch plan found");
            return response;
        }
    };

    let body = match ReplayableBody::from_body(body).await {
        Ok(body) => body,
        Err(err) => {
            return http::error_response(
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {err}"),
            )
        }
    };
    let meta = parse_request_meta_best_effort(&path, &body).await;
    let outbound_path = plan.outbound_path.unwrap_or(path.as_str());
    let outbound_path_with_query = uri
        .query()
        .map(|query| format!("{outbound_path}?{query}"))
        .unwrap_or_else(|| outbound_path.to_string());

    let outbound_body = match maybe_transform_request_body(plan.request_transform, body).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let outbound_body = match maybe_force_openai_stream_options_include_usage(
        plan.provider,
        outbound_path,
        &meta,
        outbound_body,
    )
    .await
    {
        Ok(body) => body,
        Err(response) => return response,
    };
    let request_auth = match http::resolve_request_auth(&state.config, &headers) {
        Ok(auth) => auth,
        Err(response) => return response,
    };
    forward_upstream_request(
        state,
        method,
        plan.provider,
        &path,
        &outbound_path_with_query,
        headers,
        outbound_body,
        meta,
        request_auth,
        plan.response_transform,
    )
    .await
}

fn extract_request_path(uri: &Uri) -> (String, String) {
    let path = uri.path().to_string();
    let path_with_query = uri
        .query()
        .map(|query| format!("{path}?{query}"))
        .unwrap_or_else(|| path.clone());
    (path, path_with_query)
}

fn is_anthropic_path(path: &str) -> bool {
    if path == ANTHROPIC_COMPLETE_PATH || path == ANTHROPIC_MESSAGES_PREFIX {
        return true;
    }
    if !path.starts_with(ANTHROPIC_MESSAGES_PREFIX) {
        return false;
    }
    path.as_bytes()
        .get(ANTHROPIC_MESSAGES_PREFIX.len())
        .is_some_and(|byte| *byte == b'/')
}

fn is_gemini_path(path: &str) -> bool {
    if !path.starts_with(GEMINI_MODELS_PREFIX) {
        return false;
    }
    path.ends_with(GEMINI_GENERATE_SUFFIX) || path.ends_with(GEMINI_STREAM_SUFFIX)
}

fn is_gemini_stream_path(path: &str) -> bool {
    path.starts_with(GEMINI_MODELS_PREFIX) && path.ends_with(GEMINI_STREAM_SUFFIX)
}

fn parse_gemini_model_from_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix(GEMINI_MODELS_PREFIX)?;
    let (model, _) = rest.split_once(':')?;
    let model = model.trim();
    if model.is_empty() {
        None
    } else {
        Some(model.to_string())
    }
}

async fn parse_request_meta_best_effort(path: &str, body: &ReplayableBody) -> RequestMeta {
    let stream_from_path = is_gemini_stream_path(path);
    let model_from_path = parse_gemini_model_from_path(path);

    let Some(bytes) = body
        .read_bytes_if_small(REQUEST_META_LIMIT_BYTES)
        .await
        .unwrap_or(None)
    else {
        return RequestMeta {
            stream: stream_from_path,
            model: model_from_path,
        };
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return RequestMeta {
                stream: stream_from_path,
                model: model_from_path,
            }
        }
    };
    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || stream_from_path;
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let model = model.or(model_from_path);
    RequestMeta { stream, model }
}

async fn maybe_transform_request_body(
    transform: FormatTransform,
    body: ReplayableBody,
) -> Result<ReplayableBody, Response> {
    if transform == FormatTransform::None {
        return Ok(body);
    }

    let Some(bytes) = body
        .read_bytes_if_small(REQUEST_TRANSFORM_LIMIT_BYTES)
        .await
        .map_err(|err| {
            http::error_response(
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {err}"),
            )
        })?
    else {
        return Err(http::error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Request body is too large to transform.",
        ));
    };

    let outbound_bytes = transform_request_body(transform, &bytes)
        .map_err(|message| http::error_response(StatusCode::BAD_REQUEST, message))?;
    Ok(ReplayableBody::from_bytes(outbound_bytes))
}

async fn maybe_force_openai_stream_options_include_usage(
    provider: &str,
    outbound_path: &str,
    meta: &RequestMeta,
    body: ReplayableBody,
) -> Result<ReplayableBody, Response> {
    if provider != PROVIDER_CHAT || outbound_path != CHAT_PATH || !meta.stream {
        return Ok(body);
    }

    let Some(bytes) = body
        .read_bytes_if_small(REQUEST_TRANSFORM_LIMIT_BYTES)
        .await
        .map_err(|err| {
            http::error_response(
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {err}"),
            )
        })?
    else {
        // Best-effort: request body too large, keep original.
        return Ok(body);
    };

    let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(body);
    };
    let Some(object) = value.as_object_mut() else {
        return Ok(body);
    };

    let include_usage = object
        .get("stream_options")
        .and_then(Value::as_object)
        .and_then(|options| options.get("include_usage"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if include_usage {
        return Ok(body);
    }

    let options = match object.get_mut("stream_options") {
        Some(Value::Object(options)) => options,
        _ => {
            object.insert("stream_options".to_string(), Value::Object(Map::new()));
            object
                .get_mut("stream_options")
                .and_then(Value::as_object_mut)
                .expect("stream_options must be object")
        }
    };
    options.insert("include_usage".to_string(), Value::Bool(true));

    let outbound_bytes = serde_json::to_vec(&value)
        .map(Bytes::from)
        .map_err(|err| http::error_response(StatusCode::BAD_REQUEST, format!("Failed to serialize request: {err}")))?;
    Ok(ReplayableBody::from_bytes(outbound_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{collections::HashMap, path::PathBuf};

    use axum::body::Bytes;
    use crate::proxy::config::{ProviderUpstreams, ProxyConfig, UpstreamStrategy};

    fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Runtime::new()
            .expect("create tokio runtime")
            .block_on(future)
    }

    fn config_with_providers(providers: &[&'static str], enable_api_format_conversion: bool) -> ProxyConfig {
        let mut upstreams = HashMap::new();
        for provider in providers {
            upstreams.insert((*provider).to_string(), ProviderUpstreams { groups: Vec::new() });
        }
        ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 9208,
            local_api_key: None,
            log_path: PathBuf::from("proxy.log"),
            enable_api_format_conversion,
            upstream_strategy: UpstreamStrategy::PriorityRoundRobin,
            upstreams,
        }
    }

    #[test]
    fn chat_fallback_requires_format_conversion_enabled() {
        let config = config_with_providers(&[PROVIDER_RESPONSES], false);
        let response = resolve_dispatch_plan(&config, CHAT_PATH)
            .err()
            .expect("should reject");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let config = config_with_providers(&[PROVIDER_RESPONSES], true);
        let plan = resolve_dispatch_plan(&config, CHAT_PATH).expect("should fallback");
        assert_eq!(plan.provider, PROVIDER_RESPONSES);
        assert_eq!(plan.outbound_path, Some(RESPONSES_PATH));
        assert_eq!(plan.request_transform, FormatTransform::ChatToResponses);
        assert_eq!(plan.response_transform, FormatTransform::ResponsesToChat);
    }

    #[test]
    fn responses_fallback_requires_format_conversion_enabled() {
        let config = config_with_providers(&[PROVIDER_CHAT], false);
        let response = resolve_dispatch_plan(&config, RESPONSES_PATH)
            .err()
            .expect("should reject");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let config = config_with_providers(&[PROVIDER_CHAT], true);
        let plan = resolve_dispatch_plan(&config, RESPONSES_PATH).expect("should fallback");
        assert_eq!(plan.provider, PROVIDER_CHAT);
        assert_eq!(plan.outbound_path, Some(CHAT_PATH));
        assert_eq!(plan.request_transform, FormatTransform::ResponsesToChat);
        assert_eq!(plan.response_transform, FormatTransform::ChatToResponses);
    }

    #[test]
    fn force_openai_chat_stream_usage_inserts_stream_options_include_usage() {
        run_async(async {
            let input = Bytes::from_static(br#"{"stream":true,"messages":[]}"#);
            let meta = RequestMeta {
                stream: true,
                model: None,
            };
            let body = ReplayableBody::from_bytes(input);
            let output = maybe_force_openai_stream_options_include_usage(
                PROVIDER_CHAT,
                CHAT_PATH,
                &meta,
                body,
            )
            .await
            .expect("ok");
            let bytes = output
                .read_bytes_if_small(1024)
                .await
                .expect("read")
                .expect("bytes");
            let value: Value = serde_json::from_slice(&bytes).expect("json");
            assert_eq!(value["stream_options"]["include_usage"], Value::Bool(true));
        });
    }

    #[test]
    fn gemini_route_requires_gemini_provider() {
        let config = config_with_providers(&[PROVIDER_CHAT], false);
        let response = resolve_dispatch_plan(&config, "/v1beta/models/gemini-1.5-flash:generateContent")
            .err()
            .expect("should reject");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn gemini_route_dispatches_to_gemini() {
        let config = config_with_providers(&[PROVIDER_GEMINI], false);
        let plan = resolve_dispatch_plan(&config, "/v1beta/models/gemini-1.5-flash:generateContent")
            .expect("should dispatch");
        assert_eq!(plan.provider, PROVIDER_GEMINI);
        assert_eq!(plan.request_transform, FormatTransform::None);
        assert_eq!(plan.response_transform, FormatTransform::None);
    }

    #[test]
    fn gemini_meta_prefers_path_for_stream_and_model() {
        let body = ReplayableBody::from_bytes(Bytes::from_static(b"{}"));
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let meta = rt.block_on(parse_request_meta_best_effort(
            "/v1beta/models/gemini-1.5-flash:streamGenerateContent",
            &body,
        ));
        assert!(meta.stream);
        assert_eq!(meta.model.as_deref(), Some("gemini-1.5-flash"));
    }
}
