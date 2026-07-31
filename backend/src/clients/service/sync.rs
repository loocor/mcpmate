//! Push profile-based **native** server lists to **transparent** clients only.

use super::core::{ClientConfigService, ClientRenderOptions};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientCapabilityConfig, ConfigMode, UnifyRouteMode};
use crate::config::client::init::resolve_default_client_config_mode;
use crate::core::capability::mode_policy::{
    EffectiveConfigMode, ProfileScopePolicy, SurfaceParticipation, resolve_surface_composition_policy,
};

fn profile_change_affects_native_config(
    scope: ProfileScopePolicy,
    capability_config: &ClientCapabilityConfig,
    profile_id: &str,
) -> bool {
    match scope {
        ProfileScopePolicy::Ignored => false,
        ProfileScopePolicy::Activated => true,
        ProfileScopePolicy::Selected => capability_config
            .selected_profile_ids
            .iter()
            .any(|selected| selected == profile_id),
        ProfileScopePolicy::Custom => capability_config.custom_profile_id.as_deref() == Some(profile_id),
    }
}

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
        let descriptors = self.list_clients(false, false).await?;
        let default_config_mode = resolve_default_client_config_mode(&self.db_pool)
            .await
            .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;

        let mut ok = 0usize;
        let mut failures = std::collections::HashMap::new();

        for descriptor in descriptors {
            let client_id = descriptor.state.identifier().to_string();
            if !descriptor.state.is_approved() {
                continue;
            }

            let state = states.get(&client_id).ok_or_else(|| {
                ConfigError::DataAccessError(format!(
                    "Client state not found while synchronizing native profile: {client_id}"
                ))
            })?;
            let config_mode = crate::config::client::init::effective_client_config_mode(
                state.config_mode.as_deref(),
                &default_config_mode,
            );
            let effective_mode = EffectiveConfigMode::parse(config_mode).ok_or_else(|| {
                ConfigError::DataAccessError(format!(
                    "Invalid effective client config mode '{config_mode}' for {client_id}"
                ))
            })?;
            let capability_config = state.capability_config()?;
            let policy = resolve_surface_composition_policy(
                effective_mode,
                capability_config.capability_source,
                UnifyRouteMode::BrokerOnly,
            );
            if policy.participation != SurfaceParticipation::Native
                || !profile_change_affects_native_config(policy.profile_scope, &capability_config, profile_id)
            {
                continue;
            }

            let options = ClientRenderOptions {
                client_id: client_id.clone(),
                mode: ConfigMode::Native,
                profile_id: None,
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

#[cfg(test)]
mod tests {
    use crate::clients::models::{CapabilitySource, ClientCapabilityConfig, UnifyRouteMode};
    use crate::core::capability::mode_policy::{
        EffectiveConfigMode, SurfaceParticipation, resolve_surface_composition_policy,
    };

    use super::profile_change_affects_native_config;

    #[test]
    fn native_sync_uses_the_shared_mode_policy_and_client_profile_scope() {
        let selected = ClientCapabilityConfig {
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: vec!["profile-a".to_string()],
            custom_profile_id: None,
        };
        for mode in [EffectiveConfigMode::Unify, EffectiveConfigMode::Hosted] {
            let policy =
                resolve_surface_composition_policy(mode, selected.capability_source, UnifyRouteMode::BrokerOnly);
            assert_ne!(policy.participation, SurfaceParticipation::Native);
        }

        let selected_policy = resolve_surface_composition_policy(
            EffectiveConfigMode::Transparent,
            selected.capability_source,
            UnifyRouteMode::BrokerOnly,
        );
        assert_eq!(selected_policy.participation, SurfaceParticipation::Native);
        assert!(profile_change_affects_native_config(
            selected_policy.profile_scope,
            &selected,
            "profile-a",
        ));
        assert!(!profile_change_affects_native_config(
            selected_policy.profile_scope,
            &selected,
            "profile-b",
        ));

        let custom = ClientCapabilityConfig {
            capability_source: CapabilitySource::Custom,
            selected_profile_ids: Vec::new(),
            custom_profile_id: Some("profile-custom".to_string()),
        };
        let custom_policy = resolve_surface_composition_policy(
            EffectiveConfigMode::Transparent,
            custom.capability_source,
            UnifyRouteMode::BrokerOnly,
        );
        assert!(profile_change_affects_native_config(
            custom_policy.profile_scope,
            &custom,
            "profile-custom",
        ));
        assert!(!profile_change_affects_native_config(
            custom_policy.profile_scope,
            &custom,
            "profile-a",
        ));
    }
}
