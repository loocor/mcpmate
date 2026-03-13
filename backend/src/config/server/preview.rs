// Capability preview (no DB / no REDB / no pool)
// Provides helpers to discover capabilities for an arbitrary server config safely.

use crate::common::server::ServerType;
use crate::core::models::MCPServerConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

/// Preview capabilities for a single server config.
/// Uses direct discovery without touching connection pool or caches.
pub async fn preview_capabilities(
    name: &str,
    kind: ServerType,
    command: Option<String>,
    url: Option<String>,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    _timeout: Option<Duration>,
) -> Result<crate::config::server::capabilities::CapabilitySnapshot> {
    tracing::info!(
        target: "mcpmate::config::server::preview",
        server = %name,
        kind = ?kind,
        "Preview capabilities (no side effects)"
    );
    // Build a minimal MCPServerConfig; include args/env from request for accurate preview
    let cfg = MCPServerConfig {
        kind,
        command,
        url,
        args,
        env,
        headers: None,
    };
    // Reuse existing discovery (no dual-write, no pool)
    crate::config::server::capabilities::discover_from_config(name, &cfg, kind).await
}
