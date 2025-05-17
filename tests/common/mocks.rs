//! Mock implementations for testing
//!
//! Provides mock implementations of various components for testing.

use rmcp::model::{CallToolResult, Content};

/// Create a mock tool result
pub fn mock_tool_result(content: serde_json::Value) -> CallToolResult {
    CallToolResult {
        content: vec![Content::json(content).unwrap()],
        is_error: Some(false),
    }
}

/// Create a mock error result
pub fn mock_error_result(message: &str) -> CallToolResult {
    CallToolResult {
        content: vec![Content::text(message.to_string())],
        is_error: Some(true),
    }
}

// Note: The following mock functions are commented out because they require more detailed
// knowledge of the codebase. They will be implemented in a later phase when we have a better
// understanding of the internal structure of the UpstreamConnectionPool and RunningService.

// Create a mock connection pool with a single server
// pub async fn mock_connection_pool(
// server_name: &str,
// instance_id: &str,
// tools: Vec<Tool>,
// ) -> Result<Arc<Mutex<UpstreamConnectionPool>>> {
// Implementation will be added in a later phase
// unimplemented!("This function will be implemented in a later phase")
// }
//
// Create a mock service that returns a specific result
// pub fn mock_service_with_result(result: CallToolResult) -> RunningService<RoleClient, ()> {
// Implementation will be added in a later phase
// unimplemented!("This function will be implemented in a later phase")
// }
//
// Create a mock service that returns an error
// pub fn mock_service_with_error(error_message: &str) -> RunningService<RoleClient, ()> {
// Implementation will be added in a later phase
// unimplemented!("This function will be implemented in a later phase")
// }
//
// Create a mock service that validates the request
// pub fn mock_service_with_validation<F>(
// validator: F,
// result: CallToolResult,
// ) -> RunningService<RoleClient, ()>
// where
// F: Fn(&CallToolRequestParam) -> bool + Send + Sync + 'static,
// {
// Implementation will be added in a later phase
// unimplemented!("This function will be implemented in a later phase")
// }
//
// Add a mock service to a connection pool
// pub async fn add_mock_service_to_pool(
// pool: &Arc<Mutex<UpstreamConnectionPool>>,
// server_name: &str,
// instance_id: &str,
// service: RunningService<RoleClient, ()>,
// ) -> Result<()> {
// Implementation will be added in a later phase
// unimplemented!("This function will be implemented in a later phase")
// }
