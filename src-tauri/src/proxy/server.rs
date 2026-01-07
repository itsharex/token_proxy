use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::Response,
    routing::post,
    Router,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::net::TcpListener;

use super::{
    config::ProxyConfig,
    http,
    openai_compat::{
        inbound_format, transform_request_body, ApiFormat, FormatTransform, CHAT_PATH,
        PROVIDER_CHAT, PROVIDER_RESPONSES, RESPONSES_PATH,
    },
    upstream::forward_upstream_request,
    ProxyState,
    RequestMeta,
};
use super::log::LogWriter;

struct DispatchPlan {
    provider: &'static str,
    outbound_path: Option<&'static str>,
    request_transform: FormatTransform,
    response_transform: FormatTransform,
}

fn resolve_dispatch_plan(config: &ProxyConfig, path: &str) -> Result<DispatchPlan, Response> {
    let has_chat = config.provider_upstreams(PROVIDER_CHAT).is_some();
    let has_responses = config.provider_upstreams(PROVIDER_RESPONSES).is_some();

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
    let log = Arc::new(LogWriter::new(&config.log_path).await?);
    let client = reqwest::Client::new();
    let cursors = build_upstream_cursors(&config);
    let state = Arc::new(ProxyState {
        config,
        client,
        log,
        cursors,
    });

    let app = Router::new().route("/*path", post(proxy_request)).with_state(state);

    let listener = TcpListener::bind(&addr).await?;
    println!("proxy listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy_request(
    State(state): State<Arc<ProxyState>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = http::ensure_local_auth(&state.config, &headers) {
        return response;
    }
    let meta = match parse_request_meta(&body) {
        Ok(meta) => meta,
        Err(response) => return response,
    };
    let (path, _) = extract_request_path(&uri);
    let plan = match resolve_dispatch_plan(&state.config, &path) {
        Ok(plan) => plan,
        Err(response) => return response,
    };
    let outbound_path = plan.outbound_path.unwrap_or(path.as_str());
    let outbound_path_with_query = uri
        .query()
        .map(|query| format!("{outbound_path}?{query}"))
        .unwrap_or_else(|| outbound_path.to_string());
    let outbound_body = match transform_request_body(plan.request_transform, &body) {
        Ok(bytes) => bytes,
        Err(message) => return http::error_response(StatusCode::BAD_REQUEST, message),
    };
    let request_auth = match http::resolve_request_auth(&state.config, &headers) {
        Ok(auth) => auth,
        Err(response) => return response,
    };
    forward_upstream_request(
        state,
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

fn parse_request_meta(body: &Bytes) -> Result<RequestMeta, Response> {
    let value: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return Err(http::error_response(
                StatusCode::BAD_REQUEST,
                "Request body must be JSON.",
            ))
        }
    };
    let stream = value.get("stream").and_then(Value::as_bool).unwrap_or(false);
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    Ok(RequestMeta { stream, model })
}
