//! Connection integration tests
//!
//! Tests for connection pool creation and management.

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use mcpmate::{
    core::{connection::UpstreamConnection, models::Config},
    http::pool::UpstreamConnectionPool,
};
use rmcp::model::Tool;
use tokio::sync::Mutex;

/// Test connection pool creation
#[tokio::test]
async fn test_connection_pool_creation() -> Result<()> {
    // Create empty connection pool with empty config
    let empty_config = Config {
        mcp_servers: HashMap::new(),
    };
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(
        empty_config,
    ))));

    // Add test connections to connection pool
    {
        let mut pool_guard = pool.lock().await;

        // Add first server connection
        let mut server1_connections = HashMap::new();
        let mut conn1 = UpstreamConnection::new("test_echo".to_string());
        conn1.tools.push(Tool {
            name: "tool1".to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });
        conn1.tools.push(Tool {
            name: "tool2".to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });
        server1_connections.insert("instance1".to_string(), conn1);
        pool_guard
            .connections
            .insert("test_echo".to_string(), server1_connections);

        // Add second server connection
        let mut server2_connections = HashMap::new();
        let mut conn2 = UpstreamConnection::new("test_sse".to_string());
        conn2.tools.push(Tool {
            name: "tool3".to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });
        conn2.tools.push(Tool {
            name: "tool4".to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });
        server2_connections.insert("instance1".to_string(), conn2);
        pool_guard
            .connections
            .insert("test_sse".to_string(), server2_connections);
    }

    // Verify connection pool
    let pool_guard = pool.lock().await;

    // Verify server connection
    assert!(pool_guard.connections.contains_key("test_echo"));
    assert!(pool_guard.connections.contains_key("test_sse"));

    // Verify instance
    let echo_connections = pool_guard.connections.get("test_echo").unwrap();
    assert!(echo_connections.contains_key("instance1"));

    // Verify tool
    let echo_instance = echo_connections.get("instance1").unwrap();
    assert_eq!(echo_instance.tools.len(), 2);
    assert_eq!(echo_instance.tools[0].name, "tool1");
    assert_eq!(echo_instance.tools[1].name, "tool2");

    Ok(())
}
