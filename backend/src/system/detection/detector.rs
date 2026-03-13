// Main application detector service

use crate::system::detection::models::{Client, DetectedApp, DetectionMethod, DetectionResult, DetectionRule};
use crate::system::detection::platform::PlatformDetector;
use crate::system::paths::PathMapper;
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;

/// Main application detector service
pub struct AppDetector {
    db_pool: Arc<SqlitePool>,
    path_mapper: PathMapper,
    platform_detector: PlatformDetector,
}

impl AppDetector {
    /// Create a new AppDetector instance
    pub async fn new(db_pool: Arc<SqlitePool>) -> Result<Self> {
        let platform_detector = PlatformDetector::new();
        let path_mapper = PathMapper::new()?;

        Ok(Self {
            db_pool,
            path_mapper,
            platform_detector,
        })
    }

    /// Get current platform identifier
    fn get_current_platform(&self) -> String {
        #[cfg(target_os = "macos")]
        return "macos".to_string();

        #[cfg(target_os = "windows")]
        return "windows".to_string();

        #[cfg(target_os = "linux")]
        return "linux".to_string();

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return "unknown".to_string();
    }

    /// Get all enabled clientlications
    pub async fn get_enabled_apps(&self) -> Result<Vec<Client>> {
        let rows = sqlx::query(
            r#"
            SELECT id, identifier, display_name, description, enabled
            FROM client
            WHERE enabled = TRUE
            ORDER BY display_name
            "#,
        )
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut apps = Vec::new();
        for row in rows {
            apps.push(Client {
                id: row.get("id"),
                identifier: row.get("identifier"),
                display_name: row.get("display_name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
            });
        }

        Ok(apps)
    }

    /// Get all known clientlications (including disabled ones)
    pub async fn get_all_known_apps(&self) -> Result<Vec<Client>> {
        let rows = sqlx::query(
            r#"
            SELECT id, identifier, display_name, description, enabled
            FROM client
            ORDER BY display_name
            "#,
        )
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut apps = Vec::new();
        for row in rows {
            apps.push(Client {
                id: row.get("id"),
                identifier: row.get("identifier"),
                display_name: row.get("display_name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
            });
        }

        Ok(apps)
    }

    /// Get detection rules for a clientlication
    async fn get_detection_rules_for_client(
        &self,
        client_id: &str,
    ) -> Result<Vec<DetectionRule>> {
        let rows = sqlx::query(
            r#"
            SELECT id, client_id, platform, detection_method, detection_value,
                   config_path, priority, enabled
            FROM client_detection_rules
            WHERE client_id = ? AND enabled = TRUE
            ORDER BY priority ASC
            "#,
        )
        .bind(client_id)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut detection_rules = Vec::new();
        for row in rows {
            let detection_method: String = row.get("detection_method");
            if let Ok(method) = DetectionMethod::from_str(&detection_method) {
                detection_rules.push(DetectionRule {
                    id: row.get("id"),
                    client_id: row.get("client_id"),
                    platform: row.get("platform"),
                    detection_method: method,
                    detection_value: row.get("detection_value"),
                    config_path: row.get::<Option<String>, _>("config_path").unwrap_or_default(),
                    priority: row.get("priority"),
                    enabled: row.get("enabled"),
                });
            }
        }

        Ok(detection_rules)
    }

    /// Detect a specific application by identifier
    pub async fn detect_by_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<DetectedApp>> {
        let row = sqlx::query(
            r#"
            SELECT id, identifier, display_name, description, enabled
            FROM client
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        if let Some(row) = row {
            let app = Client {
                id: row.get("id"),
                identifier: row.get("identifier"),
                display_name: row.get("display_name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
            };
            self.detect_by_client(&app).await
        } else {
            Ok(None)
        }
    }

    /// Detect an application using its client definition
    async fn detect_by_client(
        &self,
        client: &Client,
    ) -> Result<Option<DetectedApp>> {
        let detection_rules = self.get_detection_rules_for_client(&client.id).await?;

        if detection_rules.is_empty() {
            return Ok(None);
        }

        let mut detection_results = Vec::new();
        let current_platform = self.get_current_platform();

        // Filter rules for current platform and sort by priority
        let mut platform_rules: Vec<_> = detection_rules
            .into_iter()
            .filter(|rule| rule.platform == current_platform && rule.enabled)
            .collect();
        platform_rules.sort_by_key(|rule| rule.priority);

        // Try each detection method
        for rule in &platform_rules {
            let result = self.execute_detection_rule(rule).await?;
            if result.success {
                detection_results.push(result);
            }
        }

        if detection_results.is_empty() {
            return Ok(None);
        }

        // Calculate overall confidence and combine results
        let confidence = if detection_results.len() >= 2 {
            1.0 // High confidence with multiple verification methods
        } else {
            0.5 // Partial confidence with single method
        };

        // Find the best application path result (prefer .app bundles over config files)
        let app_result = detection_results.iter().find(|result| {
            if let Some(path) = &result.install_path {
                path.extension().is_some_and(|ext| ext == "app") || path.to_string_lossy().contains("/Applications/")
            } else {
                false
            }
        });

        // Determine install_path and version based on detection results
        // Only accept real application paths, not config file paths
        let (install_path, version) = if let Some(app_result) = app_result {
            // Found a real application installation path
            (
                app_result.install_path.as_ref().unwrap().clone(),
                app_result.version.clone(),
            )
        } else {
            // No real application path found, check if any result has a valid executable path
            let valid_app_result = detection_results.iter().find(|result| {
                if let Some(path) = &result.install_path {
                    let path_str = path.to_string_lossy();
                    // Only consider it valid if it's clearly an executable or application
                    (path_str.ends_with(".exe") ||
                     path_str.ends_with(".app") ||
                     path_str.contains("/bin/") ||
                     path_str.contains("/Applications/")) &&
                    // And it's not obviously a config file
                    !path_str.contains(".json") &&
                    !path_str.contains(".config") &&
                    !path_str.contains("settings") &&
                    !path_str.contains("globalStorage") &&
                    !path_str.contains("Application Support") &&
                    !path_str.contains("AppData") &&
                    !path_str.contains("Library/")
                } else {
                    false
                }
            });

            if let Some(valid_result) = valid_app_result {
                (
                    valid_result.install_path.as_ref().unwrap().clone(),
                    valid_result.version.clone(),
                )
            } else {
                // No valid application path found - this is likely an extension
                // Return None to indicate no real application was detected
                return Ok(None);
            }
        };

        // Resolve config path from the first rule (they should all have the same template)
        let config_path = if let Some(first_rule) = platform_rules.first() {
            self.path_mapper.resolve_template(&first_rule.config_path)?
        } else {
            // This should not happen if we have detection results, but provide a fallback
            std::path::PathBuf::from(format!("~/.config/{}/config.json", client.identifier))
        };

        let verified_methods: Vec<String> = detection_results
            .iter()
            .map(|r| r.method.as_str().to_string())
            .collect();

        let detected_app = DetectedApp {
            client: client.clone(),
            version,
            install_path,
            config_path,
            confidence,
            verified_methods,
        };

        Ok(Some(detected_app))
    }

    /// Detect all enabled applications
    pub async fn detect_enabled_apps(&self) -> Result<Vec<DetectedApp>> {
        let enabled_apps = self.get_enabled_apps().await?;
        let mut detected_apps = Vec::new();

        for app in enabled_apps {
            if let Ok(Some(detected)) = self.detect_by_client(&app).await {
                detected_apps.push(detected);
            }
        }

        Ok(detected_apps)
    }

    /// Enable a clientlication
    pub async fn enable_client(
        &self,
        identifier: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE client
            SET enabled = TRUE, updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }

    /// Disable a clientlication
    pub async fn disable_client(
        &self,
        identifier: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE client
            SET enabled = FALSE, updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }

    /// Execute a detection rule
    async fn execute_detection_rule(
        &self,
        rule: &DetectionRule,
    ) -> Result<DetectionResult> {
        match rule.detection_method {
            DetectionMethod::BundleId => {
                #[cfg(target_os = "macos")]
                {
                    self.platform_detector.detect_by_bundle_id(&rule.detection_value).await
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok(DetectionResult::failure(DetectionMethod::BundleId))
                }
            }
            DetectionMethod::FilePath => {
                // Resolve path template before checking
                let resolved_path = self.path_mapper.resolve_template(&rule.detection_value)?;
                self.platform_detector
                    .detect_by_file_path(&resolved_path.to_string_lossy())
                    .await
            }
            DetectionMethod::Registry => {
                // TODO: Implement Windows registry detection
                Ok(DetectionResult::failure(DetectionMethod::Registry))
            }
            DetectionMethod::Command => {
                // TODO: Implement command-based detection
                Ok(DetectionResult::failure(DetectionMethod::Command))
            }
        }
    }

    /// Scan all known applications and enable those that are detected
    pub async fn scan_all_known_apps(&self) -> Result<Vec<DetectedApp>> {
        let all_apps = self.get_all_known_apps().await?;
        let mut detected_apps = Vec::new();

        for app in all_apps {
            if let Ok(Some(detected)) = self.detect_by_client(&app).await {
                detected_apps.push(detected);

                // If the app was not enabled, enable it now
                if !app.enabled {
                    self.enable_client(&app.identifier).await?;
                }
            }
        }

        Ok(detected_apps)
    }
}
