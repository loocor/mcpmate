//! Client configuration file I/O contract.
//!
//! All reads and parses of user-managed client configuration files must go through this module.
//! Repo-authored template catalogs (`source.rs`) and raw backup restore (`storage.rs`) are
//! intentionally excluded.
//!
//! Policy for declared `json` configs is read-tolerant / write-strict: parsing falls back to JSON5
//! so editor-relaxed files (for example Zed `settings.json` with trailing commas) can be inspected
//! and mutated, while serialization always emits strict JSON. Comments, trailing commas, and
//! original formatting are not preserved on write-back.
//!
//! [`JsonReadPolicy`] selects whether declared `json` accepts JSON5 syntax on read. Rule-bound and
//! path-hinted reads use [`JsonReadPolicy::Tolerant`]; autodetect format guessing uses
//! [`JsonReadPolicy::Strict`] so JSON5-only content is not misclassified as JSON.

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{BackupPolicySetting, ClientConfigFileParse, TemplateFormat};
use crate::clients::storage::DynConfigStorage;
use serde_json::Value;

/// How declared `json` input is interpreted on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonReadPolicy {
    /// Format is declared by rule or path hint: accept JSON5 syntax while treating the file as JSON.
    Tolerant,
    /// Format is a guess during autodetect: strict JSON only so JSON and JSON5 can be distinguished.
    Strict,
}

/// Parsed client configuration file content plus the format used to interpret it.
#[derive(Debug, Clone)]
pub(crate) struct ClientConfigDocument {
    pub format: TemplateFormat,
    pub value: Value,
}

pub(crate) fn is_blank_config_content(content: &str) -> bool {
    content.trim().is_empty()
}

/// Read raw text from a resolved filesystem path.
pub(crate) async fn read_config_file(resolved_path: &str) -> ConfigResult<String> {
    tokio::fs::read_to_string(resolved_path)
        .await
        .map_err(ConfigError::IoError)
}

/// Parse using the effective client parse rule. Declared `json` is JSON5-tolerant on read.
pub(crate) fn parse_config(
    content: &str,
    rule: &ClientConfigFileParse,
) -> ConfigResult<ClientConfigDocument> {
    let value = parse_config_for_format(content, rule.format, JsonReadPolicy::Tolerant)?;
    Ok(ClientConfigDocument {
        format: rule.format,
        value,
    })
}

/// Lenient parse for inspection and UI fallbacks: blank input and parse failures become `Null`.
pub(crate) fn parse_config_lenient(
    content: &str,
    rule: &ClientConfigFileParse,
) -> Value {
    if is_blank_config_content(content) {
        return Value::Null;
    }

    parse_config_for_format(content, rule.format, JsonReadPolicy::Tolerant).unwrap_or(Value::Null)
}

/// Fallback parse for config details when structured inspection is unavailable.
pub(crate) fn parse_config_fallback(
    raw_content: Option<&str>,
    parse_rule: Option<&ClientConfigFileParse>,
    config_path: Option<&str>,
) -> Value {
    let Some(raw_content) = raw_content else {
        return Value::Null;
    };

    if is_blank_config_content(raw_content) {
        return Value::Null;
    }

    if let Some(rule) = parse_rule {
        return parse_config_for_format(raw_content, rule.format, JsonReadPolicy::Tolerant)
            .unwrap_or(Value::Null);
    }

    if let Ok(document) = parse_config_autodetect(raw_content, config_path) {
        return document.value;
    }

    Value::String(raw_content.to_string())
}

/// Parse an existing on-disk document before merge/render. Blank files become an empty document.
pub(crate) fn parse_config_for_merge(
    content: &str,
    format: TemplateFormat,
) -> ConfigResult<Value> {
    if is_blank_config_content(content) {
        return Ok(Value::Null);
    }

    parse_config_for_format(content, format, JsonReadPolicy::Tolerant)
}

/// Autodetect format from a path hint and parse content for parse-rule inspection flows.
///
/// Path hints use [`JsonReadPolicy::Tolerant`] because the extension declares the on-disk format.
/// The fallback loop uses [`JsonReadPolicy::Strict`] so JSON5-only content is not misclassified as JSON.
pub(crate) fn parse_config_autodetect(
    content: &str,
    path_hint: Option<&str>,
) -> ConfigResult<ClientConfigDocument> {
    let hinted = infer_format_from_path(path_hint);
    let mut primary_error: Option<ConfigError> = None;

    if let Some(format) = hinted {
        match parse_config_for_format(content, format, JsonReadPolicy::Tolerant) {
            Ok(value) => return Ok(ClientConfigDocument { format, value }),
            Err(err) => primary_error = Some(err),
        }
    }

    for format in autodetect_format_order(hinted) {
        if matches!((hinted, format), (Some(TemplateFormat::Json), TemplateFormat::Json)) {
            continue;
        }
        if let Ok(value) = parse_config_for_format(content, format, JsonReadPolicy::Strict) {
            return Ok(ClientConfigDocument { format, value });
        }
    }

    Err(primary_error.unwrap_or_else(|| {
        ConfigError::DataAccessError(
            "Unable to parse the selected configuration file as json, json5, toml, or yaml.".to_string(),
        )
    }))
}

/// Serialize a [`Value`] into the target format string.
///
/// JSON output is pretty-printed with escaped-slash normalization (`\/` → `/`).
pub(crate) fn serialize_config(
    value: &Value,
    format: TemplateFormat,
) -> ConfigResult<String> {
    match format {
        TemplateFormat::Json => {
            let pretty = serde_json::to_string_pretty(value).map_err(ConfigError::from)?;
            Ok(pretty.replace("\\/", "/"))
        }
        TemplateFormat::Json5 => {
            json5::to_string(value).map_err(|err| ConfigError::TemplateParseError(err.to_string()))
        }
        TemplateFormat::Yaml => serde_yaml::to_string(value).map_err(ConfigError::from),
        TemplateFormat::Toml => toml::to_string(value).map_err(|err| ConfigError::TomlSerializeError(err.to_string())),
    }
}

/// Serialize a [`ClientConfigDocument`] using its stored format.
pub(crate) fn serialize_document(document: &ClientConfigDocument) -> ConfigResult<String> {
    serialize_config(&document.value, document.format)
}

/// Serialize a parsed document and write it through the configured storage adapter.
pub(crate) async fn persist_config_document(
    storage: &DynConfigStorage,
    client_id: &str,
    config_path: &str,
    document: &ClientConfigDocument,
    backup_policy: &BackupPolicySetting,
) -> ConfigResult<(Option<String>, String)> {
    let content = serialize_document(document)?;
    let backup_path = storage
        .write_atomic(client_id, config_path, &content, backup_policy)
        .await?;
    Ok((backup_path, content))
}

pub(crate) fn map_config_file_error(
    context: &str,
    err: ConfigError,
) -> ConfigError {
    let message = match err {
        ConfigError::TemplateParseError(message) => message,
        ConfigError::JsonError(err) => err.to_string(),
        ConfigError::YamlError(err) => err.to_string(),
        ConfigError::TomlError(err) => err.to_string(),
        ConfigError::TomlSerializeError(message) => message,
        ConfigError::IoError(err) => err.to_string(),
        ConfigError::FileOperationError(message) => message,
        other => return other,
    };

    ConfigError::DataAccessError(format!("{context}: {message}"))
}

/// Core parse dispatch by format and JSON read policy.
///
/// [`JsonReadPolicy::Tolerant`] falls back to JSON5 for declared `json` so editor-relaxed files
/// (trailing commas, comments) are accepted. [`JsonReadPolicy::Strict`] requires valid JSON.
pub(crate) fn parse_config_for_format(
    content: &str,
    format: TemplateFormat,
    json_read: JsonReadPolicy,
) -> ConfigResult<Value> {
    let trimmed = content.trim();

    match format {
        TemplateFormat::Json => match json_read {
            JsonReadPolicy::Tolerant => parse_json_with_json5_fallback(trimmed),
            JsonReadPolicy::Strict => serde_json::from_str(trimmed).map_err(ConfigError::from),
        },
        TemplateFormat::Json5 => {
            json5::from_str(trimmed).map_err(|err| ConfigError::TemplateParseError(err.to_string()))
        }
        TemplateFormat::Toml => {
            let value: toml::Value = toml::from_str(trimmed).map_err(ConfigError::from)?;
            serde_json::to_value(value).map_err(|err| ConfigError::TemplateParseError(err.to_string()))
        }
        TemplateFormat::Yaml => serde_yaml::from_str(trimmed).map_err(ConfigError::from),
    }
}

fn parse_json_with_json5_fallback(content: &str) -> ConfigResult<Value> {
    match serde_json::from_str::<Value>(content) {
        Ok(value) => Ok(value),
        Err(json_err) => json5::from_str::<Value>(content).map_err(|json5_err| {
            ConfigError::TemplateParseError(format!(
                "Failed to parse JSON config; json error: {json_err}; json5 fallback error: {json5_err}"
            ))
        }),
    }
}

fn autodetect_format_order(hinted: Option<TemplateFormat>) -> Vec<TemplateFormat> {
    let mut order = Vec::new();
    if let Some(format) = hinted {
        order.push(format);
    }

    for format in [
        TemplateFormat::Json,
        TemplateFormat::Json5,
        TemplateFormat::Toml,
        TemplateFormat::Yaml,
    ] {
        if !order.contains(&format) {
            order.push(format);
        }
    }

    order
}

pub(crate) fn infer_format_from_path(path_hint: Option<&str>) -> Option<TemplateFormat> {
    let path = path_hint?.to_ascii_lowercase();
    if path.ends_with(".json5") {
        Some(TemplateFormat::Json5)
    } else if path.ends_with(".json") {
        Some(TemplateFormat::Json)
    } else if path.ends_with(".toml") {
        Some(TemplateFormat::Toml)
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        Some(TemplateFormat::Yaml)
    } else {
        None
    }
}

pub(crate) fn get_config_last_modified(config_path: &str) -> Option<String> {
    use std::fs;
    use std::time::SystemTime;

    let expanded_path = if config_path.starts_with("~/") {
        let home = std::env::var("HOME").ok()?;
        config_path.replacen("~", &home, 1)
    } else {
        config_path.to_string()
    };

    let metadata = fs::metadata(&expanded_path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    let datetime = chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)?;
    Some(datetime.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::analyzer::inspect_config_content;
    use crate::clients::document::serialize_document;
    use crate::clients::models::ContainerType;
    use crate::clients::models::{ClientRenderDefinition, ConfigMapping, MergeStrategy};
    use crate::clients::mutate::merge_config_document;
    use serde_json::json;

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
    fn blank_content_is_treated_consistently() {
        let rule = zed_parse_rule();

        assert!(is_blank_config_content("  "));
        assert_eq!(parse_config_lenient("  ", &rule), Value::Null);
        assert_eq!(
            parse_config_for_merge("  ", TemplateFormat::Json).expect("merge base"),
            Value::Null
        );
        assert!(parse_config("  ", &rule).is_err());
    }

    #[test]
    fn map_config_file_error_avoids_template_error_prefix() {
        let error = map_config_file_error(
            "Failed to parse config for detach",
            parse_config_for_format("{", TemplateFormat::Json, JsonReadPolicy::Tolerant).expect_err("invalid json"),
        );

        assert!(error.to_string().contains("Failed to parse config for detach"));
        assert!(!error.to_string().contains("Client template file parsing error"));
    }

    #[test]
    fn parse_config_fallback_uses_effective_parse_rule() {
        let rule = zed_parse_rule();
        let value = parse_config_fallback(Some(zed_json5_fixture()), Some(&rule), None);

        assert_eq!(value["context_servers"]["MCPMate"]["command"], "bridge");
    }

    #[test]
    fn parse_config_fallback_blank_without_rule_returns_null() {
        assert_eq!(
            parse_config_fallback(Some("  "), None, Some("settings.json")),
            Value::Null
        );
    }

    #[test]
    fn json_read_policy_strict_rejects_json5_only_content_as_json() {
        assert!(parse_config_for_format(zed_json5_fixture(), TemplateFormat::Json, JsonReadPolicy::Strict).is_err());
        parse_config_for_format(zed_json5_fixture(), TemplateFormat::Json, JsonReadPolicy::Tolerant)
            .expect("tolerant json read accepts json5-style content");
    }

    #[test]
    fn autodetect_json_hint_accepts_json5_as_declared_json() {
        let document = parse_config_autodetect(zed_json5_fixture(), Some("settings.json"))
            .expect("json hint accepts json5-style content");

        assert_eq!(document.format, TemplateFormat::Json);
    }

    #[test]
    fn autodetect_no_extension_classifies_json5_as_json5() {
        let document = parse_config_autodetect(zed_json5_fixture(), None).expect("no-extension json5-style content");

        assert_eq!(document.format, TemplateFormat::Json5);
    }

    #[test]
    fn cross_path_matrix_accepts_zed_json5_style_config() {
        let rule = zed_parse_rule();
        let fixture = zed_json5_fixture();

        parse_config(fixture, &rule).expect("strict parse");
        assert_ne!(parse_config_lenient(fixture, &rule), Value::Null);
        parse_config_for_merge(fixture, TemplateFormat::Json).expect("merge parse");
        parse_config_autodetect(fixture, Some("settings.json")).expect("autodetect parse");
        assert_eq!(
            parse_config_autodetect(fixture, Some("/tmp/settings.json"))
                .expect("json extension autodetect")
                .format,
            TemplateFormat::Json
        );

        let inspected = inspect_config_content(fixture, &rule, None);
        assert!(inspected.inspection.matched_container);
        assert!(inspected.analysis.mcpmate_present);

        let definition = ClientRenderDefinition {
            identifier: "zed".to_string(),
            format: TemplateFormat::Json,
            config_mapping: ConfigMapping {
                container_keys: vec!["context_servers".to_string()],
                container_type: ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                ..ConfigMapping::default()
            },
            ..ClientRenderDefinition::default()
        };
        let document = merge_config_document(fixture, &json!({ "MCPMate": { "command": "bridge" } }), &definition)
            .expect("merge config");
        let merged = serialize_document(&document).expect("serialize merged document");
        let reparsed: Value = serde_json::from_str(&merged).expect("strict json output");
        assert_eq!(reparsed["context_servers"]["MCPMate"]["command"], "bridge");
    }
}
