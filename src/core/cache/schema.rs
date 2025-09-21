//! Redb schema definitions for the cache system

use redb::{MultimapTableDefinition, TableDefinition};

/// Server data table: server_id -> serialized CachedServerData
pub const SERVERS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("servers");

/// Tools table: (server_id, tool_name) -> serialized CachedToolInfo
pub const TOOLS_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("tools");

/// Resources table: (server_id, resource_uri) -> serialized CachedResourceInfo
pub const RESOURCES_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("resources");

/// Prompts table: (server_id, prompt_name) -> serialized CachedPromptInfo
pub const PROMPTS_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("prompts");

/// Resource templates table: (server_id, template_uri) -> serialized CachedResourceTemplateInfo
pub const RESOURCE_TEMPLATES_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("resource_templates");

/// Fingerprints table: server_id -> serialized MCPServerFingerprint
pub const FINGERPRINTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("fingerprints");

/// Instance metadata table: instance_id -> serialized InstanceMetadata
pub const INSTANCE_METADATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("instance_metadata");

/// Cache statistics table: "stats" -> serialized CacheStats
pub const CACHE_STATS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("cache_stats");

/// Server-to-tools mapping (multimap): server_id -> [tool_name, ...]
pub const SERVER_TOOLS_INDEX: MultimapTableDefinition<&str, &str> = MultimapTableDefinition::new("server_tools_index");

/// Server-to-resources mapping (multimap): server_id -> [resource_uri, ...]
pub const SERVER_RESOURCES_INDEX: MultimapTableDefinition<&str, &str> =
    MultimapTableDefinition::new("server_resources_index");

/// Server-to-prompts mapping (multimap): server_id -> [prompt_name, ...]
pub const SERVER_PROMPTS_INDEX: MultimapTableDefinition<&str, &str> =
    MultimapTableDefinition::new("server_prompts_index");

/// Server-to-resource-templates mapping (multimap): server_id -> [template_uri, ...]
pub const SERVER_RESOURCE_TEMPLATES_INDEX: MultimapTableDefinition<&str, &str> =
    MultimapTableDefinition::new("server_resource_templates_index");

/// Instance metadata for connection pool integration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstanceMetadata {
    pub instance_id: String,
    pub server_id: String,
    pub instance_type: crate::core::cache::types::InstanceType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub ttl: std::time::Duration,
    pub access_count: u64,
    pub visible_to_downstream: bool,
}

/// All table definitions for easy iteration during database initialization
pub const ALL_TABLES: &[&str] = &[
    "servers",
    "tools",
    "resources",
    "prompts",
    "resource_templates",
    "fingerprints",
    "instance_metadata",
    "cache_stats",
];

/// All multimap table definitions
pub const ALL_MULTIMAPS: &[&str] = &[
    "server_tools_index",
    "server_resources_index",
    "server_prompts_index",
    "server_resource_templates_index",
];

/// Database version for migration compatibility
pub const CACHE_DB_VERSION: u32 = 1;

/// Database metadata key for version tracking
pub const VERSION_KEY: &str = "db_version";

/// Database metadata key for creation timestamp
pub const CREATED_AT_KEY: &str = "created_at";

/// Database metadata key for last migration
pub const LAST_MIGRATION_KEY: &str = "last_migration";
