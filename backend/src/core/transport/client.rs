use once_cell::sync::OnceCell;
use rmcp::{
    ClientHandler, RoleClient,
    model::{ClientCapabilities, ClientInfo, Implementation, ProtocolVersion},
};

use crate::core::proxy::server::ProxyServer;

fn global_proxy_server() -> Option<ProxyServer> {
    ProxyServer::global().and_then(|server| server.try_lock().ok().map(|guard| guard.clone()))
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
        // Use widely supported version for upstream compatibility; headers handle 2025-06-18 at proxy edge
        ClientInfo::new(ClientCapabilities::default(), Self::build_client_impl())
            .with_protocol_version(ProtocolVersion::V_2025_03_26)
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
                    .forward_upstream_progress(server_id, params.clone(), context.meta.get_progress_token())
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
        if let Some(server_id) = self.server_id.get() {
            if let Some(proxy_server) = global_proxy_server() {
                let _ = proxy_server.forward_upstream_cancelled(server_id, params.clone()).await;
            }
        }
        let _ = crate::inspector::service::inspector_forward_cancel(&params.request_id, params.reason.clone()).await;
    }

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
                    .forward_upstream_log(server_id, params.clone(), context.meta.get_progress_token())
                    .await;
            }
        }
        let token = context.meta.get_progress_token();
        let token_ref = token.as_ref();
        let _ = crate::inspector::service::inspector_forward_log(token_ref, &params).await;
    }
}
