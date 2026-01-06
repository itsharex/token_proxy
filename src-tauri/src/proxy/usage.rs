use axum::body::Bytes;
use serde_json::Value;

use super::log::TokenUsage;

pub(crate) struct SseUsageCollector {
    buffer: String,
    current_data: String,
    usage: Option<TokenUsage>,
}

impl SseUsageCollector {
    pub(crate) fn new() -> Self {
        Self {
            buffer: String::new(),
            current_data: String::new(),
            usage: None,
        }
    }

    pub(crate) fn push_chunk(&mut self, chunk: &[u8]) {
        let text = String::from_utf8_lossy(chunk);
        self.buffer.push_str(&text);
        while let Some(pos) = self.buffer.find('\n') {
            let mut line = self.buffer[..pos].to_string();
            self.buffer.drain(..=pos);
            if line.ends_with('\r') {
                line.pop();
            }
            self.process_line(&line);
        }
    }

    pub(crate) fn finish(&mut self) -> Option<TokenUsage> {
        if !self.buffer.is_empty() {
            let mut buffer = std::mem::take(&mut self.buffer);
            if buffer.ends_with('\r') {
                buffer.pop();
            }
            self.process_line(&buffer);
        }
        self.flush_event();
        self.usage.clone()
    }

    fn process_line(&mut self, line: &str) {
        if line.is_empty() {
            self.flush_event();
            return;
        }
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim_start();
            if !self.current_data.is_empty() {
                self.current_data.push('\n');
            }
            self.current_data.push_str(data);
        }
    }

    fn flush_event(&mut self) {
        if self.current_data.is_empty() {
            return;
        }
        let data = std::mem::take(&mut self.current_data);
        let data = data.trim();
        if data == "[DONE]" {
            return;
        }
        if let Ok(value) = serde_json::from_str::<Value>(data) {
            if let Some(usage) = extract_usage_from_event(&value) {
                self.usage = Some(usage);
            }
        }
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
    let input_tokens = value.get("input_tokens").and_then(Value::as_u64);
    let output_tokens = value.get("output_tokens").and_then(Value::as_u64);
    let total_tokens = value.get("total_tokens").and_then(Value::as_u64);
    if input_tokens.is_some() || output_tokens.is_some() || total_tokens.is_some() {
        return Some(TokenUsage {
            input_tokens,
            output_tokens,
            total_tokens,
        });
    }

    let prompt_tokens = value.get("prompt_tokens").and_then(Value::as_u64);
    let completion_tokens = value.get("completion_tokens").and_then(Value::as_u64);
    let total_tokens = value.get("total_tokens").and_then(Value::as_u64);
    if prompt_tokens.is_some() || completion_tokens.is_some() || total_tokens.is_some() {
        return Some(TokenUsage {
            input_tokens: prompt_tokens,
            output_tokens: completion_tokens,
            total_tokens,
        });
    }
    None
}
