use super::*;
use axum::body::Bytes;
use serde_json::{json, Value};

fn bytes_from_json(value: Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("serialize JSON"))
}

fn json_from_bytes(bytes: Bytes) -> Value {
    serde_json::from_slice(&bytes).expect("parse JSON")
}

#[test]
fn chat_request_to_responses_maps_common_fields() {
    let input_messages = json!([
        { "role": "user", "content": "hi" },
        { "role": "assistant", "content": "hello" }
    ]);
    let input = bytes_from_json(json!({
        "model": "gpt-4.1",
        "messages": input_messages,
        "stream": true,
        "temperature": 0.7,
        "top_p": 0.9,
        // Prefer `max_completion_tokens` over `max_tokens`.
        "max_tokens": 111,
        "max_completion_tokens": 222
    }));

    let output = transform_request_body(FormatTransform::ChatToResponses, &input).expect("transform");
    let value = json_from_bytes(output);

    assert_eq!(value["model"], json!("gpt-4.1"));
    assert_eq!(value["input"], input_messages);
    assert_eq!(value["stream"], json!(true));
    assert_eq!(value["temperature"], json!(0.7));
    assert_eq!(value["top_p"], json!(0.9));
    assert_eq!(value["max_output_tokens"], json!(222));
    assert!(value.get("messages").is_none());
}

#[test]
fn responses_request_to_chat_instructions_becomes_system_message() {
    let input = bytes_from_json(json!({
        "model": "gpt-4.1",
        "input": "hello",
        "instructions": "be concise",
        "stream": false,
        "max_output_tokens": 99
    }));

    let output = transform_request_body(FormatTransform::ResponsesToChat, &input).expect("transform");
    let value = json_from_bytes(output);
    let messages = value["messages"].as_array().expect("messages array");

    assert_eq!(value["model"], json!("gpt-4.1"));
    assert_eq!(value["stream"], json!(false));
    assert_eq!(value["max_completion_tokens"], json!(99));
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], json!("system"));
    assert_eq!(messages[0]["content"], json!("be concise"));
    assert_eq!(messages[1]["role"], json!("user"));
    assert_eq!(messages[1]["content"], json!("hello"));
}

#[test]
fn responses_request_to_chat_accepts_message_array_input() {
    let input_messages = json!([{ "role": "user", "content": "hi" }]);
    let input = bytes_from_json(json!({
        "model": "gpt-4.1",
        "input": input_messages,
        "stream": true
    }));

    let output = transform_request_body(FormatTransform::ResponsesToChat, &input).expect("transform");
    let value = json_from_bytes(output);

    assert_eq!(value["model"], json!("gpt-4.1"));
    assert_eq!(value["stream"], json!(true));
    assert_eq!(value["messages"], input_messages);
}

#[test]
fn responses_response_to_chat_extracts_output_text_and_maps_usage() {
    let input = bytes_from_json(json!({
        "id": "resp_123",
        "created_at": 1700000000,
        "model": "gpt-4.1",
        "output": [
            {
                "type": "message",
                "role": "assistant",
                "content": [
                    { "type": "output_text", "text": "Hello", "annotations": [] },
                    { "type": "output_text", "text": " world", "annotations": [] }
                ]
            }
        ],
        "usage": { "input_tokens": 1, "output_tokens": 2, "total_tokens": 3 }
    }));

    let output = transform_response_body(FormatTransform::ResponsesToChat, &input, None).expect("transform");
    let value = json_from_bytes(output);

    assert_eq!(value["id"], json!("resp_123"));
    assert_eq!(value["object"], json!("chat.completion"));
    assert_eq!(value["created"], json!(1700000000));
    assert_eq!(value["model"], json!("gpt-4.1"));
    assert_eq!(value["choices"][0]["message"]["role"], json!("assistant"));
    assert_eq!(value["choices"][0]["message"]["content"], json!("Hello world"));
    assert_eq!(value["usage"]["prompt_tokens"], json!(1));
    assert_eq!(value["usage"]["completion_tokens"], json!(2));
    assert_eq!(value["usage"]["total_tokens"], json!(3));
}

#[test]
fn chat_response_to_responses_extracts_choice_text_and_maps_usage() {
    let input = bytes_from_json(json!({
        "id": "chatcmpl_123",
        "created": 1700000000,
        "model": "gpt-4.1",
        "choices": [
            { "index": 0, "message": { "role": "assistant", "content": "Hello" } }
        ],
        "usage": { "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3 }
    }));

    let output = transform_response_body(FormatTransform::ChatToResponses, &input, None).expect("transform");
    let value = json_from_bytes(output);

    assert_eq!(value["id"], json!("chatcmpl_123"));
    assert_eq!(value["object"], json!("response"));
    assert_eq!(value["created_at"], json!(1700000000));
    assert_eq!(value["model"], json!("gpt-4.1"));
    assert_eq!(value["output"][0]["type"], json!("message"));
    assert_eq!(value["output"][0]["role"], json!("assistant"));
    assert_eq!(value["output"][0]["content"][0]["type"], json!("output_text"));
    assert_eq!(value["output"][0]["content"][0]["text"], json!("Hello"));
    assert_eq!(value["usage"]["input_tokens"], json!(1));
    assert_eq!(value["usage"]["output_tokens"], json!(2));
    assert_eq!(value["usage"]["total_tokens"], json!(3));
}

#[test]
fn chat_request_to_responses_rejects_missing_messages() {
    let input = bytes_from_json(json!({ "model": "gpt-4.1" }));
    let err = transform_request_body(FormatTransform::ChatToResponses, &input).expect_err("should fail");
    assert!(err.contains("messages"));
}

#[test]
fn transform_request_body_rejects_non_json() {
    let input = Bytes::from_static(b"not-json");
    let err = transform_request_body(FormatTransform::ChatToResponses, &input).expect_err("should fail");
    assert!(err.contains("JSON"));
}

