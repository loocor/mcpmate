// Server import functionality for client configurations
// Thin layer that leverages existing configuration parsing and server management

use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::api::models::clients::ClientImportedServer;
use crate::config::models::server::Server;
use crate::config::server::{args, crud, env};

/// Import servers from client configuration content
/// Very thin layer that leverages existing server management
pub async fn import_servers_from_config(
    config_content: &Value,
    db_pool: &SqlitePool,
) -> Result<Vec<ClientImportedServer>> {
    let mut imported_servers = Vec::new();

    // Extract servers using simple parsing
    let servers = extract_servers(config_content)?;

    // Get existing servers for duplicate checking
    let existing_servers = crud::get_all_servers(db_pool).await?;

    for (name, config) in servers {
        // Check if command already exists
        if !existing_servers
            .iter()
            .any(|s| s.command.as_ref() == Some(&config.command))
        {
            match create_and_save_server(&name, &config, db_pool).await {
                Ok(imported) => {
                    imported_servers.push(imported);
                    tracing::info!("Successfully imported server: {}", name);
                }
                Err(e) => {
                    tracing::warn!("Failed to import server {}: {}", name, e);
                }
            }
        }
    }

    Ok(imported_servers)
}

/// Extract servers from configuration - simple parsing
fn extract_servers(config: &Value) -> Result<HashMap<String, ServerConfig>> {
    let mut servers = HashMap::new();

    // Standard object format (Claude Desktop, etc.)
    if let Some(mcp_servers) = config.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, server_config) in mcp_servers {
            if let Some(config) = parse_server_config(server_config) {
                servers.insert(name.clone(), config);
            }
        }
    }
    // Array format (Augment)
    else if let Some(array) = config.as_array() {
        for (index, server_config) in array.iter().enumerate() {
            let name = server_config
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("server_{}", index))
                .to_string();

            if let Some(config) = parse_server_config(server_config) {
                servers.insert(name, config);
            }
        }
    }

    Ok(servers)
}

/// Parse individual server configuration - simplified
fn parse_server_config(config: &Value) -> Option<ServerConfig> {
    let command = config.get("command")?.as_str()?.to_string();

    let args = config
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(String::from).collect())
        .unwrap_or_default();

    let env = config
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    Some(ServerConfig { command, args, env })
}

/// Create and save server to database - leverages existing CRUD
async fn create_and_save_server(
    name: &str,
    config: &ServerConfig,
    db_pool: &SqlitePool,
) -> Result<ClientImportedServer> {
    // Create server using existing constructor
    let server = Server::new_stdio(name.to_string(), Some(config.command.clone()));

    // Insert server using existing CRUD
    let server_id = crud::upsert_server(db_pool, &server).await?;

    // Insert args and env using existing functions
    if !config.args.is_empty() {
        args::upsert_server_args(db_pool, &server_id, &config.args).await?;
    }
    if !config.env.is_empty() {
        env::upsert_server_env(db_pool, &server_id, &config.env).await?;
    }

    Ok(ClientImportedServer {
        name: name.to_string(),
        command: config.command.clone(),
        args: config.args.clone(),
        env: config.env.clone(),
        transport_type: "stdio".to_string(), // Default to stdio
    })
}

/// Simplified server configuration structure
#[derive(Debug, Clone)]
struct ServerConfig {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}
