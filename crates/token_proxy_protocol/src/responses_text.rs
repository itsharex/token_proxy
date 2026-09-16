//! Responses 文本增量与快照对账；不缓存整份响应，不改写已经交付的文本。
use serde_json::Value;
use std::collections::HashMap;

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_TEXT_PARTS: usize = 128;

#[derive(Default)]
pub struct ResponsesTextRecovery {
    parts: HashMap<(String, u64), String>,
    item_ids: HashMap<u64, String>,
    bytes: usize,
    unindexed_delta: bool,
    disabled: bool,
    finished: bool,
}

impl ResponsesTextRecovery {
    pub fn process(&mut self, event: &Value) -> Vec<String> {
        if self.finished {
            return Vec::new();
        }
        let mut output = Vec::new();
        match event.get("type").and_then(Value::as_str) {
            Some("response.output_item.added") => {
                // 声明事件先建立身份关联，后续帧可能只携带 id 或 output_index。
                self.key(
                    event.pointer("/item/id").and_then(Value::as_str),
                    event.get("output_index").and_then(Value::as_u64),
                    0,
                );
            }
            Some("response.output_text.delta") => {
                if let Some(delta) = event
                    .get("delta")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                {
                    let id = event
                        .get("item_id")
                        .and_then(Value::as_str)
                        .filter(|id| !id.is_empty());
                    let index = event.get("output_index").and_then(Value::as_u64);
                    self.unindexed_delta |= id.is_none() && index.is_none();
                    if let Some(key) = self.key(
                        id,
                        index,
                        event
                            .get("content_index")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                    ) {
                        self.remember(key, delta);
                    } else {
                        // 已发增量若无法可靠归属，后续快照不能再猜测补发。
                        self.disable();
                    }
                    output.push(delta.to_string());
                }
            }
            Some("response.output_text.done") => {
                if let Some(text) = event.get("text").and_then(Value::as_str) {
                    self.recover(
                        event.get("item_id").and_then(Value::as_str),
                        event.get("output_index").and_then(Value::as_u64),
                        event
                            .get("content_index")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        text,
                        &mut output,
                    );
                }
            }
            Some("response.output_item.done") => {
                if let Some(item) = event.get("item") {
                    self.recover_item(
                        item,
                        event.get("output_index").and_then(Value::as_u64),
                        &mut output,
                    );
                }
            }
            Some("response.completed" | "response.incomplete") => {
                if let Some(items) = event.pointer("/response/output").and_then(Value::as_array) {
                    for (index, item) in items.iter().enumerate() {
                        self.recover_item(item, Some(index as u64), &mut output);
                    }
                }
                self.finished = true;
            }
            _ => {}
        }
        output
    }

    fn recover_item(&mut self, item: &Value, index: Option<u64>, output: &mut Vec<String>) {
        if item.get("type").and_then(Value::as_str) != Some("message")
            || item.get("role").and_then(Value::as_str) != Some("assistant")
        {
            return;
        }
        if let Some(parts) = item.get("content").and_then(Value::as_array) {
            for (content_index, part) in parts.iter().enumerate() {
                if part.get("type").and_then(Value::as_str) == Some("output_text") {
                    if let Some(text) = part.get("text").and_then(Value::as_str) {
                        self.recover(
                            item.get("id").and_then(Value::as_str),
                            index,
                            content_index as u64,
                            text,
                            output,
                        );
                    }
                }
            }
        }
    }

    fn recover(
        &mut self,
        id: Option<&str>,
        index: Option<u64>,
        content: u64,
        text: &str,
        output: &mut Vec<String>,
    ) {
        if self.disabled || text.is_empty() {
            return;
        }
        let Some(key) = self.key(id, index, content) else {
            return;
        };
        // 已发送匿名文本时，不能猜测新身份的终态属于哪一段。
        if !self.parts.contains_key(&key)
            && (self.unindexed_delta || (key.0 == "unknown" && !self.parts.is_empty()))
        {
            return;
        }
        let previous = self.parts.get(&key).map(String::as_str).unwrap_or("");
        let Some(tail) = text.strip_prefix(previous).filter(|tail| !tail.is_empty()) else {
            return;
        };
        let tail = tail.to_string();
        self.remember(key, &tail);
        tracing::debug!(
            recovered_bytes = tail.len(),
            "recovered missing Responses text suffix"
        );
        output.push(tail);
    }

    fn key(&mut self, id: Option<&str>, index: Option<u64>, content: u64) -> Option<(String, u64)> {
        if self.disabled {
            return None;
        }
        let id = id.filter(|id| !id.is_empty());
        if let (Some(id), Some(index)) = (id, index) {
            if self.item_ids.get(&index).is_some_and(|known| known != id) {
                tracing::debug!(
                    output_index = index,
                    "skipped ambiguous Responses text snapshot identity"
                );
                return None;
            }
            if self.item_ids.len() >= MAX_TEXT_PARTS && !self.item_ids.contains_key(&index) {
                self.disable();
                return None;
            }
            self.item_ids.entry(index).or_insert_with(|| id.to_string());
        }
        let known_id =
            id.or_else(|| index.and_then(|index| self.item_ids.get(&index).map(String::as_str)));
        let identity = match known_id {
            Some(id) => format!("id:{id}"),
            None => index
                .map(|index| format!("index:{index}"))
                .unwrap_or_else(|| "unknown".to_string()),
        };
        let key = (identity, content);
        if let Some(index) = index {
            let indexed = (format!("index:{index}"), content);
            if indexed != key && !self.parts.contains_key(&key) {
                if let Some(text) = self.parts.remove(&indexed) {
                    self.parts.insert(key.clone(), text);
                }
            }
        }
        Some(key)
    }

    fn remember(&mut self, key: (String, u64), text: &str) {
        if self.disabled {
            return;
        }
        if self.bytes.saturating_add(text.len()) > MAX_TEXT_BYTES
            || (!self.parts.contains_key(&key) && self.parts.len() >= MAX_TEXT_PARTS)
        {
            self.disable();
            return;
        }
        self.bytes += text.len();
        self.parts.entry(key).or_default().push_str(text);
    }

    fn disable(&mut self) {
        self.disabled = true;
        self.parts.clear();
        self.item_ids.clear();
        tracing::debug!(
            bytes = self.bytes,
            "Responses text recovery budget reached; preserving live deltas only"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn added_identity_connects_id_only_deltas_and_index_only_done() {
        let mut state = ResponsesTextRecovery::default();
        let events = [
            json!({"type":"response.output_item.added","output_index":0,"item":{"id":"m","type":"message"}}),
            json!({"type":"response.output_text.delta","item_id":"m","delta":"a"}),
            json!({"type":"response.output_text.done","output_index":0,"text":"abc"}),
        ];
        assert_eq!(
            events
                .iter()
                .flat_map(|event| state.process(event))
                .collect::<String>(),
            "abc"
        );
    }

    #[test]
    fn reconciles_done_and_terminal_without_repeating_multiple_parts() {
        let mut state = ResponsesTextRecovery::default();
        let events = [
            json!({"type":"response.output_text.delta","item_id":"m","output_index":0,"delta":"你"}),
            json!({"type":"response.output_text.done","item_id":"m","output_index":0,"text":"你好"}),
            json!({"type":"response.output_text.done","item_id":"m","output_index":0,"text":"你好"}),
            json!({"type":"response.completed","response":{"output":[
                {"id":"m","type":"message","role":"assistant","content":[{"type":"output_text","text":"你好"},{"type":"output_text","text":"世界"}]},
                {"id":"n","type":"message","role":"assistant","content":[{"type":"output_text","text":"!"}]}]}}),
            json!({"type":"response.output_text.done","text":"late"}),
        ];
        let text = events
            .iter()
            .flat_map(|event| state.process(event))
            .collect::<String>();
        assert_eq!(text, "你好世界!");
    }

    #[test]
    fn skips_conflicts_and_bounds_recovery_without_truncating_live_text() {
        let mut state = ResponsesTextRecovery::default();
        state.process(&json!({"type":"response.output_text.delta","item_id":"a","output_index":0,"delta":"hi"}));
        assert!(state.process(&json!({"type":"response.output_text.done","item_id":"b","output_index":0,"text":"wrong"})).is_empty());
        assert!(state.process(&json!({"type":"response.output_text.done","item_id":"a","output_index":0,"text":"different"})).is_empty());
        let long = "x".repeat(MAX_TEXT_BYTES + 1);
        assert_eq!(
            state.process(&json!({"type":"response.output_text.delta","delta":long}))[0].len(),
            long.len()
        );
        assert!(state
            .process(&json!({"type":"response.output_text.done","text":"ignored"}))
            .is_empty());
    }
}
