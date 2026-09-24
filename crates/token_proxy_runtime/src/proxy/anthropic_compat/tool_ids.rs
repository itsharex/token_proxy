//! 工具结果先按原始 ID 配对，再为每个 assistant 轮次分配确定且无碰撞的 ID。

use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn normalize_tool_ids(messages: &mut [Value]) {
    let mut mapped = HashMap::<String, String>::new();
    let mut occupied = HashSet::new();
    let mut suffixes = HashMap::<String, usize>::new();
    let mut changed = 0usize;

    for message in messages {
        let is_assistant = message.get("role").and_then(Value::as_str) == Some("assistant");
        let Some(blocks) = message.get_mut("content").and_then(Value::as_array_mut) else {
            continue;
        };
        if is_assistant {
            mapped.clear();
            occupied.clear();
            suffixes.clear();
            // 合法原始 ID 优先，分配后缀不能占用本轮稍后出现的合法 ID。
            occupied.extend(blocks.iter().filter_map(|block| {
                if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                    return None;
                }
                let id = block.get("id")?.as_str()?;
                is_valid_id(id).then(|| id.to_string())
            }));
        }
        for block in blocks {
            let field = match block.get("type").and_then(Value::as_str) {
                Some("tool_use") => "id",
                Some("tool_result") => "tool_use_id",
                _ => continue,
            };
            let raw = block.get(field).and_then(Value::as_str).unwrap_or("");
            let id = mapped.entry(raw.to_string()).or_insert_with(|| {
                if is_valid_id(raw) {
                    return raw.to_string();
                }
                let base = sanitize_id(raw);
                let mut candidate = base.clone();
                let suffix = suffixes.entry(base.clone()).or_default();
                while !occupied.insert(candidate.clone()) {
                    *suffix += 1;
                    candidate = format!("{base}_{suffix}");
                }
                candidate
            });
            if raw != id {
                block[field] = Value::String(id.clone());
                changed += 1;
            }
        }
    }
    if changed > 0 {
        tracing::debug!(
            changed,
            "normalized paired Anthropic tool IDs without collisions"
        );
    }
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(is_valid_character)
}

fn is_valid_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}

fn sanitize_id(id: &str) -> String {
    if id.is_empty() {
        return "tool_use_id".to_string();
    }
    id.chars()
        .map(|character| {
            if is_valid_character(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}
