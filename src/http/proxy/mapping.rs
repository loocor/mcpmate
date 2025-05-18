// Tool mapping implementation for the HTTP proxy server

use std::collections::HashMap;

use crate::{core::tool, http::proxy::core::HttpProxyServer};

/// Get the tool name mapping directly from the connection pool
///
/// This function builds a mapping of client-facing tool names to upstream tool names
/// by querying the connection pool directly.
pub async fn get_tool_name_mapping(
    server: &HttpProxyServer
) -> HashMap<String, crate::core::tool::ToolNameMapping> {
    // Build a new tool name mapping
    let start_time = std::time::Instant::now();
    let mapping = tool::build_name_mapping(&server.connection_pool).await;
    let build_time = start_time.elapsed();

    tracing::debug!(
        "Built tool name mapping with {} entries (build time: {:?})",
        mapping.len(),
        build_time
    );

    mapping
}
