use axum::body::Bytes;
use serde_json::Value;

use super::sse::SseEventParser;
use super::log::TokenUsage;

pub(crate) struct SseUsageCollector {
    parser: SseEventParser,
    usage: Option<TokenUsage>,
}

impl SseUsageCollector {
    pub(crate) fn new() -> Self {
        Self {
            parser: SseEventParser::new(),
            usage: None,
        }
    }

    pub(crate) fn push_chunk(&mut self, chunk: &[u8]) {
        let usage = &mut self.usage;
        self.parser.push_chunk(chunk, |data| update_usage(usage, &data));
    }

    pub(crate) fn finish(&mut self) -> Option<TokenUsage> {
        let usage = &mut self.usage;
        self.parser.finish(|data| update_usage(usage, &data));
        self.usage.clone()
    }
}

pub(crate) fn extract_usage_from_response(bytes: &Bytes) -> Option<TokenUsage> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let usage = value.get("usage")?;
    usage_from_value(usage)
}

fn extract_usage_from_event(value: &Value) -> Option<TokenUsage> {
    if let Some(usage) = value.get("usage").and_then(usage_from_value) {
        return Some(usage);
    }
    value
        .get("response")
        .and_then(|response| response.get("usage"))
        .and_then(usage_from_value)
}

fn usage_from_value(value: &Value) -> Option<TokenUsage> {
    // Normalize both OpenAI Responses usage (`input_tokens`/`output_tokens`) and
    // Chat Completions usage (`prompt_tokens`/`completion_tokens`) into a single shape.
    let input_tokens = value
        .get("input_tokens")
        .and_then(Value::as_u64)
        .or_else(|| value.get("prompt_tokens").and_then(Value::as_u64));
    let output_tokens = value
        .get("output_tokens")
        .and_then(Value::as_u64)
        .or_else(|| value.get("completion_tokens").and_then(Value::as_u64));
    let total_tokens = value.get("total_tokens").and_then(Value::as_u64);
    if input_tokens.is_some() || output_tokens.is_some() || total_tokens.is_some() {
        return Some(TokenUsage {
            input_tokens,
            output_tokens,
            total_tokens,
        });
    }
    None
}

fn update_usage(usage: &mut Option<TokenUsage>, data: &str) {
    if data == "[DONE]" {
        return;
    }
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return;
    };
    if let Some(updated) = extract_usage_from_event(&value) {
        *usage = Some(updated);
    }
}
