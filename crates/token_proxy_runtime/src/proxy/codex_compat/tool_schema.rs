//! 只遍历 JSON Schema 节点；default/enum 等用户数据不能作为 Schema 清理。
use serde_json::{json, Map, Value};

const SCHEMA_MAPS: &[&str] = &[
    "properties",
    "$defs",
    "definitions",
    "patternProperties",
    "dependentSchemas",
    "dependencies",
];
const SCHEMA_VALUES: &[&str] = &[
    "items",
    "prefixItems",
    "contains",
    "additionalProperties",
    "propertyNames",
    "unevaluatedProperties",
    "unevaluatedItems",
    "additionalItems",
    "contentSchema",
    "anyOf",
    "oneOf",
    "allOf",
    "not",
    "if",
    "then",
    "else",
];

pub(super) fn normalize(object: &mut Map<String, Value>) {
    let mut changed = 0;
    if let Some(tools) = object.get_mut("tools") {
        normalize_tools(tools, &mut changed);
    }
    if let Some(input) = object.get_mut("input").and_then(Value::as_array_mut) {
        for item in input {
            if let Some(tools) = item.get_mut("tools") {
                normalize_tools(tools, &mut changed);
            }
        }
    }
    if changed > 0 {
        tracing::debug!(changed, "normalized unsupported Codex tool schema fields");
    }
}

fn normalize_tools(tools: &mut Value, changed: &mut usize) {
    let Some(tools) = tools.as_array_mut() else {
        return;
    };
    for tool in tools {
        let Some(tool) = tool.as_object_mut() else {
            continue;
        };
        if tool.get("type").and_then(Value::as_str) == Some("function") {
            let function = if tool.get("function").is_some_and(Value::is_object) {
                tool.get_mut("function")
                    .and_then(Value::as_object_mut)
                    .expect("checked object")
            } else {
                &mut *tool
            };
            let schema = function.entry("parameters").or_insert_with(|| {
                *changed += 1;
                json!({"type":"object","properties":{}})
            });
            if schema.is_null() {
                *schema = json!({"type":"object","properties":{}});
                *changed += 1;
            }
            if let Some(root) = schema.as_object_mut() {
                if root.get("type").is_some_and(Value::is_null) {
                    root.insert("type".to_string(), json!("object"));
                    *changed += 1;
                }
            }
            clean_schema(schema, changed);
        }
        if let Some(nested) = tool.get_mut("tools") {
            normalize_tools(nested, changed);
        }
    }
}

fn clean_schema(schema: &mut Value, changed: &mut usize) {
    let Some(schema) = schema.as_object_mut() else {
        return;
    };
    for key in ["$schema", "$id"] {
        *changed += usize::from(schema.remove(key).is_some());
    }
    if schema
        .get("pattern")
        .and_then(Value::as_str)
        .is_some_and(unsupported_pattern)
    {
        schema.remove("pattern");
        *changed += 1;
    }
    for key in SCHEMA_MAPS {
        if let Some(children) = schema.get_mut(*key).and_then(Value::as_object_mut) {
            if *key == "patternProperties" {
                children.retain(|pattern, _| {
                    let keep = !unsupported_pattern(pattern);
                    *changed += usize::from(!keep);
                    keep
                });
            }
            for child in children.values_mut() {
                clean_schema(child, changed);
            }
        }
    }
    for key in SCHEMA_VALUES {
        if let Some(child) = schema.get_mut(*key) {
            if let Some(children) = child.as_array_mut() {
                for child in children {
                    clean_schema(child, changed);
                }
            } else {
                clean_schema(child, changed);
            }
        }
    }
}

fn unsupported_pattern(pattern: &str) -> bool {
    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\'
            && matches!(chars.next(), Some('p' | 'P'))
            && chars.clone().next() == Some('{')
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_schema_nodes_without_touching_user_values_or_large_numbers() {
        let value: Value = serde_json::from_str(r#"{"tools":[{"type":"function","name":"f","parameters":{"type":null,"$schema":"dialect","properties":{"$id":{"type":"string","pattern":"\\p{L}"},"x":{"$id":"dialect","default":{"$schema":"keep","pattern":"\\p{L}","n":9007199254740993}}},"patternProperties":{"\\P{N}":{},"literal\\\\p{L}":{}},"allOf":[{"$id":"nested"}]}}],"input":[{"type":"additional_tools","tools":[{"type":"function","name":"g"}]}]}"#).unwrap();
        let mut object = value.as_object().unwrap().clone();
        normalize(&mut object);
        let p = &object["tools"][0]["parameters"];
        assert_eq!(p["type"], "object");
        assert!(p.get("$schema").is_none());
        assert!(p["properties"]["$id"].get("pattern").is_none());
        assert!(p["properties"]["x"].get("$id").is_none());
        assert_eq!(
            p["properties"]["x"]["default"]["n"].to_string(),
            "9007199254740993"
        );
        assert_eq!(p["properties"]["x"]["default"]["$schema"], "keep");
        assert_eq!(p["patternProperties"].as_object().unwrap().len(), 1);
        assert!(p["allOf"][0].get("$id").is_none());
        assert_eq!(
            object["input"][0]["tools"][0]["parameters"],
            json!({"type":"object","properties":{}})
        );
    }
}
