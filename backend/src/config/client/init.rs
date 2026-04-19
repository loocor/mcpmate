use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

const DEFAULT_BACKUP_POLICY: &str = "keep_n";
const DEFAULT_BACKUP_LIMIT: i64 = 5;
const DEFAULT_CAPABILITY_SOURCE: &str = "activated";
const DEFAULT_CONNECTION_MODE: &str = "local_config_detected";
const DEFAULT_GOVERNANCE_KIND: &str = "passive";
const DEFAULT_RECORD_KIND: &str = "template_known";
pub(crate) const CLIENT_RUNTIME_SETTINGS_TABLE: &str = "client_runtime_settings";
pub(crate) const CLIENT_TEMPLATE_RUNTIME_TABLE: &str = "client_template_runtime";
pub(crate) const FIRST_CONTACT_BEHAVIOR_SETTING_KEY: &str = "first_contact_behavior";
pub(crate) const DEFAULT_CONFIG_MODE_SETTING_KEY: &str = "default_config_mode";
pub(crate) const DEFAULT_CONFIG_MODE: &str = "unify";
const OPTIONAL_CONFIG_MODE_SCHEMA_FRAGMENT: &str =
    "config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent'))";

/// Initialize client management table (identifier-managed/policy metadata)
pub async fn initialize_client_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing client management table");

    migrate_client_table_for_optional_config_mode(pool).await?;

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table} (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            display_name TEXT,
            identifier TEXT NOT NULL UNIQUE,
            config_path TEXT,
            managed INTEGER NOT NULL DEFAULT 1 CHECK (managed IN (0, 1)),
            -- Management mode: unify|hosted|transparent; NULL means use default mode
            config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
            -- Transport protocol: auto|stdio|streamable_http (default: auto)
            transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                transport IN ('auto', 'stdio', 'streamable_http')
            ),
            -- Client version string (optional)
            client_version TEXT,
            backup_policy TEXT NOT NULL DEFAULT '{default_policy}' CHECK (
                backup_policy IN ('keep_last', 'keep_n', 'off')
            ),
            backup_limit INTEGER DEFAULT {default_backup_limit},
            capability_source TEXT NOT NULL DEFAULT '{default_capability_source}' CHECK (
                capability_source IN ('activated', 'profiles', 'custom')
            ),
            governance_kind TEXT NOT NULL DEFAULT '{default_governance_kind}' CHECK (
                governance_kind IN ('passive', 'active')
            ),
            connection_mode TEXT NOT NULL DEFAULT '{default_connection_mode}' CHECK (
                connection_mode IN ('local_config_detected', 'remote_http', 'manual')
            ),
            record_kind TEXT NOT NULL DEFAULT '{default_record_kind}' CHECK (
                record_kind IN ('template_known', 'observed_unknown')
            ),
            template_identifier TEXT,
            selected_profile_ids TEXT,
            custom_profile_id TEXT,
            unify_route_mode TEXT NOT NULL DEFAULT 'broker_only' CHECK (
                unify_route_mode IN ('broker_only', 'server_live', 'capability_level')
            ),
            unify_selected_server_ids TEXT,
            unify_selected_tool_surfaces TEXT,
            unify_selected_prompt_surfaces TEXT,
            unify_selected_resource_surfaces TEXT,
            unify_selected_template_surfaces TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        table = tables::CLIENT,
        default_policy = DEFAULT_BACKUP_POLICY,
        default_backup_limit = DEFAULT_BACKUP_LIMIT,
        default_capability_source = DEFAULT_CAPABILITY_SOURCE,
        default_governance_kind = DEFAULT_GOVERNANCE_KIND,
        default_connection_mode = DEFAULT_CONNECTION_MODE,
        default_record_kind = DEFAULT_RECORD_KIND,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to create {} table: {}", tables::CLIENT, e)
    })?;

    ensure_column(
        pool,
        tables::CLIENT,
        "capability_source",
        "TEXT NOT NULL DEFAULT 'activated' CHECK (capability_source IN ('activated', 'profiles', 'custom'))",
    )
    .await?;
    ensure_column(pool, tables::CLIENT, "selected_profile_ids", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "custom_profile_id", "TEXT").await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "unify_route_mode",
        "TEXT NOT NULL DEFAULT 'broker_only' CHECK (unify_route_mode IN ('broker_only', 'server_live', 'capability_level'))",
    )
    .await?;
    ensure_column(pool, tables::CLIENT, "unify_selected_server_ids", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "unify_selected_tool_surfaces", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "unify_selected_prompt_surfaces", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "unify_selected_resource_surfaces", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "unify_selected_template_surfaces", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "display_name", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "config_path", "TEXT").await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "governance_kind",
        "TEXT NOT NULL DEFAULT 'passive' CHECK (governance_kind IN ('passive', 'active'))",
    )
    .await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "connection_mode",
        "TEXT NOT NULL DEFAULT 'local_config_detected' CHECK (connection_mode IN ('local_config_detected', 'remote_http', 'manual'))",
    )
    .await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "record_kind",
        "TEXT NOT NULL DEFAULT 'template_known' CHECK (record_kind IN ('template_known', 'observed_unknown'))",
    )
    .await?;
    ensure_column(pool, tables::CLIENT, "template_identifier", "TEXT").await?;
    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table} (
            identifier TEXT PRIMARY KEY,
            payload_json TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        table = CLIENT_TEMPLATE_RUNTIME_TABLE,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", CLIENT_TEMPLATE_RUNTIME_TABLE, e);
        anyhow::anyhow!("Failed to create {} table: {}", CLIENT_TEMPLATE_RUNTIME_TABLE, e)
    })?;

    ensure_column(
        pool,
        tables::CLIENT,
        "approval_status",
        "TEXT NOT NULL DEFAULT 'approved' CHECK (approval_status IN ('pending', 'approved', 'suspended', 'rejected'))",
    )
    .await?;
    ensure_column(pool, tables::CLIENT, "template_id", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "template_version", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "approval_metadata", "TEXT").await?;

    // Template configuration fields (persisted from template at initialization)
    ensure_column(pool, tables::CLIENT, "config_format", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "protocol_revision", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "container_type", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "container_keys", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "storage_kind", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "storage_adapter", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "storage_path_strategy", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "merge_strategy", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "keep_original_config", "INTEGER").await?;
    ensure_column(pool, tables::CLIENT, "managed_source", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "format_rules", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "config_file_parse", "TEXT").await?;

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table} (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
                CHECK (key != '{default_mode_key}' OR value IN ('unify', 'hosted', 'transparent'))
        )
        "#,
        table = CLIENT_RUNTIME_SETTINGS_TABLE,
        default_mode_key = DEFAULT_CONFIG_MODE_SETTING_KEY,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", CLIENT_RUNTIME_SETTINGS_TABLE, e);
        anyhow::anyhow!("Failed to create {} table: {}", CLIENT_RUNTIME_SETTINGS_TABLE, e)
    })?;

    sqlx::query(&format!(
        "INSERT OR IGNORE INTO {table} (key, value) VALUES (?, ?)",
        table = CLIENT_RUNTIME_SETTINGS_TABLE,
    ))
    .bind(DEFAULT_CONFIG_MODE_SETTING_KEY)
    .bind(DEFAULT_CONFIG_MODE)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to initialize {}.{}: {}",
            CLIENT_RUNTIME_SETTINGS_TABLE,
            DEFAULT_CONFIG_MODE_SETTING_KEY,
            e
        );
        anyhow::anyhow!(
            "Failed to initialize {}.{}: {}",
            CLIENT_RUNTIME_SETTINGS_TABLE,
            DEFAULT_CONFIG_MODE_SETTING_KEY,
            e
        )
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET capability_source = ? WHERE capability_source IS NULL OR capability_source = ''",
        table = tables::CLIENT,
    ))
    .bind(DEFAULT_CAPABILITY_SOURCE)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} capability_source: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} capability_source: {}", tables::CLIENT, e)
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET config_mode = NULL WHERE config_mode = ''",
        table = tables::CLIENT
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to normalize {} config_mode: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to normalize {} config_mode: {}", tables::CLIENT, e)
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET display_name = name WHERE display_name IS NULL OR display_name = ''",
        table = tables::CLIENT
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} display_name: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} display_name: {}", tables::CLIENT, e)
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET governance_kind = CASE \
            WHEN config_mode IS NOT NULL AND TRIM(config_mode) <> '' THEN 'active' \
            WHEN transport IS NOT NULL AND TRIM(transport) <> '' AND transport <> 'auto' THEN 'active' \
            WHEN client_version IS NOT NULL AND TRIM(client_version) <> '' THEN 'active' \
            WHEN backup_policy IS NOT NULL AND backup_policy <> 'keep_n' THEN 'active' \
WHEN backup_limit IS NOT NULL AND backup_limit <> {default_backup_limit} THEN 'active' \
            WHEN capability_source IS NOT NULL AND capability_source <> 'activated' THEN 'active' \
            WHEN selected_profile_ids IS NOT NULL AND TRIM(selected_profile_ids) <> '' THEN 'active' \
            WHEN custom_profile_id IS NOT NULL AND TRIM(custom_profile_id) <> '' THEN 'active' \
            WHEN approval_status IN ('rejected', 'suspended') THEN 'active' \
            WHEN approval_status = 'approved' AND managed = 0 THEN 'active' \
            ELSE ? END \
         WHERE governance_kind IS NULL OR governance_kind = ''",
        table = tables::CLIENT,
        default_backup_limit = DEFAULT_BACKUP_LIMIT,
    ))
    .bind(DEFAULT_GOVERNANCE_KIND)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} governance_kind: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} governance_kind: {}", tables::CLIENT, e)
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET record_kind = ? WHERE record_kind IS NULL OR record_kind = ''",
        table = tables::CLIENT
    ))
    .bind(DEFAULT_RECORD_KIND)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} record_kind: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} record_kind: {}", tables::CLIENT, e)
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET template_identifier = identifier WHERE template_identifier IS NULL OR template_identifier = ''",
        table = tables::CLIENT
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} template_identifier: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} template_identifier: {}", tables::CLIENT, e)
    })?;

    sqlx::query(&format!(
        "UPDATE {table} SET connection_mode = ? WHERE connection_mode IS NULL OR connection_mode = ''",
        table = tables::CLIENT
    ))
    .bind(DEFAULT_CONNECTION_MODE)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} connection_mode: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} connection_mode: {}", tables::CLIENT, e)
    })?;

    tracing::debug!("{} table initialized", tables::CLIENT);
    Ok(())
}

pub async fn resolve_default_client_config_mode(pool: &Pool<Sqlite>) -> Result<String> {
    let mode: Option<String> = sqlx::query_scalar(&format!(
        "SELECT value FROM {table} WHERE key = ?",
        table = CLIENT_RUNTIME_SETTINGS_TABLE,
    ))
    .bind(DEFAULT_CONFIG_MODE_SETTING_KEY)
    .fetch_optional(pool)
    .await?;

    Ok(mode.unwrap_or_else(|| DEFAULT_CONFIG_MODE.to_string()))
}

pub async fn set_default_client_config_mode(
    pool: &Pool<Sqlite>,
    mode: &str,
) -> Result<()> {
    anyhow::ensure!(
        matches!(mode, "unify" | "hosted" | "transparent"),
        "invalid default client config mode: {mode}"
    );

    sqlx::query(&format!(
        "INSERT INTO {table} (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        table = CLIENT_RUNTIME_SETTINGS_TABLE,
    ))
    .bind(DEFAULT_CONFIG_MODE_SETTING_KEY)
    .bind(mode)
    .execute(pool)
    .await?;

    Ok(())
}

async fn migrate_client_table_for_optional_config_mode(pool: &Pool<Sqlite>) -> Result<()> {
    let table_exists: Option<String> = sqlx::query_scalar(&format!(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
        tables::CLIENT
    ))
    .fetch_optional(pool)
    .await?;

    if table_exists.is_none() {
        return Ok(());
    }

    let create_sql: Option<String> = sqlx::query_scalar(&format!(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
        tables::CLIENT
    ))
    .fetch_optional(pool)
    .await?;

    let Some(create_sql) = create_sql else {
        return Ok(());
    };

    if create_sql.contains(OPTIONAL_CONFIG_MODE_SCHEMA_FRAGMENT)
        && !create_sql.contains("config_mode TEXT NOT NULL DEFAULT 'hosted'")
    {
        return Ok(());
    }

    tracing::info!(
        "Migrating {} table to allow unset config_mode for default-mode fallback",
        tables::CLIENT
    );

    let migration_result = async {
        let mut tx = pool.begin().await?;
        let temp_table = format!("{}_config_mode_nullable", tables::CLIENT);

        sqlx::query(&format!(
            r#"
            CREATE TABLE {temp_table} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                identifier TEXT NOT NULL UNIQUE,
                managed INTEGER NOT NULL DEFAULT 1 CHECK (managed IN (0, 1)),
                config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
                transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                    transport IN ('auto', 'stdio', 'streamable_http')
                ),
                client_version TEXT,
                backup_policy TEXT NOT NULL DEFAULT '{default_policy}' CHECK (
                    backup_policy IN ('keep_last', 'keep_n', 'off')
                ),
                backup_limit INTEGER DEFAULT {default_backup_limit},
                capability_source TEXT NOT NULL DEFAULT '{default_capability_source}' CHECK (
                    capability_source IN ('activated', 'profiles', 'custom')
                ),
                selected_profile_ids TEXT,
                custom_profile_id TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            temp_table = temp_table,
            default_policy = DEFAULT_BACKUP_POLICY,
            default_backup_limit = DEFAULT_BACKUP_LIMIT,
            default_capability_source = DEFAULT_CAPABILITY_SOURCE,
        ))
        .execute(&mut *tx)
        .await?;

        sqlx::query(&format!(
            r#"
            INSERT INTO {temp_table} (
                id, name, identifier, managed, config_mode, transport, client_version,
                backup_policy, backup_limit, capability_source, selected_profile_ids,
                custom_profile_id, created_at, updated_at
            )
            SELECT
                id, name, identifier, managed,
                config_mode, transport, client_version,
                backup_policy, backup_limit, capability_source, selected_profile_ids,
                custom_profile_id, created_at, updated_at
            FROM {table}
            "#,
            temp_table = temp_table,
            table = tables::CLIENT,
        ))
        .execute(&mut *tx)
        .await?;

        sqlx::query(&format!("DROP TABLE {table}", table = tables::CLIENT))
            .execute(&mut *tx)
            .await?;

        sqlx::query(&format!(
            "ALTER TABLE {temp_table} RENAME TO {table}",
            temp_table = temp_table,
            table = tables::CLIENT,
        ))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok::<(), sqlx::Error>(())
    }
    .await;

    match migration_result {
        Ok(()) => Ok(()),
        Err(error) => Err(anyhow::anyhow!(error)),
    }
}

async fn ensure_column(
    pool: &Pool<Sqlite>,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let stmt = format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}",
        table = table,
        column = column,
        definition = definition
    );

    match sqlx::query(&stmt).execute(pool).await {
        Ok(_) => {
            tracing::debug!("Added column {}.{}", table, column);
            Ok(())
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column name") => {
            tracing::trace!("Column {}.{} already exists", table, column);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to add column {}.{}: {}", table, column, e);
            Err(anyhow::anyhow!("Failed to add column {}.{}: {}", table, column, e))
        }
    }
}

pub async fn initialize_system_settings_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing system_settings table");

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table} (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        table = tables::SYSTEM_SETTINGS,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", tables::SYSTEM_SETTINGS, e);
        anyhow::anyhow!("Failed to create {} table: {}", tables::SYSTEM_SETTINGS, e)
    })?;

    sqlx::query(&format!(
        r#"
        INSERT OR IGNORE INTO {table} (key, value)
        VALUES ('onboarding_policy', 'auto_manage')
        "#,
        table = tables::SYSTEM_SETTINGS,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert default onboarding_policy: {}", e);
        anyhow::anyhow!("Failed to insert default onboarding_policy: {}", e)
    })?;

    let existing_first_contact_behavior = sqlx::query_scalar::<_, String>(&format!(
        r#"SELECT value FROM {table} WHERE key = ?"#,
        table = tables::SYSTEM_SETTINGS,
    ))
    .bind(FIRST_CONTACT_BEHAVIOR_SETTING_KEY)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to read first_contact_behavior: {}", e);
        anyhow::anyhow!("Failed to read first_contact_behavior: {}", e)
    })?;

    if existing_first_contact_behavior.is_none() {
        let onboarding_policy = sqlx::query_scalar::<_, String>(&format!(
            r#"SELECT value FROM {table} WHERE key = 'onboarding_policy'"#,
            table = tables::SYSTEM_SETTINGS,
        ))
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to read onboarding_policy for first_contact_behavior migration: {}",
                e
            );
            anyhow::anyhow!(
                "Failed to read onboarding_policy for first_contact_behavior migration: {}",
                e
            )
        })?;

        let initial_behavior = match onboarding_policy.as_deref() {
            Some("require_approval") => "review",
            Some("manual") => "deny",
            Some("auto_manage") => "review",
            Some(_) | None => "review",
        };

        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (key, value)
            VALUES (?, ?)
            "#,
            table = tables::SYSTEM_SETTINGS,
        ))
        .bind(FIRST_CONTACT_BEHAVIOR_SETTING_KEY)
        .bind(initial_behavior)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert default first_contact_behavior: {}", e);
            anyhow::anyhow!("Failed to insert default first_contact_behavior: {}", e)
        })?;
    }

    tracing::debug!("{} table initialized", tables::SYSTEM_SETTINGS);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FIRST_CONTACT_BEHAVIOR_SETTING_KEY, initialize_client_table, initialize_system_settings_table,
        resolve_default_client_config_mode, set_default_client_config_mode,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create sqlite pool");

        initialize_client_table(&pool)
            .await
            .expect("failed to initialize client tables");
        initialize_system_settings_table(&pool)
            .await
            .expect("failed to initialize system settings table");

        pool
    }

    #[tokio::test]
    async fn default_client_config_mode_defaults_to_unify() {
        let pool = setup_pool().await;

        let mode = resolve_default_client_config_mode(&pool)
            .await
            .expect("failed to resolve default client config mode");

        assert_eq!(mode, "unify");
    }

    #[tokio::test]
    async fn default_client_config_mode_can_be_updated_and_read_back() {
        let pool = setup_pool().await;

        set_default_client_config_mode(&pool, "unify")
            .await
            .expect("failed to persist unify mode");
        assert_eq!(
            resolve_default_client_config_mode(&pool)
                .await
                .expect("failed to resolve unify mode"),
            "unify"
        );

        set_default_client_config_mode(&pool, "transparent")
            .await
            .expect("failed to persist transparent mode");
        assert_eq!(
            resolve_default_client_config_mode(&pool)
                .await
                .expect("failed to resolve transparent mode"),
            "transparent"
        );
    }

    #[tokio::test]
    async fn first_contact_behavior_backfills_from_existing_onboarding_policy() {
        let pool = setup_pool().await;

        sqlx::query("DELETE FROM system_settings WHERE key = ?")
            .bind(FIRST_CONTACT_BEHAVIOR_SETTING_KEY)
            .execute(&pool)
            .await
            .expect("delete first_contact_behavior");

        sqlx::query("UPDATE system_settings SET value = 'manual' WHERE key = 'onboarding_policy'")
            .execute(&pool)
            .await
            .expect("update onboarding policy");

        initialize_system_settings_table(&pool)
            .await
            .expect("reinitialize system settings");

        let behavior = sqlx::query_scalar::<_, String>("SELECT value FROM system_settings WHERE key = ?")
            .bind(FIRST_CONTACT_BEHAVIOR_SETTING_KEY)
            .fetch_one(&pool)
            .await
            .expect("fetch first_contact_behavior");

        assert_eq!(behavior, "deny");
    }
}
