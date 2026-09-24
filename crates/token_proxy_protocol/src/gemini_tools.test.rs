use super::*;

// 严格模式仅影响 auto/缺省，不得覆盖明确的调用选择。
#[test]
fn strict_tools_preserve_explicit_tool_choice() {
    let tools = json!([{"type":"function", "function":{"name":"lookup", "strict":true}}]);
    for (choice, mode, name) in [
        (None, "VALIDATED", None),
        (Some(Value::Null), "VALIDATED", None),
        (Some(json!("auto")), "VALIDATED", None),
        (Some(json!("none")), "NONE", None),
        (Some(json!("required")), "ANY", None),
        (
            Some(json!({"type":"function","function":{"name":"lookup"}})),
            "ANY",
            Some("lookup"),
        ),
        (
            Some(json!({"type":"function","name":"lookup"})),
            "ANY",
            Some("lookup"),
        ),
    ] {
        let mapped = map_chat_tool_choice_to_gemini(choice.as_ref(), Some(&tools)).unwrap();
        assert_eq!(mapped["functionCallingConfig"]["mode"], mode);
        if let Some(name) = name {
            assert_eq!(
                mapped["functionCallingConfig"]["allowedFunctionNames"],
                json!([name])
            );
        }
    }
    let tools =
        json!([{"type":"function", "strict":true, "function":{"name":"lookup", "strict":false}}]);
    assert!(map_chat_tool_choice_to_gemini(None, Some(&tools)).is_none());
    assert_eq!(
        map_chat_tool_choice_to_gemini(Some(&json!("auto")), Some(&tools)).unwrap()
            ["functionCallingConfig"]["mode"],
        "AUTO"
    );
    assert!(map_chat_tool_choice_to_gemini(
        None,
        Some(&json!([{"type":"web_search", "strict":true}]))
    )
    .is_none());
}

#[test]
fn json_schema_keeps_constraints_and_instance_values() {
    let example = json!({"required":null,"minLength":4,"id":"data","$ref":"literal"});
    let tools = json!([{"type":"function", "name":"lookup", "strict":true, "parameters":{
        "type":"object", "additionalProperties":false,
        "properties":{
            "value":{"type":"string", "pattern":"^ok$", "minLength":2, "maxLength":5},
            "list":{"type":"array", "minItems":1, "maxItems":4, "uniqueItems":true, "items":{"type":"integer", "enum":[1,2]}},
            "dict":{"type":"object", "additionalProperties":{"type":"string", "minLength":1}},
            "nested":{"type":"object", "properties":{"x":{"type":"string"}}, "required":["x"]},
            "free":{"type":"object", "additionalProperties":true, "default":example, "examples":[example]}
        }, "required":["value","nested"]
    }}]);
    let mapped = map_chat_tools_to_gemini(&tools);
    let declaration = &mapped[0]["functionDeclarations"][0];
    assert!(declaration.get("parameters").is_none());
    assert!(declaration.get("strict").is_none());
    assert_eq!(declaration["parametersJsonSchema"], tools[0]["parameters"]);
}

#[test]
fn validated_mode_round_trips_strictness_and_allowed_names() {
    let tools = json!([{"functionDeclarations":[
        {"name":"yes", "parametersJsonSchema":{"type":"object","additionalProperties":false}},
        {"name":"no", "parametersJsonSchema":{"type":"object"}}
    ]}]);
    let config =
        json!({"functionCallingConfig":{"mode":"VALIDATED","allowedFunctionNames":["yes"]}});
    let mapped = map_gemini_tools_to_chat(&tools, Some(&config));
    assert_eq!(mapped.as_array().unwrap().len(), 1);
    assert_eq!(mapped[0]["function"]["name"], "yes");
    assert_eq!(mapped[0]["function"]["strict"], true);
    assert_eq!(
        mapped[0]["function"]["parameters"]["additionalProperties"],
        false
    );
    let choice = map_gemini_tool_config_to_chat(&config).unwrap();
    assert_eq!(choice, "auto");
    assert_eq!(
        map_chat_tool_choice_to_gemini(Some(&choice), Some(&mapped)).unwrap()
            ["functionCallingConfig"]["mode"],
        "VALIDATED"
    );
}
