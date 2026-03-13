// Resource models for MCPMate
// Contains data models for resource configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Profile resource association model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileResource {
    /// Unique ID (UUID)
    pub id: Option<String>,
    /// Profile ID
    pub profile_id: String,
    /// Server ID
    pub server_id: String,
    /// Server name (for human identification during development)
    pub server_name: String,
    /// Resource URI (original URI from upstream server)
    pub resource_uri: String,
    /// Whether the resource is enabled in this profile
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Resource configuration update model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUpdate {
    /// Whether the resource is enabled
    pub enabled: bool,
}
