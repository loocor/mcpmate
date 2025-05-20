//! Test fixtures
//!
//! Provides fixtures for common test data and objects.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use mcpmate::{conf::models::Server, http::pool::UpstreamConnectionPool};
use rmcp::model::Tool;
use tempfile::TempDir;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Configuration fixture for tests
///
/// Provides access to configuration data for tests, either using
/// the real configuration file or a temporary copy.
pub struct ConfigFixture {
    /// Temporary directory, if using a copy
    temp_dir: Option<TempDir>,

    /// Path to the configuration file
    config_path: PathBuf,
}

impl ConfigFixture {
    /// Use the real config file directly (read-only)
    pub fn real() -> Self {
        Self {
            temp_dir: None,
            config_path: PathBuf::from("config/mcp.json"),
        }
    }

    /// Create a temporary copy of the real config for modification
    pub fn temp_copy() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let config_path = temp_dir.path().join("mcp.json");

        // Copy the real config to the temp directory
        fs::copy("config/mcp.json", &config_path).context("Failed to copy real config file")?;

        Ok(Self {
            temp_dir: Some(temp_dir),
            config_path,
        })
    }

    /// Get the path to the config file
    pub fn path(&self) -> &Path {
        &self.config_path
    }

    /// Modify the config (only works with temp copy)
    pub fn modify<F>(
        &self,
        modifier: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut serde_json::Value) -> (),
    {
        if self.temp_dir.is_none() {
            return Err(anyhow::anyhow!("Cannot modify the real config file"));
        }

        // Read, modify, and write back
        let content =
            fs::read_to_string(&self.config_path).context("Failed to read config file")?;
        let mut json: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse config file")?;

        modifier(&mut json);

        let modified =
            serde_json::to_string_pretty(&json).context("Failed to serialize modified config")?;
        fs::write(&self.config_path, modified).context("Failed to write modified config")?;

        Ok(())
    }
}

/// Server fixture for tests
///
/// Provides utilities for creating and managing test servers.
pub struct ServerFixture {
    /// Server name
    pub name: String,

    /// Server type (stdio, sse, etc.)
    pub kind: String,

    /// Server command (for stdio servers)
    pub command: Option<String>,

    /// Server URL (for sse/http servers)
    pub url: Option<String>,

    /// Tools available on this server
    pub tools: Vec<Tool>,
}

impl ServerFixture {
    /// Create a new server fixture
    pub fn new(
        name: &str,
        kind: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            kind: kind.to_string(),
            command: None,
            url: None,
            tools: Vec::new(),
        }
    }

    /// Set the command for this server
    pub fn with_command(
        mut self,
        command: &str,
    ) -> Self {
        self.command = Some(command.to_string());
        self
    }

    /// Set the URL for this server
    pub fn with_url(
        mut self,
        url: &str,
    ) -> Self {
        self.url = Some(url.to_string());
        self
    }

    /// Add a tool to this server
    pub async fn add_tool(
        &mut self,
        name: &str,
    ) -> Result<&mut Self> {
        self.tools.push(Tool {
            name: name.to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });

        Ok(self)
    }

    /// Add this server to a connection pool
    pub async fn add_to_pool(
        &self,
        pool: &Arc<Mutex<UpstreamConnectionPool>>,
        instance_id: &str,
    ) -> Result<()> {
        // Create a server config
        let _server = Server {
            id: Some(Uuid::new_v4().to_string()),
            name: self.name.clone(),
            server_type: self.kind.clone(),
            command: self.command.clone(),
            url: self.url.clone(),
            transport_type: None,
            enabled: Some(true),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        // Add the server to the pool
        let mut pool = pool.lock().await;

        // Create server connections map if it doesn't exist
        if !pool.connections.contains_key(&self.name) {
            pool.connections.insert(self.name.clone(), HashMap::new());
        }

        // Create a connection
        let mut conn = mcpmate::core::connection::UpstreamConnection::new(self.name.clone());

        // Add tools to the connection
        conn.tools = self.tools.clone();

        // Add the connection to the pool
        pool.connections
            .get_mut(&self.name)
            .unwrap()
            .insert(instance_id.to_string(), conn);

        Ok(())
    }

    /// Convert to a Server model
    pub fn to_server(&self) -> Server {
        Server {
            id: Some(Uuid::new_v4().to_string()),
            name: self.name.clone(),
            server_type: self.kind.clone(),
            command: self.command.clone(),
            url: self.url.clone(),
            transport_type: None,
            enabled: Some(true),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
}
