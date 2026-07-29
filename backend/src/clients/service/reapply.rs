//! Re-apply hosted (managed) client configs when runtime MCP endpoint changes.

use super::core::{ApplyOutcome, ClientConfigService, ClientRenderOptions};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{AttachmentState, ConfigMode};
use crate::config::client::init::resolve_default_client_config_mode;

/// Outcome of [`ClientConfigService::reapply_hosted_managed_clients_after_mcp_port_change`].
#[derive(Debug, Default, Clone)]
pub struct HostedClientReapplySummary {
    /// Hosted + managed clients we tried to update.
    pub attempted: usize,
    /// Config files written immediately.
    pub applied: usize,
    /// Apply attempts that were deferred by the storage layer.
    pub scheduled: usize,
    /// Pairs of (client identifier, error message).
    pub failures: Vec<(String, String)>,
}

fn record_reapply_outcome(
    summary: &mut HostedClientReapplySummary,
    identifier: String,
    result: ConfigResult<ApplyOutcome>,
) {
    match result {
        Ok(outcome) if outcome.scheduled => summary.scheduled += 1,
        Ok(outcome) if outcome.applied => summary.applied += 1,
        Ok(_) => summary.failures.push((
            identifier,
            "apply finished without applied or scheduled write".to_string(),
        )),
        Err(error) => summary.failures.push((identifier, error.to_string())),
    }
}

impl ClientConfigService {
    /// Rewrite attached local config files for approved clients that inherit the global mode.
    ///
    /// Explicit per-client modes are left untouched. The caller owns the durable transition
    /// lifecycle and must inspect `failures` before marking that transition complete.
    pub async fn reapply_inherited_clients_after_default_mode_change(
        &self,
        target_mode: &str,
    ) -> ConfigResult<HostedClientReapplySummary> {
        let render_mode = match target_mode {
            "unify" | "hosted" => ConfigMode::Managed,
            "transparent" => ConfigMode::Native,
            _ => {
                return Err(ConfigError::DataAccessError(format!(
                    "invalid default client config mode: {target_mode}"
                )));
            }
        };
        let states = self.fetch_client_states().await?;
        let mut identifiers = states
            .iter()
            .filter_map(|(identifier, state)| {
                let inherits_default = state.config_mode.as_deref().map(str::trim).is_none_or(str::is_empty);
                (state.is_approved()
                    && inherits_default
                    && state.has_local_config_target()
                    && state.attachment_state() == AttachmentState::Attached)
                    .then(|| identifier.clone())
            })
            .collect::<Vec<_>>();
        identifiers.sort();

        let mut summary = HostedClientReapplySummary::default();
        for identifier in identifiers {
            summary.attempted += 1;
            let options = ClientRenderOptions {
                client_id: identifier.clone(),
                mode: render_mode.clone(),
                profile_id: None,
                server_ids: None,
                dry_run: false,
            };
            record_reapply_outcome(&mut summary, identifier, self.apply_with_deferred(options).await);
        }

        Ok(summary)
    }

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
            .map_err(|error| crate::clients::error::ConfigError::DataAccessError(error.to_string()))?;

        let mut summary = HostedClientReapplySummary::default();

        for template in templates {
            let identifier = template.identifier.clone();
            let state = states.get(&identifier);
            if !state.map(|s| s.is_approved()).unwrap_or(true) {
                continue;
            }

            let config_mode = state
                .and_then(|s| s.config_mode.as_deref())
                .unwrap_or(default_config_mode.as_str());
            if config_mode.eq_ignore_ascii_case("transparent") {
                tracing::debug!(
                    client = %identifier,
                    config_mode = %config_mode,
                    "Skipping client reapply: transparent mode"
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

            record_reapply_outcome(&mut summary, identifier, self.apply_with_deferred(options).await);
        }

        Ok(summary)
    }
}
