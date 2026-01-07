use axum::{
    body::Body,
    http::{
        header::{
            HeaderName, HeaderValue, AUTHORIZATION, CONNECTION, CONTENT_LENGTH, HOST,
            PROXY_AUTHENTICATE, PROXY_AUTHORIZATION, TE, TRAILER, TRANSFER_ENCODING, UPGRADE,
        },
        HeaderMap, StatusCode,
    },
    response::Response,
};
use reqwest::header::HeaderMap as ReqwestHeaderMap;
use serde_json::json;

use super::config::{ProxyConfig, UpstreamRuntime};

const KEEP_ALIVE: HeaderName = HeaderName::from_static("keep-alive");

pub(crate) fn ensure_local_auth(config: &ProxyConfig, headers: &HeaderMap) -> Result<(), Response> {
    let Some(expected) = config.local_api_key.as_ref() else {
        return Ok(());
    };
    let Some(header) = headers.get(AUTHORIZATION) else {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Missing local access key.",
        ));
    };
    let Ok(value) = header.to_str() else {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Local access key is invalid.",
        ));
    };
    let expected_value = format!("Bearer {expected}");
    if value != expected_value {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Local access key is invalid.",
        ));
    }
    Ok(())
}

pub(crate) fn resolve_request_auth(
    config: &ProxyConfig,
    headers: &HeaderMap,
) -> Result<Option<HeaderValue>, Response> {
    if let Some(value) = headers.get("x-openai-api-key") {
        let Ok(value) = value.to_str() else {
            return Err(error_response(StatusCode::UNAUTHORIZED, "Upstream API key is invalid."));
        };
        return bearer_header(value)
            .ok_or_else(|| {
                error_response(
                    StatusCode::UNAUTHORIZED,
                    "Upstream API key contains invalid characters.",
                )
            })
            .map(Some);
    }
    if config.local_api_key.is_none() {
        if let Some(auth) = headers.get(AUTHORIZATION) {
            return Ok(Some(auth.clone()));
        }
    }
    Ok(None)
}

pub(crate) fn resolve_upstream_auth(
    upstream: &UpstreamRuntime,
    fallback: Option<&HeaderValue>,
) -> Result<Option<HeaderValue>, Response> {
    if let Some(key) = upstream.api_key.as_ref() {
        return bearer_header(key)
            .ok_or_else(|| {
                error_response(
                    StatusCode::UNAUTHORIZED,
                    "Upstream API key contains invalid characters.",
                )
            })
            .map(Some);
    }
    Ok(fallback.cloned())
}

pub(crate) fn bearer_header(value: &str) -> Option<HeaderValue> {
    let header = format!("Bearer {value}");
    HeaderValue::from_str(&header).ok()
}

pub(crate) fn build_upstream_headers(headers: &HeaderMap, auth: HeaderValue) -> ReqwestHeaderMap {
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

pub(crate) fn filter_response_headers(headers: &ReqwestHeaderMap) -> HeaderMap {
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

pub(crate) fn build_response(status: StatusCode, headers: HeaderMap, body: Body) -> Response {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

pub(crate) fn error_response(status: StatusCode, message: impl AsRef<str>) -> Response {
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

pub(crate) fn extract_request_id(headers: &ReqwestHeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .or_else(|| headers.get("openai-request-id"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

