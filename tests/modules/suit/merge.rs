//! Configuration suit merge tests
//!
//! Tests for merging and deduplicating configuration suits.

use std::sync::Arc;

use anyhow::Result;
use mcpmate::{
    conf::{
        database::Database,
        models::{ConfigSuit, ConfigSuitServer, Server},
    },
    core::suit::ConfigSuitMergeService,
};
use uuid::Uuid;

/// Create test database
async fn create_test_db() -> Result<Arc<Database>> {
    // Use in-memory database
    let db = Database::new().await?;

    // Initialize database
    db.initialize_defaults().await?;

    Ok(Arc::new(db))
}

/// Create test configuration suits
async fn create_test_suits(_db: &Database) -> Result<Vec<ConfigSuit>> {
    let mut suits = Vec::new();

    // Create first configuration suit
    let suit1 = ConfigSuit {
        id: Some(Uuid::new_v4().to_string()),
        name: "Test Suit 1".to_string(),
        description: Some("First test suit".to_string()),
        suit_type: "Scenario".to_string(),
        multi_select: true,
        priority: 10,
        is_active: true,
        is_default: true,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Create second configuration suit
    let suit2 = ConfigSuit {
        id: Some(Uuid::new_v4().to_string()),
        name: "Test Suit 2".to_string(),
        description: Some("Second test suit".to_string()),
        suit_type: "Scenario".to_string(),
        multi_select: true,
        priority: 5,
        is_active: true,
        is_default: false,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Save configuration suits to database
    // Note: In actual tests, we need to use database operations functions
    // Here we simplify the process, not actually saving to the database

    suits.push(suit1);
    suits.push(suit2);

    Ok(suits)
}

/// Create test servers
async fn create_test_servers(_db: &Database) -> Result<Vec<Server>> {
    let mut servers = Vec::new();

    // Create first server
    let server1 = Server {
        id: Some(Uuid::new_v4().to_string()),
        name: "test_server1".to_string(),
        server_type: "stdio".to_string(),
        command: Some("echo".to_string()),
        url: None,
        transport_type: None,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Create second server
    let server2 = Server {
        id: Some(Uuid::new_v4().to_string()),
        name: "test_server2".to_string(),
        server_type: "sse".to_string(),
        command: None,
        url: Some("http://localhost:8080/sse".to_string()),
        transport_type: None,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Save servers to database
    // Note: In actual tests, we need to use database operations functions
    // Here we simplify the process, not actually saving to the database

    servers.push(server1);
    servers.push(server2);

    Ok(servers)
}

/// Create test configuration suit servers
async fn create_test_suit_servers(
    _db: &Database,
    suits: &[ConfigSuit],
    servers: &[Server],
) -> Result<()> {
    // Add first server to first configuration suit
    let _suit_server1 = ConfigSuitServer {
        id: Some(Uuid::new_v4().to_string()),
        config_suit_id: suits[0].id.clone().unwrap(),
        server_id: servers[0].id.clone().unwrap(),
        enabled: true,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Add second server to second configuration suit
    let _suit_server2 = ConfigSuitServer {
        id: Some(Uuid::new_v4().to_string()),
        config_suit_id: suits[1].id.clone().unwrap(),
        server_id: servers[1].id.clone().unwrap(),
        enabled: true,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Save configuration suit servers to database
    // Note: In actual tests, we need to use database operations functions
    // Here we simplify the process, not actually saving to the database

    Ok(())
}

/// Test the basic functionality of the configuration suit merge service
#[tokio::test]
async fn test_config_suit_merge_service_basic() -> Result<()> {
    // Create test database
    let db = create_test_db().await?;

    // Create test configuration suits
    let suits = create_test_suits(&db).await?;

    // Create test servers
    let servers = create_test_servers(&db).await?;

    // Create test configuration suit servers
    create_test_suit_servers(&db, &suits, &servers).await?;

    // Create configuration suit merge service
    let merge_service = ConfigSuitMergeService::new(db.clone());

    // Get merged servers
    let merged_servers = merge_service.get_merged_servers().await?;

    // Verify merged results
    assert_eq!(merged_servers.len(), 2);

    // Verify server names
    let server_names: Vec<String> = merged_servers.iter().map(|s| s.name.clone()).collect();
    assert!(server_names.contains(&"test_server1".to_string()));
    assert!(server_names.contains(&"test_server2".to_string()));

    Ok(())
}
