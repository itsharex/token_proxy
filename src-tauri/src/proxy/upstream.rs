use axum::{
    body::Bytes,
    http::{header::HeaderValue, HeaderMap, StatusCode},
    response::Response,
};
use std::{
    sync::{
        atomic::Ordering,
        Arc,
    },
    time::Instant,
};

use super::{
    config::{UpstreamRuntime, UpstreamStrategy},
    http,
    openai_compat::FormatTransform,
    response::build_proxy_response,
    ProxyState,
    RequestMeta,
};

pub(super) async fn forward_upstream_request(
    state: Arc<ProxyState>,
    provider: &str,
    inbound_path: &str,
    upstream_path_with_query: &str,
    headers: HeaderMap,
    body: Bytes,
    meta: RequestMeta,
    request_auth: Option<HeaderValue>,
    response_transform: FormatTransform,
) -> Response {
    let mut last_retry_error: Option<String> = None;
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
            provider,
            group_index,
            &group.items,
            inbound_path,
            upstream_path_with_query,
            &headers,
            &body,
            &meta,
            request_auth.as_ref(),
            response_transform,
        )
        .await;
        attempted += result.attempted;
        missing_auth |= result.missing_auth;
        if let Some(response) = result.response {
            return response;
        }
        if result.last_retry_error.is_some() {
            last_retry_error = result.last_retry_error;
        }
    }

    if attempted == 0 && missing_auth {
        return http::error_response(StatusCode::UNAUTHORIZED, "Missing upstream API key.");
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
}

enum AttemptOutcome {
    Success(Response),
    Retryable(String),
    Fatal(Response),
    SkippedAuth,
}

async fn try_group_upstreams(
    state: &ProxyState,
    provider: &str,
    group_index: usize,
    items: &[UpstreamRuntime],
    inbound_path: &str,
    upstream_path_with_query: &str,
    headers: &HeaderMap,
    body: &Bytes,
    meta: &RequestMeta,
    request_auth: Option<&HeaderValue>,
    response_transform: FormatTransform,
) -> GroupAttemptResult {
    let mut last_retry_error = None;
    let mut attempted = 0;
    let mut missing_auth = false;
    let start = resolve_group_start(state, provider, group_index, items.len());
    for item_index in build_group_order(items.len(), start) {
        let upstream = &items[item_index];
        let outcome = attempt_upstream(
            state,
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
                };
            }
            AttemptOutcome::Retryable(message) => {
                last_retry_error = Some(message);
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
    }
}

async fn attempt_upstream(
    state: &ProxyState,
    provider: &str,
    upstream: &UpstreamRuntime,
    inbound_path: &str,
    upstream_path_with_query: &str,
    headers: &HeaderMap,
    body: &Bytes,
    meta: &RequestMeta,
    request_auth: Option<&HeaderValue>,
    response_transform: FormatTransform,
) -> AttemptOutcome {
    let auth = match http::resolve_upstream_auth(upstream, request_auth) {
        Ok(Some(auth)) => auth,
        Ok(None) => return AttemptOutcome::SkippedAuth,
        Err(response) => return AttemptOutcome::Fatal(response),
    };
    let upstream_url = upstream.upstream_url(upstream_path_with_query);
    let request_headers = http::build_upstream_headers(headers, auth);
    let start_time = Instant::now();
    let upstream_res = state
        .client
        .post(upstream_url)
        .headers(request_headers)
        .body(body.clone())
        .send()
        .await;
    match upstream_res {
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
        Err(err) if is_retryable_error(&err) => AttemptOutcome::Retryable(err.to_string()),
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

