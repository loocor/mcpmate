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
            detected BOOLEAN DEFAULT FALSE,
            last_detected_at DATETIME,
            install_path TEXT,
            config_path TEXT,
            version TEXT,
            detection_method TEXT,
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
            client_identifier TEXT NOT NULL,
            platform TEXT NOT NULL,
            detection_method TEXT NOT NULL,
            detection_value TEXT NOT NULL,
            config_path TEXT NOT NULL,
            priority INTEGER DEFAULT 0,
            enabled BOOLEAN DEFAULT TRUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (client_app_id) REFERENCES client_apps(id) ON DELETE CASCADE,
            UNIQUE(client_app_id, platform, detection_method)
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
            client_app_id TEXT NOT NULL,
            client_identifier TEXT NOT NULL,
            top_level_key TEXT NOT NULL,
            is_mixed_config BOOLEAN DEFAULT FALSE,
            supported_transports TEXT NOT NULL,
            supported_runtimes TEXT NOT NULL,
            format_rules TEXT NOT NULL,
            security_features TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (client_app_id) REFERENCES client_apps(id) ON DELETE CASCADE,
            UNIQUE(client_app_id)
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
        ("zed", "Zed", "High-performance text editor"),
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

        // Preload config rules for this client
        preload_config_rules_for_client(pool, &actual_client_id, identifier).await?;
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
        "zed" => {
            preload_zed_rules(pool, client_id).await?;
        }
        _ => {}
    }
    Ok(())
}

/// Preload config rules for a specific client
async fn preload_config_rules_for_client(
    pool: &SqlitePool,
    client_id: &str,
    identifier: &str,
) -> Result<()> {
    match identifier {
        "claude_desktop" => {
            preload_claude_desktop_config_rules(pool, client_id).await?;
        }
        "cursor" => {
            preload_cursor_config_rules(pool, client_id).await?;
        }
        "windsurf" => {
            preload_windsurf_config_rules(pool, client_id).await?;
        }
        "zed" => {
            preload_zed_config_rules(pool, client_id).await?;
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
        // macOS rules - only one primary method per platform
        (
            "macos",
            "bundle_id",
            "com.anthropic.claude",
            "~/Library/Application Support/Claude/claude_desktop_config.json",
            1,
        ),
        // Windows rules
        (
            "windows",
            "file_path",
            "~/AppData/Roaming/Claude/claude_desktop_config.json",
            "~/AppData/Roaming/Claude/claude_desktop_config.json",
            1,
        ),
    ];

    for (platform, method, value, config_path, priority) in rules {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO client_detection_rules
            (id, client_app_id, client_identifier, platform, detection_method, detection_value, config_path, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind("claude_desktop")
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_path)
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
        // macOS rules - only one primary method per platform
        (
            "macos",
            "file_path",
            "~/.cursor/mcp.json",
            "~/.cursor/mcp.json",
            1,
        ),
    ];

    for (platform, method, value, config_path, priority) in rules {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO client_detection_rules
            (id, client_app_id, client_identifier, platform, detection_method, detection_value, config_path, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind("cursor")
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_path)
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
        // macOS rules - only one primary method per platform
        (
            "macos",
            "file_path",
            "~/.codeium/windsurf/mcp_config.json",
            "~/.codeium/windsurf/mcp_config.json",
            1,
        ),
        // Windows rules
        (
            "windows",
            "file_path",
            "~/AppData/Roaming/Codeium/windsurf/mcp_config.json",
            "~/AppData/Roaming/Codeium/windsurf/mcp_config.json",
            1,
        ),
    ];

    for (platform, method, value, config_path, priority) in rules {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO client_detection_rules
            (id, client_app_id, client_identifier, platform, detection_method, detection_value, config_path, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind("windsurf")
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_path)
        .bind(priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Preload Zed detection rules
async fn preload_zed_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let rules = vec![
        // macOS rules - only one primary method per platform
        (
            "macos",
            "file_path",
            "~/.config/zed/settings.json",
            "~/.config/zed/settings.json",
            1,
        ),
    ];

    for (platform, method, value, config_path, priority) in rules {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO client_detection_rules
            (id, client_app_id, client_identifier, platform, detection_method, detection_value, config_path, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(generate_id!("rule"))
        .bind(client_id)
        .bind("zed")
        .bind(platform)
        .bind(method)
        .bind(value)
        .bind(config_path)
        .bind(priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Preload Claude Desktop config rules
async fn preload_claude_desktop_config_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let supported_transports = r#"["stdio"]"#;
    let supported_runtimes = r#"{"macos":["npx","uvx","docker","binary"],"linux":["npx","uvx","docker","binary"],"windows":["npx","uvx","binary"]}"#;
    let format_rules = r#"{"stdio":{"template":{"command":"{{command}}","args":"{{args}}","env":"{{env}}"},"requires_type_field":false}}"#;

    sqlx::query(r#"
        INSERT OR REPLACE INTO client_config_rules
        (id, client_app_id, client_identifier, top_level_key, is_mixed_config, supported_transports, supported_runtimes, format_rules)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(generate_id!("conf"))
    .bind(client_id)
    .bind("claude_desktop")
    .bind("mcpServers")
    .bind(true)
    .bind(supported_transports)
    .bind(supported_runtimes)
    .bind(format_rules)
    .execute(pool)
    .await?;

    Ok(())
}

/// Preload Cursor config rules
async fn preload_cursor_config_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let supported_transports = r#"["stdio","sse","streamableHttp"]"#;
    let supported_runtimes = r#"{"macos":["npx","uvx","docker","binary"],"linux":["npx","uvx","docker","binary"],"windows":["npx","uvx","binary"]}"#;
    let format_rules = r#"{"stdio":{"template":{"type":"stdio","command":"{{command}}","args":"{{args}}","env":"{{env}}"},"requires_type_field":false},"sse":{"template":{"url":"{{url}}","type":"sse","headers":"{{headers}}"},"requires_type_field":false},"streamableHttp":{"template":{"type":"streamableHttp","url":"{{url}}"},"requires_type_field":false}}"#;

    sqlx::query(r#"
        INSERT OR REPLACE INTO client_config_rules
        (id, client_app_id, client_identifier, top_level_key, is_mixed_config, supported_transports, supported_runtimes, format_rules)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(generate_id!("conf"))
    .bind(client_id)
    .bind("cursor")
    .bind("mcpServers")
    .bind(false)
    .bind(supported_transports)
    .bind(supported_runtimes)
    .bind(format_rules)
    .execute(pool)
    .await?;

    Ok(())
}

/// Preload Windsurf config rules
async fn preload_windsurf_config_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let supported_transports = r#"["stdio","sse"]"#;
    let supported_runtimes = r#"{"macos":["npx","uvx","docker","binary"],"linux":["npx","uvx","docker","binary"],"windows":["npx","uvx","binary"]}"#;
    let format_rules = r#"{"stdio":{"template":{"type":"stdio","command":"{{command}}","args":"{{args}}","env":"{{env}}"},"requires_type_field":true},"sse":{"template":{"serverUrl":"{{url}}","headers":"{{headers}}"},"requires_type_field":false}}"#;
    let security_features = r#"{"supports_inputs":true,"supports_env_file":true}"#;

    sqlx::query(r#"
        INSERT OR REPLACE INTO client_config_rules
        (id, client_app_id, client_identifier, top_level_key, is_mixed_config, supported_transports, supported_runtimes, format_rules, security_features)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(generate_id!("conf"))
    .bind(client_id)
    .bind("windsurf")
    .bind("mcpServers")
    .bind(false)
    .bind(supported_transports)
    .bind(supported_runtimes)
    .bind(format_rules)
    .bind(security_features)
    .execute(pool)
    .await?;

    Ok(())
}

/// Preload Zed config rules
async fn preload_zed_config_rules(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<()> {
    let supported_transports = r#"["stdio"]"#;
    let supported_runtimes = r#"{"macos":["npx","uvx","docker","binary"],"linux":["npx","uvx","docker","binary"],"windows":["npx","uvx","binary"]}"#;
    let format_rules = r#"{"stdio":{"template":{"type":"stdio","command":"{{command}}","args":"{{args}}","env":"{{env}}"},"requires_type_field":false}}"#;

    sqlx::query(r#"
        INSERT OR REPLACE INTO client_config_rules
        (id, client_app_id, client_identifier, top_level_key, is_mixed_config, supported_transports, supported_runtimes, format_rules)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(generate_id!("conf"))
    .bind(client_id)
    .bind("zed")
    .bind("context_servers")
    .bind(true)
    .bind(supported_transports)
    .bind(supported_runtimes)
    .bind(format_rules)
    .execute(pool)
    .await?;

    Ok(())
}
