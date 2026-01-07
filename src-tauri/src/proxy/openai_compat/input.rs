use serde_json::{json, Map, Value};

pub(super) fn responses_input_to_chat_messages(items: &[Value]) -> Result<Vec<Value>, String> {
    let mut messages = Vec::with_capacity(items.len());
    for item in items {
        messages.push(responses_input_item_to_chat_message(item)?);
    }
    Ok(messages)
}

fn responses_input_item_to_chat_message(item: &Value) -> Result<Value, String> {
    let Some(item) = item.as_object() else {
        return Err("Responses input item must be an object.".to_string());
    };

    // Cherry Studio / Codex CLI 可能会直接传 `[{ role, content:[{type,text}...] }]`，
    // 这里需要把 content parts 归一化成 Chat API 需要的字符串。
    if item.get("role").and_then(Value::as_str).is_some() {
        let mut output = item.clone();
        if let Some(content) = item.get("content").and_then(responses_message_content_to_text) {
            output.insert("content".to_string(), Value::String(content));
        }
        return Ok(Value::Object(output));
    }

    let Some(item_type) = item.get("type").and_then(Value::as_str) else {
        return Err("Responses input item must include role or type.".to_string());
    };

    match item_type {
        "message" => responses_message_item_to_chat_message(item),
        "function_call_output" => responses_function_call_output_item_to_chat_message(item),
        "function_call" => responses_function_call_item_to_chat_message(item),
        _ => Err(format!("Unsupported Responses input item type: {item_type}")),
    }
}

fn responses_message_item_to_chat_message(item: &Map<String, Value>) -> Result<Value, String> {
    let role = item
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| "Responses message item must include role.".to_string())?;
    let content = item
        .get("content")
        .and_then(responses_message_content_to_text)
        .unwrap_or_default();
    Ok(json!({ "role": role, "content": content }))
}

fn responses_function_call_output_item_to_chat_message(item: &Map<String, Value>) -> Result<Value, String> {
    let call_id = item
        .get("call_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "function_call_output must include call_id.".to_string())?;
    let output = item.get("output").and_then(Value::as_str).unwrap_or("");
    Ok(json!({
        "role": "tool",
        "tool_call_id": call_id,
        "content": output
    }))
}

fn responses_function_call_item_to_chat_message(item: &Map<String, Value>) -> Result<Value, String> {
    let call_id = item
        .get("call_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "function_call must include call_id.".to_string())?;
    let name = item.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = item.get("arguments").and_then(Value::as_str).unwrap_or("");
    Ok(json!({
        "role": "assistant",
        "content": "",
        "tool_calls": [
            {
                "id": call_id,
                "type": "function",
                "function": { "name": name, "arguments": arguments }
            }
        ]
    }))
}

fn responses_message_content_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.to_string()),
        Value::Array(parts) => {
            let mut combined = String::new();
            for part in parts {
                let Some(part) = part.as_object() else {
                    continue;
                };
                let part_type = part.get("type").and_then(Value::as_str);
                if !matches!(part_type, Some("input_text") | Some("text")) {
                    continue;
                }
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    combined.push_str(text);
                }
            }
            Some(combined)
        }
        _ => None,
    }
}

