use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(crate) struct ModelMappingRules {
    exact: HashMap<String, String>,
    prefix: Vec<PrefixRule>,
    wildcard: Option<String>,
}

#[derive(Clone, Debug)]
struct PrefixRule {
    prefix: String,
    target: String,
}

impl ModelMappingRules {
    pub(crate) fn map_model(&self, model: &str) -> Option<&str> {
        if let Some(target) = self.exact.get(model) {
            return Some(target.as_str());
        }
        for rule in &self.prefix {
            if model.starts_with(&rule.prefix) {
                return Some(rule.target.as_str());
            }
        }
        self.wildcard.as_deref()
    }
}

pub(crate) fn compile_model_mappings(
    upstream_id: &str,
    mappings: &HashMap<String, String>,
) -> Result<Option<ModelMappingRules>, String> {
    if mappings.is_empty() {
        return Ok(None);
    }
    let mut builder = ModelMappingBuilder::new(upstream_id);
    for (pattern, target) in mappings {
        builder.push(pattern, target)?;
    }
    Ok(Some(builder.finish()))
}

struct ModelMappingBuilder<'a> {
    upstream_id: &'a str,
    exact: HashMap<String, String>,
    prefix: Vec<PrefixRule>,
    wildcard: Option<String>,
    seen_patterns: HashSet<String>,
}

impl<'a> ModelMappingBuilder<'a> {
    fn new(upstream_id: &'a str) -> Self {
        Self {
            upstream_id,
            exact: HashMap::new(),
            prefix: Vec::new(),
            wildcard: None,
            seen_patterns: HashSet::new(),
        }
    }

    fn push(&mut self, pattern_raw: &str, target_raw: &str) -> Result<(), String> {
        let pattern = pattern_raw.trim();
        let target = target_raw.trim();
        if pattern.is_empty() {
            return Err(self.error("model mapping pattern cannot be empty"));
        }
        if target.is_empty() {
            return Err(format!(
                "Upstream {} model mapping target for \"{}\" cannot be empty.",
                self.upstream_id, pattern
            ));
        }
        if pattern == "*" {
            if self.wildcard.is_some() {
                return Err(format!(
                    "Upstream {} model mapping wildcard \"*\" can only be defined once.",
                    self.upstream_id
                ));
            }
            self.wildcard = Some(target.to_string());
            return Ok(());
        }
        if !self.seen_patterns.insert(pattern.to_string()) {
            return Err(format!(
                "Upstream {} model mapping pattern is duplicated: {}.",
                self.upstream_id, pattern
            ));
        }
        if pattern.ends_with('*') {
            // 前缀模式：只允许尾部通配，且前缀不能为空。
            let prefix_value = pattern.trim_end_matches('*');
            if prefix_value.is_empty() {
                return Err(self.error("model mapping prefix cannot be empty"));
            }
            if prefix_value.contains('*') {
                return Err(self.invalid_pattern(pattern));
            }
            self.prefix.push(PrefixRule {
                prefix: prefix_value.to_string(),
                target: target.to_string(),
            });
            return Ok(());
        }
        if pattern.contains('*') {
            return Err(self.invalid_pattern(pattern));
        }
        self.exact.insert(pattern.to_string(), target.to_string());
        Ok(())
    }

    fn finish(mut self) -> ModelMappingRules {
        self.prefix.sort_by(|left, right| {
            right
                .prefix
                .len()
                .cmp(&left.prefix.len())
                .then_with(|| left.prefix.cmp(&right.prefix))
        });
        ModelMappingRules {
            exact: self.exact,
            prefix: self.prefix,
            wildcard: self.wildcard,
        }
    }

    fn error(&self, message: &str) -> String {
        format!("Upstream {} {}.", self.upstream_id, message)
    }

    fn invalid_pattern(&self, pattern: &str) -> String {
        format!(
            "Upstream {} model mapping pattern is invalid: {}.",
            self.upstream_id, pattern
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_mapping_prefers_exact_then_prefix_then_wildcard() {
        let mut mappings = HashMap::new();
        mappings.insert("gpt-4".to_string(), "gpt-4.1".to_string());
        mappings.insert("gpt-4*".to_string(), "gpt-4.1-mini".to_string());
        mappings.insert("*".to_string(), "gpt-default".to_string());
        let rules = compile_model_mappings("demo", &mappings)
            .expect("compile")
            .expect("rules");

        assert_eq!(rules.map_model("gpt-4"), Some("gpt-4.1"));
        assert_eq!(rules.map_model("gpt-4-vision"), Some("gpt-4.1-mini"));
        assert_eq!(rules.map_model("other"), Some("gpt-default"));
    }

    #[test]
    fn model_mapping_prefix_prefers_longer_prefix() {
        let mut mappings = HashMap::new();
        mappings.insert("gpt-4*".to_string(), "wide".to_string());
        mappings.insert("gpt-4.1*".to_string(), "narrow".to_string());
        let rules = compile_model_mappings("demo", &mappings)
            .expect("compile")
            .expect("rules");
        assert_eq!(rules.map_model("gpt-4.1-mini"), Some("narrow"));
    }

    #[test]
    fn model_mapping_rejects_multiple_wildcards() {
        let mut mappings = HashMap::new();
        mappings.insert("*".to_string(), "a".to_string());
        mappings.insert(" * ".to_string(), "b".to_string());
        let err = compile_model_mappings("demo", &mappings).unwrap_err();
        assert!(err.contains("wildcard"));
    }
}
