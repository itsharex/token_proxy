use axum::{
    body::{Body, Bytes},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use futures_util::{stream::try_unfold, StreamExt};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use super::{
    http,
    log::{build_log_entry, LogContext, LogWriter},
    model,
    openai_compat::{transform_response_body, FormatTransform},
    sse::SseEventParser,
    usage::{extract_usage_from_response, SseUsageCollector},
    RequestMeta,
};

const PROVIDER_OPENAI: &str = "openai";
const PROVIDER_OPENAI_RESPONSES: &str = "openai-response";
const PROVIDER_GEMINI: &str = "gemini";

pub(super) async fn build_proxy_response(
    meta: &RequestMeta,
    provider: &str,
    upstream_id: &str,
    inbound_path: &str,
    upstream_res: reqwest::Response,
    log: Arc<LogWriter>,
    start: Instant,
    response_transform: FormatTransform,
) -> Response {
    let status = upstream_res.status();
    let mut response_headers = http::filter_response_headers(upstream_res.headers());
    let context = LogContext {
        path: inbound_path.to_string(),
        provider: provider.to_string(),
        upstream_id: upstream_id.to_string(),
        model: meta.original_model.clone(),
        mapped_model: meta.mapped_model.clone(),
        stream: meta.stream,
        status: status.as_u16(),
        upstream_request_id: http::extract_request_id(upstream_res.headers()),
        start,
    };
    let model_override = meta.model_override();
    if response_transform != FormatTransform::None {
        // The body will change; let hyper recalculate the content length.
        response_headers.remove(axum::http::header::CONTENT_LENGTH);
    }
    if meta.stream {
        build_stream_response(
            status,
            upstream_res,
            response_headers,
            context,
            log,
            response_transform,
            model_override,
        )
        .await
    } else {
        build_buffered_response(
            status,
            upstream_res,
            response_headers,
            context,
            log,
            response_transform,
            model_override,
        )
        .await
    }
}

pub(super) async fn build_proxy_response_buffered(
    meta: &RequestMeta,
    provider: &str,
    upstream_id: &str,
    inbound_path: &str,
    upstream_res: reqwest::Response,
    log: Arc<LogWriter>,
    start: Instant,
    response_transform: FormatTransform,
) -> Response {
    let status = upstream_res.status();
    let mut response_headers = http::filter_response_headers(upstream_res.headers());
    let context = LogContext {
        path: inbound_path.to_string(),
        provider: provider.to_string(),
        upstream_id: upstream_id.to_string(),
        model: meta.original_model.clone(),
        mapped_model: meta.mapped_model.clone(),
        stream: meta.stream,
        status: status.as_u16(),
        upstream_request_id: http::extract_request_id(upstream_res.headers()),
        start,
    };
    let model_override = meta.model_override();
    if response_transform != FormatTransform::None {
        response_headers.remove(axum::http::header::CONTENT_LENGTH);
    }
    build_buffered_response(
        status,
        upstream_res,
        response_headers,
        context,
        log,
        response_transform,
        model_override,
    )
    .await
}

async fn build_stream_response(
    status: StatusCode,
    upstream_res: reqwest::Response,
    headers: HeaderMap,
    context: LogContext,
    log: Arc<LogWriter>,
    response_transform: FormatTransform,
    model_override: Option<&str>,
) -> Response {
    let stream = match response_transform {
        FormatTransform::None => {
            if let Some(model_override) = model_override {
                if should_rewrite_sse_model(&context.provider) {
                    stream_with_logging_and_model_override(
                        upstream_res.bytes_stream(),
                        context,
                        log,
                        model_override.to_string(),
                    )
                    .boxed()
                } else {
                    stream_with_logging(upstream_res.bytes_stream(), context, log).boxed()
                }
            } else {
                stream_with_logging(upstream_res.bytes_stream(), context, log).boxed()
            }
        }
        FormatTransform::ResponsesToChat => {
            stream_responses_to_chat(upstream_res.bytes_stream(), context, log).boxed()
        }
        FormatTransform::ChatToResponses => {
            stream_chat_to_responses(upstream_res.bytes_stream(), context, log).boxed()
        }
    };
    let body = Body::from_stream(stream);
    http::build_response(status, headers, body)
}

async fn build_buffered_response(
    status: StatusCode,
    upstream_res: reqwest::Response,
    headers: HeaderMap,
    context: LogContext,
    log: Arc<LogWriter>,
    response_transform: FormatTransform,
    model_override: Option<&str>,
) -> Response {
    let bytes = match upstream_res.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            return http::error_response(
                StatusCode::BAD_GATEWAY,
                format!("Failed to read upstream response: {err}"),
            )
        }
    };
    let usage = extract_usage_from_response(&bytes);
    let entry = build_log_entry(&context, usage);
    log.write(&entry).await;

    let output = if response_transform != FormatTransform::None && status.is_success() {
        match transform_response_body(response_transform, &bytes, context.model.as_deref()) {
            Ok(converted) => converted,
            Err(message) => {
                return http::error_response(
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to transform upstream response: {message}"),
                )
            }
        }
    } else {
        bytes
    };

    let output = maybe_override_response_model(output, model_override);

    http::build_response(status, headers, Body::from(output))
}

// 只对 data-only SSE 的提供商做行级重写，避免破坏带 event: 行的流。
fn should_rewrite_sse_model(provider: &str) -> bool {
    provider == PROVIDER_OPENAI
        || provider == PROVIDER_OPENAI_RESPONSES
        || provider == PROVIDER_GEMINI
}

fn maybe_override_response_model(bytes: Bytes, model_override: Option<&str>) -> Bytes {
    let Some(model_override) = model_override else {
        return bytes;
    };
    model::rewrite_response_model(&bytes, model_override).unwrap_or(bytes)
}

fn stream_responses_to_chat(
    upstream: impl futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>>
        + Unpin
        + Send
        + 'static,
    context: LogContext,
    log: Arc<LogWriter>,
) -> impl futures_util::stream::Stream<Item = Result<Bytes, std::io::Error>> + Send {
    let state = ResponsesToChatState::new(upstream, context, log);
    try_unfold(state, |state| async move { state.step().await })
}

fn stream_chat_to_responses(
    upstream: impl futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>>
        + Unpin
        + Send
        + 'static,
    context: LogContext,
    log: Arc<LogWriter>,
) -> impl futures_util::stream::Stream<Item = Result<Bytes, std::io::Error>> + Send {
    chat_to_responses::stream_chat_to_responses(upstream, context, log)
}

struct ResponsesToChatState<S> {
    upstream: S,
    parser: SseEventParser,
    collector: SseUsageCollector,
    log: Arc<LogWriter>,
    context: LogContext,
    out: VecDeque<Bytes>,
    chat_id: String,
    created: i64,
    model: String,
    sent_role: bool,
    sent_done: bool,
    logged: bool,
    upstream_ended: bool,
}

impl<S> ResponsesToChatState<S>
where
    S: futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    fn new(upstream: S, context: LogContext, log: Arc<LogWriter>) -> Self {
        let now_ms = now_ms();
        Self {
            upstream,
            parser: SseEventParser::new(),
            collector: SseUsageCollector::new(),
            log,
            model: context
                .model
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            context,
            out: VecDeque::new(),
            chat_id: format!("chatcmpl_proxy_{now_ms}"),
            created: (now_ms / 1000) as i64,
            sent_role: false,
            sent_done: false,
            logged: false,
            upstream_ended: false,
        }
    }

    async fn step(mut self) -> Result<Option<(Bytes, Self)>, std::io::Error> {
        loop {
            if let Some(next) = self.out.pop_front() {
                return Ok(Some((next, self)));
            }

            if self.upstream_ended {
                return Ok(None);
            }

            match self.upstream.next().await {
                Some(Ok(chunk)) => {
                    self.collector.push_chunk(&chunk);
                    let mut events = Vec::new();
                    self.parser.push_chunk(&chunk, |data| events.push(data));
                    for data in events {
                        self.handle_event(&data);
                    }
                }
                Some(Err(err)) => {
                    self.log_usage_once().await;
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
                }
                None => {
                    self.upstream_ended = true;
                    let mut events = Vec::new();
                    self.parser.finish(|data| events.push(data));
                    for data in events {
                        self.handle_event(&data);
                    }
                    if !self.sent_done {
                        self.push_done();
                    }
                    self.log_usage_once().await;
                    if self.out.is_empty() {
                        return Ok(None);
                    }
                }
            }
        }
    }

    fn handle_event(&mut self, data: &str) {
        if self.sent_done {
            return;
        }
        if data == "[DONE]" {
            self.push_done();
            return;
        }
        let Ok(value) = serde_json::from_str::<Value>(data) else {
            return;
        };
        let Some(event_type) = value.get("type").and_then(Value::as_str) else {
            return;
        };
        if !event_type.ends_with("output_text.delta") {
            return;
        }
        let Some(delta) = value.get("delta").and_then(Value::as_str) else {
            return;
        };

        if !self.sent_role {
            self.sent_role = true;
            self.out.push_back(chat_chunk_sse(
                &self.chat_id,
                self.created,
                &self.model,
                json!({ "role": "assistant", "content": "" }),
                None,
            ));
        }

        self.out.push_back(chat_chunk_sse(
            &self.chat_id,
            self.created,
            &self.model,
            json!({ "content": delta }),
            None,
        ));
    }

    fn push_done(&mut self) {
        if self.sent_done {
            return;
        }
        self.sent_done = true;
        self.out.push_back(chat_chunk_sse(
            &self.chat_id,
            self.created,
            &self.model,
            json!({}),
            Some("stop"),
        ));
        self.out.push_back(Bytes::from("data: [DONE]\n\n"));
    }

    async fn log_usage_once(&mut self) {
        if self.logged {
            return;
        }
        self.logged = true;
        let entry = build_log_entry(&self.context, self.collector.finish());
        self.log.write(&entry).await;
    }
}

fn chat_chunk_sse(
    id: &str,
    created: i64,
    model: &str,
    delta: Value,
    finish_reason: Option<&str>,
) -> Bytes {
    let chunk = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
        "choices": [
            {
                "index": 0,
                "delta": delta,
                "finish_reason": finish_reason
            }
        ]
    });
    Bytes::from(format!("data: {}\n\n", chunk.to_string()))
}

fn responses_event_sse(event: Value) -> Bytes {
    Bytes::from(format!("data: {}\n\n", event.to_string()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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

fn stream_with_logging_and_model_override(
    upstream: impl futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>>
        + Unpin
        + Send
        + 'static,
    context: LogContext,
    log: Arc<LogWriter>,
    model_override: String,
) -> impl futures_util::stream::Stream<Item = Result<Bytes, std::io::Error>> + Send {
    let state = ModelOverrideStreamState::new(upstream, context, log, model_override);
    try_unfold(state, |state| async move { state.step().await })
}

struct ModelOverrideStreamState<S> {
    upstream: S,
    parser: SseEventParser,
    collector: SseUsageCollector,
    log: Arc<LogWriter>,
    context: LogContext,
    out: VecDeque<Bytes>,
    model_override: String,
    upstream_ended: bool,
    logged: bool,
}

impl<S> ModelOverrideStreamState<S>
where
    S: futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    fn new(upstream: S, context: LogContext, log: Arc<LogWriter>, model_override: String) -> Self {
        Self {
            upstream,
            parser: SseEventParser::new(),
            collector: SseUsageCollector::new(),
            log,
            context,
            out: VecDeque::new(),
            model_override,
            upstream_ended: false,
            logged: false,
        }
    }

    async fn step(mut self) -> Result<Option<(Bytes, Self)>, std::io::Error> {
        loop {
            if let Some(next) = self.out.pop_front() {
                return Ok(Some((next, self)));
            }
            if self.upstream_ended {
                self.log_usage_once().await;
                return Ok(None);
            }

            match self.upstream.next().await {
                Some(Ok(chunk)) => {
                    self.collector.push_chunk(&chunk);
                    let mut events = Vec::new();
                    self.parser.push_chunk(&chunk, |data| events.push(data));
                    for data in events {
                        self.push_event(&data);
                    }
                }
                Some(Err(err)) => {
                    self.log_usage_once().await;
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
                }
                None => {
                    self.upstream_ended = true;
                    let mut events = Vec::new();
                    self.parser.finish(|data| events.push(data));
                    for data in events {
                        self.push_event(&data);
                    }
                }
            }
        }
    }

    fn push_event(&mut self, data: &str) {
        let output = rewrite_sse_data(data, &self.model_override);
        self.out.push_back(Bytes::from(format!("data: {output}\n\n")));
    }

    async fn log_usage_once(&mut self) {
        if self.logged {
            return;
        }
        let entry = build_log_entry(&self.context, self.collector.finish());
        self.log.write(&entry).await;
        self.logged = true;
    }
}

fn rewrite_sse_data(data: &str, model_override: &str) -> String {
    if data == "[DONE]" {
        return data.to_string();
    }
    let bytes = Bytes::copy_from_slice(data.as_bytes());
    model::rewrite_response_model(&bytes, model_override)
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .unwrap_or_else(|| data.to_string())
}

mod chat_to_responses;

#[cfg(test)]
mod tests;
