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
const X_OPENAI_API_KEY: &str = "x-openai-api-key";
const X_API_KEY: &str = "x-api-key";
const X_ANTHROPIC_API_KEY: &str = "x-anthropic-api-key";
const X_CLAUDE_API_KEY: &str = "x-claude-api-key";

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

#[derive(Clone, Default)]
pub(crate) struct RequestAuth {
    pub(crate) openai_bearer: Option<HeaderValue>,
    pub(crate) claude_api_key: Option<HeaderValue>,
    pub(crate) authorization_fallback: Option<HeaderValue>,
}

pub(crate) struct UpstreamAuthHeader {
    pub(crate) name: HeaderName,
    pub(crate) value: HeaderValue,
}

pub(crate) fn resolve_request_auth(
    config: &ProxyConfig,
    headers: &HeaderMap,
) -> Result<RequestAuth, Response> {
    let mut auth = RequestAuth::default();

    if let Some(value) = headers.get(X_OPENAI_API_KEY) {
        let Ok(value) = value.to_str() else {
            return Err(error_response(StatusCode::UNAUTHORIZED, "Upstream API key is invalid."));
        };
        auth.openai_bearer = Some(bearer_header(value).ok_or_else(|| {
            error_response(
                StatusCode::UNAUTHORIZED,
                "Upstream API key contains invalid characters.",
            )
        })?);
    }

    // Anthropic uses `x-api-key`; allow explicit overrides as well.
    if let Some(value) = headers
        .get(X_API_KEY)
        .or_else(|| headers.get(X_ANTHROPIC_API_KEY))
        .or_else(|| headers.get(X_CLAUDE_API_KEY))
    {
        let Ok(_) = value.to_str() else {
            return Err(error_response(StatusCode::UNAUTHORIZED, "Upstream API key is invalid."));
        };
        auth.claude_api_key = Some(value.clone());
    }

    if config.local_api_key.is_none() {
        if let Some(value) = headers.get(AUTHORIZATION) {
            auth.authorization_fallback = Some(value.clone());
        }
    }
    Ok(auth)
}

pub(crate) fn resolve_upstream_auth(
    provider: &str,
    upstream: &UpstreamRuntime,
    request_auth: &RequestAuth,
) -> Result<Option<UpstreamAuthHeader>, Response> {
    match provider {
        "claude" => {
            let value = match upstream.api_key.as_ref() {
                Some(key) => HeaderValue::from_str(key).map_err(|_| {
                    error_response(
                        StatusCode::UNAUTHORIZED,
                        "Upstream API key contains invalid characters.",
                    )
                })?,
                None => {
                    let Some(value) = request_auth.claude_api_key.clone() else {
                        return Ok(None);
                    };
                    value
                }
            };

            Ok(Some(UpstreamAuthHeader {
                name: HeaderName::from_static(X_API_KEY),
                value,
            }))
        }
        _ => {
            if let Some(key) = upstream.api_key.as_ref() {
                let value = bearer_header(key).ok_or_else(|| {
                    error_response(
                        StatusCode::UNAUTHORIZED,
                        "Upstream API key contains invalid characters.",
                    )
                })?;
                return Ok(Some(UpstreamAuthHeader {
                    name: AUTHORIZATION,
                    value,
                }));
            }

            if let Some(value) = request_auth.openai_bearer.clone() {
                return Ok(Some(UpstreamAuthHeader {
                    name: AUTHORIZATION,
                    value,
                }));
            }

            if let Some(value) = request_auth.authorization_fallback.clone() {
                return Ok(Some(UpstreamAuthHeader {
                    name: AUTHORIZATION,
                    value,
                }));
            }

            Ok(None)
        }
    }
}

pub(crate) fn bearer_header(value: &str) -> Option<HeaderValue> {
    let header = format!("Bearer {value}");
    HeaderValue::from_str(&header).ok()
}

pub(crate) fn build_upstream_headers(
    headers: &HeaderMap,
    auth: UpstreamAuthHeader,
) -> ReqwestHeaderMap {
    let mut output = ReqwestHeaderMap::new();
    for (name, value) in headers.iter() {
        if should_skip_request_header(name) {
            continue;
        }
        if name == AUTHORIZATION
            || name == &auth.name
            || name.as_str().eq_ignore_ascii_case(X_OPENAI_API_KEY)
            || name.as_str().eq_ignore_ascii_case(X_ANTHROPIC_API_KEY)
            || name.as_str().eq_ignore_ascii_case(X_CLAUDE_API_KEY)
        {
            continue;
        }
        output.append(name.clone(), value.clone());
    }
    output.insert(auth.name, auth.value);
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
