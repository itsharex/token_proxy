//! Responses 扩展输入项在不支持该类型的协议中的正文表示。
use serde_json::{Map, Value};

/// 自定义 provider 的 agent_message carrier 是任务明文；不解码普通 reasoning。
pub fn agent_message_text(item: &Map<String, Value>) -> Option<String> {
    if item.get("type").and_then(Value::as_str) != Some("agent_message") {
        return None;
    }
    let text = match item.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                Some("input_text" | "text") => part.get("text").and_then(Value::as_str),
                Some("encrypted_content") => part.get("encrypted_content").and_then(Value::as_str),
                _ => None,
            })
            .collect::<String>(),
        _ => String::new(),
    };
    tracing::debug!(
        content_bytes = text.len(),
        "preserved Codex agent message in protocol bridge"
    );
    Some(text)
}
