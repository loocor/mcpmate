//! Core Suit Module
//!
//! Business logic layer for configuration suits, responsible for configuration merging,
//! tool checking and other core business functions.
//!
//! ## Module Responsibilities
//! - Configuration suit merging algorithms
//! - Tool enablement status checking
//! - Server configuration aggregation
//! - Business rule validation
//!
//! ## Architecture Principles
//! - Only depends on config/suit data interfaces
//! - No direct database connection operations
//! - Communicates with other modules through event mechanisms

pub mod merge;
pub mod service;
pub mod types;

// Re-export core types and services
pub use merge::SuitMerger;
pub use service::SuitService;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::database::Database;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_suit_service_creation() {
        // This test just verifies that we can create a SuitService instance
        // without panicking. It doesn't require a real database connection.

        // Create a mock database (this will fail if we try to use it, but that's ok for this test)
        let db = Arc::new(Database::new().await.unwrap());

        // Create SuitService
        let _suit_service = SuitService::new(db);

        // If we get here without panicking, the test passes
        // We can't access private fields, so we just verify creation succeeded
    }

    #[test]
    fn test_suit_types_creation() {
        use std::collections::HashMap;

        // Test MergedServerConfig creation
        let server_config = MergedServerConfig {
            server_id: "test-server".to_string(),
            name: "Test Server".to_string(),
            address: "localhost:8080".to_string(),
            enabled_tools: vec!["tool1".to_string(), "tool2".to_string()],
            source_suits: vec!["suit1".to_string()],
        };

        assert_eq!(server_config.server_id, "test-server");
        assert_eq!(server_config.enabled_tools.len(), 2);

        // Test MergedToolConfig creation
        let tool_config = MergedToolConfig {
            tool_name: "test-tool".to_string(),
            enabled: true,
            server_ids: vec!["server1".to_string()],
            config: HashMap::new(),
            source_suits: vec!["suit1".to_string()],
        };

        assert_eq!(tool_config.tool_name, "test-tool");
        assert!(tool_config.enabled);

        // Test ToolEnabledResult creation
        let enabled_result = ToolEnabledResult {
            tool_name: "test-tool".to_string(),
            enabled: true,
            enabled_servers: vec!["server1".to_string()],
            related_suits: vec!["suit1".to_string()],
        };

        assert_eq!(enabled_result.tool_name, "test-tool");
        assert!(enabled_result.enabled);
    }
}
