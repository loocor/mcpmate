//! Derived SQLite capability-index row models.
//!
//! These compact values exist only to maintain the searchable identity and
//! Profile-association tables. Standard MCP payloads remain authoritative in
//! the transactional capability catalog.

use chrono::{DateTime, Utc};
use rmcp::model::Icon;

#[derive(Debug, Clone)]
pub struct CachedToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema_json: String,
    pub output_schema_json: Option<String>,
    pub unique_name: Option<String>,
    pub icons: Option<Vec<Icon>>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

impl CachedToolInfo {
    pub fn input_schema(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.input_schema_json)
    }

    pub fn output_schema(&self) -> Option<serde_json::Value> {
        self.output_schema_json
            .as_ref()
            .and_then(|schema| serde_json::from_str::<serde_json::Value>(schema).ok())
    }
}

#[derive(Debug, Clone)]
pub struct CachedResourceInfo {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub icons: Option<Vec<Icon>>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CachedPromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
    pub icons: Option<Vec<Icon>>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CachedResourceTemplateInfo {
    pub uri_template: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
