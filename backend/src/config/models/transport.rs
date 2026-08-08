use std::collections::{BTreeMap, HashMap};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{common::server::ServerType, core::models::MCPServerConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigValue {
    Literal { value: String },
    SecretRef { alias: String },
}

impl ConfigValue {
    /// Parses a persisted runtime value into its structured representation.
    pub fn from_runtime_value(value: String) -> Self {
        match value
            .strip_prefix("[[secret:")
            .and_then(|value| value.strip_suffix("]]"))
            .filter(|alias| !alias.is_empty())
        {
            Some(alias) => Self::SecretRef {
                alias: alias.to_string(),
            },
            None => Self::Literal { value },
        }
    }

    pub fn runtime_value(&self) -> String {
        match self {
            Self::Literal { value } => value.clone(),
            Self::SecretRef { alias } => format!("[[secret:{alias}]]"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HttpTransportKind {
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerTransportDraft {
    Stdio {
        command: Option<String>,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, ConfigValue>,
    },
    Http {
        protocol: HttpTransportKind,
        endpoint: Option<String>,
        #[serde(default)]
        headers: BTreeMap<String, ConfigValue>,
    },
    Unrecognized {
        declared_type: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, ConfigValue>,
    },
    Sse {
        endpoint: Url,
        headers: BTreeMap<String, ConfigValue>,
    },
    StreamableHttp {
        endpoint: Url,
        headers: BTreeMap<String, ConfigValue>,
    },
}

impl ValidatedTransport {
    pub fn server_type(&self) -> ServerType {
        match self {
            Self::Stdio { .. } => ServerType::Stdio,
            Self::Sse { .. } => ServerType::Sse,
            Self::StreamableHttp { .. } => ServerType::StreamableHttp,
        }
    }

    pub fn runtime_headers(&self) -> HashMap<String, String> {
        match self {
            Self::Stdio { .. } => HashMap::new(),
            Self::Sse { headers, .. } | Self::StreamableHttp { headers, .. } => headers
                .iter()
                .map(|(key, value)| (key.clone(), value.runtime_value()))
                .collect(),
        }
    }

    pub fn to_mcp_config(&self) -> MCPServerConfig {
        match self {
            Self::Stdio { command, args, env } => MCPServerConfig {
                source_fingerprint: None,
                kind: ServerType::Stdio,
                command: Some(command.clone()),
                args: (!args.is_empty()).then(|| args.clone()),
                url: None,
                env: config_values_to_runtime(env),
                headers: None,
            },
            Self::Sse { endpoint, headers } => MCPServerConfig {
                source_fingerprint: None,
                kind: ServerType::Sse,
                command: None,
                args: None,
                url: Some(endpoint.to_string()),
                env: None,
                headers: config_values_to_runtime(headers),
            },
            Self::StreamableHttp { endpoint, headers } => MCPServerConfig {
                source_fingerprint: None,
                kind: ServerType::StreamableHttp,
                command: None,
                args: None,
                url: Some(endpoint.to_string()),
                env: None,
                headers: config_values_to_runtime(headers),
            },
        }
    }
}

fn config_values_to_runtime(values: &BTreeMap<String, ConfigValue>) -> Option<HashMap<String, String>> {
    (!values.is_empty()).then(|| {
        values
            .iter()
            .map(|(name, value)| (name.clone(), value.runtime_value()))
            .collect()
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDiagnostic {
    pub code: &'static str,
    pub field: String,
}

impl ServerTransportDraft {
    pub fn validate(&self) -> Result<ValidatedTransport, Vec<FieldDiagnostic>> {
        match self {
            Self::Stdio { command, args, env } => validate_stdio(command, args, env),
            Self::Http {
                protocol,
                endpoint,
                headers,
            } => validate_http(*protocol, endpoint, headers),
            Self::Unrecognized { .. } => Err(vec![FieldDiagnostic {
                code: "transport_unrecognized",
                field: "transport".into(),
            }]),
        }
    }
}

fn validate_stdio(
    command: &Option<String>,
    args: &[String],
    env: &BTreeMap<String, ConfigValue>,
) -> Result<ValidatedTransport, Vec<FieldDiagnostic>> {
    let mut diagnostics = config_value_diagnostics("env", env);
    let command = command.as_deref().filter(|command| !command.trim().is_empty());
    if command.is_none() {
        diagnostics.push(FieldDiagnostic {
            code: "stdio_command_missing",
            field: "command".into(),
        });
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(ValidatedTransport::Stdio {
        command: command.expect("checked above").to_owned(),
        args: args.to_vec(),
        env: env.clone(),
    })
}

fn validate_http(
    protocol: HttpTransportKind,
    endpoint: &Option<String>,
    headers: &BTreeMap<String, ConfigValue>,
) -> Result<ValidatedTransport, Vec<FieldDiagnostic>> {
    let mut diagnostics = config_value_diagnostics("headers", headers);
    let endpoint = endpoint.as_deref().filter(|endpoint| !endpoint.trim().is_empty());
    if endpoint.is_none() {
        diagnostics.push(FieldDiagnostic {
            code: "remote_url_missing",
            field: "endpoint".into(),
        });
    }
    let parsed_endpoint = endpoint.and_then(|endpoint| Url::parse(endpoint).ok());
    if endpoint.is_some()
        && !parsed_endpoint
            .as_ref()
            .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
    {
        diagnostics.push(FieldDiagnostic {
            code: "url_invalid",
            field: "endpoint".into(),
        });
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let endpoint = parsed_endpoint.expect("checked above");
    match protocol {
        HttpTransportKind::Sse => Ok(ValidatedTransport::Sse {
            endpoint,
            headers: headers.clone(),
        }),
        HttpTransportKind::StreamableHttp => Ok(ValidatedTransport::StreamableHttp {
            endpoint,
            headers: headers.clone(),
        }),
    }
}

fn config_value_diagnostics(
    scope: &str,
    values: &BTreeMap<String, ConfigValue>,
) -> Vec<FieldDiagnostic> {
    values
        .iter()
        .filter_map(|(name, value)| match value {
            ConfigValue::SecretRef { alias } if alias.trim().is_empty() => Some(FieldDiagnostic {
                code: "secret_alias_missing",
                field: format!("{scope}.{name}"),
            }),
            ConfigValue::Literal { .. } | ConfigValue::SecretRef { .. } => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ConfigValue, HttpTransportKind, ServerTransportDraft, ValidatedTransport};
    use std::collections::BTreeMap;

    #[test]
    fn rejects_blank_stdio_command_at_the_command_field() {
        let draft = ServerTransportDraft::Stdio {
            command: Some("  ".into()),
            args: vec![],
            env: BTreeMap::new(),
        };

        assert_eq!(
            draft.validate(),
            Err(vec![super::FieldDiagnostic {
                code: "stdio_command_missing",
                field: "command".into(),
            }]),
        );
    }

    #[test]
    fn validates_streamable_http_without_interpreting_secret_references() {
        let mut headers = BTreeMap::new();
        headers.insert(
            "authorization".into(),
            ConfigValue::SecretRef {
                alias: "access-token".into(),
            },
        );
        let draft = ServerTransportDraft::Http {
            protocol: HttpTransportKind::StreamableHttp,
            endpoint: Some("https://example.test/mcp".into()),
            headers,
        };

        let validated = draft.validate().expect("validate HTTP draft");
        assert!(matches!(validated, ValidatedTransport::StreamableHttp { .. }));
    }

    #[test]
    fn rejects_an_empty_secret_alias_at_its_config_field() {
        let mut env = BTreeMap::new();
        env.insert("TOKEN".into(), ConfigValue::SecretRef { alias: " ".into() });
        let draft = ServerTransportDraft::Stdio {
            command: Some("echo".into()),
            args: vec![],
            env,
        };

        assert_eq!(
            draft.validate(),
            Err(vec![super::FieldDiagnostic {
                code: "secret_alias_missing",
                field: "env.TOKEN".into(),
            }]),
        );
    }

    #[test]
    fn projects_validated_transport_without_flat_field_selection() {
        let mut env = BTreeMap::new();
        env.insert("TOKEN".into(), ConfigValue::SecretRef { alias: "token".into() });
        let validated = ServerTransportDraft::Stdio {
            command: Some("echo".into()),
            args: vec!["hello".into()],
            env,
        }
        .validate()
        .expect("validate stdio");

        let config = validated.to_mcp_config();
        assert_eq!(config.command.as_deref(), Some("echo"));
        assert_eq!(config.url, None);
        assert_eq!(config.env.unwrap().get("TOKEN"), Some(&"[[secret:token]]".into()));
    }
}
