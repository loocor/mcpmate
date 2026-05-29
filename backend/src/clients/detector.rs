use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientTemplate, DetectionMethod};
use crate::clients::source::ClientConfigSource;
use crate::system::paths::PathService;
use chrono::{DateTime, Utc};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedClient {
    pub identifier: String,
    pub display_name: Option<String>,
    pub config_path: Option<String>,
    pub detected_method: DetectionMethod,
    pub detected_at: DateTime<Utc>,
}

pub struct ClientDetector {
    config_source: Arc<dyn ClientConfigSource>,
    path_service: PathService,
}

impl ClientDetector {
    pub fn new(config_source: Arc<dyn ClientConfigSource>) -> ConfigResult<Self> {
        let path_service = PathService::new().map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

        Ok(Self {
            config_source,
            path_service,
        })
    }

    pub async fn detect_installed_client(&self) -> ConfigResult<Vec<DetectedClient>> {
        let templates = self.config_source.list_client().await?;
        let mut detected = Vec::new();

        for template in templates {
            if let Some(result) = self.detect_single_client(&template).await? {
                detected.push(result);
            }
        }

        Ok(detected)
    }

    async fn detect_single_client(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<Option<DetectedClient>> {
        let platform = Self::current_platform();
        let Some(rules) = template.platform_rules(platform) else {
            return Ok(None);
        };

        for rule in rules {
            if self.check_detection_rule(rule).await? {
                let config_path = rule.config_path.as_ref().or(Some(&rule.value)).and_then(|path| {
                    self.path_service
                        .resolve_user_path(path)
                        .ok()
                        .map(|resolved| resolved.to_string_lossy().to_string())
                });

                return Ok(Some(DetectedClient {
                    identifier: template.identifier.clone(),
                    display_name: template.display_name.clone(),
                    config_path,
                    detected_method: rule.method.clone(),
                    detected_at: Utc::now(),
                }));
            }
        }

        Ok(None)
    }

    async fn check_detection_rule(
        &self,
        rule: &crate::clients::models::DetectionRule,
    ) -> ConfigResult<bool> {
        match rule.method {
            DetectionMethod::ConfigPath => {
                let candidate = rule.config_path.as_ref().unwrap_or(&rule.value);
                let resolved = self
                    .path_service
                    .resolve_detection_path(candidate)
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
                Ok(resolved.exists())
            }
            DetectionMethod::FilePath | DetectionMethod::BundleId => Ok(false),
        }
    }

    fn current_platform() -> &'static str {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::models::{ConfigMapping, ContainerType, DetectionRule, FormatRule, StorageConfig, StorageKind};
    use async_trait::async_trait;
    use tempfile::tempdir;

    struct StaticSource {
        templates: Vec<ClientTemplate>,
    }

    #[async_trait]
    impl ClientConfigSource for StaticSource {
        async fn list_client(&self) -> ConfigResult<Vec<ClientTemplate>> {
            Ok(self.templates.clone())
        }

        async fn get_template(
            &self,
            _client_id: &str,
            _platform: &str,
        ) -> ConfigResult<Option<ClientTemplate>> {
            Ok(None)
        }

        async fn get_config_path(
            &self,
            _client_id: &str,
            _platform: &str,
        ) -> ConfigResult<Option<String>> {
            Ok(None)
        }

        async fn reload(&self) -> ConfigResult<()> {
            Ok(())
        }
    }

    fn template_with_rules(rules: Vec<DetectionRule>) -> ClientTemplate {
        let mut detection = std::collections::HashMap::new();
        detection.insert(ClientDetector::current_platform().to_string(), rules);

        ClientTemplate {
            identifier: "test-client".to_string(),
            display_name: Some("Test Client".to_string()),
            storage: StorageConfig {
                kind: StorageKind::File,
                path_strategy: Some("config_path".to_string()),
                adapter: None,
            },
            detection,
            config_mapping: ConfigMapping {
                container_keys: vec!["mcpServers".to_string()],
                container_type: ContainerType::ObjectMap,
                format_rules: {
                    let mut rules = std::collections::HashMap::new();
                    rules.insert(
                        "stdio".to_string(),
                        FormatRule {
                            command_field: Some("command".to_string()),
                            ..Default::default()
                        },
                    );
                    rules
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn detects_only_config_path_rules() {
        let directory = tempdir().expect("temp dir");
        let app_path = directory.path().join("Client.app");
        let config_path = directory.path().join("mcp.json");
        std::fs::create_dir(&app_path).expect("app path");

        let detector = ClientDetector::new(Arc::new(StaticSource {
            templates: vec![template_with_rules(vec![DetectionRule {
                method: DetectionMethod::FilePath,
                value: app_path.to_string_lossy().to_string(),
                config_path: Some(config_path.to_string_lossy().to_string()),
                priority: None,
            }])],
        }))
        .expect("detector");

        let detected = detector.detect_installed_client().await.expect("file path detection");
        assert!(detected.is_empty());

        std::fs::write(&config_path, "{}").expect("config file");
        let detector = ClientDetector::new(Arc::new(StaticSource {
            templates: vec![template_with_rules(vec![DetectionRule {
                method: DetectionMethod::ConfigPath,
                value: config_path.to_string_lossy().to_string(),
                config_path: None,
                priority: None,
            }])],
        }))
        .expect("detector");

        let detected = detector.detect_installed_client().await.expect("config path detection");
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].detected_method, DetectionMethod::ConfigPath);
    }
}
