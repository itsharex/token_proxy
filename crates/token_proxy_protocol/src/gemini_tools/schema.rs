//! parametersJsonSchema 清洗：保留标准约束与实例数据，仅沿 Schema 节点修复输入。

use serde_json::{json, Map, Value};

const GEMINI_UNSUPPORTED_SCHEMA_KEYS: &[&str] = &[
    "$schema",
    "$id",
    "id",
    "$anchor",
    "$vocabulary",
    "$dynamicRef",
    "$dynamicAnchor",
    "$ref",
    "$defs",
    "definitions",
    "additionalItems",
    "unevaluatedProperties",
    "unevaluatedItems",
    "contentSchema",
    "patternProperties",
    "if",
    "then",
    "else",
    "deprecated",
];

const SCHEMA_CONTAINER_KEYS: &[&str] = &[
    "type",
    "properties",
    "required",
    "items",
    "prefixItems",
    "anyOf",
    "oneOf",
    "allOf",
    "$defs",
    "definitions",
    "description",
    "title",
    "enum",
    "format",
    "default",
    "const",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "minProperties",
    "maxProperties",
    "uniqueItems",
    "multipleOf",
    "examples",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "pattern",
    "additionalProperties",
    "additionalItems",
    "contains",
    "not",
    "if",
    "then",
    "else",
];

pub(super) fn clean_tool_schema(schema: &Value) -> Value {
    match schema {
        Value::Object(object) => {
            clean_tool_schema_object(&normalize_malformed_schema_object(object))
        }
        Value::Array(items) => Value::Array(items.iter().map(clean_tool_schema).collect()),
        Value::Bool(true) => json!({}),
        other => other.clone(),
    }
}

fn clean_tool_schema_object(object: &Map<String, Value>) -> Value {
    let mut source = object.clone();
    normalize_prefix_items(&mut source);
    merge_conditional_properties(&mut source, object.get("then"));
    merge_conditional_properties(&mut source, object.get("else"));
    let all_of = source.get("allOf").cloned();
    let any_of = source.get("anyOf").cloned();
    let one_of = source.get("oneOf").cloned();
    let mut cleaned = Map::new();
    for (key, value) in &source {
        if GEMINI_UNSUPPORTED_SCHEMA_KEYS.contains(&key.as_str())
            || key == "allOf"
            || (key == "required" && value.is_null())
        {
            continue;
        }
        if key == "additionalProperties" && value.is_boolean() {
            cleaned.insert(key.clone(), value.clone());
        } else if key == "properties" {
            let properties = value
                .as_object()
                .map(|properties| {
                    properties
                        .iter()
                        .map(|(name, schema)| (name.clone(), clean_tool_schema(schema)))
                        .collect::<Map<String, Value>>()
                })
                .unwrap_or_default();
            cleaned.insert(key.clone(), Value::Object(properties));
        } else if matches!(key.as_str(), "$defs" | "definitions") {
            let definitions = value
                .as_object()
                .map(|definitions| {
                    definitions
                        .iter()
                        .map(|(name, schema)| (name.clone(), clean_tool_schema(schema)))
                        .collect::<Map<String, Value>>()
                })
                .unwrap_or_default();
            cleaned.insert(key.clone(), Value::Object(definitions));
        } else if matches!(
            key.as_str(),
            "items"
                | "additionalProperties"
                | "contains"
                | "prefixItems"
                | "anyOf"
                | "oneOf"
                | "not"
                | "additionalItems"
                | "propertyNames"
                | "unevaluatedItems"
                | "unevaluatedProperties"
                | "contentSchema"
        ) {
            cleaned.insert(key.clone(), clean_tool_schema(value));
        } else {
            // default/enum/const 是用户数据，不递归执行 Schema 关键字删除。
            cleaned.insert(key.clone(), value.clone());
        }
    }
    let removed = source
        .keys()
        .filter(|key| GEMINI_UNSUPPORTED_SCHEMA_KEYS.contains(&key.as_str()))
        .count();
    if removed > 0 {
        tracing::debug!(removed, "removed unsupported Gemini schema keywords");
    }
    normalize_gemini_schema_type(&mut cleaned);
    merge_all_of_properties(&mut cleaned, all_of.as_ref());
    merge_union_properties(&mut cleaned, "anyOf", any_of.as_ref());
    merge_union_properties(&mut cleaned, "oneOf", one_of.as_ref());
    if cleaned.get("items").is_some() {
        let keep_items = cleaned
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|schema_type| schema_type == "array");
        if !keep_items {
            cleaned.remove("items");
        }
    }
    if cleaned.get("type").and_then(Value::as_str) == Some("array") {
        let invalid_items = cleaned.get("items").is_none_or(|items| !items.is_object());
        if invalid_items {
            cleaned.insert("items".to_string(), json!({ "type": "string" }));
        }
    }
    if !cleaned.contains_key("properties") {
        cleaned.remove("required");
    }
    Value::Object(cleaned)
}

fn normalize_prefix_items(source: &mut Map<String, Value>) {
    let Some(prefix_items) = source.remove("prefixItems") else {
        return;
    };
    let needs_items = source
        .get("items")
        .is_none_or(|items| matches!(items, Value::Array(_) | Value::Bool(true)));
    if !needs_items {
        return;
    }
    let replacement = match prefix_items {
        Value::Array(mut items) => items.drain(..).next().unwrap_or_else(|| json!({})),
        Value::Bool(true) => json!({}),
        _ => json!({}),
    };
    source.insert("items".to_string(), replacement);
}

fn append_schema_description(cleaned: &mut Map<String, Value>, hint: &str) {
    let description = cleaned
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let combined = if description.is_empty() {
        hint.to_string()
    } else {
        format!("{description}; {hint}")
    };
    cleaned.insert("description".to_string(), Value::String(combined));
}

fn merge_union_properties(cleaned: &mut Map<String, Value>, key: &str, union: Option<&Value>) {
    let Some(union) = union.and_then(Value::as_array) else {
        return;
    };
    let branches = union.iter().map(clean_tool_schema).collect::<Vec<_>>();
    let mut branch_properties = Map::new();
    let mut accepted_types = Vec::new();
    for branch in &branches {
        if let Some(schema_type) = branch.get("type").and_then(Value::as_str) {
            if !accepted_types.iter().any(|value| value == schema_type) {
                accepted_types.push(schema_type.to_string());
            }
        }
        if let Some(properties) = branch.get("properties").and_then(Value::as_object) {
            for (name, schema) in properties {
                branch_properties
                    .entry(name.clone())
                    .or_insert_with(|| schema.clone());
            }
        }
    }

    if !branch_properties.is_empty() {
        let properties = cleaned
            .entry("properties".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(properties) = properties.as_object_mut() {
            for (name, schema) in branch_properties {
                properties.entry(name).or_insert(schema);
            }
            cleaned
                .entry("type".to_string())
                .or_insert_with(|| Value::String("object".to_string()));
        }
    } else if let Some(selected) = branches.first().and_then(Value::as_object) {
        for (name, value) in selected {
            if name != "description" {
                cleaned.entry(name.clone()).or_insert_with(|| value.clone());
            }
        }
    }

    cleaned.remove(key);
    if accepted_types.len() > 1 {
        append_schema_description(cleaned, &format!("Accepts: {}", accepted_types.join(" | ")));
    }
}

fn normalize_malformed_schema_object(object: &Map<String, Value>) -> Map<String, Value> {
    let mut normalized = object.clone();
    let is_bare_property_map = !object.is_empty()
        && !object.contains_key("type")
        && !object.keys().any(|key| {
            SCHEMA_CONTAINER_KEYS.contains(&key.as_str())
                || GEMINI_UNSUPPORTED_SCHEMA_KEYS.contains(&key.as_str())
        })
        && object.values().all(Value::is_object);
    if is_bare_property_map {
        let (properties, required) = normalize_properties(object);
        normalized.clear();
        normalized.insert("type".to_string(), Value::String("object".to_string()));
        normalized.insert("properties".to_string(), Value::Object(properties));
        if !required.is_empty() {
            normalized.insert("required".to_string(), json!(required));
        }
        return normalized;
    }

    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        let (properties, promoted) = normalize_properties(properties);
        normalized.insert("properties".to_string(), Value::Object(properties));
        if !promoted.is_empty() {
            let mut required = object
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            for name in promoted {
                if !required.contains(&name) {
                    required.push(name);
                }
            }
            normalized.insert("required".to_string(), json!(required));
        }
    }
    normalized
}

fn normalize_properties(properties: &Map<String, Value>) -> (Map<String, Value>, Vec<String>) {
    let mut normalized = Map::new();
    let mut required = Vec::new();
    for (name, value) in properties {
        let Some(object) = value.as_object() else {
            normalized.insert(name.clone(), value.clone());
            continue;
        };
        let mut child = object.clone();
        // 布尔 required 是畸形字段，提升到父对象；合法嵌套 required 数组必须原样保留。
        if let Some(is_required) = child.get("required").and_then(Value::as_bool) {
            child.remove("required");
            if is_required {
                required.push(name.clone());
            }
        }
        normalized.insert(name.clone(), Value::Object(child));
    }
    (normalized, required)
}

fn merge_conditional_properties(target: &mut Map<String, Value>, branch: Option<&Value>) {
    let Some(branch_properties) = branch
        .and_then(|value| value.get("properties"))
        .and_then(Value::as_object)
    else {
        return;
    };
    let properties = target
        .entry("properties".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(properties) = properties.as_object_mut() else {
        return;
    };
    for (name, schema) in branch_properties {
        properties
            .entry(name.clone())
            .or_insert_with(|| schema.clone());
    }
}

fn merge_all_of_properties(cleaned: &mut Map<String, Value>, all_of: Option<&Value>) {
    let Some(branches) = all_of.and_then(Value::as_array) else {
        return;
    };
    for branch in branches {
        let branch = clean_tool_schema(branch);
        let Some(branch_properties) = branch.get("properties").and_then(Value::as_object) else {
            continue;
        };
        let properties = cleaned
            .entry("properties".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let Some(properties) = properties.as_object_mut() else {
            return;
        };
        for (name, schema) in branch_properties {
            properties
                .entry(name.clone())
                .or_insert_with(|| schema.clone());
        }
    }
}

fn normalize_gemini_schema_type(object: &mut Map<String, Value>) {
    match object.get("type") {
        Some(Value::String(schema_type)) => {
            object.insert(
                "type".to_string(),
                Value::String(schema_type.to_ascii_lowercase()),
            );
        }
        Some(Value::Array(schema_types)) => {
            let normalized = schema_types
                .iter()
                .filter_map(Value::as_str)
                .filter(|schema_type| !schema_type.eq_ignore_ascii_case("null"))
                .find(|schema_type| {
                    object.contains_key("items") && schema_type.eq_ignore_ascii_case("array")
                })
                .or_else(|| {
                    schema_types
                        .iter()
                        .filter_map(Value::as_str)
                        .find(|schema_type| !schema_type.eq_ignore_ascii_case("null"))
                })
                .map(str::to_ascii_lowercase);
            match normalized {
                Some(schema_type) => {
                    object.insert("type".to_string(), Value::String(schema_type));
                }
                None => {
                    object.remove("type");
                }
            }
        }
        Some(Value::Null) if object.contains_key("items") => {
            object.insert("type".to_string(), Value::String("array".to_string()));
        }
        Some(Value::Null) => {
            object.remove("type");
        }
        None if object.contains_key("items") => {
            object.insert("type".to_string(), Value::String("array".to_string()));
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "schema.test.rs"]
mod tests;
