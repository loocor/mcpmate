// Database configuration loader for MCPMate
// Contains functions for loading configuration from the database

use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::conf::{
    operations::{get_all_servers, get_server_args, get_server_env},
    Database,
};
use crate::core::{
    models::{Config, MCPServerConfig},
    transport::TransportType,
};

/// Load the MCP server configuration from the database
pub async fn load_server_config(db: &Database) -> Result<Config> {
    tracing::info!("Loading server configuration from database");

    // Get all servers from the database
    let servers = get_all_servers(&db.pool)
        .await
        .context("Failed to get servers from database")?;

    // Create a new Config object
    let mut config = Config {
        mcp_servers: HashMap::new(),
    };

    // Convert each server to MCPServerConfig
    for server in servers {
        // Get server arguments
        let args = if let Some(id) = server.id {
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
        let env = if let Some(id) = server.id {
            let env_map = get_server_env(&db.pool, id)
                .await
                .context("Failed to get server environment variables")?;

            if env_map.is_empty() {
                None
            } else {
                Some(env_map)
            }
        } else {
            None
        };

        // Parse transport type
        let transport_type = server.transport_type.as_deref().map(|t| match t {
            "Stdio" => TransportType::Stdio,
            "Sse" => TransportType::Sse,
            "StreamableHttp" => TransportType::StreamableHttp,
            _ => {
                tracing::warn!("Unknown transport type: {}, defaulting to SSE", t);
                TransportType::Sse
            }
        });

        // Create MCPServerConfig
        let server_config = MCPServerConfig {
            kind: server.server_type,
            command: server.command,
            args,
            url: server.url,
            env,
            transport_type,
        };

        // Add to config
        config.mcp_servers.insert(server.name, server_config);
    }

    tracing::info!(
        "Successfully loaded {} servers from database",
        config.mcp_servers.len()
    );
    Ok(config)
}
