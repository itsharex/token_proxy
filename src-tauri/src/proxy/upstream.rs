use axum::{
    http::{header::HeaderValue, HeaderMap, Method, StatusCode},
    response::Response,
};
use std::{
    sync::{
        atomic::Ordering,
        Arc,
    },
    time::Instant,
};

const ANTHROPIC_VERSION_HEADER: &str = "anthropic-version";
const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";

use super::{
    config::{UpstreamRuntime, UpstreamStrategy},
    http,
    http::RequestAuth,
    openai_compat::FormatTransform,
    request_body::ReplayableBody,
    response::{build_proxy_response, build_proxy_response_buffered},
    ProxyState,
    RequestMeta,
};

pub(super) async fn forward_upstream_request(
    state: Arc<ProxyState>,
    method: Method,
    provider: &str,
    inbound_path: &str,
    upstream_path_with_query: &str,
    headers: HeaderMap,
    body: ReplayableBody,
    meta: RequestMeta,
    request_auth: RequestAuth,
    response_transform: FormatTransform,
) -> Response {
    let mut last_retry_error: Option<String> = None;
    let mut last_retry_response: Option<Response> = None;
    let mut attempted = 0;
    let mut missing_auth = false;

    let upstreams = match state.config.provider_upstreams(provider) {
        Some(upstreams) => upstreams,
        None => return http::error_response(StatusCode::BAD_GATEWAY, "No available upstream configured."),
    };

    for (group_index, group) in upstreams.groups.iter().enumerate() {
        // Only rotate within the highest priority group; retry network failures before degrading.
        if group.items.is_empty() {
            continue;
        }
        let result = try_group_upstreams(
            &state,
            method.clone(),
            provider,
            group_index,
            &group.items,
            inbound_path,
            upstream_path_with_query,
            &headers,
            &body,
            &meta,
            &request_auth,
            response_transform,
        )
        .await;
        attempted += result.attempted;
        missing_auth |= result.missing_auth;
        if let Some(response) = result.response {
            return response;
        }
        if let Some(response) = result.last_retry_response {
            last_retry_response = Some(response);
        }
        if result.last_retry_error.is_some() {
            last_retry_error = result.last_retry_error;
        }
    }

    if attempted == 0 && missing_auth {
        return http::error_response(StatusCode::UNAUTHORIZED, "Missing upstream API key.");
    }
    if let Some(response) = last_retry_response {
        return response;
    }
    if let Some(err) = last_retry_error {
        return http::error_response(StatusCode::BAD_GATEWAY, format!("Upstream request failed: {err}"));
    }
    http::error_response(StatusCode::BAD_GATEWAY, "No available upstream configured.")
}

struct GroupAttemptResult {
    response: Option<Response>,
    attempted: usize,
    missing_auth: bool,
    last_retry_error: Option<String>,
    last_retry_response: Option<Response>,
}

enum AttemptOutcome {
    Success(Response),
    Retryable {
        message: String,
        response: Option<Response>,
    },
    Fatal(Response),
    SkippedAuth,
}

async fn try_group_upstreams(
    state: &ProxyState,
    method: Method,
    provider: &str,
    group_index: usize,
    items: &[UpstreamRuntime],
    inbound_path: &str,
    upstream_path_with_query: &str,
    headers: &HeaderMap,
    body: &ReplayableBody,
    meta: &RequestMeta,
    request_auth: &RequestAuth,
    response_transform: FormatTransform,
) -> GroupAttemptResult {
    let mut last_retry_error = None;
    let mut last_retry_response = None;
    let mut attempted = 0;
    let mut missing_auth = false;
    let start = resolve_group_start(state, provider, group_index, items.len());
    for item_index in build_group_order(items.len(), start) {
        let upstream = &items[item_index];
        let outcome = attempt_upstream(
            state,
            method.clone(),
            provider,
            upstream,
            inbound_path,
            upstream_path_with_query,
            headers,
            body,
            meta,
            request_auth,
            response_transform,
        )
        .await;
        if !matches!(outcome, AttemptOutcome::SkippedAuth) {
            attempted += 1;
        }
        match outcome {
            AttemptOutcome::Success(response) | AttemptOutcome::Fatal(response) => {
                return GroupAttemptResult {
                    response: Some(response),
                    attempted,
                    missing_auth,
                    last_retry_error,
                    last_retry_response,
                };
            }
            AttemptOutcome::Retryable { message, response } => {
                last_retry_error = Some(message);
                if response.is_some() {
                    last_retry_response = response;
                }
            }
            AttemptOutcome::SkippedAuth => {
                missing_auth = true;
            }
        }
    }
    GroupAttemptResult {
        response: None,
        attempted,
        missing_auth,
        last_retry_error,
        last_retry_response,
    }
}

async fn attempt_upstream(
    state: &ProxyState,
    method: Method,
    provider: &str,
    upstream: &UpstreamRuntime,
    inbound_path: &str,
    upstream_path_with_query: &str,
    headers: &HeaderMap,
    body: &ReplayableBody,
    meta: &RequestMeta,
    request_auth: &RequestAuth,
    response_transform: FormatTransform,
) -> AttemptOutcome {
    let auth = match http::resolve_upstream_auth(provider, upstream, request_auth) {
        Ok(Some(auth)) => auth,
        Ok(None) => return AttemptOutcome::SkippedAuth,
        Err(response) => return AttemptOutcome::Fatal(response),
    };
    let upstream_url = upstream.upstream_url(upstream_path_with_query);
    let mut request_headers = http::build_upstream_headers(headers, auth);
    if provider == "claude" && !request_headers.contains_key(ANTHROPIC_VERSION_HEADER) {
        // Anthropic 官方 API 需要 `anthropic-version`；缺省时补一个稳定默认值，允许客户端覆盖。
        request_headers.insert(
            ANTHROPIC_VERSION_HEADER,
            HeaderValue::from_static(DEFAULT_ANTHROPIC_VERSION),
        );
    }
    let start_time = Instant::now();

    let upstream_body = match body.to_reqwest_body().await {
        Ok(body) => body,
        Err(err) => {
            return AttemptOutcome::Fatal(http::error_response(
                StatusCode::BAD_GATEWAY,
                format!("Failed to read cached request body: {err}"),
            ))
        }
    };

    let upstream_res = state
        .client
        .request(method, upstream_url)
        .headers(request_headers)
        .body(upstream_body)
        .send()
        .await;
    match upstream_res {
        Ok(res) if is_retryable_status(res.status()) => {
            let response = build_proxy_response_buffered(
                meta,
                provider,
                &upstream.id,
                inbound_path,
                res,
                state.log.clone(),
                start_time,
                response_transform,
            )
            .await;
            AttemptOutcome::Retryable {
                message: format!("Upstream responded with {}", response.status()),
                response: Some(response),
            }
        }
        Ok(res) => {
            let response = build_proxy_response(
                meta,
                provider,
                &upstream.id,
                inbound_path,
                res,
                state.log.clone(),
                start_time,
                response_transform,
            )
            .await;
            AttemptOutcome::Success(response)
        }
        Err(err) if is_retryable_error(&err) => AttemptOutcome::Retryable {
            message: err.to_string(),
            response: None,
        },
        Err(err) => AttemptOutcome::Fatal(http::error_response(
            StatusCode::BAD_GATEWAY,
            format!("Upstream request failed: {err}"),
        )),
    }
}

fn resolve_group_start(
    state: &ProxyState,
    provider: &str,
    group_index: usize,
    group_len: usize,
) -> usize {
    match state.config.upstream_strategy {
        UpstreamStrategy::PriorityFillFirst => 0,
        UpstreamStrategy::PriorityRoundRobin => state
            .cursors
            .get(provider)
            .and_then(|cursors| cursors.get(group_index))
            .map(|cursor| cursor.fetch_add(1, Ordering::Relaxed) % group_len)
            .unwrap_or(0),
    }
}

fn build_group_order(group_len: usize, start: usize) -> Vec<usize> {
    (0..group_len)
        .map(|offset| (start + offset) % group_len)
        .collect()
}

fn is_retryable_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

fn is_retryable_status(status: StatusCode) -> bool {
    // 对齐 new-api 的重试策略：429/307/5xx（排除 504/524）。
    if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::TEMPORARY_REDIRECT {
        return true;
    }
    if status == StatusCode::GATEWAY_TIMEOUT {
        return false;
    }
    if status.as_u16() == 524 {
        // Cloudflare timeout.
        return false;
    }
    status.is_server_error()
}

#[cfg(test)]
mod tests;
