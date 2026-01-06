pub(crate) mod config;
mod log;
mod usage;

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{
        header::{
            HeaderName, HeaderValue, AUTHORIZATION, CONNECTION, CONTENT_LENGTH, HOST,
            PROXY_AUTHENTICATE, PROXY_AUTHORIZATION, TE, TRAILER, TRANSFER_ENCODING, UPGRADE,
        },
        HeaderMap, StatusCode,
    },
    response::Response,
    routing::post,
    Router,
};
use futures_util::{stream::try_unfold, StreamExt};
use reqwest::header::HeaderMap as ReqwestHeaderMap;
use serde_json::{json, Value};
use std::{sync::Arc, time::Instant};
use tokio::net::TcpListener;

use config::ProxyConfig;
use log::{build_log_entry, LogContext, LogWriter};
use usage::{extract_usage_from_response, SseUsageCollector};

#[derive(Clone)]
struct ProxyState {
    config: ProxyConfig,
    client: reqwest::Client,
    log: Arc<LogWriter>,
}

struct RequestMeta {
    stream: bool,
    model: Option<String>,
}

const KEEP_ALIVE: HeaderName = HeaderName::from_static("keep-alive");

pub fn spawn(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_proxy(app).await {
            eprintln!("proxy server failed: {err}");
        }
    });
}

async fn run_proxy(
    app: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ProxyConfig::load(&app)
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let addr = config.addr();
    let log = Arc::new(LogWriter::new(&config.log_path).await?);
    let client = reqwest::Client::new();
    let state = Arc::new(ProxyState {
        config,
        client,
        log,
    });

    let app = Router::new()
        .route("/v1/chat/completions", post(proxy_chat))
        .route("/v1/responses", post(proxy_responses))
        .with_state(state);

    let listener = TcpListener::bind(&addr).await?;
    println!("proxy listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy_chat(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_request("/v1/chat/completions", state, headers, body).await
}

async fn proxy_responses(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_request("/v1/responses", state, headers, body).await
}

async fn proxy_request(
    path: &'static str,
    state: Arc<ProxyState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = ensure_local_auth(&state.config, &headers) {
        return response;
    }
    let meta = match parse_request_meta(&body) {
        Ok(meta) => meta,
        Err(response) => return response,
    };
    let auth_header = match resolve_upstream_auth(&state.config, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let upstream_url = state.config.upstream_url(path);
    let request_headers = build_upstream_headers(&headers, auth_header);
    let start = Instant::now();
    let upstream_res = match state
        .client
        .post(upstream_url)
        .headers(request_headers)
        .body(body)
        .send()
        .await
    {
        Ok(res) => res,
        Err(err) => return error_response(StatusCode::BAD_GATEWAY, format!("上游请求失败: {err}")),
    };

    let status = upstream_res.status();
    let response_headers = filter_response_headers(upstream_res.headers());
    let context = LogContext {
        path: path.to_string(),
        model: meta.model,
        stream: meta.stream,
        status: status.as_u16(),
        upstream_request_id: extract_request_id(upstream_res.headers()),
        start,
    };

    if meta.stream {
        build_stream_response(
            status,
            upstream_res,
            response_headers,
            context,
            state.log.clone(),
        )
        .await
    } else {
        build_buffered_response(
            status,
            upstream_res,
            response_headers,
            context,
            state.log.clone(),
        )
        .await
    }
}

async fn build_stream_response(
    status: StatusCode,
    upstream_res: reqwest::Response,
    headers: HeaderMap,
    context: LogContext,
    log: Arc<LogWriter>,
) -> Response {
    let stream = stream_with_logging(upstream_res.bytes_stream(), context, log);
    let body = Body::from_stream(stream);
    build_response(status, headers, body)
}

async fn build_buffered_response(
    status: StatusCode,
    upstream_res: reqwest::Response,
    headers: HeaderMap,
    context: LogContext,
    log: Arc<LogWriter>,
) -> Response {
    let bytes = match upstream_res.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            return error_response(StatusCode::BAD_GATEWAY, format!("读取上游响应失败: {err}"))
        }
    };
    let usage = extract_usage_from_response(&bytes);
    let entry = build_log_entry(&context, usage);
    log.write(&entry).await;
    build_response(status, headers, Body::from(bytes))
}

fn stream_with_logging(
    upstream: impl futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>>
        + Unpin
        + Send
        + 'static,
    context: LogContext,
    log: Arc<LogWriter>,
) -> impl futures_util::stream::Stream<Item = Result<Bytes, std::io::Error>> + Send {
    let collector = SseUsageCollector::new();
    try_unfold(
        (upstream, collector, log, context),
        |(mut upstream, mut collector, log, context)| async move {
            match upstream.next().await {
                Some(Ok(chunk)) => {
                    collector.push_chunk(&chunk);
                    Ok(Some((chunk, (upstream, collector, log, context))))
                }
                Some(Err(err)) => {
                    let entry = build_log_entry(&context, collector.finish());
                    log.write(&entry).await;
                    Err(std::io::Error::new(std::io::ErrorKind::Other, err))
                }
                None => {
                    let entry = build_log_entry(&context, collector.finish());
                    log.write(&entry).await;
                    Ok(None)
                }
            }
        },
    )
}

fn parse_request_meta(body: &Bytes) -> Result<RequestMeta, Response> {
    let value: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => return Err(error_response(StatusCode::BAD_REQUEST, "请求体必须是 JSON")),
    };
    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    Ok(RequestMeta { stream, model })
}

fn ensure_local_auth(config: &ProxyConfig, headers: &HeaderMap) -> Result<(), Response> {
    let Some(expected) = config.local_api_key.as_ref() else {
        return Ok(());
    };
    let Some(header) = headers.get(AUTHORIZATION) else {
        return Err(error_response(StatusCode::UNAUTHORIZED, "缺少本地访问密钥"));
    };
    let Ok(value) = header.to_str() else {
        return Err(error_response(StatusCode::UNAUTHORIZED, "本地访问密钥无效"));
    };
    let expected_value = format!("Bearer {expected}");
    if value != expected_value {
        return Err(error_response(StatusCode::UNAUTHORIZED, "本地访问密钥无效"));
    }
    Ok(())
}

fn resolve_upstream_auth(
    config: &ProxyConfig,
    headers: &HeaderMap,
) -> Result<HeaderValue, Response> {
    if let Some(key) = config.upstream_api_key.as_ref() {
        return bearer_header(key)
            .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "上游密钥包含非法字符"));
    }
    if let Some(value) = headers.get("x-openai-api-key") {
        let Ok(value) = value.to_str() else {
            return Err(error_response(StatusCode::UNAUTHORIZED, "上游密钥无效"));
        };
        return bearer_header(value)
            .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "上游密钥包含非法字符"));
    }
    if config.local_api_key.is_none() {
        if let Some(auth) = headers.get(AUTHORIZATION) {
            return Ok(auth.clone());
        }
    }
    Err(error_response(
        StatusCode::UNAUTHORIZED,
        "缺少上游 OPENAI_API_KEY",
    ))
}

fn bearer_header(value: &str) -> Option<HeaderValue> {
    let header = format!("Bearer {value}");
    HeaderValue::from_str(&header).ok()
}

fn build_upstream_headers(headers: &HeaderMap, auth: HeaderValue) -> ReqwestHeaderMap {
    let mut output = ReqwestHeaderMap::new();
    for (name, value) in headers.iter() {
        if should_skip_request_header(name) {
            continue;
        }
        if name == AUTHORIZATION || name.as_str().eq_ignore_ascii_case("x-openai-api-key") {
            continue;
        }
        output.append(name.clone(), value.clone());
    }
    output.insert(AUTHORIZATION, auth);
    output
}

fn should_skip_request_header(name: &HeaderName) -> bool {
    is_hop_header(name) || name == HOST || name == CONTENT_LENGTH
}

fn filter_response_headers(headers: &ReqwestHeaderMap) -> HeaderMap {
    let mut output = HeaderMap::new();
    for (name, value) in headers.iter() {
        if is_hop_header(name) {
            continue;
        }
        output.append(name.clone(), value.clone());
    }
    output
}

fn is_hop_header(name: &HeaderName) -> bool {
    name == CONNECTION
        || name == KEEP_ALIVE
        || name == PROXY_AUTHENTICATE
        || name == PROXY_AUTHORIZATION
        || name == TE
        || name == TRAILER
        || name == TRANSFER_ENCODING
        || name == UPGRADE
}

fn build_response(status: StatusCode, headers: HeaderMap, body: Body) -> Response {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

fn error_response(status: StatusCode, message: impl AsRef<str>) -> Response {
    let body = json!({
        "error": {
            "message": message.as_ref(),
            "type": "proxy_error"
        }
    });
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

fn extract_request_id(headers: &ReqwestHeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .or_else(|| headers.get("openai-request-id"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}
