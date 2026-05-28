use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    ClientConfigFileParse, ClientTemplate, ConfigMapping, ContainerType, DetectionMethod, DetectionRule, FormatRule,
    MergeStrategy, StorageConfig, StorageKind, TemplateFormat, canonical_config_transport_key,
};
use serde_json::{Map, Value};
use std::{collections::HashMap, time::Duration};

pub const DEFAULT_ADMIN_DISCOVERY_BASE_URL: &str = "https://public.mcp.umate.ai";
pub const ADMIN_DISCOVERY_BASE_URL_ENV: &str = "MCPMATE_ADMIN_DISCOVERY_BASE_URL";
const ADMIN_DISCOVERY_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const ADMIN_DISCOVERY_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const ADMIN_DISCOVERY_USER_AGENT: &str = "MCPMate/0.1.0 (+https://mcp.umate.ai)";
const ADMIN_DISCOVERY_CLIENT_PAGE_LIMIT: u64 = 100;
const ADMIN_DISCOVERY_MAX_CLIENT_PAGES: usize = 1_000;

#[derive(Debug, Clone, Copy)]
struct AdminDiscoveryHttpConfig {
    request_timeout: Duration,
    connect_timeout: Duration,
    user_agent: &'static str,
}

impl Default for AdminDiscoveryHttpConfig {
    fn default() -> Self {
        Self {
            request_timeout: ADMIN_DISCOVERY_REQUEST_TIMEOUT,
            connect_timeout: ADMIN_DISCOVERY_CONNECT_TIMEOUT,
            user_agent: ADMIN_DISCOVERY_USER_AGENT,
        }
    }
}

pub fn admin_discovery_base_url() -> String {
    std::env::var(ADMIN_DISCOVERY_BASE_URL_ENV)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ADMIN_DISCOVERY_BASE_URL.to_string())
}

pub async fn fetch_admin_discovery_client_templates(base_url: &str) -> ConfigResult<Vec<ClientTemplate>> {
    fetch_admin_discovery_client_templates_with_config(base_url, AdminDiscoveryHttpConfig::default()).await
}

async fn fetch_admin_discovery_client_templates_with_config(
    base_url: &str,
    http_config: AdminDiscoveryHttpConfig,
) -> ConfigResult<Vec<ClientTemplate>> {
    let url = format!("{}/discovery/clients", base_url.trim_end_matches('/'));
    let client = build_admin_discovery_http_client(http_config)?;
    let mut offset = 0;
    let mut page_count = 0;
    let mut clients = Vec::new();

    loop {
        if page_count >= ADMIN_DISCOVERY_MAX_CLIENT_PAGES {
            return Err(ConfigError::DataAccessError(format!(
                "Admin discovery pagination exceeded maximum page count {ADMIN_DISCOVERY_MAX_CLIENT_PAGES}"
            )));
        }
        page_count += 1;

        let payload =
            fetch_admin_discovery_clients_page(&client, &url, ADMIN_DISCOVERY_CLIENT_PAGE_LIMIT, offset).await?;
        let page_clients = admin_discovery_clients_array(&payload)?;
        clients.extend(page_clients.iter().cloned());
        let page = admin_discovery_page(&payload)?;

        let Some(next_offset) = next_admin_discovery_offset(&page, offset)? else {
            break;
        };
        offset = next_offset;
    }

    map_admin_discovery_clients(admin_discovery_clients_payload(clients))
}

fn build_admin_discovery_http_client(http_config: AdminDiscoveryHttpConfig) -> ConfigResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(http_config.request_timeout)
        .connect_timeout(http_config.connect_timeout)
        .user_agent(http_config.user_agent)
        .build()
        .map_err(|err| ConfigError::DataAccessError(format!("Failed to build Admin discovery HTTP client: {err}")))
}

fn ensure_admin_discovery_success_status(status: reqwest::StatusCode) -> ConfigResult<()> {
    if status.is_success() {
        return Ok(());
    }

    Err(ConfigError::DataAccessError(format!(
        "Admin discovery request failed with HTTP {status}"
    )))
}

async fn fetch_admin_discovery_clients_page(
    client: &reqwest::Client,
    url: &str,
    limit: u64,
    offset: u64,
) -> ConfigResult<Value> {
    let response = client
        .get(url)
        .query(&[("limit", limit), ("offset", offset)])
        .send()
        .await
        .map_err(|err| ConfigError::DataAccessError(format!("Admin discovery request failed: {err}")))?;
    let status = response.status();
    ensure_admin_discovery_success_status(status)?;
    response
        .json::<Value>()
        .await
        .map_err(|err| ConfigError::DataAccessError(format!("Failed to parse Admin discovery response: {err}")))
}

#[derive(Debug, Clone, Copy)]
struct AdminDiscoveryPage {
    limit: Option<u64>,
    offset: Option<u64>,
    total: Option<u64>,
    has_more: Option<bool>,
    next_offset: Option<u64>,
}

fn admin_discovery_clients_array(payload: &Value) -> ConfigResult<&Vec<Value>> {
    record(payload)
        .and_then(|root| root.get("clients"))
        .and_then(Value::as_array)
        .ok_or_else(|| ConfigError::TemplateParseError("Admin discovery response is missing clients array".to_string()))
}

fn admin_discovery_clients_payload(clients: Vec<Value>) -> Value {
    Value::Object(Map::from_iter([("clients".to_string(), Value::Array(clients))]))
}

fn admin_discovery_page(payload: &Value) -> ConfigResult<AdminDiscoveryPage> {
    let Some(page) = record(payload)
        .and_then(|root| root.get("page"))
        .and_then(Value::as_object)
    else {
        return Err(ConfigError::DataAccessError(
            "Admin discovery response is missing page metadata".to_string(),
        ));
    };

    Ok(AdminDiscoveryPage {
        limit: optional_u64_page_field(page, "limit")?,
        offset: optional_u64_page_field(page, "offset")?,
        total: optional_u64_page_field(page, "total")?,
        has_more: optional_bool_page_field(page, "hasMore")?,
        next_offset: optional_u64_page_field(page, "nextOffset")?,
    })
}

fn next_admin_discovery_offset(
    page: &AdminDiscoveryPage,
    requested_offset: u64,
) -> ConfigResult<Option<u64>> {
    match page.has_more {
        Some(true) => {
            let next_offset = page.next_offset.ok_or_else(|| {
                ConfigError::DataAccessError("Admin discovery pagination did not provide a next offset".to_string())
            })?;
            ensure_admin_discovery_pagination_advances(requested_offset, next_offset)?;
            Ok(Some(next_offset))
        }
        Some(false) => Ok(None),
        None => {
            let limit = required_admin_discovery_page_field(page.limit, "limit")?;
            let offset = required_admin_discovery_page_field(page.offset, "offset")?;
            let total = required_admin_discovery_page_field(page.total, "total")?;
            if limit == 0 {
                return Err(ConfigError::DataAccessError(
                    "Admin discovery pagination limit must be greater than 0".to_string(),
                ));
            }
            if offset != requested_offset {
                return Err(ConfigError::DataAccessError(format!(
                    "Admin discovery pagination offset {offset} did not match requested offset {requested_offset}"
                )));
            }
            let next_offset = offset.checked_add(limit).ok_or_else(|| {
                ConfigError::DataAccessError("Admin discovery pagination offset overflowed".to_string())
            })?;
            if next_offset >= total {
                return Ok(None);
            }
            ensure_admin_discovery_pagination_advances(requested_offset, next_offset)?;
            Ok(Some(next_offset))
        }
    }
}

fn required_admin_discovery_page_field(
    value: Option<u64>,
    key: &str,
) -> ConfigResult<u64> {
    value.ok_or_else(|| ConfigError::DataAccessError(format!("Admin discovery pagination is missing page.{key}")))
}

fn ensure_admin_discovery_pagination_advances(
    current_offset: u64,
    next_offset: u64,
) -> ConfigResult<()> {
    if next_offset > current_offset {
        return Ok(());
    }

    Err(ConfigError::DataAccessError(format!(
        "Admin discovery pagination did not advance from offset {current_offset} to {next_offset}"
    )))
}

fn optional_u64_page_field(
    page: &Map<String, Value>,
    key: &str,
) -> ConfigResult<Option<u64>> {
    let Some(value) = page.get(key) else {
        return Ok(None);
    };
    value.as_u64().map(Some).ok_or_else(|| {
        ConfigError::DataAccessError(format!(
            "Admin discovery pagination page.{key} must be a non-negative integer"
        ))
    })
}

fn optional_bool_page_field(
    page: &Map<String, Value>,
    key: &str,
) -> ConfigResult<Option<bool>> {
    let Some(value) = page.get(key) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| ConfigError::DataAccessError(format!("Admin discovery pagination page.{key} must be a boolean")))
}

pub(crate) fn map_admin_discovery_clients(payload: Value) -> ConfigResult<Vec<ClientTemplate>> {
    let Some(clients) = record(&payload)
        .and_then(|root| root.get("clients"))
        .and_then(Value::as_array)
    else {
        return Err(ConfigError::TemplateParseError(
            "Admin discovery response is missing clients array".to_string(),
        ));
    };

    clients
        .iter()
        .map(map_admin_discovery_client)
        .collect::<ConfigResult<Vec<Option<ClientTemplate>>>>()
        .map(|templates| templates.into_iter().flatten().collect())
}

fn map_admin_discovery_client(value: &Value) -> ConfigResult<Option<ClientTemplate>> {
    let Some(client) = record(value) else {
        return Err(ConfigError::TemplateParseError(
            "Admin discovery client entry must be an object".to_string(),
        ));
    };
    let identifier = first_compact_string(client, &["identifier", "id", "name"])
        .ok_or_else(|| ConfigError::TemplateParseError("Admin discovery client is missing identifier".to_string()))?;
    let Some(config) = field_record(client, "config") else {
        return Ok(None);
    };
    if compact_string(config.get("kind")) != Some("file") {
        return Ok(None);
    }
    let Some(file) = field_record(config, "file") else {
        return Ok(None);
    };
    let detection = detection_rules_from_paths(file)?;
    if detection.is_empty() {
        return Ok(None);
    }

    let parse = parse_rule_from_file(file, identifier)?;
    let merge_strategy = merge_strategy_from_file(file, identifier)?;
    let keep_original_config = keep_original_config_from_file(file, identifier)?;
    let managed_source = managed_source_from_file(file, identifier)?;
    let transports = transport_rules_from_config(config, identifier)?;
    let format = parse.format;
    let container_keys = parse.container_keys.clone();
    let container_type = parse.container_type;

    Ok(Some(ClientTemplate {
        identifier: identifier.to_string(),
        display_name: first_compact_string(client, &["displayName", "display_name"]).map(str::to_string),
        format,
        storage: StorageConfig {
            kind: StorageKind::File,
            path_strategy: Some("config_path".to_string()),
            adapter: None,
        },
        detection,
        config_mapping: ConfigMapping {
            container_keys,
            container_type,
            merge_strategy,
            keep_original_config,
            managed_endpoint: None,
            managed_source: Some(managed_source),
            parse: Some(parse),
            format_rules: transports,
        },
        metadata: metadata_from_client(client),
        ..Default::default()
    }))
}

fn detection_rules_from_paths(file: &Map<String, Value>) -> ConfigResult<HashMap<String, Vec<DetectionRule>>> {
    let mut detection = HashMap::new();
    let Some(paths) = field_record(file, "paths") else {
        return Ok(detection);
    };

    for platform in ["macos", "windows", "linux"] {
        if let Some(path) = compact_string(paths.get(platform)) {
            detection.insert(
                platform.to_string(),
                vec![DetectionRule {
                    method: DetectionMethod::ConfigPath,
                    value: path.to_string(),
                    config_path: None,
                    priority: None,
                }],
            );
        }
    }

    Ok(detection)
}

fn parse_rule_from_file(
    file: &Map<String, Value>,
    identifier: &str,
) -> ConfigResult<ClientConfigFileParse> {
    let format = compact_string(file.get("format")).unwrap_or("json");
    let format = template_format(format).ok_or_else(|| {
        ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} has unsupported config.file.format '{format}'"
        ))
    })?;

    let container = field_record(file, "container").ok_or_else(|| {
        ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} is missing config.file.container"
        ))
    })?;
    let container_type = match compact_string(container.get("type")) {
        Some("array") => ContainerType::Array,
        Some("standard" | "object_map") => ContainerType::ObjectMap,
        Some(value) => {
            return Err(ConfigError::TemplateParseError(format!(
                "Admin discovery client {identifier} has unsupported config.file.container.type '{value}'"
            )));
        }
        None => ContainerType::ObjectMap,
    };
    let container_keys = string_array(container.get("keys"));
    if container_keys.is_empty() {
        return Err(ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} is missing config.file.container.keys"
        )));
    }

    Ok(ClientConfigFileParse {
        format,
        container_type,
        container_keys,
    })
}

fn merge_strategy_from_file(
    file: &Map<String, Value>,
    identifier: &str,
) -> ConfigResult<MergeStrategy> {
    let Some(merge) = field_record(file, "merge") else {
        return Ok(MergeStrategy::Replace);
    };
    let Some(value) = merge.get("strategy") else {
        return Ok(MergeStrategy::Replace);
    };
    let Some(strategy) = compact_string(Some(value)) else {
        return Err(ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} has unsupported config.file.merge.strategy"
        )));
    };

    match strategy {
        "replace" => Ok(MergeStrategy::Replace),
        "deep_merge" => Ok(MergeStrategy::DeepMerge),
        value => Err(ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} has unsupported config.file.merge.strategy '{value}'"
        ))),
    }
}

fn keep_original_config_from_file(
    file: &Map<String, Value>,
    identifier: &str,
) -> ConfigResult<bool> {
    let Some(merge) = field_record(file, "merge") else {
        return Ok(false);
    };
    let Some(value) = merge.get("keepOriginal") else {
        return Ok(false);
    };
    value.as_bool().ok_or_else(|| {
        ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} has invalid config.file.merge.keepOriginal"
        ))
    })
}

fn managed_source_from_file(
    file: &Map<String, Value>,
    identifier: &str,
) -> ConfigResult<String> {
    let Some(value) = file.get("managedSource") else {
        return Ok("profile".to_string());
    };
    compact_string(Some(value)).map(str::to_string).ok_or_else(|| {
        ConfigError::TemplateParseError(format!(
            "Admin discovery client {identifier} has invalid config.file.managedSource"
        ))
    })
}

fn transport_rules_from_config(
    config: &Map<String, Value>,
    identifier: &str,
) -> ConfigResult<HashMap<String, FormatRule>> {
    let mut rules = HashMap::new();
    let Some(transports) = field_record(config, "transports") else {
        return Ok(rules);
    };

    for (key, value) in transports {
        let Some(canonical_key) = canonical_config_transport_key(key) else {
            continue;
        };
        if !value.is_object() {
            return Err(ConfigError::TemplateParseError(format!(
                "Admin discovery client {identifier} has invalid transport rule '{key}'"
            )));
        }
        let rule = serde_json::from_value::<FormatRule>(value.clone()).map_err(|err| {
            ConfigError::TemplateParseError(format!(
                "Admin discovery client {identifier} transport rule '{key}' is invalid: {err}"
            ))
        })?;
        rules.insert(canonical_key.to_string(), rule);
    }

    Ok(rules)
}

fn metadata_from_client(client: &Map<String, Value>) -> HashMap<String, Value> {
    let mut metadata = HashMap::new();
    let nested_metadata = field_record(client, "metadata");
    let links = field_record(client, "links");
    let icon = field_record(client, "icon");

    insert_metadata_string(
        &mut metadata,
        "description",
        compact_string(client.get("description"))
            .or_else(|| nested_metadata.and_then(|metadata| compact_string(metadata.get("description")))),
    );
    insert_metadata_string(
        &mut metadata,
        "homepage_url",
        links
            .and_then(|links| compact_string(links.get("homepage")))
            .or_else(|| compact_string(client.get("homepageUrl")))
            .or_else(|| compact_string(client.get("homepage_url")))
            .or_else(|| nested_metadata.and_then(|metadata| compact_string(metadata.get("homepage_url")))),
    );
    insert_metadata_string(
        &mut metadata,
        "docs_url",
        links
            .and_then(|links| compact_string(links.get("docs")))
            .or_else(|| compact_string(client.get("docsUrl")))
            .or_else(|| compact_string(client.get("docs_url")))
            .or_else(|| nested_metadata.and_then(|metadata| compact_string(metadata.get("docs_url")))),
    );
    insert_metadata_string(
        &mut metadata,
        "support_url",
        links
            .and_then(|links| compact_string(links.get("support")))
            .or_else(|| compact_string(client.get("supportUrl")))
            .or_else(|| compact_string(client.get("support_url")))
            .or_else(|| nested_metadata.and_then(|metadata| compact_string(metadata.get("support_url")))),
    );
    insert_metadata_string(
        &mut metadata,
        "logo_url",
        icon.and_then(|icon| compact_string(icon.get("url")))
            .or_else(|| compact_string(client.get("logoUrl")))
            .or_else(|| compact_string(client.get("logo_url")))
            .or_else(|| nested_metadata.and_then(|metadata| compact_string(metadata.get("logo_url")))),
    );
    insert_metadata_string(
        &mut metadata,
        "category",
        compact_string(client.get("category"))
            .or_else(|| nested_metadata.and_then(|metadata| compact_string(metadata.get("category")))),
    );

    metadata
}

fn insert_metadata_string(
    metadata: &mut HashMap<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn template_format(value: &str) -> Option<TemplateFormat> {
    match value {
        "json" => Some(TemplateFormat::Json),
        "json5" => Some(TemplateFormat::Json5),
        "toml" => Some(TemplateFormat::Toml),
        "yaml" | "yml" => Some(TemplateFormat::Yaml),
        _ => None,
    }
}

fn record(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn field_record<'a>(
    record: &'a Map<String, Value>,
    key: &str,
) -> Option<&'a Map<String, Value>> {
    record.get(key).and_then(Value::as_object)
}

fn compact_string(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn first_compact_string<'a>(
    record: &'a Map<String, Value>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter().find_map(|key| compact_string(record.get(*key)))
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| compact_string(Some(item)).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::models::{ContainerType, DetectionMethod, MergeStrategy, TemplateFormat};
    use std::time::Duration;

    fn admin_discovery_file_client(identifier: &str) -> Value {
        serde_json::json!({
            "identifier": identifier,
            "displayName": identifier,
            "config": {
                "kind": "file",
                "file": {
                    "format": "json",
                    "paths": {
                        "macos": format!("~/.{identifier}/mcp.json")
                    },
                    "container": {
                        "type": "standard",
                        "keys": ["mcpServers"]
                    }
                },
                "transports": {
                    "stdio": {
                        "command_field": "command"
                    }
                }
            }
        })
    }

    fn assert_data_access_error(
        result: ConfigResult<Vec<ClientTemplate>>,
        expected: &str,
    ) {
        let Err(ConfigError::DataAccessError(message)) = result else {
            panic!("expected data access error");
        };
        assert!(
            message.contains(expected),
            "expected error to contain '{expected}', got '{message}'"
        );
    }

    #[test]
    fn maps_admin_v2_file_client_into_config_path_template() {
        let payload = serde_json::json!({
            "clients": [
                {
                    "identifier": "cursor",
                    "displayName": "Cursor",
                    "description": "AI code editor",
                    "links": {
                        "homepage": "https://cursor.com",
                        "docs": "https://cursor.com/docs",
                        "support": "https://forum.cursor.com"
                    },
                    "icon": {
                        "url": "https://cursor.com/favicon.svg"
                    },
                    "config": {
                        "kind": "file",
                        "file": {
                            "format": "json",
                            "paths": {
                                "macos": "~/.cursor/mcp.json",
                                "windows": "%APPDATA%\\Cursor\\mcp.json",
                                "linux": "~/.config/cursor/mcp.json"
                            },
                            "container": {
                                "type": "standard",
                                "keys": ["mcpServers"]
                            }
                        },
                        "transports": {
                            "stdio": {
                                "command_field": "command",
                                "args_field": "args",
                                "env_field": "env"
                            },
                            "streamable_http": {
                                "type_value": "streamable_http",
                                "url_field": "url"
                            }
                        }
                    }
                }
            ],
            "page": {
                "limit": 50,
                "offset": 0,
                "total": 1
            }
        });

        let templates = map_admin_discovery_clients(payload).expect("map discovery clients");

        assert_eq!(templates.len(), 1);
        let template = &templates[0];
        assert_eq!(template.identifier, "cursor");
        assert_eq!(template.display_name.as_deref(), Some("Cursor"));
        assert_eq!(template.format, TemplateFormat::Json);
        assert_eq!(template.config_mapping.container_type, ContainerType::ObjectMap);
        assert_eq!(template.config_mapping.container_keys, vec!["mcpServers"]);
        assert_eq!(
            template
                .config_mapping
                .parse
                .as_ref()
                .expect("parse rule")
                .container_keys,
            vec!["mcpServers"]
        );
        assert!(template.config_mapping.format_rules.contains_key("stdio"));
        assert!(template.config_mapping.format_rules.contains_key("streamable_http"));
        assert_eq!(
            template.metadata.get("description").and_then(|value| value.as_str()),
            Some("AI code editor")
        );
        assert_eq!(
            template.metadata.get("homepage_url").and_then(|value| value.as_str()),
            Some("https://cursor.com")
        );
        assert_eq!(
            template.metadata.get("docs_url").and_then(|value| value.as_str()),
            Some("https://cursor.com/docs")
        );
        assert_eq!(
            template.metadata.get("support_url").and_then(|value| value.as_str()),
            Some("https://forum.cursor.com")
        );
        assert_eq!(
            template.metadata.get("logo_url").and_then(|value| value.as_str()),
            Some("https://cursor.com/favicon.svg")
        );

        let macos_rules = template.platform_rules("macos").expect("macos rules");
        assert_eq!(macos_rules.len(), 1);
        assert_eq!(macos_rules[0].method, DetectionMethod::ConfigPath);
        assert_eq!(macos_rules[0].value, "~/.cursor/mcp.json");
    }

    #[test]
    fn maps_admin_v2_file_merge_policy_into_config_mapping() {
        let payload = serde_json::json!({
            "clients": [
                {
                    "identifier": "cursor",
                    "displayName": "Cursor",
                    "config": {
                        "kind": "file",
                        "file": {
                            "format": "json",
                            "paths": {
                                "macos": "~/.cursor/mcp.json"
                            },
                            "container": {
                                "type": "standard",
                                "keys": ["mcpServers"]
                            },
                            "merge": {
                                "strategy": "deep_merge",
                                "keepOriginal": true
                            },
                            "managedSource": "admin-catalog"
                        },
                        "transports": {
                            "stdio": {
                                "command_field": "command"
                            }
                        }
                    }
                }
            ]
        });

        let templates = map_admin_discovery_clients(payload).expect("map discovery clients");

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].config_mapping.merge_strategy, MergeStrategy::DeepMerge);
        assert!(templates[0].config_mapping.keep_original_config);
        assert_eq!(
            templates[0].config_mapping.managed_source.as_deref(),
            Some("admin-catalog")
        );
    }

    #[test]
    fn maps_admin_v2_file_client_with_default_format_and_container_type() {
        let payload = serde_json::json!({
            "clients": [
                {
                    "identifier": "minimal-client",
                    "config": {
                        "kind": "file",
                        "file": {
                            "paths": {
                                "macos": "~/.minimal/mcp.json"
                            },
                            "container": {
                                "keys": ["mcpServers"]
                            }
                        },
                        "transports": {
                            "stdio": {
                                "command_field": "command"
                            }
                        }
                    }
                }
            ]
        });

        let templates = map_admin_discovery_clients(payload).expect("map minimal discovery client");

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].identifier, "minimal-client");
        assert_eq!(templates[0].format, TemplateFormat::Json);
        assert_eq!(templates[0].config_mapping.container_type, ContainerType::ObjectMap);
        assert_eq!(templates[0].config_mapping.container_keys, vec!["mcpServers"]);
    }

    #[test]
    fn rejects_admin_v2_file_client_with_invalid_merge_strategy() {
        let payload = serde_json::json!({
            "clients": [
                {
                    "identifier": "cursor",
                    "config": {
                        "kind": "file",
                        "file": {
                            "format": "json",
                            "paths": {
                                "macos": "~/.cursor/mcp.json"
                            },
                            "container": {
                                "type": "standard",
                                "keys": ["mcpServers"]
                            },
                            "merge": {
                                "strategy": "append"
                            }
                        }
                    }
                }
            ]
        });

        let Err(ConfigError::TemplateParseError(message)) = map_admin_discovery_clients(payload) else {
            panic!("expected template parse error");
        };
        assert!(message.contains("unsupported config.file.merge.strategy 'append'"));
    }

    #[test]
    fn rejects_admin_discovery_non_success_status() {
        let result =
            ensure_admin_discovery_success_status(reqwest::StatusCode::SERVICE_UNAVAILABLE).map(|()| Vec::new());

        assert_data_access_error(result, "HTTP 503");
    }

    #[test]
    fn builds_admin_discovery_http_client_with_bounded_timeouts() {
        let config = AdminDiscoveryHttpConfig::default();

        assert_eq!(config.request_timeout, ADMIN_DISCOVERY_REQUEST_TIMEOUT);
        assert_eq!(config.connect_timeout, ADMIN_DISCOVERY_CONNECT_TIMEOUT);
        assert_eq!(config.user_agent, ADMIN_DISCOVERY_USER_AGENT);
        build_admin_discovery_http_client(AdminDiscoveryHttpConfig {
            request_timeout: Duration::from_millis(20),
            connect_timeout: Duration::from_millis(20),
            user_agent: ADMIN_DISCOVERY_USER_AGENT,
        })
        .expect("build Admin discovery HTTP client");
    }

    #[tokio::test]
    async fn fetch_admin_discovery_returns_error_for_request_failure() {
        let result = fetch_admin_discovery_client_templates("http://[::1").await;

        assert_data_access_error(result, "Admin discovery request failed");
    }

    #[tokio::test]
    async fn fetch_admin_discovery_aggregates_paginated_clients() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/discovery/clients"))
            .and(wiremock::matchers::query_param("limit", "100"))
            .and(wiremock::matchers::query_param("offset", "0"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clients": [admin_discovery_file_client("cursor")],
                "page": {
                    "limit": 100,
                    "offset": 0,
                    "total": 2,
                    "hasMore": true,
                    "nextOffset": 1
                }
            })))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/discovery/clients"))
            .and(wiremock::matchers::query_param("limit", "100"))
            .and(wiremock::matchers::query_param("offset", "1"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clients": [admin_discovery_file_client("vscode")],
                "page": {
                    "limit": 100,
                    "offset": 1,
                    "total": 2,
                    "hasMore": false
                }
            })))
            .mount(&server)
            .await;

        let templates = fetch_admin_discovery_client_templates_with_config(
            &server.uri(),
            AdminDiscoveryHttpConfig {
                request_timeout: Duration::from_secs(1),
                connect_timeout: Duration::from_secs(1),
                user_agent: ADMIN_DISCOVERY_USER_AGENT,
            },
        )
        .await
        .expect("fetch paginated Admin discovery clients");

        let identifiers = templates
            .iter()
            .map(|template| template.identifier.as_str())
            .collect::<Vec<_>>();
        assert_eq!(identifiers, vec!["cursor", "vscode"]);
    }

    #[tokio::test]
    async fn fetch_admin_discovery_uses_total_when_has_more_is_absent() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/discovery/clients"))
            .and(wiremock::matchers::query_param("limit", "100"))
            .and(wiremock::matchers::query_param("offset", "0"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clients": [admin_discovery_file_client("cursor")],
                "page": {
                    "limit": 1,
                    "offset": 0,
                    "total": 2
                }
            })))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/discovery/clients"))
            .and(wiremock::matchers::query_param("limit", "100"))
            .and(wiremock::matchers::query_param("offset", "1"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clients": [admin_discovery_file_client("vscode")],
                "page": {
                    "limit": 1,
                    "offset": 1,
                    "total": 2
                }
            })))
            .mount(&server)
            .await;

        let templates = fetch_admin_discovery_client_templates_with_config(
            &server.uri(),
            AdminDiscoveryHttpConfig {
                request_timeout: Duration::from_secs(1),
                connect_timeout: Duration::from_secs(1),
                user_agent: ADMIN_DISCOVERY_USER_AGENT,
            },
        )
        .await
        .expect("fetch Admin discovery clients with total pagination");

        let identifiers = templates
            .iter()
            .map(|template| template.identifier.as_str())
            .collect::<Vec<_>>();
        assert_eq!(identifiers, vec!["cursor", "vscode"]);
    }

    #[tokio::test]
    async fn fetch_admin_discovery_rejects_non_advancing_pagination() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/discovery/clients"))
            .and(wiremock::matchers::query_param("limit", "100"))
            .and(wiremock::matchers::query_param("offset", "0"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clients": [admin_discovery_file_client("cursor")],
                "page": {
                    "limit": 100,
                    "offset": 0,
                    "total": 2,
                    "hasMore": true
                }
            })))
            .mount(&server)
            .await;

        let result = fetch_admin_discovery_client_templates_with_config(
            &server.uri(),
            AdminDiscoveryHttpConfig {
                request_timeout: Duration::from_secs(1),
                connect_timeout: Duration::from_secs(1),
                user_agent: ADMIN_DISCOVERY_USER_AGENT,
            },
        )
        .await;

        assert_data_access_error(result, "Admin discovery pagination did not provide a next offset");
    }

    #[test]
    fn skips_admin_clients_without_local_config_file_paths() {
        let payload = serde_json::json!({
            "clients": [
                {
                    "identifier": "runtime-only",
                    "displayName": "Runtime Only",
                    "config": {
                        "kind": "none"
                    }
                },
                {
                    "identifier": "missing-paths",
                    "displayName": "Missing Paths",
                    "config": {
                        "kind": "file",
                        "file": {
                            "format": "json",
                            "container": {
                                "type": "standard",
                                "keys": ["mcpServers"]
                            }
                        }
                    }
                }
            ],
            "page": {
                "limit": 50,
                "offset": 0,
                "total": 2
            }
        });

        let templates = map_admin_discovery_clients(payload).expect("map discovery clients");

        assert!(templates.is_empty());
    }
}
