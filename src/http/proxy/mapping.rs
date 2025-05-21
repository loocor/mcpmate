// Tool mapping implementation for the HTTP proxy server

use std::collections::HashMap;

use crate::{core::tool, http::proxy::core::HttpProxyServer};

/// Get the tool name mapping directly from the connection pool
///
/// This function builds a mapping of tool names to server information
/// by querying the connection pool directly.
pub async fn get_tool_name_mapping(
    server: &HttpProxyServer
) -> HashMap<String, crate::core::tool::ToolMapping> {
    // Build a new tool mapping
    let start_time = std::time::Instant::now();
    let mapping = tool::build_tool_mapping(&server.connection_pool).await;
    let build_time = start_time.elapsed();

    tracing::debug!(
        "Built tool mapping with {} entries (build time: {:?})",
        mapping.len(),
        build_time
    );

    mapping
}
