// Prompt models for MCPMate
// Contains data models for prompt configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Profile prompt association model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfilePrompt {
    /// Unique ID (UUID)
    pub id: Option<String>,
    /// Profile ID
    pub profile_id: String,
    /// Server ID
    pub server_id: String,
    /// Server name (for human identification during development)
    pub server_name: String,
    /// Prompt name (original name from upstream server)
    pub prompt_name: String,
    /// Whether the prompt is enabled in this profile
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Prompt configuration update model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUpdate {
    /// Whether the prompt is enabled
    pub enabled: bool,
}
