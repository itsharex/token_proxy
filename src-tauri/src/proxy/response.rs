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
    openai_compat::{transform_response_body, FormatTransform},
    sse::SseEventParser,
    usage::{extract_usage_from_response, SseUsageCollector},
    RequestMeta,
};

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
        model: meta.model.clone(),
        stream: meta.stream,
        status: status.as_u16(),
        upstream_request_id: http::extract_request_id(upstream_res.headers()),
        start,
    };
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
    response_transform: FormatTransform,
) -> Response {
    let stream = match response_transform {
        FormatTransform::None => stream_with_logging(upstream_res.bytes_stream(), context, log).boxed(),
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

    http::build_response(status, headers, Body::from(output))
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
    let state = ChatToResponsesState::new(upstream, context, log);
    try_unfold(state, |state| async move { state.step().await })
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

struct ChatToResponsesState<S> {
    upstream: S,
    parser: SseEventParser,
    collector: SseUsageCollector,
    log: Arc<LogWriter>,
    context: LogContext,
    out: VecDeque<Bytes>,
    response_id: String,
    sequence: u64,
    sent_created: bool,
    sent_done: bool,
    logged: bool,
    upstream_ended: bool,
}

impl<S> ChatToResponsesState<S>
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
            context,
            out: VecDeque::new(),
            response_id: format!("resp_proxy_{now_ms}"),
            sequence: 0,
            sent_created: false,
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
        let Some(delta) = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(Value::as_str)
        else {
            return;
        };

        if !self.sent_created {
            self.sent_created = true;
            self.out.push_back(responses_event_sse(json!({
                "type": "response.created",
                "response_id": self.response_id.as_str()
            })));
        }
        self.sequence += 1;
        self.out.push_back(responses_event_sse(json!({
            "type": "response.output_text.delta",
            "delta": delta,
            "sequence_number": self.sequence
        })));
    }

    fn push_done(&mut self) {
        if self.sent_done {
            return;
        }
        self.sent_done = true;
        self.out.push_back(responses_event_sse(json!({
            "type": "response.completed",
            "response_id": self.response_id.as_str(),
            "status": "success"
        })));
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

