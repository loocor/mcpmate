use std::sync::Arc;

use json5;
use serde_json::{Map, Value};
use serde_yaml;
use toml;

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientTemplate, ContainerType, MergeStrategy, TemplateFormat};
use crate::clients::utils::{get_nested_value, set_nested_value};

/// Configuration difference information, used for dry-run display
#[derive(Debug, Clone, Default)]
pub struct ConfigDiff {
    pub format: TemplateFormat,
    pub before: Option<String>,
    pub after: Option<String>,
    pub summary: Option<String>,
}

/// Template renderer abstract, different output formats correspond to different implementations
pub trait ConfigRenderer: Send + Sync {
    fn format(&self) -> TemplateFormat;
    fn merge(
        &self,
        base: &str,
        patch: &Value,
        template: &ClientTemplate,
    ) -> ConfigResult<String>;

    fn diff(
        &self,
        before: &str,
        after: &str,
    ) -> ConfigResult<ConfigDiff>;
}

pub type DynConfigRenderer = Arc<dyn ConfigRenderer>;

pub struct StructuredRenderer {
    format: TemplateFormat,
}

impl StructuredRenderer {
    pub fn new(format: TemplateFormat) -> DynConfigRenderer {
        Arc::new(Self { format })
    }

    fn parse(
        &self,
        content: &str,
    ) -> ConfigResult<Value> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(Value::Null);
        }

        let parsed = match self.format {
            TemplateFormat::Json => serde_json::from_str(trimmed).map_err(ConfigError::from)?,
            TemplateFormat::Json5 => {
                json5::from_str(trimmed).map_err(|err| ConfigError::TemplateParseError(err.to_string()))?
            }
            TemplateFormat::Yaml => serde_yaml::from_str(trimmed).map_err(ConfigError::from)?,
            TemplateFormat::Toml => toml::from_str(trimmed).map_err(ConfigError::from)?,
        };

        Ok(parsed)
    }

    fn serialize(
        &self,
        value: &Value,
    ) -> ConfigResult<String> {
        let rendered = match self.format {
            TemplateFormat::Json => {
                let pretty = serde_json::to_string_pretty(value).map_err(ConfigError::from)?;
                pretty.replace("\\/", "/")
            }
            TemplateFormat::Json5 => {
                json5::to_string(value).map_err(|err| ConfigError::TemplateParseError(err.to_string()))?
            }
            TemplateFormat::Yaml => serde_yaml::to_string(value).map_err(ConfigError::from)?,
            TemplateFormat::Toml => {
                toml::to_string(value).map_err(|err| ConfigError::TomlSerializeError(err.to_string()))?
            }
        };

        Ok(rendered)
    }

    fn merge_values(
        &self,
        base: Value,
        patch: &Value,
        template: &ClientTemplate,
    ) -> Value {
        match template.config_mapping.container_type {
            ContainerType::Array => self.merge_array_container(base, patch, template.config_mapping.merge_strategy),
            ContainerType::ObjectMap | ContainerType::Mixed => self.merge_object_container(base, patch, template),
        }
    }

    fn merge_object_container(
        &self,
        base: Value,
        patch: &Value,
        template: &ClientTemplate,
    ) -> Value {
        let container_key = template.config_mapping.container_key.as_str();
        if container_key.is_empty() {
            return patch.clone();
        }

        let mut root = match base {
            Value::Object(map) => Value::Object(map),
            Value::Null => Value::Object(Map::new()),
            _ => Value::Object(Map::new()),
        };

        let existing = get_nested_value(&root, container_key).cloned();
        let fragment = match template.config_mapping.merge_strategy {
            MergeStrategy::Replace => patch.clone(),
            MergeStrategy::DeepMerge => {
                let base_fragment = existing.unwrap_or_else(|| Value::Object(Map::new()));
                deep_merge(base_fragment, patch)
            }
        };
        set_nested_value(&mut root, container_key, fragment);
        root
    }

    fn merge_array_container(
        &self,
        base: Value,
        patch: &Value,
        strategy: MergeStrategy,
    ) -> Value {
        let incoming = patch.as_array().cloned().unwrap_or_default();
        match strategy {
            MergeStrategy::Replace => Value::Array(incoming),
            MergeStrategy::DeepMerge => merge_array_by_name(base, incoming),
        }
    }
}

impl ConfigRenderer for StructuredRenderer {
    fn format(&self) -> TemplateFormat {
        self.format
    }

    fn merge(
        &self,
        base: &str,
        patch: &Value,
        template: &ClientTemplate,
    ) -> ConfigResult<String> {
        let base_value = self.parse(base)?;
        let merged = self.merge_values(base_value, patch, template);
        self.serialize(&merged)
    }

    fn diff(
        &self,
        before: &str,
        after: &str,
    ) -> ConfigResult<ConfigDiff> {
        let summary = if before == after {
            Some("Configuration has no changes".to_string())
        } else {
            None
        };

        Ok(ConfigDiff {
            format: self.format,
            before: if before.is_empty() {
                None
            } else {
                Some(before.to_string())
            },
            after: if after.is_empty() {
                None
            } else {
                Some(after.to_string())
            },
            summary,
        })
    }
}

fn deep_merge(
    base: Value,
    patch: &Value,
) -> Value {
    match (base, patch) {
        (Value::Object(mut base_map), Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                let existing = base_map.remove(key).unwrap_or(Value::Null);
                base_map.insert(key.clone(), deep_merge(existing, value));
            }
            Value::Object(base_map)
        }
        _ => patch.clone(),
    }
}

fn merge_array_by_name(
    existing: Value,
    patch_items: Vec<Value>,
) -> Value {
    let mut base_items = match existing {
        Value::Array(items) => items,
        _ => Vec::new(),
    };

    for item in patch_items {
        let potential_name = item.get("name").and_then(|value| value.as_str()).map(|s| s.to_string());

        if let Some(ref name) = potential_name {
            if let Some(existing_item) = base_items.iter_mut().find(|entry| {
                entry
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(|current| current == name)
                    .unwrap_or(false)
            }) {
                let merged = deep_merge(existing_item.clone(), &item);
                *existing_item = merged;
                continue;
            }
        }

        base_items.push(item);
    }

    Value::Array(base_items)
}
