// Capability preview (no DB / no REDB / no pool)
// Provides helpers to discover capabilities for an arbitrary server config safely.

use crate::common::server::ServerType;
use crate::core::models::MCPServerConfig;
use anyhow::Result;
use std::time::Duration;

/// Preview capabilities for a single server config.
/// Uses direct discovery without touching connection pool or caches.
pub async fn preview_capabilities(
    name: &str,
    kind: ServerType,
    command: Option<String>,
    url: Option<String>,
    _timeout: Option<Duration>,
) -> Result<crate::config::server::capabilities::CapabilitySnapshot> {
    tracing::info!(
        target: "mcpmate::config::server::preview",
        server = %name,
        kind = ?kind,
        "Preview capabilities (no side effects)"
    );
    // Build a minimal MCPServerConfig; args/env are None for preview (handlers may enrich later if needed)
    let cfg = MCPServerConfig {
        kind,
        command,
        url,
        args: None,
        env: None,
        transport_type: None,
    };
    // Reuse existing discovery (no dual-write, no pool)
    crate::config::server::capabilities::discover_from_config(name, &cfg, kind).await
}
