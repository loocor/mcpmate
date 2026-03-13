use crate::error::{CherryDbError, Result};
use crate::types::{
    CherryMcpConfig, McpConfigRequest, McpConfigResponse, ServerListResponse, ServerRequest,
    ServerResponse,
};
use crate::utils::{encode_json_to_bytes, open_database_and_read_entries};
use rusty_leveldb::{Options, DB};
use serde_json::Value;

/// Trait for managing Cherry Studio LevelDB configurations
pub trait CherryDbManager {
    /// Read the MCP server configuration
    fn read_mcp_config(&self, db_path: &str) -> Result<McpConfigResponse>;

    /// Write/update the MCP server configuration
    fn write_mcp_config(&self, db_path: &str, config: &McpConfigRequest) -> Result<()>;

    /// List all MCP servers
    fn list_servers(&self, db_path: &str) -> Result<ServerListResponse>;

    /// Add or update a MCP server
    fn add_server(&self, db_path: &str, server: &ServerRequest) -> Result<()>;

    /// Remove a MCP server by ID
    fn remove_server(&self, db_path: &str, server_id: &str) -> Result<()>;

    /// Check if a server exists
    fn server_exists(&self, db_path: &str, server_id: &str) -> Result<bool>;
}

/// Default implementation of CherryDbManager
#[derive(Debug, Default)]
pub struct DefaultCherryDbManager;

impl DefaultCherryDbManager {
    /// Create a new instance of the default manager
    pub fn new() -> Self {
        Self
    }

    /// Internal helper to find MCP config in database entries
    fn find_mcp_config_internal(&self, db_path: &str) -> Result<(CherryMcpConfig, Value)> {
        let entries = open_database_and_read_entries(db_path)?;

        for entry in entries {
            if let Some(json_data) = entry.json_data {
                if let Some(mcp_config_value) = json_data.get("mcp") {
                    // MCP config is stored as a string, not an object
                    if let Some(mcp_config_str) = mcp_config_value.as_str() {
                        let mcp_config: CherryMcpConfig = serde_json::from_str(mcp_config_str)?;
                        return Ok((mcp_config, json_data));
                    }
                }
            }
        }

        Err(CherryDbError::ConfigNotFound)
    }

    /// Internal helper to update the complete database entry
    fn update_database_entry(&self, db_path: &str, updated_json: &Value) -> Result<()> {
        // Find the original entry key
        let entries = open_database_and_read_entries(db_path)?;
        let target_entry = entries
            .into_iter()
            .find(|entry| entry.json_data.is_some())
            .ok_or(CherryDbError::ConfigNotFound)?;

        // Open database for writing
        let options = Options {
            create_if_missing: false,
            ..Default::default()
        };

        let mut db = DB::open(db_path, options).map_err(|e| {
            CherryDbError::DatabaseError(format!("Failed to open database: {:?}", e))
        })?;

        // Encode and write
        let encoded_data = encode_json_to_bytes(updated_json);
        db.put(&target_entry.key, &encoded_data).map_err(|e| {
            CherryDbError::DatabaseError(format!("Failed to write to database: {:?}", e))
        })?;

        Ok(())
    }
}

impl CherryDbManager for DefaultCherryDbManager {
    fn read_mcp_config(&self, db_path: &str) -> Result<McpConfigResponse> {
        let (config, _) = self.find_mcp_config_internal(db_path)?;
        Ok(McpConfigResponse {
            servers: config.servers,
        })
    }

    fn write_mcp_config(&self, db_path: &str, config: &McpConfigRequest) -> Result<()> {
        // Read existing data to preserve UV/Bun settings
        let (_, mut json_data) = self.find_mcp_config_internal(db_path)?;

        // Create updated Cherry config preserving non-server settings
        let updated_cherry_config = CherryMcpConfig {
            servers: config
                .servers
                .iter()
                .map(|s| ServerResponse::from(s.clone()))
                .collect(),
        };

        // Update MCP section
        let mcp_config_str = serde_json::to_string(&updated_cherry_config)?;
        json_data["mcp"] = Value::String(mcp_config_str);

        // Write back to database
        self.update_database_entry(db_path, &json_data)
    }

    fn list_servers(&self, db_path: &str) -> Result<ServerListResponse> {
        let config = self.read_mcp_config(db_path)?;
        Ok(ServerListResponse {
            total_count: config.servers.len(),
            servers: config.servers,
        })
    }

    fn add_server(&self, db_path: &str, server: &ServerRequest) -> Result<()> {
        let mut config = self.read_mcp_config(db_path)?;

        // Remove existing server with same ID if exists
        config.servers.retain(|s| s.id != server.id);

        // Add new server
        config.servers.push(ServerResponse::from(server.clone()));

        // Convert to request format and write
        let request_config = McpConfigRequest {
            servers: config
                .servers
                .into_iter()
                .map(ServerRequest::from)
                .collect(),
        };

        self.write_mcp_config(db_path, &request_config)
    }

    fn remove_server(&self, db_path: &str, server_id: &str) -> Result<()> {
        let mut config = self.read_mcp_config(db_path)?;

        let original_len = config.servers.len();
        config.servers.retain(|s| s.id != server_id);

        if config.servers.len() == original_len {
            return Err(CherryDbError::ServerNotFound(server_id.to_string()));
        }

        // Convert to request format and write
        let request_config = McpConfigRequest {
            servers: config
                .servers
                .into_iter()
                .map(ServerRequest::from)
                .collect(),
        };

        self.write_mcp_config(db_path, &request_config)
    }

    fn server_exists(&self, db_path: &str, server_id: &str) -> Result<bool> {
        let config = self.read_mcp_config(db_path)?;
        Ok(config.servers.iter().any(|s| s.id == server_id))
    }
}
