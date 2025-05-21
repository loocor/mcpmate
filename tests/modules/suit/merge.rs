//! Configuration suit merge tests
//!
//! Tests for merging and deduplicating configuration suits.

use std::sync::Arc;

use anyhow::Result;
use mcpmate::{
    common::types::{ConfigSuitType, EnabledStatus, ServerType},
    conf::{
        database::Database,
        models::{ConfigSuit, ConfigSuitServer, Server},
    },
    core::suit::ConfigSuitMergeService,
};
use nanoid::nanoid;

/// Create test database
async fn create_test_db() -> Result<Arc<Database>> {
    // Use in-memory database
    unsafe {
        std::env::set_var("DATABASE_URL", "sqlite::memory:");
    }
    let db = Database::new().await?;

    // Initialize database
    db.initialize_defaults().await?;

    Ok(Arc::new(db))
}

/// Create test configuration suits
async fn create_test_suits(db: &Database) -> Result<Vec<ConfigSuit>> {
    let mut suits = Vec::new();

    // Create first configuration suit
    let mut suit1 = ConfigSuit {
        id: Some(format!("suit{}", nanoid!(12))),
        name: "Test Suit 1".to_string(),
        description: Some("First test suit".to_string()),
        suit_type: ConfigSuitType::Scenario,
        multi_select: true,
        priority: 10,
        is_active: true,
        is_default: true,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Create second configuration suit
    let mut suit2 = ConfigSuit {
        id: Some(format!("suit{}", nanoid!(12))),
        name: "Test Suit 2".to_string(),
        description: Some("Second test suit".to_string()),
        suit_type: ConfigSuitType::Scenario,
        multi_select: true,
        priority: 5,
        is_active: true,
        is_default: false,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Save configuration suits to database
    suit1.id = Some(mcpmate::conf::operations::upsert_config_suit(&db.pool, &suit1).await?);
    suit2.id = Some(mcpmate::conf::operations::upsert_config_suit(&db.pool, &suit2).await?);

    suits.push(suit1);
    suits.push(suit2);

    Ok(suits)
}

/// Create test servers
async fn create_test_servers(db: &Database) -> Result<Vec<Server>> {
    let mut servers = Vec::new();

    // Create first server
    let mut server1 = Server {
        id: Some(format!("ssrv{}", nanoid!(12))),
        name: "test_server1".to_string(),
        server_type: ServerType::Stdio,
        command: Some("echo".to_string()),
        url: None,
        transport_type: None,
        enabled: EnabledStatus::Enabled,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Create second server
    let mut server2 = Server {
        id: Some(format!("ssrv{}", nanoid!(12))),
        name: "test_server2".to_string(),
        server_type: ServerType::Sse,
        command: None,
        url: Some("http://localhost:8080/sse".to_string()),
        transport_type: None,
        enabled: EnabledStatus::Enabled,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Save servers to database
    server1.id = Some(mcpmate::conf::operations::upsert_server(&db.pool, &server1).await?);
    server2.id = Some(mcpmate::conf::operations::upsert_server(&db.pool, &server2).await?);

    servers.push(server1);
    servers.push(server2);

    Ok(servers)
}

/// Create test configuration suit servers
async fn create_test_suit_servers(
    db: &Database,
    suits: &[ConfigSuit],
    servers: &[Server],
) -> Result<()> {
    // Add first server to first configuration suit
    let suit_server1 = ConfigSuitServer {
        id: Some(format!("suit{}", nanoid!(12))),
        config_suit_id: suits[0].id.clone().unwrap(),
        server_id: servers[0].id.clone().unwrap(),
        enabled: true,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Add second server to second configuration suit
    let suit_server2 = ConfigSuitServer {
        id: Some(format!("suit{}", nanoid!(12))),
        config_suit_id: suits[1].id.clone().unwrap(),
        server_id: servers[1].id.clone().unwrap(),
        enabled: true,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    };

    // Save configuration suit servers to database
    mcpmate::conf::operations::suit::add_server_to_config_suit(
        &db.pool,
        &suit_server1.config_suit_id,
        &suit_server1.server_id,
        suit_server1.enabled,
    )
    .await?;

    mcpmate::conf::operations::suit::add_server_to_config_suit(
        &db.pool,
        &suit_server2.config_suit_id,
        &suit_server2.server_id,
        suit_server2.enabled,
    )
    .await?;

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

    // Verify that our test servers are in the merged results
    let server_names: Vec<String> = merged_servers.iter().map(|s| s.name.clone()).collect();
    assert!(server_names.contains(&"test_server1".to_string()));
    assert!(server_names.contains(&"test_server2".to_string()));

    // Verify that we have at least our 2 test servers
    assert!(merged_servers.len() >= 2);

    Ok(())
}
