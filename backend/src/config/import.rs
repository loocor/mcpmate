// Configuration import for MCPMate
// Contains functions for importing configuration from JSON files to database

use crate::core::cache::RedbCacheManager;
use crate::core::models::Config;
use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::{fs::File, path::Path, sync::Arc};

/// Import configuration from JSON files to database
pub async fn import_from_mcp_config(
    pool: &Pool<Sqlite>,
    mcp_config_path: &Path,
) -> Result<()> {
    tracing::info!("Importing configuration from JSON files to database");

    // Check if database already has server configurations
    let has_server_configs = match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_config")
        .fetch_one(pool)
        .await
    {
        Ok(count) => count > 0,
        Err(e) => {
            tracing::error!("Failed to check if server_config table has data: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to check if server_config table has data: {}",
                e
            ));
        }
    };

    // If database already has server configurations, skip import
    if has_server_configs {
        tracing::info!("Database already has server configurations, skipping import");
        return Ok(());
    }

    // Check if configuration files exist
    if !mcp_config_path.exists() {
        tracing::warn!(
            "MCP configuration file not found at {}, skipping import",
            mcp_config_path.display()
        );
        return Ok(());
    }

    // Load MCP server configuration
    let mcp_config = match load_mcp_config_from_file(mcp_config_path) {
        Ok(config) => {
            tracing::info!(
                "Successfully loaded MCP configuration from {}",
                mcp_config_path.display()
            );
            config
        }
        Err(e) => {
            tracing::error!("Failed to load MCP configuration: {}", e);
            return Err(anyhow::anyhow!("Failed to load MCP configuration: {}", e));
        }
    };

    // Initialize Redb cache manager using global singleton
    let redb_cache =
        RedbCacheManager::global().map_err(|e| anyhow::anyhow!(format!("Failed to init REDB cache: {}", e)))?;

    // Convert Config to ServersImportConfig map
    let mut items: HashMap<String, crate::api::models::server::ServersImportConfig> = HashMap::new();
    for (name, sc) in mcp_config.mcp_servers.into_iter() {
        items.insert(
            name,
            crate::api::models::server::ServersImportConfig {
                kind: sc.kind.as_str().to_string(),
                command: sc.command,
                args: sc.args,
                url: sc.url,
                env: sc.env,
                headers: None,
                registry_server_id: None,
                meta: None,
            },
        );
    }

    // Use unified import core (by_name + by_fingerprint, skip on conflict)
    let dummy_pool = Arc::new(tokio::sync::Mutex::new(crate::core::pool::UpstreamConnectionPool::new(
        Arc::new(Default::default()),
        None,
    )));
    let _ = crate::config::server::import_batch(
        pool,
        &dummy_pool,
        &redb_cache,
        items,
        crate::config::server::ImportOptions {
            by_name: true,
            by_fingerprint: true,
            conflict_policy: crate::config::server::ConflictPolicy::Skip,
            preview: false,
            target_profile: None,
        },
    )
    .await?;

    tracing::info!("Configuration import completed successfully");
    Ok(())
}

/// Load MCP configuration from a file
fn load_mcp_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
    use std::io::Read;

    // Read the file content as a string
    let mut content = String::new();
    let mut file = File::open(path.as_ref())
        .with_context(|| format!("Failed to open config file: {}", path.as_ref().display()))?;
    file.read_to_string(&mut content)
        .with_context(|| format!("Failed to read config file content: {}", path.as_ref().display()))?;

    // Try to parse the JSON
    match serde_json::from_str::<Config>(&content) {
        Ok(config) => {
            tracing::debug!("Successfully parsed config file");
            Ok(config)
        }
        Err(e) => {
            tracing::error!("Failed to parse config file: {}", e);

            // Try to parse as Value to get more information
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    tracing::debug!("File is valid JSON, but doesn't match Config struct: {:?}", value);
                }
                Err(e) => {
                    tracing::error!("File is not valid JSON: {}", e);
                }
            }

            Err(anyhow::anyhow!("Failed to parse config file: {}", e))
        }
    }
}
