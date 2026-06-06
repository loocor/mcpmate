//! Value-level mutations for user-managed client configuration files.
//!
//! Parse and serialize policy lives in `document.rs`; this module owns merge (attach/apply)
//! and remove-managed-entries (detach) semantics on parsed `Value` trees.

use crate::clients::document::{ClientConfigDocument, parse_config_for_merge};
use crate::clients::error::ConfigResult;
use crate::clients::models::{
    ClientConfigFileParse, ClientRenderDefinition, ContainerType, MergeStrategy, TemplateFormat,
};
use crate::clients::utils::{get_nested_value, get_nested_value_mut, set_nested_value};
use crate::common::constants::{client_headers, profile_keys};
use serde_json::{Map, Value};

/// Configuration difference information, used for dry-run display.
#[derive(Debug, Clone, Default)]
pub struct ConfigDiff {
    pub format: TemplateFormat,
    pub before: Option<String>,
    pub after: Option<String>,
    pub summary: Option<String>,
}

/// Parse existing on-disk content and merge a rendered fragment into a document.
pub(crate) fn merge_config_document(
    existing: &str,
    patch: &Value,
    definition: &ClientRenderDefinition,
) -> ConfigResult<ClientConfigDocument> {
    let base_value = parse_config_for_merge(existing, definition.format)?;
    let merged = merge_container(base_value, patch, definition);
    Ok(ClientConfigDocument {
        format: definition.format,
        value: merged,
    })
}

/// Merge a rendered fragment into a parsed configuration tree.
pub(crate) fn merge_container(
    base: Value,
    patch: &Value,
    definition: &ClientRenderDefinition,
) -> Value {
    let chosen_path = choose_container_path(&base, definition);
    match definition.config_mapping.container_type {
        ContainerType::Array => {
            merge_array_at_path(base, patch, &chosen_path, definition.config_mapping.merge_strategy)
        }
        ContainerType::ObjectMap => merge_object_at_path(base, patch, &chosen_path, definition),
    }
}

/// Remove MCPMate-managed entries from a parsed configuration document.
pub(crate) fn remove_managed_entries(
    mut document: ClientConfigDocument,
    rule: &ClientConfigFileParse,
) -> (ClientConfigDocument, bool) {
    let is_array = matches!(rule.container_type, ContainerType::Array);
    let (value, changed) = filter_managed_entries(document.value, &rule.container_keys, is_array);
    document.value = value;
    (document, changed)
}

/// Build a dry-run diff between existing content and a serialized mutation result.
pub(crate) fn config_content_diff(
    before: &str,
    after: &str,
    format: TemplateFormat,
) -> ConfigResult<ConfigDiff> {
    let unchanged = before == after || content_semantically_equal(before, after, format)?;
    let summary = if unchanged {
        Some("Configuration has no changes".to_string())
    } else {
        None
    };

    Ok(ConfigDiff {
        format,
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

fn content_semantically_equal(
    before: &str,
    after: &str,
    format: TemplateFormat,
) -> ConfigResult<bool> {
    let before_value = parse_config_for_merge(before, format)?;
    let after_value = parse_config_for_merge(after, format)?;
    Ok(before_value == after_value)
}

fn choose_container_path(
    base: &Value,
    definition: &ClientRenderDefinition,
) -> String {
    let type_is_ok = |value: &Value| match definition.config_mapping.container_type {
        ContainerType::ObjectMap => value.is_object(),
        ContainerType::Array => value.is_array(),
    };

    for key in &definition.config_mapping.container_keys {
        if let Some(value) = get_nested_value(base, key) {
            if type_is_ok(value) {
                return key.clone();
            }
        }
    }

    definition
        .config_mapping
        .container_keys
        .first()
        .cloned()
        .unwrap_or_default()
}

fn merge_object_at_path(
    base: Value,
    patch: &Value,
    path: &str,
    definition: &ClientRenderDefinition,
) -> Value {
    let mut root = match base {
        Value::Object(map) => Value::Object(map),
        _ => Value::Object(Map::new()),
    };

    let existing = get_nested_value(&root, path).cloned();
    let fragment = match definition.config_mapping.merge_strategy {
        MergeStrategy::Replace => patch.clone(),
        MergeStrategy::DeepMerge => {
            let base_fragment = existing.unwrap_or_else(|| Value::Object(Map::new()));
            deep_merge(base_fragment, patch)
        }
    };
    set_nested_value(&mut root, path, fragment);
    root
}

fn merge_array_at_path(
    base: Value,
    patch: &Value,
    path: &str,
    strategy: MergeStrategy,
) -> Value {
    let mut root = match base {
        Value::Object(map) => Value::Object(map),
        _ => Value::Object(Map::new()),
    };

    let existing = get_nested_value(&root, path).cloned().unwrap_or(Value::Array(vec![]));
    let incoming = patch.as_array().cloned().unwrap_or_default();
    let merged = match strategy {
        MergeStrategy::Replace => Value::Array(incoming),
        MergeStrategy::DeepMerge => merge_array_by_name(existing, incoming),
    };
    set_nested_value(&mut root, path, merged);
    root
}

fn filter_managed_entries(
    mut value: Value,
    container_keys: &[String],
    is_array: bool,
) -> (Value, bool) {
    let mut changed = false;
    for key in container_keys {
        if let Some(container) = get_nested_value_mut(&mut value, key) {
            if is_array {
                if let Some(entries) = container.as_array_mut() {
                    let before_len = entries.len();
                    entries.retain(|entry| !is_attached_server_entry(entry));
                    changed |= entries.len() != before_len;
                }
            } else if let Some(entries) = container.as_object_mut() {
                let before_len = entries.len();
                entries.retain(|name, entry| !is_attached_server_name(name) && !is_attached_server_entry(entry));
                changed |= entries.len() != before_len;
            }
        }
    }
    (value, changed)
}

fn is_attached_server_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(profile_keys::MCPMATE)
}

fn is_attached_server_entry(entry: &Value) -> bool {
    let Some(object) = entry.as_object() else {
        return false;
    };

    if object
        .get("name")
        .and_then(|name| name.as_str())
        .map(is_attached_server_name)
        .unwrap_or(false)
    {
        return true;
    }

    object
        .get("headers")
        .and_then(|headers| headers.as_object())
        .map(|headers| {
            headers.contains_key(client_headers::MCPMATE_CLIENT_ID)
                || headers.contains_key(client_headers::MCPMATE_PROFILE_ID)
        })
        .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::document::parse_config;
    use crate::clients::document::serialize_document;
    use crate::clients::models::ConfigMapping;
    use serde_json::json;

    fn zed_definition() -> ClientRenderDefinition {
        ClientRenderDefinition {
            identifier: "zed".to_string(),
            format: TemplateFormat::Json,
            config_mapping: ConfigMapping {
                container_keys: vec!["context_servers".to_string()],
                container_type: ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                ..ConfigMapping::default()
            },
            ..ClientRenderDefinition::default()
        }
    }

    fn zed_json5_fixture() -> &'static str {
        r#"{
            "context_servers": {
                "MCPMate": {
                    "command": "bridge",
                },
                "other": {
                    "command": "node",
                },
            },
        }"#
    }

    fn zed_parse_rule() -> ClientConfigFileParse {
        ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["context_servers".to_string()],
        }
    }

    #[test]
    fn merge_config_document_accepts_json5_style_declared_json_input() {
        let document = merge_config_document(
            zed_json5_fixture(),
            &json!({ "MCPMate": { "command": "bridge" } }),
            &zed_definition(),
        )
        .expect("merge json5-style input");
        let merged = serialize_document(&document).expect("serialize merged document");

        let reparsed: Value = serde_json::from_str(&merged).expect("strict json output");
        assert_eq!(reparsed["context_servers"]["MCPMate"]["command"], "bridge");
    }

    #[test]
    fn merge_config_document_deep_merge_preserves_existing_container_entries() {
        let mut definition = zed_definition();
        definition.config_mapping.merge_strategy = MergeStrategy::DeepMerge;

        let document = merge_config_document(
            zed_json5_fixture(),
            &json!({ "MCPMate": { "command": "bridge", "source": "custom" } }),
            &definition,
        )
        .expect("deep merge json5-style input");
        let merged = serialize_document(&document).expect("serialize merged document");

        let reparsed: Value = serde_json::from_str(&merged).expect("strict json output");
        assert_eq!(reparsed["context_servers"]["MCPMate"]["command"], "bridge");
        assert_eq!(reparsed["context_servers"]["other"]["command"], "node");
    }

    #[test]
    fn remove_managed_entries_removes_attached_object_entry_from_recorded_container() {
        let config = json!({
            "servers": {
                "MCPMate": {
                    "type": "streamable_http",
                    "url": "http://127.0.0.1:8000/mcp?client_id=client"
                },
                "other": {
                    "type": "stdio",
                    "command": "other"
                }
            }
        });

        let (updated, changed) = remove_managed_entries(
            ClientConfigDocument {
                format: TemplateFormat::Json,
                value: config,
            },
            &ClientConfigFileParse {
                format: TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["servers".to_string()],
            },
        );

        assert!(changed);
        assert!(updated.value["servers"].get("MCPMate").is_none());
        assert!(updated.value["servers"].get("other").is_some());
    }

    #[test]
    fn remove_managed_entries_removes_attached_array_entry_from_recorded_nested_container() {
        let config = json!({
            "mcp": {
                "servers": [
                    { "name": "MCPMate", "type": "stdio" },
                    { "name": "other", "type": "stdio" }
                ]
            }
        });

        let (updated, changed) = remove_managed_entries(
            ClientConfigDocument {
                format: TemplateFormat::Json,
                value: config,
            },
            &ClientConfigFileParse {
                format: TemplateFormat::Json,
                container_type: ContainerType::Array,
                container_keys: vec!["mcp.servers".to_string()],
            },
        );

        assert!(changed);
        let servers = updated.value["mcp"]["servers"].as_array().expect("servers array");
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0]["name"], "other");
    }

    #[test]
    fn merge_then_remove_managed_entries_round_trips_zed_fixture() {
        let mut definition = zed_definition();
        definition.config_mapping.merge_strategy = MergeStrategy::DeepMerge;

        let document = merge_config_document(
            zed_json5_fixture(),
            &json!({ "MCPMate": { "command": "bridge", "source": "custom" } }),
            &definition,
        )
        .expect("merge");
        let merged = serialize_document(&document).expect("serialize merged document");

        let document = parse_config(&merged, &zed_parse_rule()).expect("parse merged");
        let (updated, changed) = remove_managed_entries(document, &zed_parse_rule());

        assert!(changed);
        assert!(updated.value["context_servers"].get("MCPMate").is_none());
        assert_eq!(updated.value["context_servers"]["other"]["command"], "node");
    }

    #[test]
    fn config_content_diff_reports_unchanged_summary() {
        let diff = config_content_diff("{}", "{}", TemplateFormat::Json).expect("diff");
        assert_eq!(diff.summary.as_deref(), Some("Configuration has no changes"));
    }

    #[test]
    fn config_content_diff_treats_formatting_only_json_changes_as_unchanged() {
        let before = r#"{"url":"https:\/\/example.com"}"#;
        let after = "{\n  \"url\": \"https://example.com\"\n}";

        let diff = config_content_diff(before, after, TemplateFormat::Json).expect("diff");

        assert_eq!(diff.summary.as_deref(), Some("Configuration has no changes"));
    }

    #[test]
    fn remove_managed_entries_no_managed_entries_returns_unchanged() {
        let config = json!({
            "servers": {
                "custom-server": {
                    "type": "stdio",
                    "command": "my-tool"
                }
            }
        });

        let (updated, changed) = remove_managed_entries(
            ClientConfigDocument {
                format: TemplateFormat::Json,
                value: config.clone(),
            },
            &ClientConfigFileParse {
                format: TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["servers".to_string()],
            },
        );

        assert!(!changed);
        assert_eq!(updated.value, config);
    }

    #[test]
    fn deep_merge_replaces_non_object_base_with_patch() {
        let result = deep_merge(json!("old_string"), &json!("new_string"));
        assert_eq!(result, json!("new_string"));

        let result = deep_merge(json!(42), &json!(99));
        assert_eq!(result, json!(99));

        let result = deep_merge(json!(null), &json!({"key": "val"}));
        assert_eq!(result, json!({"key": "val"}));
    }

    #[test]
    fn merge_array_by_name_appends_items_without_name_field() {
        let existing = json!([
            { "type": "stdio", "command": "tool-a" },
            { "type": "stdio", "command": "tool-b" }
        ]);
        let patch = vec![
            json!({ "type": "stdio", "command": "tool-c" }),
        ];

        let result = merge_array_by_name(existing, patch);
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[2]["command"], "tool-c");
    }

    #[test]
    fn config_content_diff_reports_error_for_malformed_declared_json_content() {
        let result = config_content_diff("{invalid", "{}", TemplateFormat::Json);
        assert!(result.is_err());
    }
}
