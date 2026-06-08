use std::fmt;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKindInput {
    Generic,
    Token,
    ApiKey,
    Password,
    OAuthAccessToken,
    OAuthRefreshToken,
    UrlCredential,
    HeaderValue,
}

impl SecretKindInput {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Token => "token",
            Self::ApiKey => "api_key",
            Self::Password => "password",
            Self::OAuthAccessToken => "oauth_access_token",
            Self::OAuthRefreshToken => "oauth_refresh_token",
            Self::UrlCredential => "url_credential",
            Self::HeaderValue => "header_value",
        }
    }
}

impl fmt::Display for SecretKindInput {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for SecretKindInput {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "generic" => Ok(Self::Generic),
            "token" => Ok(Self::Token),
            "api_key" => Ok(Self::ApiKey),
            "password" => Ok(Self::Password),
            "oauth_access_token" => Ok(Self::OAuthAccessToken),
            "oauth_refresh_token" => Ok(Self::OAuthRefreshToken),
            "url_credential" => Ok(Self::UrlCredential),
            "header_value" => Ok(Self::HeaderValue),
            other => Err(anyhow::anyhow!("Unsupported secret kind '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecretCreateInput {
    pub alias: String,
    pub kind: SecretKindInput,
    pub value: String,
    pub label: Option<String>,
    pub origin: Option<SecretOriginInput>,
}

#[derive(Debug, Clone)]
pub struct SecretUpdateInput {
    pub alias: String,
    pub kind: Option<SecretKindInput>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub origin: Option<SecretOriginInput>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretOriginInput {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub server_kind: Option<String>,
    pub source: Option<String>,
    pub field_group: Option<String>,
    pub field_key: Option<String>,
    pub field_index: Option<i64>,
    pub field_path: Option<String>,
}

impl SecretOriginInput {
    pub(crate) fn is_empty(&self) -> bool {
        self.server_id.as_deref().unwrap_or_default().is_empty()
            && self.server_name.as_deref().unwrap_or_default().is_empty()
            && self.server_kind.as_deref().unwrap_or_default().is_empty()
            && self.source.as_deref().unwrap_or_default().is_empty()
            && self.field_group.as_deref().unwrap_or_default().is_empty()
            && self.field_key.as_deref().unwrap_or_default().is_empty()
            && self.field_index.is_none()
            && self.field_path.as_deref().unwrap_or_default().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretUsageLocationInput {
    StdioCommand,
    StdioArgument { index: usize },
    StdioEnv { name: String },
    StreamableHttpUrl,
    StreamableHttpHeader { name: String },
    OAuthToken,
}

impl SecretUsageLocationInput {
    pub fn binding_key(
        &self,
        server_id: &str,
    ) -> String {
        let (location_kind, location_name, location_index) = self.parts();
        format!(
            "{}|{}|{}|{}",
            server_id,
            location_kind,
            location_name.unwrap_or_default(),
            location_index.unwrap_or_default()
        )
    }

    pub(crate) fn parts(&self) -> (&'static str, Option<&str>, Option<i64>) {
        match self {
            Self::StdioCommand => ("stdio_command", None, None),
            Self::StdioArgument { index } => ("stdio_argument", None, Some(*index as i64)),
            Self::StdioEnv { name } => ("stdio_env", Some(name.as_str()), None),
            Self::StreamableHttpUrl => ("streamable_http_url", None, None),
            Self::StreamableHttpHeader { name } => ("streamable_http_header", Some(name.as_str()), None),
            Self::OAuthToken => ("oauth_token", None, None),
        }
    }

    pub(crate) fn from_parts(
        kind: &str,
        name: Option<String>,
        index: Option<i64>,
    ) -> Result<Self> {
        match kind {
            "stdio_command" => Ok(Self::StdioCommand),
            "stdio_argument" => Ok(Self::StdioArgument {
                index: index.unwrap_or_default() as usize,
            }),
            "stdio_env" => Ok(Self::StdioEnv {
                name: name.unwrap_or_default(),
            }),
            "streamable_http_url" => Ok(Self::StreamableHttpUrl),
            "streamable_http_header" => Ok(Self::StreamableHttpHeader {
                name: name.unwrap_or_default(),
            }),
            "oauth_token" => Ok(Self::OAuthToken),
            other => Err(anyhow::anyhow!("Unsupported secret usage location '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecretUsageUpsertInput {
    pub alias: String,
    pub server_id: String,
    pub location: SecretUsageLocationInput,
}

impl From<SecretUsageUpsertInput> for SecretUsageView {
    fn from(input: SecretUsageUpsertInput) -> Self {
        Self {
            alias: input.alias,
            server_id: input.server_id,
            location: input.location,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SecretMetadataView {
    pub alias: String,
    pub placeholder: String,
    pub kind: String,
    pub label: Option<String>,
    pub origin: Option<SecretOriginInput>,
    pub provider_id: String,
    pub provider_kind: String,
    pub version: u64,
    pub used_by_count: u64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecretUsageView {
    pub alias: String,
    pub server_id: String,
    pub location: SecretUsageLocationInput,
}
