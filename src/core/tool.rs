// Core tool module for MCPMan
// Contains shared tool mapping and routing functions

use rmcp::model::Tool;

/// Parse a tool name to extract server prefix if present
/// Returns (Option<server_prefix>, original_tool_name)
pub fn parse_tool_name(tool_name: &str) -> (Option<&str>, &str) {
    if let Some(pos) = tool_name.find('_') {
        let server_prefix = &tool_name[0..pos];
        let remaining = &tool_name[pos + 1..];

        // Check if the remaining part starts with the same prefix
        // This handles cases like "playwright_playwright_navigate"
        let prefix_repeated = remaining.starts_with(&format!("{}_", server_prefix));

        if prefix_repeated {
            // Skip the repeated prefix
            if let Some(second_pos) = remaining.find('_') {
                let original_tool_name = &remaining[second_pos + 1..];
                tracing::debug!(
                    "Detected repeated prefix in tool name: '{}' -> server: '{}', tool: '{}'",
                    tool_name,
                    server_prefix,
                    original_tool_name
                );
                return (Some(server_prefix), original_tool_name);
            }
        }

        (Some(server_prefix), remaining)
    } else {
        (None, tool_name)
    }
}

/// Detect if a server's tools already have a common prefix
pub fn detect_common_prefix(tools: &[Tool], server_name: &str) -> bool {
    if tools.is_empty() {
        return false;
    }

    // Check if all tools start with the server name followed by underscore
    let server_prefix = format!("{}_", server_name.to_lowercase());
    let all_have_server_prefix = tools.iter().all(|tool| {
        tool.name
            .to_string()
            .to_lowercase()
            .starts_with(&server_prefix)
    });

    // If all tools have the server prefix, return true
    if all_have_server_prefix {
        tracing::debug!(
            "All tools for server '{}' already have the server prefix",
            server_name
        );
        return true;
    }

    // Check for other common prefixes (e.g., playwright_, firecrawl_)
    if tools.len() >= 2 {
        // Get the first tool name as String
        let first_tool = tools[0].name.to_string();

        // Find the position of the first underscore
        if let Some(pos) = first_tool.find('_') {
            // Extract the prefix
            let potential_prefix = &first_tool[0..=pos];

            // Check if all tools have this prefix
            let all_have_same_prefix = tools
                .iter()
                .all(|tool| tool.name.to_string().starts_with(potential_prefix));

            if all_have_same_prefix {
                tracing::debug!(
                    "Detected common prefix '{}' for server '{}' tools",
                    potential_prefix,
                    server_name
                );

                // Check if the detected prefix is the same as the server name
                // This handles cases like "playwright_" prefix for "playwright" server
                if potential_prefix.to_lowercase() == server_prefix.to_lowercase() {
                    tracing::debug!("Common prefix matches server name for '{}'", server_name);
                }

                return true;
            }
        }
    }

    false
}

/// Tool name mapping information
#[derive(Debug, Clone)]
pub struct ToolNameMapping {
    /// Client-facing tool name (with prefix if needed)
    pub client_tool_name: String,
    /// Server name
    pub server_name: String,
    /// Instance ID
    pub instance_id: String,
    /// Original upstream tool name (without any modifications)
    pub upstream_tool_name: String,
}
