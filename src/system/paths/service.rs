// Unified path service to eliminate path handling duplication
// Centralizes all path resolution, template processing, and platform-specific logic

use super::PathMapper;
use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};

/// Unified path service for consistent path handling across the application
pub struct PathService {
    path_mapper: PathMapper,
}

impl PathService {
    /// Create a new path service with system variables
    pub fn new() -> Result<Self> {
        Ok(Self {
            path_mapper: PathMapper::new()?,
        })
    }

    /// Get client configuration path with unified logic
    /// This replaces scattered client config path logic
    pub async fn get_client_config_path(
        &self,
        pool: &Pool<Sqlite>,
        client_identifier: &str,
    ) -> Result<String> {
        // Get current platform consistently
        let current_platform = Self::get_current_platform();

        // Query the database for the config path from detection rules
        let config_path = sqlx::query_scalar::<_, String>(
            r#"
            SELECT config_path
            FROM client_detection_rules
            WHERE client_id = (
                SELECT id FROM client WHERE identifier = ?
            ) AND platform = ?
            ORDER BY priority ASC
            LIMIT 1
            "#,
        )
        .bind(client_identifier)
        .bind(current_platform)
        .fetch_optional(pool)
        .await
        .context("Failed to query client config path")?;

        match config_path {
            Some(path) => {
                // Resolve template variables using PathMapper
                let resolved_path = self
                    .path_mapper
                    .resolve_template(&path)
                    .context(format!("Failed to resolve config path template: {}", path))?;
                Ok(resolved_path.to_string_lossy().to_string())
            }
            None => {
                // Fallback to default path
                Ok(format!("~/.config/{}/config.json", client_identifier))
            }
        }
    }

    /// Resolve any path template with consistent logic
    /// This replaces scattered template resolution logic
    pub fn resolve_path_template(
        &self,
        template: &str,
    ) -> Result<PathBuf> {
        self.path_mapper
            .resolve_template(template)
            .context(format!("Failed to resolve path template: {}", template))
    }

    /// Get runtime binary path with unified logic
    /// This replaces scattered runtime path logic
    pub fn resolve_runtime_path(
        &self,
        relative_bin_path: &str,
    ) -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Unable to determine home directory"))?;

        let bin_path = if relative_bin_path.starts_with('/') {
            // Absolute path (system runtime) - use as-is
            PathBuf::from(relative_bin_path)
        } else if relative_bin_path.starts_with(".mcpmate/") {
            // Already properly formatted relative path
            home_dir.join(relative_bin_path)
        } else {
            // Relative path that needs .mcpmate prefix
            home_dir.join(format!(".mcpmate/{}", relative_bin_path))
        };

        Ok(bin_path)
    }

    /// Get detection rule path with unified logic
    /// This replaces scattered detection path logic
    pub fn resolve_detection_path(
        &self,
        detection_value: &str,
    ) -> Result<PathBuf> {
        self.path_mapper
            .resolve_template(detection_value)
            .context(format!("Failed to resolve detection path: {}", detection_value))
    }

    /// Get current platform string consistently
    /// This replaces scattered platform detection logic
    pub fn get_current_platform() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "macos"
        }
        #[cfg(target_os = "windows")]
        {
            "windows"
        }
        #[cfg(target_os = "linux")]
        {
            "linux"
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            "unknown"
        }
    }

    /// Create parent directories if they don't exist
    /// This replaces scattered directory creation logic
    pub async fn ensure_parent_dirs(
        &self,
        file_path: &Path,
    ) -> Result<()> {
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await.context(format!(
                "Failed to create parent directories for: {}",
                file_path.display()
            ))?;
        }
        Ok(())
    }

    /// Validate that a path exists and is accessible
    /// This adds consistent path validation across the application
    pub async fn validate_path_exists(
        &self,
        path: &PathBuf,
    ) -> Result<bool> {
        match tokio::fs::metadata(path).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed to check path {}: {}", path.display(), e)),
        }
    }

    /// Get the path mapper for advanced operations
    pub fn path_mapper(&self) -> &PathMapper {
        &self.path_mapper
    }
}

impl Default for PathService {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            path_mapper: PathMapper::default(),
        })
    }
}

/// Global path service instance for consistent usage
static PATH_SERVICE: std::sync::OnceLock<PathService> = std::sync::OnceLock::new();

/// Get the global path service instance
pub fn get_path_service() -> &'static PathService {
    PATH_SERVICE.get_or_init(PathService::default)
}
