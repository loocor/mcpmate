// Database migration for MCPMate
// Contains functions for migrating configuration from files to database
// This file is temporary and can be removed after migration is complete

use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    config::models::{Server, ServerMeta},
    config::server::{upsert_server, upsert_server_args, upsert_server_env, upsert_server_meta},
    core::models::Config,
};

// Imports for capability discovery and dual-write
use crate::common::server::ServerType;
use crate::core::cache::{
    CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedToolInfo, RedbCacheManager,
    manager::CacheConfig,
};

// use crate::config::server::store_dual_write;
// imported where used via fully-qualified path
use crate::core::transport::{TransportType, connect_http_server, connect_server_simple};

/// Discover capabilities and perform dual-write (SQLite shadow tables + REDB)
async fn discover_and_dual_write(
    pool: &Pool<Sqlite>,
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    server_type: ServerType,
    server_config: &crate::core::models::MCPServerConfig,
) -> anyhow::Result<()> {
    // Connect using unified transport
    let (service, tools, capabilities, _pid) = match server_type {
        ServerType::Stdio => {
            connect_server_simple(server_name, server_config, ServerType::Stdio, TransportType::Stdio).await?
        }
        ServerType::Sse => connect_http_server(server_name, server_config, TransportType::Sse)
            .await
            .map(|(s, t, c)| (s, t, c, None))?,
        ServerType::StreamableHttp => connect_http_server(server_name, server_config, TransportType::StreamableHttp)
            .await
            .map(|(s, t, c)| (s, t, c, None))?,
    };

    // Tools
    let mut cached_tools: Vec<CachedToolInfo> = Vec::new();
    for t in &tools {
        let schema = t.schema_as_json_value();
        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        cached_tools.push(CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            unique_name: None,
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }

    // Prompts
    let mut cached_prompts: Vec<CachedPromptInfo> = Vec::new();
    if capabilities.as_ref().and_then(|c| c.prompts.as_ref()).is_some() {
        if let Ok(list_result) = service.list_prompts(None).await {
            for p in list_result.prompts {
                let converted_args = p
                    .arguments
                    .unwrap_or_default()
                    .into_iter()
                    .map(|arg| crate::core::cache::PromptArgument {
                        name: arg.name,
                        description: arg.description,
                        required: arg.required.unwrap_or(false),
                    })
                    .collect();
                cached_prompts.push(CachedPromptInfo {
                    name: p.name,
                    description: p.description,
                    arguments: converted_args,
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
        }
    }

    // Resources & Templates
    let mut cached_resources: Vec<CachedResourceInfo> = Vec::new();
    let mut cached_templates: Vec<CachedResourceTemplateInfo> = Vec::new();
    if capabilities.as_ref().and_then(|c| c.resources.as_ref()).is_some() {
        // Resources (single page; many servers return full list; pagination可后续扩展)
        if let Ok(list_result) = service.list_resources(None).await {
            for r in list_result.resources {
                cached_resources.push(CachedResourceInfo {
                    uri: r.uri.clone(),
                    name: Some(r.name.clone()),
                    description: r.description.clone(),
                    mime_type: r.mime_type.clone(),
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
        }
        // Resource templates (分页)
        let mut cursor = None;
        while let Ok(result) = service
            .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
            .await
        {
            for t in result.resource_templates {
                cached_templates.push(CachedResourceTemplateInfo {
                    uri_template: t.uri_template.clone(),
                    name: Some(t.name.clone()),
                    description: t.description.clone(),
                    mime_type: t.mime_type.clone(),
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
    }

    // Dual-write through shared helper
    crate::config::server::store_dual_write(
        pool,
        redb,
        server_id,
        server_name,
        cached_tools,
        cached_resources,
        cached_prompts,
        cached_templates,
    )
    .await?;

    Ok(())
}

/// Migrate configuration from files to database
pub async fn migrate_from_files(
    pool: &Pool<Sqlite>,
    mcp_config_path: &Path,
) -> Result<()> {
    tracing::info!("Migrating configuration from files to database");

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

    // If database already has server configurations, skip migration
    if has_server_configs {
        tracing::info!("Database already has server configurations, skipping migration");
        return Ok(());
    }

    // Check if configuration files exist
    if !mcp_config_path.exists() {
        tracing::warn!(
            "MCP configuration file not found at {}, skipping migration",
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

    // Initialize Redb cache manager (match API default path logic)
    let redb_cache_path = if let Ok(p) = std::env::var("MCPMATE_REDB_CACHE_PATH") {
        std::path::PathBuf::from(p)
    } else {
        std::env::temp_dir().join("mcpmate").join("cache.redb")
    };
    let redb_cache = RedbCacheManager::new(redb_cache_path, CacheConfig::default())
        .map_err(|e| anyhow::anyhow!(format!("Failed to init REDB cache: {}", e)))?;

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
            enabled: crate::common::status::EnabledStatus::Enabled,
            created_at: None,
            updated_at: None,
        };

        // Insert server configuration
        let server_id = match upsert_server(pool, &server).await {
            Ok(id) => {
                tracing::info!("Successfully migrated server '{}' (ID: {})", name, id);
                id
            }
            Err(e) => {
                tracing::error!("Failed to migrate server '{}': {}", name, e);
                continue;
            }
        };

        // Insert server arguments if any
        if let Some(args) = server_config.args {
            match upsert_server_args(pool, &server_id, &args).await {
                Ok(_) => tracing::info!("Successfully migrated {} arguments for server '{}'", args.len(), name),
                Err(e) => {
                    tracing::error!("Failed to migrate arguments for server '{}': {}", name, e)
                }
            }
        }

        // Insert server environment variables if any
        if let Some(env) = server_config.env {
            match upsert_server_env(pool, &server_id, &env).await {
                Ok(_) => tracing::info!(
                    "Successfully migrated {} environment variables for server '{}'",
                    env.len(),
                    name
                ),
                Err(e) => tracing::error!("Failed to migrate environment variables for server '{}': {}", name, e),
            }
        }

        // Create basic server metadata
        let meta = ServerMeta {
            id: None,
            server_id,
            description: Some(format!("Migrated from {}", mcp_config_path.display())),
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

        // Discover capabilities and dual-write (SQLite + REDB)
        if let Err(e) = discover_and_dual_write(
            pool,
            &redb_cache,
            &meta.server_id,
            &name,
            server.server_type,
            &server_cfg_for_discovery,
        )
        .await
        {
            tracing::warn!("Capability discovery failed for '{}': {}", name, e);
        }
    }

    tracing::info!("Configuration migration completed successfully");
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
