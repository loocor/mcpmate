use crate::clients::ClientConfigService;
use crate::clients::models::UnifyDirectExposureConfig;
use crate::core::proxy::server::ProxyServer;

pub(super) async fn sync_bound_client_runtime_state(
    service: &ClientConfigService,
    identifier: &str,
    notify_visible_change: bool,
) {
    let Some((effective_mode, unify_workspace)) = load_bound_client_runtime_state(service, identifier).await else {
        return;
    };

    if let Some(proxy_server) = ProxyServer::global().and_then(|proxy| proxy.try_lock().ok().map(|guard| guard.clone()))
    {
        if let Err(err) = proxy_server
            .apply_persisted_client_runtime_state(identifier, Some(effective_mode.clone()), unify_workspace.clone())
            .await
        {
            tracing::warn!(client = %identifier, error = %err, mode = %effective_mode, "Failed to sync bound client runtime state");
        }
    }

    if notify_visible_change && matches!(effective_mode.as_str(), "hosted" | "unify") {
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ClientVisibleDirectSurfaceChanged {
                client_id: identifier.to_string(),
            },
        );
    }
}

async fn load_bound_client_runtime_state(
    service: &ClientConfigService,
    identifier: &str,
) -> Option<(String, Option<UnifyDirectExposureConfig>)> {
    let effective_mode = match service.get_effective_config_mode(identifier).await {
        Ok(mode) => mode,
        Err(err) => {
            tracing::warn!(client = %identifier, error = %err, "Failed to resolve effective config mode for runtime state sync");
            return None;
        }
    };

    let unify_workspace = if effective_mode == "unify" {
        match service.get_unify_direct_exposure_config(identifier).await {
            Ok(workspace) => Some(workspace.unwrap_or_default()),
            Err(err) => {
                tracing::error!(client = %identifier, error = %err, "Failed to load unify direct exposure config for runtime state sync");
                return None;
            }
        }
    } else {
        None
    };

    Some((effective_mode, unify_workspace))
}
