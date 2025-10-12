use super::core::{ClientConfigService, ClientDescriptor};
use crate::clients::detector::DetectedClient;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::source::ClientConfigSource;
use crate::system::paths::get_path_service;
use std::collections::HashMap;
use std::path::PathBuf;

impl ClientConfigService {
    /// List known clients enriched with detection and filesystem information
    pub async fn list_clients(
        &self,
        _force_detect: bool,
    ) -> ConfigResult<Vec<ClientDescriptor>> {
        let templates = self.template_source.list_client().await?;
        let detected = self.detector.detect_installed_client().await?;
        let states = self.fetch_client_states().await?;

        let mut detected_map: HashMap<String, DetectedClient> = HashMap::new();
        for entry in detected {
            detected_map.insert(entry.identifier.clone(), entry);
        }

        let mut results = Vec::with_capacity(templates.len());
        for mut template in templates {
            let identifier = template.identifier.clone();
            let state_entry = states.get(&identifier);

            if let Some(state) = state_entry {
                tracing::trace!(
                    client_state_id = %state.id,
                    client_identifier = %identifier,
                    "Loaded client state metadata"
                );
            }

            if template.display_name.is_none() {
                if let Some(state) = state_entry {
                    if !state.name.is_empty() {
                        template.display_name = Some(state.name.clone());
                    }
                }
            }

            let resolved_path = self.resolved_config_path(&identifier).await?;
            let config_exists = if let Some(path_str) = &resolved_path {
                let path = PathBuf::from(path_str);
                get_path_service()
                    .validate_path_exists(&path)
                    .await
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?
            } else {
                false
            };

            let detection = detected_map.remove(&identifier);
            let detected_at = detection.as_ref().map(|entry| entry.detected_at);
            let managed = state_entry.map(|state| state.managed()).unwrap_or(true);
            results.push(ClientDescriptor {
                detection,
                template,
                config_path: resolved_path,
                config_exists,
                detected_at,
                managed,
            });
        }

        Ok(results)
    }
}
