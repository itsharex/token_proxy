use axum::body::Bytes;
use serde_json::{json, Map, Value};

pub(crate) const CHAT_PATH: &str = "/v1/chat/completions";
pub(crate) const RESPONSES_PATH: &str = "/v1/responses";

pub(crate) const PROVIDER_CHAT: &str = "openai";
pub(crate) const PROVIDER_RESPONSES: &str = "openai-response";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApiFormat {
    ChatCompletions,
    Responses,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatTransform {
    None,
    ChatToResponses,
    ResponsesToChat,
}

pub(crate) fn inbound_format(path: &str) -> Option<ApiFormat> {
    match path {
        CHAT_PATH => Some(ApiFormat::ChatCompletions),
        RESPONSES_PATH => Some(ApiFormat::Responses),
        _ => None,
    }
}

pub(crate) fn transform_request_body(transform: FormatTransform, body: &Bytes) -> Result<Bytes, String> {
    match transform {
        FormatTransform::None => Ok(body.clone()),
        FormatTransform::ChatToResponses => chat_request_to_responses(body),
        FormatTransform::ResponsesToChat => responses_request_to_chat(body),
    }
}

pub(crate) fn transform_response_body(
    transform: FormatTransform,
    bytes: &Bytes,
    model_hint: Option<&str>,
) -> Result<Bytes, String> {
    match transform {
        FormatTransform::None => Ok(bytes.clone()),
        FormatTransform::ChatToResponses => chat_response_to_responses(bytes),
        FormatTransform::ResponsesToChat => responses_response_to_chat(bytes, model_hint),
    }
}

fn chat_request_to_responses(body: &Bytes) -> Result<Bytes, String> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| "Request body must be JSON.".to_string())?;
    let Some(object) = value.as_object() else {
        return Err("Request body must be a JSON object.".to_string());
    };

    let Some(messages) = object.get("messages").and_then(Value::as_array) else {
        return Err("Chat request must include messages.".to_string());
    };

    // Responses API uses `input` for message arrays.
    let mut output = Map::new();
    copy_key(object, &mut output, "model");
    output.insert("input".to_string(), Value::Array(messages.clone()));
    copy_key(object, &mut output, "stream");
    copy_key(object, &mut output, "temperature");
    copy_key(object, &mut output, "top_p");
    copy_key(object, &mut output, "stop");
    copy_key(object, &mut output, "tools");
    copy_key(object, &mut output, "tool_choice");
    copy_key(object, &mut output, "metadata");
    copy_key(object, &mut output, "user");
    copy_key(object, &mut output, "seed");

    if let Some(max_output_tokens) = object
        .get("max_completion_tokens")
        .or_else(|| object.get("max_tokens"))
        .and_then(Value::as_i64)
    {
        output.insert("max_output_tokens".to_string(), Value::Number(max_output_tokens.into()));
    }

    serde_json::to_vec(&Value::Object(output))
        .map(Bytes::from)
        .map_err(|err| format!("Failed to serialize request: {err}"))
}

fn responses_request_to_chat(body: &Bytes) -> Result<Bytes, String> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| "Request body must be JSON.".to_string())?;
    let Some(object) = value.as_object() else {
        return Err("Request body must be a JSON object.".to_string());
    };

    let mut messages = match object.get("input") {
        Some(Value::String(text)) => vec![json!({ "role": "user", "content": text })],
        Some(Value::Array(items)) => items.clone(),
        _ => return Err("Responses request must include input.".to_string()),
    };

    // Responses API supports `instructions`; translate it to a system message.
    if let Some(instructions) = object.get("instructions").and_then(Value::as_str) {
        if !instructions.trim().is_empty() {
            messages.insert(0, json!({ "role": "system", "content": instructions }));
        }
    }

    let mut output = Map::new();
    copy_key(object, &mut output, "model");
    output.insert("messages".to_string(), Value::Array(messages));
    copy_key(object, &mut output, "stream");
    copy_key(object, &mut output, "temperature");
    copy_key(object, &mut output, "top_p");
    copy_key(object, &mut output, "stop");
    copy_key(object, &mut output, "tools");
    copy_key(object, &mut output, "tool_choice");
    copy_key(object, &mut output, "metadata");
    copy_key(object, &mut output, "user");
    copy_key(object, &mut output, "seed");

    if let Some(max_output_tokens) = object.get("max_output_tokens").and_then(Value::as_i64) {
        // Prefer the modern chat parameter.
        output.insert(
            "max_completion_tokens".to_string(),
            Value::Number(max_output_tokens.into()),
        );
    }

    serde_json::to_vec(&Value::Object(output))
        .map(Bytes::from)
        .map_err(|err| format!("Failed to serialize request: {err}"))
}

fn responses_response_to_chat(bytes: &Bytes, model_hint: Option<&str>) -> Result<Bytes, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "Upstream response must be JSON.".to_string())?;
    let Some(object) = value.as_object() else {
        return Err("Upstream response must be a JSON object.".to_string());
    };

    let content = extract_responses_output_text(&value).unwrap_or_default();
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("chatcmpl-proxy");
    let created = object.get("created_at").and_then(Value::as_i64).unwrap_or(0);
    let model = object
        .get("model")
        .and_then(Value::as_str)
        .or(model_hint)
        .unwrap_or("unknown");

    let usage = object
        .get("usage")
        .and_then(|usage| map_usage_responses_to_chat(usage));

    let output = json!({
        "id": id,
        "object": "chat.completion",
        "created": created,
        "model": model,
        "choices": [
            {
                "index": 0,
                "message": { "role": "assistant", "content": content },
                "finish_reason": "stop"
            }
        ],
        "usage": usage
    });

    serde_json::to_vec(&output)
        .map(Bytes::from)
        .map_err(|err| format!("Failed to serialize response: {err}"))
}

fn chat_response_to_responses(bytes: &Bytes) -> Result<Bytes, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "Upstream response must be JSON.".to_string())?;
    let Some(object) = value.as_object() else {
        return Err("Upstream response must be a JSON object.".to_string());
    };

    let content = extract_chat_choice_text(&value).unwrap_or_default();
    let id = object.get("id").and_then(Value::as_str).unwrap_or("resp-proxy");
    let created = object.get("created").and_then(Value::as_i64).unwrap_or(0);
    let model = object.get("model").and_then(Value::as_str).unwrap_or("unknown");

    let usage = object
        .get("usage")
        .and_then(|usage| map_usage_chat_to_responses(usage));

    let output = json!({
        "id": id,
        "object": "response",
        "created_at": created,
        "status": "completed",
        "error": null,
        "model": model,
        "output": [
            {
                "type": "message",
                "id": "msg_proxy",
                "status": "completed",
                "role": "assistant",
                "content": [
                    { "type": "output_text", "text": content, "annotations": [] }
                ]
            }
        ],
        "usage": usage
    });

    serde_json::to_vec(&output)
        .map(Bytes::from)
        .map_err(|err| format!("Failed to serialize response: {err}"))
}

fn copy_key(source: &serde_json::Map<String, Value>, target: &mut Map<String, Value>, key: &str) {
    if let Some(value) = source.get(key) {
        target.insert(key.to_string(), value.clone());
    }
}

fn extract_chat_choice_text(value: &Value) -> Option<String> {
    let choices = value.get("choices")?.as_array()?;
    let first = choices.first()?.as_object()?;
    let message = first.get("message")?.as_object()?;
    message.get("content")?.as_str().map(|text| text.to_string())
}

fn extract_responses_output_text(value: &Value) -> Option<String> {
    let output = value.get("output")?.as_array()?;
    let mut combined = String::new();
    for item in output {
        let Some(item) = item.as_object() else {
            continue;
        };
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        if item.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in content {
            let Some(part) = part.as_object() else {
                continue;
            };
            if part.get("type").and_then(Value::as_str) != Some("output_text") {
                continue;
            }
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                combined.push_str(text);
            }
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

fn map_usage_responses_to_chat(usage: &Value) -> Option<Value> {
    let usage = usage.as_object()?;
    let input = usage.get("input_tokens").and_then(Value::as_u64);
    let output = usage.get("output_tokens").and_then(Value::as_u64);
    let total = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .or_else(|| match (input, output) {
            (Some(input), Some(output)) => input.checked_add(output),
            _ => None,
        });
    if input.is_none() && output.is_none() && total.is_none() {
        return None;
    }
    Some(json!({
        "prompt_tokens": input,
        "completion_tokens": output,
        "total_tokens": total
    }))
}

fn map_usage_chat_to_responses(usage: &Value) -> Option<Value> {
    let usage = usage.as_object()?;
    let prompt = usage.get("prompt_tokens").and_then(Value::as_u64);
    let completion = usage.get("completion_tokens").and_then(Value::as_u64);
    let total = usage.get("total_tokens").and_then(Value::as_u64).or_else(|| {
        match (prompt, completion) {
            (Some(prompt), Some(completion)) => prompt.checked_add(completion),
            _ => None,
        }
    });
    if prompt.is_none() && completion.is_none() && total.is_none() {
        return None;
    }
    Some(json!({
        "input_tokens": prompt,
        "output_tokens": completion,
        "total_tokens": total
    }))
}

#[cfg(test)]
mod tests;
