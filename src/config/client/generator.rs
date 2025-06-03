// Configuration generator for client applications (Refactored)
// Main coordinator that delegates to specialized modules

use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::builder::ConfigBuilder;
use super::models::{GeneratedConfig, GenerationRequest};

/// Configuration generator for client applications
/// Now acts as a coordinator that delegates to specialized modules
pub struct ConfigGenerator {
    config_builder: ConfigBuilder,
}

impl ConfigGenerator {
    /// Create a new configuration generator
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self {
            config_builder: ConfigBuilder::new(db_pool),
        }
    }

    /// Generate configuration for a specific client
    pub async fn generate_config(
        &self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        self.config_builder.generate_config(request).await
    }

    /// Generate preview configuration (same as generate but marked as preview)
    pub async fn generate_preview(
        &self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        self.config_builder.generate_preview(request).await
    }
}
