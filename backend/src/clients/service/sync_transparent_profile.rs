//! Push profile-based **native** server lists to **transparent** clients only.

use super::core::{ClientConfigService, ClientRenderOptions};
use crate::clients::error::ConfigResult;
use crate::clients::models::ConfigMode;
use crate::config::client::init::resolve_default_client_config_mode;

impl ClientConfigService {
    /// For each managed client in **transparent** mode, re-render and apply the native configuration
    /// from the given profile (same shape as Servers → Apply in transparent mode).
    ///
    /// **Hosted** clients are skipped: their config is the MCPMate proxy entry; global enable/disable
    /// is enforced by the proxy and profile merge, not by rewriting per-server rows into client files.
    ///
    /// Globally disabled servers are omitted from the profile query (`sc.enabled = 1`), so a sync
    /// after disable removes them from transparent clients' exported configs.
    pub async fn sync_native_profile_to_transparent_clients(
        &self,
        profile_id: &str,
    ) -> ConfigResult<()> {
        let states = self.fetch_client_states().await?;
        let descriptors = self.list_clients(false).await?;
        let default_config_mode = resolve_default_client_config_mode(&self.db_pool)
            .await
            .unwrap_or_else(|_| "unify".to_string());

        let mut ok = 0usize;
        let mut failures = std::collections::HashMap::new();

        for descriptor in descriptors {
            let client_id = descriptor.template.identifier.clone();
            if !descriptor.managed {
                continue;
            }

            let config_mode = states
                .get(&client_id)
                .and_then(|s| s.config_mode.as_deref())
                .unwrap_or(default_config_mode.as_str());
            if !config_mode.eq_ignore_ascii_case("transparent") {
                continue;
            }

            let options = ClientRenderOptions {
                client_id: client_id.clone(),
                mode: ConfigMode::Native,
                profile_id: Some(profile_id.to_string()),
                server_ids: None,
                dry_run: false,
            };

            match self.apply_with_deferred(options).await {
                Ok(outcome) => {
                    if outcome.applied || outcome.scheduled {
                        tracing::debug!(
                            client = %client_id,
                            applied = outcome.applied,
                            scheduled = outcome.scheduled,
                            "Synced native profile to transparent client"
                        );
                        ok += 1;
                    } else {
                        failures.insert(
                            client_id,
                            "apply finished without applied or scheduled write".to_string(),
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        client = %client_id,
                        error = %err,
                        "Failed to sync native profile to transparent client"
                    );
                    failures.insert(client_id, err.to_string());
                }
            }
        }

        tracing::info!(
            transparent_sync_ok = ok,
            transparent_sync_failed = failures.len(),
            profile_id = %profile_id,
            "Transparent client native profile sync finished"
        );

        Ok(())
    }
}
