use axum::body::Bytes;
use serde_json::Value;

/// OpenAI Responses reasoning models reject Chat-style sampling parameters such as temperature/top_p.
pub(crate) fn is_openai_responses_reasoning_model(model: &str) -> bool {
    let model = model
        .trim()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    model.starts_with("gpt-5") || is_gpt6_sol_or_luna_model(&model)
}

pub(crate) fn is_gpt6_sol_or_luna_model(model: &str) -> bool {
    let model = model
        .trim()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    ["gpt-6-sol", "gpt-6-luna"].iter().any(|base| {
        model == *base
            || model
                .strip_prefix(base)
                .is_some_and(|suffix| suffix.starts_with('-'))
    })
}

pub(crate) fn is_claude_opus55_model(model: &str) -> bool {
    let model = model
        .trim()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    model == "claude-opus-5-5" || model == "claude-opus-5.5"
}

pub(crate) fn is_claude_model(model: &str) -> bool {
    let model = model
        .trim()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    model.starts_with("claude-") || model.starts_with("claude_")
}

pub(crate) fn rewrite_response_model(bytes: &Bytes, model: &str) -> Option<Bytes> {
    let mut value: Value = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object_mut()?;
    if object.contains_key("model") {
        object.insert("model".to_string(), Value::String(model.to_string()));
        return serde_json::to_vec(&value).ok().map(Bytes::from);
    }
    let Some(response) = object.get_mut("response").and_then(Value::as_object_mut) else {
        return None;
    };
    if !response.contains_key("model") {
        return None;
    }
    response.insert("model".to_string(), Value::String(model.to_string()));
    serde_json::to_vec(&value).ok().map(Bytes::from)
}
