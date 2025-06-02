// Client management module for MCPMate
// Provides client application detection, configuration generation, and management
// Integrates with existing system/detection module

use crate::config::client::generator::ConfigGenerator;
use crate::config::client::models::*;
use crate::system::detection::detector::AppDetector;
use crate::system::detection::models::{ClientApp, DetectedApp};
use anyhow::Result;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Main client management interface
/// Implements lazy loading architecture with Option<T> pattern
/// Wraps the existing AppDetector for client management functionality
pub struct ClientManager {
    app_detector: Option<AppDetector>,
    config_generator: Option<ConfigGenerator>,
    rule_manager: Option<ClientRuleManager>,
    db_pool: Arc<SqlitePool>,
}

impl ClientManager {
    /// Create new client manager with database pool
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self {
            app_detector: None,
            config_generator: None,
            rule_manager: None,
            db_pool,
        }
    }

    /// Ensure components are loaded, initialize if needed
    pub async fn ensure_loaded(&mut self) -> Result<()> {
        if self.app_detector.is_none() {
            self.app_detector = Some(AppDetector::new(self.db_pool.clone()).await?);
        }
        if self.config_generator.is_none() {
            self.config_generator = Some(ConfigGenerator::new(self.db_pool.clone()));
        }
        if self.rule_manager.is_none() {
            self.rule_manager = Some(ClientRuleManager::new("config/client.json".to_string()));
        }
        Ok(())
    }

    /// Unload components if idle to free memory
    pub fn unload_if_idle(&mut self) {
        // TODO: Implement idle detection logic
        self.app_detector = None;
        self.config_generator = None;
        self.rule_manager = None;
    }

    /// Detect installed client applications
    pub async fn detect_clients(&mut self) -> Result<Vec<DetectedApp>> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.detect_enabled_apps().await
    }

    /// Scan all known applications and enable detected ones
    pub async fn scan_all_clients(&mut self) -> Result<Vec<DetectedApp>> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.scan_all_known_apps().await
    }

    /// Detect specific client by identifier
    pub async fn detect_client_by_identifier(
        &mut self,
        identifier: &str,
    ) -> Result<Option<DetectedApp>> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.detect_by_identifier(identifier).await
    }

    /// Get all enabled client applications
    pub async fn get_enabled_clients(&mut self) -> Result<Vec<ClientApp>> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.get_enabled_apps().await
    }

    /// Get all known client applications (including disabled)
    pub async fn get_all_known_clients(&mut self) -> Result<Vec<ClientApp>> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.get_all_known_apps().await
    }

    /// Enable a client application
    pub async fn enable_client(
        &mut self,
        identifier: &str,
    ) -> Result<()> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.enable_client_app(identifier).await
    }

    /// Disable a client application
    pub async fn disable_client(
        &mut self,
        identifier: &str,
    ) -> Result<()> {
        self.ensure_loaded().await?;
        let detector = self.app_detector.as_ref().unwrap();
        detector.disable_client_app(identifier).await
    }

    // ========== Configuration Generation Functions ==========

    /// Generate configuration for a client (decoupled function)
    pub async fn generate_config(
        &mut self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        self.ensure_loaded().await?;
        let generator = self.config_generator.as_ref().unwrap();
        generator.generate_config(request).await
    }

    /// Generate preview configuration for a client (decoupled function)
    pub async fn generate_preview(
        &mut self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        self.ensure_loaded().await?;
        let generator = self.config_generator.as_ref().unwrap();
        generator.generate_preview(request).await
    }

    // ========== Configuration Application Functions ==========

    /// Apply configuration to a client (decoupled function)
    pub async fn apply_config(
        &mut self,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult> {
        // Validate the configuration path exists and is writable
        let config_path = Path::new(&request.config.config_path);

        // Create backup if requested
        let backup_path = if request.create_backup && config_path.exists() {
            Some(self.create_backup(config_path).await?)
        } else {
            None
        };

        // If dry run, just return success without writing
        if request.dry_run {
            return Ok(ApplicationResult {
                success: true,
                client_identifier: request.client_identifier.clone(),
                config_path: request.config.config_path.clone(),
                backup_path,
                error_message: None,
            });
        }

        // Write the configuration
        match self.write_config_file(&request.config).await {
            Ok(_) => Ok(ApplicationResult {
                success: true,
                client_identifier: request.client_identifier.clone(),
                config_path: request.config.config_path.clone(),
                backup_path,
                error_message: None,
            }),
            Err(e) => {
                // If writing failed and we created a backup, restore it
                if let Some(backup_path) = &backup_path {
                    let _ = self.restore_backup(backup_path, config_path).await;
                }

                Ok(ApplicationResult {
                    success: false,
                    client_identifier: request.client_identifier.clone(),
                    config_path: request.config.config_path.clone(),
                    backup_path,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Create backup of existing configuration file
    async fn create_backup(
        &self,
        config_path: &Path,
    ) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let backup_path = format!("{}.backup.{}", config_path.to_string_lossy(), timestamp);

        fs::copy(config_path, &backup_path)?;
        Ok(backup_path)
    }

    /// Restore backup configuration file
    async fn restore_backup(
        &self,
        backup_path: &str,
        config_path: &Path,
    ) -> Result<()> {
        fs::copy(backup_path, config_path)?;
        Ok(())
    }

    /// Write configuration content to file
    /// Supports both standalone and mixed configuration files
    async fn write_config_file(
        &self,
        config: &GeneratedConfig,
    ) -> Result<()> {
        let config_path = Path::new(&config.config_path);

        // Create parent directories if they don't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Check if this is a mixed configuration file (like Zed's settings.json)
        if self.is_mixed_config_file(config).await? {
            self.write_mixed_config_file(config_path, config).await?;
        } else {
            // Write standalone configuration file
            fs::write(config_path, &config.config_content)?;
        }

        Ok(())
    }

    /// Check if this is a mixed configuration file
    async fn is_mixed_config_file(
        &self,
        config: &GeneratedConfig,
    ) -> Result<bool> {
        // Query database to check if this client uses mixed config
        let is_mixed: bool = sqlx::query_scalar(
            "SELECT is_mixed_config FROM client_config_rules WHERE client_identifier = ?",
        )
        .bind(&config.client_identifier)
        .fetch_optional(&*self.db_pool)
        .await?
        .unwrap_or(false);

        Ok(is_mixed)
    }

    /// Write to mixed configuration file (only update MCP-related sections)
    async fn write_mixed_config_file(
        &self,
        config_path: &Path,
        config: &GeneratedConfig,
    ) -> Result<()> {
        // Read existing configuration if it exists
        let existing_content = if config_path.exists() {
            fs::read_to_string(config_path)?
        } else {
            "{}".to_string()
        };

        // Parse existing configuration
        let mut existing_config: Value =
            serde_json::from_str(&existing_content).unwrap_or_else(|_| json!({}));

        // Parse new MCP configuration
        let new_mcp_config: Value = serde_json::from_str(&config.config_content)?;

        // Get the top-level key for this client (e.g., "mcpServers" or "context_servers")
        let top_level_key = self
            .get_client_top_level_key(&config.client_identifier)
            .await?;

        // Update only the MCP-related section
        if let Some(mcp_section) = new_mcp_config.get(&top_level_key) {
            existing_config[&top_level_key] = mcp_section.clone();
        }

        // Write the merged configuration back
        let merged_content = serde_json::to_string_pretty(&existing_config)?;
        fs::write(config_path, merged_content)?;

        Ok(())
    }

    /// Get the top-level key for a client (e.g., "mcpServers", "context_servers")
    async fn get_client_top_level_key(
        &self,
        client_identifier: &str,
    ) -> Result<String> {
        let top_level_key: String = sqlx::query_scalar(
            "SELECT top_level_key FROM client_config_rules WHERE client_identifier = ?",
        )
        .bind(client_identifier)
        .fetch_one(&*self.db_pool)
        .await?;

        Ok(top_level_key)
    }

    // ========== Rule Management Functions ==========

    /// Check if rules need to be reloaded
    pub async fn check_rules_reload(&mut self) -> Result<bool> {
        self.ensure_loaded().await?;
        let rule_manager = self.rule_manager.as_ref().unwrap();
        rule_manager.needs_reload().await
    }

    /// Reload rules from configuration file
    pub async fn reload_rules(&mut self) -> Result<()> {
        self.ensure_loaded().await?;
        let rule_manager = self.rule_manager.as_mut().unwrap();

        let config = rule_manager.load_rules().await?;
        rule_manager.validate_rules(&config)?;

        tracing::info!("Client rules reloaded successfully");
        Ok(())
    }

    /// Get current rules version
    pub async fn get_rules_version(&mut self) -> Result<Option<String>> {
        self.ensure_loaded().await?;
        let rule_manager = self.rule_manager.as_ref().unwrap();
        Ok(rule_manager.current_version().cloned())
    }
}
