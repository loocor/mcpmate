use sqlx::{Pool, Sqlite};

use anyhow::Result;

use super::import;

pub const DEFAULT_MCP_CONFIG_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/mcp.json"));

pub async fn seed_default_servers(pool: &Pool<Sqlite>) -> Result<()> {
    import::import_from_mcp_config_content(pool, DEFAULT_MCP_CONFIG_JSON).await
}
