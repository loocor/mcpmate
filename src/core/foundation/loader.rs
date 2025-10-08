//! Database configuration loader for core MCPMate
//! Contains functions for loading configuration from the database - completely independent from core

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::{
    config::{
        database::Database,
        server::{ServerEnabledService, get_server_args, get_server_env},
    },
    core::profile::merge::ProfileMerger,
    core::{
        models::{Config, MCPServerConfig},
        proxy::args::StartupMode,
    },
};

/// Unified function to load servers from active profile
/// Returns both Server list and Config formats
pub async fn load_servers_from_active_profile(
    db: &Database
) -> anyhow::Result<(Vec<crate::config::models::Server>, Config)> {
    // Use ProfileMerger's merge logic
    let merger = ProfileMerger::new(Arc::new(db.clone()));
    let merge_result = merger
        .merge_all_configs()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to merge configurations: {}", e))?;

    // Convert to Server list
    let mut servers = Vec::new();
    for server_config in &merge_result.servers {
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &server_config.server_id).await {
            servers.push(server);
        }
    }

    // Convert to Config format
    let mut mcp_servers = std::collections::HashMap::new();
    for server_config in &merge_result.servers {
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &server_config.server_id).await {
            // Get server args
            let args = get_server_args(&db.pool, &server_config.server_id)
                .await
                .context("Failed to get server arguments")?;

            // Get server env
            let env = if let Some(server_id) = &server.id {
                let env_map = get_server_env(&db.pool, server_id)
                    .await
                    .context("Failed to get server environment variables")?;

                if env_map.is_empty() { None } else { Some(env_map) }
            } else {
                None
            };

            // Convert args to Option<Vec<String>>
            let args_strings = if args.is_empty() {
                None
            } else {
                Some(args.into_iter().map(|arg| arg.arg_value).collect())
            };

            // Load default HTTP headers (if any)
            let headers = if let Some(id) = &server.id {
                match crate::config::server::get_server_headers(&db.pool, id).await {
                    Ok(map) if !map.is_empty() => Some(map),
                    _ => None,
                }
            } else {
                None
            };

            // Create MCPServerConfig
            let server_config = MCPServerConfig {
                kind: server.server_type,
                command: server.command,
                args: args_strings,
                url: server.url,
                env,
                headers,
            };

            // Use server_id as key instead of server_name
            if let Some(server_id) = &server.id {
                mcp_servers.insert(server_id.clone(), server_config);
            }
        }
    }

    let config = Config {
        mcp_servers,
        pagination: None,
    };

    tracing::info!("Loaded {} servers from active profile (unified loader)", servers.len());

    Ok((servers, config))
}

/// Get enabled servers from all active profile using unified service
async fn get_enabled_servers_from_active_profile(
    pool: &sqlx::Pool<sqlx::Sqlite>
) -> anyhow::Result<Vec<crate::config::models::Server>> {
    // Use the unified server enabled service
    let service = ServerEnabledService::new(pool.clone());
    let servers = service.get_all_enabled_servers().await?;
    Ok(servers)
}

/// Load the MCP server configuration from the database with startup parameters
pub async fn load_server_config_with_params(
    db: &Database,
    startup_mode: &StartupMode,
) -> Result<Config> {
    tracing::info!(
        "Loading server configuration from database with startup mode: {:?}",
        startup_mode
    );

    // Get enabled servers based on startup mode
    let servers = match startup_mode {
        StartupMode::Minimal | StartupMode::NoProfile => {
            tracing::info!("Minimal/NoProfile mode: not loading any servers");
            Vec::new()
        }
        StartupMode::Default => {
            tracing::info!("Default mode: loading servers from all active profile");
            get_enabled_servers_from_active_profile(&db.pool)
                .await
                .context("Failed to get enabled servers from active profile")?
        }
        StartupMode::SpecificProfile(profile_ids) => {
            tracing::info!("Specific profile mode: loading servers from profile: {:?}", profile_ids);
            // Use unified service for specific profile
            let service = ServerEnabledService::new(db.pool.clone());
            service
                .get_enabled_servers_from_profile(profile_ids)
                .await
                .context("Failed to get enabled servers from specific profile")?
        }
    };

    // Create a new Config object with default pagination settings
    let mut config = Config {
        mcp_servers: HashMap::new(),
        pagination: None, // Use default pagination settings
    };

    // Convert each server to MCPServerConfig
    for server in servers {
        // Get server arguments
        let args = if let Some(id) = &server.id {
            let server_args = get_server_args(&db.pool, id)
                .await
                .context("Failed to get server arguments")?;

            if server_args.is_empty() {
                None
            } else {
                // Sort arguments by index and collect values
                let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                sorted_args.sort_by_key(|arg| arg.arg_index);
                Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect())
            }
        } else {
            None
        };

        // Get server environment variables
        let env = if let Some(id) = &server.id {
            let env_map = get_server_env(&db.pool, id)
                .await
                .context("Failed to get server environment variables")?;

            if env_map.is_empty() { None } else { Some(env_map) }
        } else {
            None
        };

        // Load default HTTP headers (if any)
        let headers = if let Some(id) = &server.id {
            match crate::config::server::get_server_headers(&db.pool, id).await {
                Ok(map) if !map.is_empty() => Some(map),
                _ => None,
            }
        } else {
            None
        };

        // Create MCPServerConfig
        let server_config = MCPServerConfig {
            kind: server.server_type,
            command: server.command,
            args,
            url: server.url,
            env,
            headers,
        };

        // Add to config using server_id as key
        if let Some(server_id) = &server.id {
            config.mcp_servers.insert(server_id.clone(), server_config);
        }
    }

    tracing::info!(
        "Successfully loaded {} enabled servers from database using core loader (mode: {:?})",
        config.mcp_servers.len(),
        startup_mode
    );

    // Publish ConfigReloaded event using core events
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ConfigReloaded);
    tracing::info!("Published ConfigReloaded event using core events");

    Ok(config)
}

/// Load the MCP server configuration from the database (legacy function for backward compatibility)
pub async fn load_server_config(db: &Database) -> Result<Config> {
    load_server_config_with_params(db, &StartupMode::Default).await
}
