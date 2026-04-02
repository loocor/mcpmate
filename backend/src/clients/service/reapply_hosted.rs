//! Re-apply hosted (managed) client configs when runtime MCP endpoint changes.

use super::core::{ClientConfigService, ClientRenderOptions};
use crate::clients::error::ConfigResult;
use crate::clients::models::ConfigMode;
use crate::clients::source::ClientConfigSource;
use crate::config::client::init::resolve_default_client_config_mode;

/// Outcome of [`ClientConfigService::reapply_hosted_managed_clients_after_mcp_port_change`].
#[derive(Debug, Default, Clone)]
pub struct HostedClientReapplySummary {
    /// Hosted + managed clients we tried to update.
    pub attempted: usize,
    /// Config files written immediately.
    pub applied: usize,
    /// Deferred writes (e.g. Cherry LevelDB locked).
    pub scheduled: usize,
    /// Pairs of (client identifier, error message).
    pub failures: Vec<(String, String)>,
}

impl ClientConfigService {
    /// Rewrite client config files for every **hosted** and **managed** client so MCP URLs match
    /// the current [`crate::system::config::get_runtime_port_config`] (after `init_port_config`).
    ///
    /// Transparent clients are skipped. Profile selection uses the same rules as Apply with default
    /// profile (`profile_id: None` → active profile resolution in [`super::query`]).
    pub async fn reapply_hosted_managed_clients_after_mcp_port_change(
        &self
    ) -> ConfigResult<HostedClientReapplySummary> {
        let states = self.fetch_client_states().await?;
        let templates = self.template_source.list_client().await?;
        let default_config_mode = resolve_default_client_config_mode(&self.db_pool)
            .await
            .unwrap_or_else(|_| "unify".to_string());

        let mut summary = HostedClientReapplySummary::default();

        for template in templates {
            let identifier = template.identifier.clone();
            let state = states.get(&identifier);
            if !state.map(|s| s.managed()).unwrap_or(true) {
                continue;
            }

            let config_mode = state
                .and_then(|s| s.config_mode.as_deref())
                .unwrap_or(default_config_mode.as_str());
            if config_mode.eq_ignore_ascii_case("transparent") {
                continue;
            }
            if !config_mode.eq_ignore_ascii_case("hosted") {
                tracing::debug!(
                    client = %identifier,
                    config_mode = %config_mode,
                    "Skipping client reapply: config_mode is neither hosted nor transparent"
                );
                continue;
            }

            summary.attempted += 1;
            let options = ClientRenderOptions {
                client_id: identifier.clone(),
                mode: ConfigMode::Managed,
                profile_id: None,
                server_ids: None,
                dry_run: false,
            };

            match self.apply_with_deferred(options).await {
                Ok(outcome) => {
                    if outcome.scheduled {
                        summary.scheduled += 1;
                    } else if outcome.applied {
                        summary.applied += 1;
                    } else {
                        summary.failures.push((
                            identifier,
                            "apply finished without applied or scheduled write".to_string(),
                        ));
                    }
                }
                Err(err) => {
                    summary.failures.push((identifier, err.to_string()));
                }
            }
        }

        Ok(summary)
    }
}
