use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientTemplate, DetectionMethod};
use crate::clients::source::ClientConfigSource;
use crate::system::paths::PathService;

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
                let config_path = rule
                    .config_path
                    .as_ref()
                    .or_else(|| Some(&rule.value))
                    .and_then(|path| {
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
            DetectionMethod::FilePath => {
                let resolved = self
                    .path_service
                    .resolve_detection_path(&rule.value)
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
                Ok(resolved.exists())
            }
            DetectionMethod::ConfigPath => {
                let candidate = rule.config_path.as_ref().unwrap_or(&rule.value);
                let resolved = self
                    .path_service
                    .resolve_detection_path(candidate)
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
                Ok(resolved.exists())
            }
            DetectionMethod::BundleId => {
                #[cfg(target_os = "macos")]
                {
                    let _ = rule;
                    Ok(false)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = rule;
                    Ok(false)
                }
            }
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
