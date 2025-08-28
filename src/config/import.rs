// Configuration import for MCPMate
// Contains functions for importing configuration from JSON files to database

use std::{fs::File, path::Path, sync::Arc};

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    config::models::{Server, ServerMeta},
    config::server::{upsert_server, upsert_server_args, upsert_server_env, upsert_server_meta},
    core::models::Config,
};

// Imports for capability discovery and dual-write
use crate::core::cache::RedbCacheManager;

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

    // Migrate server configurations
    for (name, server_config) in mcp_config.mcp_servers {
        // Keep a full clone for capability discovery before we consume args/env below
        let server_cfg_for_discovery = server_config.clone();
        // Create server configuration
        let server = Server {
            id: None,
            name: name.clone(),
            server_type: server_config.kind,
            command: server_config.command.clone(),
            url: server_config.url.clone(),
            transport_type: server_config.transport_type.map(|t| match t {
                crate::common::server::TransportType::Stdio => crate::common::server::TransportType::Stdio,
                crate::common::server::TransportType::Sse => crate::common::server::TransportType::Sse,
                crate::common::server::TransportType::StreamableHttp => {
                    crate::common::server::TransportType::StreamableHttp
                }
            }),
            capabilities: None,
            enabled: crate::common::status::EnabledStatus::Enabled,
            created_at: None,
            updated_at: None,
        };

        // Insert server configuration
        let server_id = match upsert_server(pool, &server).await {
            Ok(id) => {
                tracing::info!("Successfully imported server '{}' (ID: {})", name, id);
                id
            }
            Err(e) => {
                tracing::error!("Failed to import server '{}': {}", name, e);
                continue;
            }
        };

        // Insert server arguments if any
        if let Some(args) = server_config.args {
            match upsert_server_args(pool, &server_id, &args).await {
                Ok(_) => tracing::info!("Successfully imported {} arguments for server '{}'", args.len(), name),
                Err(e) => {
                    tracing::error!("Failed to import arguments for server '{}': {}", name, e)
                }
            }
        }

        // Insert server environment variables if any
        if let Some(env) = server_config.env {
            match upsert_server_env(pool, &server_id, &env).await {
                Ok(_) => tracing::info!(
                    "Successfully imported {} environment variables for server '{}'",
                    env.len(),
                    name
                ),
                Err(e) => tracing::error!("Failed to import environment variables for server '{}': {}", name, e),
            }
        }

        // Create basic server metadata
        let meta = ServerMeta {
            id: None,
            server_id,
            description: Some(format!("Imported from {}", mcp_config_path.display())),
            author: None,
            website: None,
            repository: None,
            category: None,
            recommended_scenario: None,
            rating: None,
            created_at: None,
            updated_at: None,
        };

        // Insert server metadata
        match upsert_server_meta(pool, &meta).await {
            Ok(_) => tracing::info!("Successfully created metadata for server '{}'", name),
            Err(e) => tracing::error!("Failed to create metadata for server '{}': {}", name, e),
        }

        // Use unified capability manager for import
        let dummy_pool = Arc::new(tokio::sync::Mutex::new(crate::core::pool::UpstreamConnectionPool::new(
            Arc::new(Default::default()),
            None,
        )));

        let capability_manager = crate::config::server::capabilities::CapabilityManager::new(
            Arc::new(pool.clone()),
            redb_cache.clone(),
            dummy_pool,
        );

        let strategy = crate::config::server::capabilities::SyncStrategy::FromConfig(
            server_cfg_for_discovery.clone(),
            server.server_type,
        );

        if let Err(e) = capability_manager
            .sync_server_capabilities(&meta.server_id, &name, strategy)
            .await
        {
            tracing::warn!("Capability discovery failed for '{}': {}", name, e);
        }
    }

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
