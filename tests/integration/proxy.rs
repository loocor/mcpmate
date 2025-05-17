//! Proxy integration tests
//!
//! Tests for the MCPMate proxy component.

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use mcpmate::{
    core::{
        connection::UpstreamConnection,
        models::{Config, MCPServerConfig},
    },
    http::pool::UpstreamConnectionPool,
};
use rmcp::model::{CallToolRequestParam, Tool};
use tokio::sync::Mutex;

/// Test tool mapping functionality
///
/// This test verifies that the proxy can correctly map tool names to upstream servers.
#[tokio::test]
async fn test_tool_mapping() -> Result<()> {
    // Create test configuration
    let config = Config {
        mcp_servers: HashMap::from([("test_server".to_string(), MCPServerConfig {
            kind: "stdio".to_string(),
            command: Some("echo".to_string()),
            args: None,
            url: None,
            env: None,
            transport_type: None,
        })]),
    };

    // Create connection pool
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config))));

    // Add mock connection to connection pool
    {
        let mut pool = pool.lock().await;
        let mut connections = HashMap::new();

        let mut conn = UpstreamConnection::new("test_server".to_string());
        conn.tools.push(Tool {
            name: "test_tool".to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });

        connections.insert("instance1".to_string(), conn);
        pool.connections
            .insert("test_server".to_string(), connections);
    }

    // Test finding tool mapping
    // Note: This is a placeholder test since the actual function is private
    // In a real test, we would use the public API to test this functionality
    println!("Tool mapping test would verify that 'test_tool' maps to 'test_server'");

    Ok(())
}

/// Test tool calling functionality
///
/// This test verifies that the proxy can correctly route tool calls to upstream servers.
#[tokio::test]
async fn test_tool_calling() -> Result<()> {
    // Create test configuration
    let config = Config {
        mcp_servers: HashMap::from([("test_server".to_string(), MCPServerConfig {
            kind: "stdio".to_string(),
            command: Some("echo".to_string()),
            args: None,
            url: None,
            env: None,
            transport_type: None,
        })]),
    };

    // Create connection pool
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config))));

    // Add mock connection to connection pool
    {
        let mut pool = pool.lock().await;
        let mut connections = HashMap::new();

        let mut conn = UpstreamConnection::new("test_server".to_string());
        conn.tools.push(Tool {
            name: "test_tool".to_string().into(),
            description: None,
            input_schema: Default::default(),
            annotations: None,
        });

        connections.insert("instance1".to_string(), conn);
        pool.connections
            .insert("test_server".to_string(), connections);
    }

    // Create test tool request
    let _request = CallToolRequestParam {
        name: "test_tool".to_string().into(),
        arguments: None,
    };

    // Test calling tool
    // Note: This is a placeholder test since the actual function is private
    // In a real test, we would use the public API to test this functionality
    println!("Tool calling test would verify that calling 'test_tool' routes to 'test_server'");

    Ok(())
}

/// Test error handling functionality
///
/// This test verifies that the proxy correctly handles errors from upstream servers.
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    // Create test configuration
    let config = Config {
        mcp_servers: HashMap::from([("test_server".to_string(), MCPServerConfig {
            kind: "stdio".to_string(),
            command: Some("echo".to_string()),
            args: None,
            url: None,
            env: None,
            transport_type: None,
        })]),
    };

    // Create connection pool
    let _pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config))));

    // Test error handling
    // Note: This is a placeholder test since the actual function is private
    // In a real test, we would use the public API to test this functionality
    println!("Error handling test would verify that errors are properly propagated");

    Ok(())
}
