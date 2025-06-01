// Client applications database initialization
// Handles client_apps and client_detection_rules tables

use crate::generate_id;
use anyhow::Result;
use sqlx::SqlitePool;

/// Initialize client applications related tables and data
pub async fn initialize_client_apps(pool: &SqlitePool) -> Result<()> {
    create_client_apps_tables(pool).await?;
    preload_client_apps_data(pool).await?;
    Ok(())
}

/// Create client applications related tables
async fn create_client_apps_tables(pool: &SqlitePool) -> Result<()> {
    // Create client_apps table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS client_apps (
            id TEXT PRIMARY KEY,
            identifier TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL,
            description TEXT,
            enabled BOOLEAN DEFAULT FALSE,
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
            client_app_id TEXT NOT NULL,
            platform TEXT NOT NULL,
            detection_method TEXT NOT NULL,
            detection_value TEXT NOT NULL,
            config_path_template TEXT,
            priority INTEGER DEFAULT 0,
            enabled BOOLEAN DEFAULT TRUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (client_app_id) REFERENCES client_apps(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for better performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_client_apps_identifier ON client_apps(identifier)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_detection_rules_client_platform ON client_detection_rules(client_app_id, platform)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Preload known client applications and their detection rules
async fn preload_client_apps_data(pool: &SqlitePool) -> Result<()> {
    let clients = vec![
        (
            "claude_desktop",
            "Claude Desktop",
            "Anthropic's Claude Desktop App",
        ),
        ("cursor", "Cursor", "AI-powered code editor"),
        ("windsurf", "Windsurf", "High-performance code editor"),
    ];

    for (identifier, display_name, description) in clients {
        let client_id = generate_id!("capp");

        // Insert client app (ignore if already exists)
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO client_apps (id, identifier, display_name, description, enabled)
            VALUES (?, ?, ?, ?, FALSE)
        "#,
        )
        .bind(&client_id)
        .bind(identifier)
        .bind(display_name)
        .bind(description)
        .execute(pool)
        .await?;

        // Get the actual client_id (in case it already existed)
        let actual_client_id: String =
            sqlx::query_scalar("SELECT id FROM client_apps WHERE identifier = ?")
                .bind(identifier)
                .fetch_one(pool)
                .await?;

        // Preload detection rules for this client
        preload_detection_rules_for_client(pool, &actual_client_id, identifier).await?;
    }

    Ok(())
}

/// Preload detection rules for a specific client
async fn preload_detection_rules_for_client(
    pool: &SqlitePool,
    client_id: &str,
    identifier: &str,
) -> Result<()> {
    match identifier {
        "claude_desktop" => {
            preload_claude_desktop_rules(pool, client_id).await?;
        }
        "cursor" => {
            preload_cursor_rules(pool, client_id).await?;
        }
        "windsurf" => {
            preload_windsurf_rules(pool, client_id).await?;
        }
        _ => {}
    }
    Ok(())
}

/// Preload Claude Desktop detection rules
async fn preload_claude_desktop_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let rules = vec![
        // macOS rules
        (
            "macos",
            "bundle_id",
            "com.anthropic.claude",
            "{{user_home}}/Library/Application Support/Claude/claude_desktop_config.json",
            1,
        ),
        (
            "macos",
            "file_path",
            "/Applications/Claude.app",
            "{{user_home}}/Library/Application Support/Claude/claude_desktop_config.json",
            2,
        ),
        // TODO: Windows rules (for future)
        (
            "windows",
            "file_path",
            r"{{user_home}}/AppData/Local/Programs/Claude/Claude.exe",
            "{{user_home}}/AppData/Roaming/Claude/claude_desktop_config.json",
            1,
        ),
    ];

    for (platform, method, value, config_template, priority) in rules {
        sqlx::query(r#"
            INSERT OR IGNORE INTO client_detection_rules
            (id, client_app_id, platform, detection_method, detection_value, config_path_template, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_template)
        .bind(priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Preload Cursor detection rules
async fn preload_cursor_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let rules = vec![
        // macOS rules
        (
            "macos",
            "bundle_id",
            "com.todesktop.230313mzl4w4u92",
            "{{user_home}}/.cursor/mcp.json",
            1,
        ),
        (
            "macos",
            "file_path",
            "/Applications/Cursor.app",
            "{{user_home}}/.cursor/mcp.json",
            2,
        ),
        // TODO: Windows rules (for future)
        (
            "windows",
            "file_path",
            r"C:\Program Files\cursor",
            "{{user_home}}/.cursor/mcp.json",
            1,
        ),
    ];

    for (platform, method, value, config_template, priority) in rules {
        sqlx::query(r#"
            INSERT OR IGNORE INTO client_detection_rules
            (id, client_app_id, platform, detection_method, detection_value, config_path_template, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_template)
        .bind(priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Preload Windsurf detection rules
async fn preload_windsurf_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let rules = vec![
        // macOS rules
        (
            "macos",
            "bundle_id",
            "com.exafunction.windsurf",
            "{{user_home}}/.codeium/windsurf/mcp_config.json",
            1,
        ),
        (
            "macos",
            "file_path",
            "/Applications/Windsurf.app",
            "{{user_home}}/.codeium/windsurf/mcp_config.json",
            2,
        ),
        // TODO: Windows rules (for future)
        (
            "windows",
            "file_path",
            r"{{user_home}}/AppData/Local/Programs/Windsurf/Windsurf.exe",
            "{{user_home}}/.codeium/windsurf/mcp_config.json",
            1,
        ),
    ];

    for (platform, method, value, config_template, priority) in rules {
        sqlx::query(r#"
            INSERT OR IGNORE INTO client_detection_rules
            (id, client_app_id, platform, detection_method, detection_value, config_path_template, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_template)
        .bind(priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}
