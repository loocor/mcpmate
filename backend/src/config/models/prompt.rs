// Prompt models for MCPMate
// Contains data models for prompt configuration

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
    /// Prompt origin name returned by the upstream server.
    pub prompt_name: String,
    /// Current external name used for management display.
    pub unique_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub state: String,
    pub state_generation: i64,
}

/// Prompt configuration update model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUpdate {
    /// Whether the prompt is enabled
    pub enabled: bool,
}
