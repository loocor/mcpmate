use crate::clients::models::TemplateFormat;
use serde_json::Value;

pub(crate) fn parse_config_to_json_value(
    content: &str,
    format: Option<&str>,
) -> Option<Value> {
    let normalized = format.map(|value| value.trim().to_ascii_lowercase());

    match normalized.as_deref() {
        Some("json") => serde_json::from_str::<Value>(content).ok(),
        Some("json5") => json5::from_str::<Value>(content).ok(),
        Some("toml") => toml::from_str::<toml::Value>(content)
            .ok()
            .and_then(|value| serde_json::to_value(value).ok()),
        Some("yaml") => serde_yaml::from_str::<Value>(content).ok(),
        Some(_) => None,
        None => serde_json::from_str::<Value>(content).ok(),
    }
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
