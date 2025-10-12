use super::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};

const VALID_TRANSPORTS: &[&str] = &["auto", "sse", "stdio", "streamable_http"];

impl ClientConfigService {
    /// Update client settings (config_mode, transport, client_version)
    /// - config_mode: optional, only update if provided
    /// - transport: optional, only update if provided; must be one of: auto, sse, stdio, streamable_http
    /// - client_version: optional, only update if provided
    pub async fn set_client_settings(
        &self,
        identifier: &str,
        config_mode: Option<String>,
        transport: Option<String>,
        client_version: Option<String>,
    ) -> ConfigResult<()> {
        tracing::info!(
            client = %identifier,
            config_mode = ?config_mode,
            transport = ?transport,
            client_version = ?client_version,
            "set_client_settings: entry"
        );

        // Validate transport value if provided
        if let Some(ref tr) = transport {
            if !VALID_TRANSPORTS.contains(&tr.as_str()) {
                let err = format!(
                    "Invalid transport value '{}', must be one of: {}",
                    tr,
                    VALID_TRANSPORTS.join(", ")
                );
                tracing::error!(client = %identifier, transport = %tr, "{}", err);
                return Err(ConfigError::DataAccessError(err));
            }
        }

        // Ensure state row exists
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        // Update name (always)
        self.update_client_name(identifier, &name).await?;

        // Update config_mode if provided
        if let Some(mode) = config_mode {
            self.update_config_mode(identifier, &mode).await?;
        }

        // Update transport if provided
        if let Some(tr) = transport {
            self.update_transport(identifier, &tr).await?;
        }

        // Update client_version if provided
        if let Some(ver) = client_version {
            self.update_client_version(identifier, &ver).await?;
        }

        tracing::info!(client = %identifier, "set_client_settings: complete");
        Ok(())
    }

    /// Update client name
    async fn update_client_name(
        &self,
        identifier: &str,
        name: &str,
    ) -> ConfigResult<()> {
        tracing::debug!(client = %identifier, name = %name, "Updating client name");

        sqlx::query("UPDATE client SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
            .bind(name)
            .bind(identifier)
            .execute(&*self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!(client = %identifier, error = %e, "Failed to update client name");
                ConfigError::DataAccessError(e.to_string())
            })?;

        Ok(())
    }

    /// Update config_mode
    async fn update_config_mode(
        &self,
        identifier: &str,
        mode: &str,
    ) -> ConfigResult<()> {
        tracing::info!(client = %identifier, config_mode = %mode, "Updating config_mode");

        let result =
            sqlx::query("UPDATE client SET config_mode = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
                .bind(mode)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!(client = %identifier, error = %e, "Failed to update config_mode");
                    ConfigError::DataAccessError(e.to_string())
                })?;

        tracing::info!(
            client = %identifier,
            rows_affected = %result.rows_affected(),
            "config_mode updated"
        );

        Ok(())
    }

    /// Update transport protocol
    async fn update_transport(
        &self,
        identifier: &str,
        transport: &str,
    ) -> ConfigResult<()> {
        tracing::info!(client = %identifier, transport = %transport, "Updating transport");

        let result =
            sqlx::query("UPDATE client SET transport = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
                .bind(transport)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!(
                        client = %identifier,
                        transport = %transport,
                        error = %e,
                        "Failed to update transport"
                    );
                    ConfigError::DataAccessError(e.to_string())
                })?;

        tracing::info!(
            client = %identifier,
            transport = %transport,
            rows_affected = %result.rows_affected(),
            "transport updated"
        );

        Ok(())
    }

    /// Update client_version
    async fn update_client_version(
        &self,
        identifier: &str,
        version: &str,
    ) -> ConfigResult<()> {
        tracing::info!(client = %identifier, version = %version, "Updating client_version");

        let result =
            sqlx::query("UPDATE client SET client_version = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
                .bind(version)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!(client = %identifier, error = %e, "Failed to update client_version");
                    ConfigError::DataAccessError(e.to_string())
                })?;

        tracing::info!(
            client = %identifier,
            rows_affected = %result.rows_affected(),
            "client_version updated"
        );

        Ok(())
    }

    /// Get client settings (config_mode, transport, client_version)
    /// Returns None if client state not found
    pub async fn get_client_settings(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<(String, String, Option<String>)>> {
        let state = self.fetch_state(identifier).await?;

        if state.is_none() {
            tracing::debug!(client = %identifier, "Client state not found");
            return Ok(None);
        }

        let state = state.unwrap();
        let transport = state.transport.unwrap_or_else(|| "auto".to_string());

        Ok(Some((state.config_mode, transport, state.client_version)))
    }
}
