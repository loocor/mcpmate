use once_cell::sync::OnceCell;
use rmcp::{
    ClientHandler, RoleClient,
    model::{
        ClientCapabilities, ClientInfo, Implementation, InitializeResult, NotificationMetaObject, ProgressToken,
        ProtocolVersion, ServerPeerInfo,
    },
};

use crate::core::proxy::server::ProxyServer;

fn global_proxy_server() -> Option<ProxyServer> {
    ProxyServer::global().and_then(|server| server.try_lock().ok().map(|guard| guard.clone()))
}

pub(crate) fn legacy_initialize_result(peer_info: Option<&ServerPeerInfo>) -> anyhow::Result<Option<InitializeResult>> {
    peer_info
        .map(|peer| {
            let server_info = peer.server_info.clone().ok_or_else(|| {
                anyhow::anyhow!("Legacy initialize completed without server implementation information")
            })?;
            let mut result = InitializeResult::new(peer.capabilities.clone())
                .with_protocol_version(peer.protocol_version.clone())
                .with_server_info(server_info);
            if let Some(instructions) = peer.instructions.clone() {
                result = result.with_instructions(instructions);
            }
            result.meta = peer.meta.clone();
            Ok(result)
        })
        .transpose()
}

fn notification_progress_token(meta: &NotificationMetaObject) -> Option<ProgressToken> {
    meta.get("progressToken")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

/// Minimal upstream client handler used by the proxy when connecting to upstream MCP servers.
///
/// Step 1 (PR6): only logs and accepts notifications; no downstream forwarding yet.
#[derive(Clone, Debug)]
pub struct UpstreamClientHandler {
    server_label: String,
    server_id: OnceCell<String>,
}

impl UpstreamClientHandler {
    pub fn new(server_label: String) -> Self {
        Self {
            server_label,
            server_id: OnceCell::new(),
        }
    }

    pub fn set_server_id(
        &self,
        server_id: &str,
    ) {
        let _ = self.server_id.set(server_id.to_string());
    }

    fn build_client_impl() -> Implementation {
        // Build a client identity for upstream initialize
        Implementation::new("mcpmate-proxy", env!("CARGO_PKG_VERSION"))
            .with_title("MCPMate Proxy Client")
            .with_icons(vec![crate::common::constants::branding::create_logo_icon()])
            .with_website_url(crate::common::constants::branding::WEBSITE_URL)
    }
}

impl ClientHandler for UpstreamClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(ClientCapabilities::default(), Self::build_client_impl())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
    }

    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        context: rmcp::service::NotificationContext<RoleClient>,
    ) {
        tracing::debug!(
            server = %self.server_label,
            progress_token = ?params.progress_token,
            progress = ?params.progress,
            total = ?params.total,
            message = ?params.message,
            "Upstream progress received"
        );
        if let Some(server_id) = self.server_id.get() {
            if let Some(proxy_server) = global_proxy_server() {
                let _ = proxy_server
                    .forward_upstream_progress(server_id, params.clone(), notification_progress_token(&context.meta))
                    .await;
            }
        }
        let _ = crate::inspector::service::inspector_forward_progress(&params).await;
    }

    async fn on_cancelled(
        &self,
        params: rmcp::model::CancelledNotificationParam,
        _context: rmcp::service::NotificationContext<RoleClient>,
    ) {
        tracing::debug!(
            server = %self.server_label,
            request_id = ?params.request_id,
            reason = ?params.reason,
            "Upstream request cancelled"
        );
        let Some(request_id) = params.request_id.as_ref() else {
            tracing::warn!(
                server = %self.server_label,
                reason = ?params.reason,
                "Ignoring upstream cancellation without a request ID"
            );
            return;
        };
        if let Some(server_id) = self.server_id.get() {
            if let Some(proxy_server) = global_proxy_server() {
                let _ = proxy_server.forward_upstream_cancelled(server_id, params.clone()).await;
            }
        }
        let _ = crate::inspector::service::inspector_forward_cancel(request_id, params.reason.clone()).await;
    }

    #[expect(deprecated, reason = "MCPMate preserves negotiated upstream logging notifications")]
    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        context: rmcp::service::NotificationContext<RoleClient>,
    ) {
        tracing::trace!(
            server = %self.server_label,
            level = ?params.level,
            logger = ?params.logger,
            data = ?params.data,
            "Upstream log message"
        );
        if let Some(server_id) = self.server_id.get() {
            if let Some(proxy_server) = global_proxy_server() {
                let _ = proxy_server
                    .forward_upstream_log(server_id, params.clone(), notification_progress_token(&context.meta))
                    .await;
            }
        }
        let token = notification_progress_token(&context.meta);
        let token_ref = token.as_ref();
        let _ = crate::inspector::service::inspector_forward_log(token_ref, &params).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_initialize_result_preserves_initialize_fields() {
        let original = InitializeResult::new(rmcp::model::ServerCapabilities::default())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_server_info(Implementation::new("test-server", "1.0.0"))
            .with_instructions("test instructions");
        let peer_info = ServerPeerInfo::from(original.clone());

        let converted = legacy_initialize_result(Some(&peer_info))
            .expect("convert legacy peer info")
            .expect("peer info should produce initialize result");

        assert_eq!(converted, original);
    }

    #[test]
    fn upstream_client_prefers_2025_11_25() {
        let handler = UpstreamClientHandler::new("test-server".to_string());

        assert_eq!(handler.get_info().protocol_version, ProtocolVersion::V_2025_11_25);
    }
}
