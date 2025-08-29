// Client applications database initialization
// Handles client and client_detection_rules tables
// Now supports data-driven initialization from configuration

use crate::config::client::models::{ClientConfigFile, load_client_config};
use crate::generate_id;
use anyhow::Result;
use sqlx::SqlitePool;

/// Initialize clientlications related tables and data
pub async fn initialize_client(pool: &SqlitePool) -> Result<()> {
    create_client_tables(pool).await?;
    ensure_default_client(pool).await?;
    Ok(())
}

/// Create clientlications related tables
async fn create_client_tables(pool: &SqlitePool) -> Result<()> {
    // Create client table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS client (
            id TEXT PRIMARY KEY,
            identifier TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL,
            description TEXT,
            logo_url TEXT,
            category TEXT DEFAULT 'application',
            enabled BOOLEAN DEFAULT FALSE,
            detected BOOLEAN DEFAULT FALSE,
            last_detected_at DATETIME,
            install_path TEXT,
            config_path TEXT,
            version TEXT,
            detection_method TEXT,
            config_mode TEXT DEFAULT 'transparent',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create client_detection_rules table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS client_detection_rules (
            id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            identifier TEXT NOT NULL,
            platform TEXT NOT NULL,
            detection_method TEXT NOT NULL,
            detection_value TEXT NOT NULL,
            config_path TEXT NOT NULL,
            priority INTEGER DEFAULT 0,
            enabled BOOLEAN DEFAULT TRUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (client_id) REFERENCES client(id) ON DELETE CASCADE,
            UNIQUE(client_id, platform, detection_method, detection_value)
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create client_config_rules table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS client_config_rules (
            id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            identifier TEXT NOT NULL,
            top_level_key TEXT NOT NULL,
            config_type TEXT DEFAULT 'standard',
            supported_transports TEXT NOT NULL,
            supported_runtimes TEXT NOT NULL,
            format_rules TEXT NOT NULL,
            security_features TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (client_id) REFERENCES client(id) ON DELETE CASCADE,
            UNIQUE(client_id)
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for better performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_client_identifier ON client(identifier)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_detection_rules_client_platform ON client_detection_rules(client_id, platform)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert default clientlications from config file
/// This replaces all hardcoded configuration functions
async fn insert_default_client(pool: &SqlitePool) -> Result<()> {
    // Try to load from config file first
    match load_client_config("config/client.json").await {
        Ok(config) => {
            tracing::info!("Loading clientlications from config/client.json");
            insert_client_from_config(pool, &config).await?;
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load config/client.json: {}, falling back to hardcoded data",
                e
            )
        }
    }

    Ok(())
}

/// Insert client from loaded configuration
async fn insert_client_from_config(
    pool: &SqlitePool,
    config: &ClientConfigFile,
) -> Result<()> {
    for mut client in config.client.clone() {
        let client_id = generate_id!("clin");

        // Prepare client for database insertion (auto-fills missing DB fields)
        client.prepare_for_db_insert(client_id.clone());

        // Insert client
        sqlx::query(
            r#"
            INSERT INTO client (id, identifier, display_name, description, logo_url, category, enabled)
            VALUES (?, ?, ?, ?, ?, ?, FALSE)
            "#,
        )
        .bind(&client_id)
        .bind(&client.identifier)
        .bind(&client.display_name)
        .bind(client.description.as_ref().unwrap())
        .bind(client.logo_url.as_ref())
        .bind(client.category.to_string())
        .execute(pool)
        .await?;

        // Insert detection rules for each platform
        for rules in client.detection_rules.values() {
            for rule in rules {
                sqlx::query(
                    r#"
                    INSERT INTO client_detection_rules
                    (id, client_id, identifier, platform, detection_method, detection_value, config_path, priority)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(rule.id.as_ref().unwrap())
                .bind(rule.client_id.as_ref().unwrap())
                .bind(rule.identifier.as_ref().unwrap())
                .bind(rule.platform.as_ref().unwrap())
                .bind(&rule.detection_method)
                .bind(&rule.detection_value)
                .bind(rule.config_path.as_ref().unwrap())
                .bind(rule.priority)
                .execute(pool)
                .await?;
            }
        }

        // Insert config rules
        let supported_transports_json = serde_json::to_string(&client.config_rules.supported_transports)?;
        let supported_runtimes_json = serde_json::to_string(&client.config_rules.supported_runtimes)?;
        let format_rules_json = serde_json::to_string(&client.config_rules.format_rules)?;
        let security_features_json = client
            .config_rules
            .security_features
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        // Convert config_type to string for database storage
        let config_type_str = match client.config_rules.config_type {
            crate::config::client::models::ClientConfigType::Standard => "standard",
            crate::config::client::models::ClientConfigType::Mixed => "mixed",
            crate::config::client::models::ClientConfigType::Array => "array",
        };

        sqlx::query(
            r#"
            INSERT INTO client_config_rules
            (id, client_id, identifier, top_level_key, config_type,
             supported_transports, supported_runtimes, format_rules, security_features)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(client.config_rules.id.as_ref().unwrap())
        .bind(client.config_rules.client_id.as_ref().unwrap())
        .bind(client.config_rules.identifier.as_ref().unwrap())
        .bind(&client.config_rules.top_level_key)
        .bind(config_type_str)
        .bind(supported_transports_json)
        .bind(supported_runtimes_json)
        .bind(format_rules_json)
        .bind(security_features_json)
        .execute(pool)
        .await?;
    }

    tracing::info!("Inserted {} clientlications from config file", config.client.len());
    Ok(())
}

/// Ensure default clientlications exist in database
/// Only inserts data if tables are empty (first-time initialization)
async fn ensure_default_client(pool: &SqlitePool) -> Result<()> {
    // Check if we already have client in the database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM client")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        tracing::info!("Client apps already exist in database, skipping initialization");
        return Ok(());
    }

    tracing::info!("Initializing default clientlications in database");

    // Insert default clientlications using SQL migrations
    // This is the ONLY place where we define default data
    insert_default_client(pool).await?;

    Ok(())
}
