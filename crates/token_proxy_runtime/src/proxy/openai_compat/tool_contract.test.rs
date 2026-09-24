use super::*;

// 必须经过实际的多段转换，确认 strict 不会在中间的 Chat/Responses 结构中丢失。
#[test]
fn strict_tools_survive_responses_anthropic_gemini_routes() {
    let clients = ProxyHttpClients::new().expect("clients");
    let schema = json!({"type":"object", "additionalProperties":false,
        "properties":{"q":{"type":"string", "pattern":"^ok$", "minLength":2}}, "required":["q"]});
    let responses = json!({"model":"gemini-2.5-pro", "input":"lookup", "tools":[
        {"type":"function","name":"lookup","strict":true,"parameters":schema}
    ]});
    let anthropic = transform_request_value(
        FormatTransform::ResponsesToAnthropic,
        responses.clone(),
        &clients,
        None,
    );
    assert_eq!(anthropic["tools"][0]["strict"], true);
    for (format, input) in [
        (FormatTransform::ResponsesToGemini, responses),
        (FormatTransform::AnthropicToGemini, anthropic),
    ] {
        let gemini = transform_request_value(format, input, &clients, None);
        assert_eq!(
            gemini["toolConfig"]["functionCallingConfig"]["mode"],
            "VALIDATED"
        );
        assert_eq!(
            gemini["tools"][0]["functionDeclarations"][0]["parametersJsonSchema"],
            schema
        );
        let responses = transform_request_value(
            FormatTransform::GeminiToResponses,
            gemini.clone(),
            &clients,
            Some("unit-model"),
        );
        assert_eq!(responses["tools"][0]["strict"], true);
        assert_eq!(responses["tool_choice"], "auto");
        let anthropic = transform_request_value(
            FormatTransform::GeminiToAnthropic,
            gemini,
            &clients,
            Some("claude-sonnet-4-5"),
        );
        assert_eq!(anthropic["tools"][0]["strict"], true);
        assert_eq!(anthropic["tools"][0]["input_schema"], schema);
        assert_eq!(anthropic["tool_choice"]["type"], "auto");
    }
}

#[test]
fn tool_result_json_references_survive_responses_and_anthropic_to_gemini() {
    let clients = ProxyHttpClients::new().expect("clients");
    let result = json!({"schema":{"items":[{"$ref":"#/definitions/Thing"}]}});
    let request = json!({"model":"gemini-2.5-pro", "input":[
        {"type":"function_call","name":"lookup","call_id":"call_a","arguments":"{}"},
        {"type":"function_call_output","call_id":"call_a","output":result.to_string()}
    ]});
    let anthropic = transform_request_value(
        FormatTransform::ResponsesToAnthropic,
        request.clone(),
        &clients,
        None,
    );
    for (format, input) in [
        (FormatTransform::ResponsesToGemini, request),
        (FormatTransform::AnthropicToGemini, anthropic),
    ] {
        let value = transform_request_value(format, input, &clients, None);
        let response = value["contents"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|c| c["parts"].as_array().unwrap())
            .find_map(|p| p.get("functionResponse"))
            .expect("tool response");
        let text = response["response"]["result"]
            .as_str()
            .expect("opaque JSON text");
        assert_eq!(serde_json::from_str::<Value>(text).unwrap(), result);
    }
}
