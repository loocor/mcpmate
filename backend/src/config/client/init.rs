use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

const DEFAULT_BACKUP_POLICY: &str = "keep_n";
const DEFAULT_BACKUP_LIMIT: i64 = 5;
const DEFAULT_CAPABILITY_SOURCE: &str = "activated";
const DEFAULT_CONNECTION_MODE: &str = "local_config_detected";
const DEFAULT_GOVERNANCE_KIND: &str = "passive";
const DEFAULT_REGISTRATION_ORIGIN: &str = "manual";
pub(crate) const CLIENT_RUNTIME_SETTINGS_TABLE: &str = "client_runtime_settings";
pub(crate) const CLIENT_TEMPLATE_RUNTIME_TABLE: &str = "client_template_runtime";
pub(crate) const DEFAULT_CONFIG_MODE_SETTING_KEY: &str = "default_config_mode";
pub(crate) const DEFAULT_CONFIG_MODE: &str = "unify";
const OPTIONAL_CONFIG_MODE_SCHEMA_FRAGMENT: &str =
    "config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent'))";

/// Initialize client configuration state table.
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
            -- Management mode: unify|hosted|transparent; NULL means use default mode
            config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
            -- Transport protocol: auto|sse|stdio|streamable_http (default: auto)
            transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                transport IN ('auto', 'sse', 'stdio', 'streamable_http')
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
                connection_mode IN ('local_config_detected', 'manual')
            ),
            registration_origin TEXT NOT NULL DEFAULT '{default_registration_origin}' CHECK (
                registration_origin IN ('manual', 'config_detection', 'runtime_initialize')
            ),
            runtime_observed INTEGER NOT NULL DEFAULT 0 CHECK (runtime_observed IN (0, 1)),
            template_identifier TEXT,
            selected_profile_ids TEXT,
            custom_profile_id TEXT,
            unify_direct_exposure_intent TEXT,
            approval_status TEXT NOT NULL DEFAULT 'approved' CHECK (
                approval_status IN ('pending', 'approved', 'suspended')
            ),
            attachment_state TEXT NOT NULL DEFAULT 'not_applicable' CHECK (
                attachment_state IN ('attached', 'detached', 'not_applicable')
            ),
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
        default_registration_origin = DEFAULT_REGISTRATION_ORIGIN,
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
    ensure_column(pool, tables::CLIENT, "unify_direct_exposure_intent", "TEXT").await?;
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
        "TEXT NOT NULL DEFAULT 'local_config_detected' CHECK (connection_mode IN ('local_config_detected', 'manual'))",
    )
    .await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "registration_origin",
        "TEXT NOT NULL DEFAULT 'manual' CHECK (registration_origin IN ('manual', 'config_detection', 'runtime_initialize'))",
    )
    .await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "runtime_observed",
        "INTEGER NOT NULL DEFAULT 0 CHECK (runtime_observed IN (0, 1))",
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
        "TEXT NOT NULL DEFAULT 'approved' CHECK (approval_status IN ('pending', 'approved', 'suspended'))",
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
    ensure_column(pool, tables::CLIENT, "transports", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "config_file_parse", "TEXT").await?;
    ensure_column(
        pool,
        tables::CLIENT,
        "attachment_state",
        "TEXT NOT NULL DEFAULT 'not_applicable' CHECK (attachment_state IN ('attached', 'detached', 'not_applicable'))",
    )
    .await?;

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
            WHEN approval_status = 'suspended' THEN 'active' \
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
        "UPDATE {table} SET \
            runtime_observed = CASE \
                WHEN connection_mode = 'remote_http' THEN 1 \
                ELSE COALESCE(runtime_observed, 0) \
            END, \
            registration_origin = CASE \
                WHEN connection_mode = 'remote_http' THEN 'runtime_initialize' \
                WHEN registration_origin IS NULL OR registration_origin = '' OR registration_origin = ? THEN \
                    CASE \
                        WHEN COALESCE(runtime_observed, 0) = 1 THEN 'runtime_initialize' \
                        WHEN config_path IS NOT NULL AND TRIM(config_path) <> '' THEN 'config_detection' \
                        ELSE ? \
                    END \
                ELSE registration_origin \
            END, \
            connection_mode = CASE \
                WHEN config_path IS NOT NULL AND TRIM(config_path) <> '' \
                THEN 'local_config_detected' \
                ELSE 'manual' \
            END",
        table = tables::CLIENT,
    ))
    .bind(DEFAULT_REGISTRATION_ORIGIN)
    .bind(DEFAULT_REGISTRATION_ORIGIN)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to normalize {} connection state: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to normalize {} connection state: {}", tables::CLIENT, e)
    })?;

    migrate_client_table_constraints(pool).await?;

    tracing::debug!("{} table initialized", tables::CLIENT);
    Ok(())
}

pub async fn resolve_default_client_config_mode(pool: &Pool<Sqlite>) -> Result<String> {
    crate::system::settings::get_default_config_mode(pool)
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

pub async fn set_default_client_config_mode(
    pool: &Pool<Sqlite>,
    mode: &str,
) -> Result<()> {
    crate::system::settings::set_default_config_mode(pool, mode)
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))
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
                config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
                transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                    transport IN ('auto', 'sse', 'stdio', 'streamable_http')
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
                id, name, identifier, config_mode, transport, client_version,
                backup_policy, backup_limit, capability_source, selected_profile_ids,
                custom_profile_id, created_at, updated_at
            )
            SELECT
                id, name, identifier,
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

async fn migrate_client_table_constraints(pool: &Pool<Sqlite>) -> Result<()> {
    let create_sql: Option<String> = sqlx::query_scalar(&format!(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
        tables::CLIENT
    ))
    .fetch_optional(pool)
    .await?;

    let Some(create_sql) = create_sql else {
        return Ok(());
    };

    let needs_sse_transport = create_sql.contains("transport IN ('auto', 'stdio', 'streamable_http')");
    let needs_connection_mode_cleanup = create_sql.contains("'remote_http'");

    if !needs_sse_transport && !needs_connection_mode_cleanup {
        return Ok(());
    }

    tracing::info!("Migrating {} client table constraints", tables::CLIENT);

    let transports_source_expression = if column_exists(pool, tables::CLIENT, "format_rules").await? {
        "COALESCE(NULLIF(transports, ''), format_rules)"
    } else {
        "transports"
    };

    let migration_result = async {
        let mut tx = pool.begin().await?;
        let temp_table = format!("{}_constraints_current", tables::CLIENT);

        sqlx::query(&format!(
            r#"
            CREATE TABLE {temp_table} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                display_name TEXT,
                identifier TEXT NOT NULL UNIQUE,
                config_path TEXT,
                config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
                transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                    transport IN ('auto', 'sse', 'stdio', 'streamable_http')
                ),
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
                    connection_mode IN ('local_config_detected', 'manual')
                ),
                registration_origin TEXT NOT NULL DEFAULT '{default_registration_origin}' CHECK (
                    registration_origin IN ('manual', 'config_detection', 'runtime_initialize')
                ),
                runtime_observed INTEGER NOT NULL DEFAULT 0 CHECK (runtime_observed IN (0, 1)),
                template_identifier TEXT,
                selected_profile_ids TEXT,
                custom_profile_id TEXT,
                unify_direct_exposure_intent TEXT,
                approval_status TEXT NOT NULL DEFAULT 'approved' CHECK (
                    approval_status IN ('pending', 'approved', 'suspended')
                ),
                template_id TEXT,
                template_version TEXT,
                approval_metadata TEXT,
                config_format TEXT,
                protocol_revision TEXT,
                container_type TEXT,
                container_keys TEXT,
                storage_kind TEXT,
                storage_adapter TEXT,
                storage_path_strategy TEXT,
                merge_strategy TEXT,
                keep_original_config INTEGER,
                managed_source TEXT,
                transports TEXT,
                config_file_parse TEXT,
                attachment_state TEXT NOT NULL DEFAULT 'not_applicable' CHECK (
                    attachment_state IN ('attached', 'detached', 'not_applicable')
                ),
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            temp_table = temp_table,
            default_policy = DEFAULT_BACKUP_POLICY,
            default_backup_limit = DEFAULT_BACKUP_LIMIT,
            default_capability_source = DEFAULT_CAPABILITY_SOURCE,
            default_governance_kind = DEFAULT_GOVERNANCE_KIND,
            default_connection_mode = DEFAULT_CONNECTION_MODE,
            default_registration_origin = DEFAULT_REGISTRATION_ORIGIN,
        ))
        .execute(&mut *tx)
        .await?;

        sqlx::query(&format!(
            r#"
            INSERT INTO {temp_table} (
                id, name, display_name, identifier, config_path, config_mode, transport,
                client_version, backup_policy, backup_limit, capability_source, governance_kind,
                connection_mode, registration_origin, runtime_observed,
                template_identifier, selected_profile_ids, custom_profile_id,
                unify_direct_exposure_intent,
                approval_status, template_id, template_version, approval_metadata, config_format, protocol_revision,
                container_type, container_keys, storage_kind, storage_adapter, storage_path_strategy,
                merge_strategy, keep_original_config, managed_source, transports, config_file_parse,
                attachment_state,
                created_at, updated_at
            )
            SELECT
                id, name, display_name, identifier, config_path, config_mode, transport,
                client_version, backup_policy, backup_limit, capability_source, governance_kind,
                CASE
                    WHEN config_path IS NOT NULL AND TRIM(config_path) <> ''
                    THEN 'local_config_detected'
                    ELSE 'manual'
                END,
                CASE
                    WHEN connection_mode = 'remote_http' THEN 'runtime_initialize'
                    ELSE registration_origin
                END,
                CASE
                    WHEN connection_mode = 'remote_http' THEN 1
                    ELSE runtime_observed
                END,
                template_identifier, selected_profile_ids, custom_profile_id,
                unify_direct_exposure_intent,
                approval_status, template_id, template_version, approval_metadata, config_format, protocol_revision,
                container_type, container_keys, storage_kind, storage_adapter, storage_path_strategy,
                merge_strategy, keep_original_config, managed_source, {transports_source_expression}, config_file_parse,
                attachment_state,
                created_at, updated_at
            FROM {table}
            "#,
            temp_table = temp_table,
            table = tables::CLIENT,
            transports_source_expression = transports_source_expression,
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

async fn column_exists(
    pool: &Pool<Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool> {
    let rows: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT name FROM pragma_table_info('{}')",
        table.replace('\'', "''")
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to inspect {} columns: {}", table, e))?;

    Ok(rows.into_iter().any(|name| name == column))
}

/// Ensures the on-disk system settings store exists (JSON). Does not create or touch any SQLite
/// `system_settings` table; schema changes for existing installs are handled out-of-band.
pub async fn initialize_system_settings(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing system settings store");

    crate::system::settings::initialize_settings_file(pool)
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        initialize_client_table, initialize_system_settings, resolve_default_client_config_mode,
        set_default_client_config_mode,
    };
    use crate::clients::models::FirstContactBehavior;
    use crate::common::constants::database::tables;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_raw_pool() -> sqlx::Pool<sqlx::Sqlite> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create sqlite pool")
    }

    async fn setup_pool() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = setup_raw_pool().await;

        initialize_client_table(&pool)
            .await
            .expect("failed to initialize client tables");
        initialize_system_settings(&pool)
            .await
            .expect("failed to initialize system settings store");

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
    async fn initialize_system_settings_file_creates_default_behavior() {
        let pool = setup_pool().await;

        let settings = crate::system::settings::get_settings(&pool)
            .await
            .expect("failed to read system settings");

        assert_eq!(settings.first_contact_behavior, FirstContactBehavior::Review);
        assert_eq!(settings.api_port, crate::common::constants::ports::API_PORT);
        assert_eq!(settings.mcp_port, crate::common::constants::ports::MCP_PORT);
        assert_eq!(settings.inspector_timeout_ms, 8_000);
        assert_eq!(settings.default_config_mode, "unify");
    }

    #[tokio::test]
    async fn initialize_client_table_normalizes_legacy_remote_http_rows() {
        let pool = setup_raw_pool().await;

        sqlx::query(&format!(
            r#"
            CREATE TABLE {table} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                identifier TEXT NOT NULL UNIQUE,
                config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
                transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                    transport IN ('auto', 'sse', 'stdio', 'streamable_http')
                ),
                client_version TEXT,
                backup_policy TEXT NOT NULL DEFAULT 'keep_n' CHECK (
                    backup_policy IN ('keep_last', 'keep_n', 'off')
                ),
                backup_limit INTEGER DEFAULT 5,
                connection_mode TEXT NOT NULL DEFAULT 'local_config_detected' CHECK (
                    connection_mode IN ('local_config_detected', 'remote_http', 'manual')
                ),
                registration_origin TEXT NOT NULL DEFAULT 'manual' CHECK (
                    registration_origin IN ('manual', 'config_detection', 'runtime_initialize')
                ),
                runtime_observed INTEGER NOT NULL DEFAULT 0 CHECK (runtime_observed IN (0, 1)),
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            table = tables::CLIENT,
        ))
        .execute(&pool)
        .await
        .expect("create legacy client table");

        sqlx::query(&format!(
            "INSERT INTO {table} (id, name, identifier, connection_mode) VALUES (?, ?, ?, ?)",
            table = tables::CLIENT,
        ))
        .bind("client_legacy")
        .bind("Legacy Runtime")
        .bind("legacy.runtime")
        .bind("remote_http")
        .execute(&pool)
        .await
        .expect("insert legacy remote_http row");

        initialize_client_table(&pool).await.expect("initialize client table");

        let (connection_mode, registration_origin, runtime_observed): (String, String, i64) = sqlx::query_as(&format!(
            "SELECT connection_mode, registration_origin, runtime_observed FROM {table} WHERE identifier = ?",
            table = tables::CLIENT,
        ))
        .bind("legacy.runtime")
        .fetch_one(&pool)
        .await
        .expect("fetch normalized row");

        assert_eq!(connection_mode, "manual");
        assert_eq!(registration_origin, "runtime_initialize");
        assert_eq!(runtime_observed, 1);

        let create_sql: String = sqlx::query_scalar(&format!(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
            tables::CLIENT,
        ))
        .fetch_one(&pool)
        .await
        .expect("fetch normalized schema");

        assert!(!create_sql.contains("'remote_http'"));
    }

    #[tokio::test]
    async fn initialize_client_table_derives_local_mode_from_legacy_config_path() {
        let pool = setup_raw_pool().await;

        sqlx::query(&format!(
            r#"
            CREATE TABLE {table} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                identifier TEXT NOT NULL UNIQUE,
                config_path TEXT,
                config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
                transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                    transport IN ('auto', 'sse', 'stdio', 'streamable_http')
                ),
                client_version TEXT,
                backup_policy TEXT NOT NULL DEFAULT 'keep_n' CHECK (
                    backup_policy IN ('keep_last', 'keep_n', 'off')
                ),
                backup_limit INTEGER DEFAULT 5,
                connection_mode TEXT NOT NULL DEFAULT 'manual' CHECK (
                    connection_mode IN ('local_config_detected', 'manual')
                ),
                registration_origin TEXT NOT NULL DEFAULT 'manual' CHECK (
                    registration_origin IN ('manual', 'config_detection', 'runtime_initialize')
                ),
                runtime_observed INTEGER NOT NULL DEFAULT 0 CHECK (runtime_observed IN (0, 1)),
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            table = tables::CLIENT,
        ))
        .execute(&pool)
        .await
        .expect("create legacy client table");

        let legacy_config_path = "/tmp/mcpmate-legacy-client.json";
        sqlx::query(&format!(
            "INSERT INTO {table} (id, name, identifier, config_path, connection_mode) VALUES (?, ?, ?, ?, ?)",
            table = tables::CLIENT,
        ))
        .bind("client_legacy_path")
        .bind("Legacy Config Path")
        .bind("legacy.path")
        .bind(legacy_config_path)
        .bind("manual")
        .execute(&pool)
        .await
        .expect("insert legacy config path row");

        initialize_client_table(&pool).await.expect("initialize client table");

        let (connection_mode, registration_origin, config_path): (String, String, String) = sqlx::query_as(&format!(
            "SELECT connection_mode, registration_origin, config_path FROM {table} WHERE identifier = ?",
            table = tables::CLIENT,
        ))
        .bind("legacy.path")
        .fetch_one(&pool)
        .await
        .expect("fetch normalized row");

        assert_eq!(connection_mode, "local_config_detected");
        assert_eq!(registration_origin, "config_detection");
        assert_eq!(config_path, legacy_config_path);
    }

    #[tokio::test]
    async fn initialize_client_table_preserves_legacy_format_rules_during_constraint_rebuild() {
        let pool = setup_raw_pool().await;

        sqlx::query(&format!(
            r#"
            CREATE TABLE {table} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                identifier TEXT NOT NULL UNIQUE,
                config_path TEXT,
                config_mode TEXT CHECK (config_mode IN ('unify','hosted','transparent')),
                transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                    transport IN ('auto', 'sse', 'stdio', 'streamable_http')
                ),
                client_version TEXT,
                backup_policy TEXT NOT NULL DEFAULT 'keep_n' CHECK (
                    backup_policy IN ('keep_last', 'keep_n', 'off')
                ),
                backup_limit INTEGER DEFAULT 5,
                connection_mode TEXT NOT NULL DEFAULT 'local_config_detected' CHECK (
                    connection_mode IN ('local_config_detected', 'remote_http', 'manual')
                ),
                registration_origin TEXT NOT NULL DEFAULT 'manual' CHECK (
                    registration_origin IN ('manual', 'config_detection', 'runtime_initialize')
                ),
                runtime_observed INTEGER NOT NULL DEFAULT 0 CHECK (runtime_observed IN (0, 1)),
                format_rules TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            table = tables::CLIENT,
        ))
        .execute(&pool)
        .await
        .expect("create legacy client table");

        let legacy_format_rules = r#"{"stdio":{"command_field":"command","args_field":"args","env_field":"env"}}"#;
        let legacy_config_path = "/tmp/mcpmate-legacy-rules.json";
        sqlx::query(&format!(
            "INSERT INTO {table} (id, name, identifier, config_path, connection_mode, format_rules) VALUES (?, ?, ?, ?, ?, ?)",
            table = tables::CLIENT,
        ))
        .bind("client_legacy_rules")
        .bind("Legacy Format Rules")
        .bind("legacy.rules")
        .bind(legacy_config_path)
        .bind("manual")
        .bind(legacy_format_rules)
        .execute(&pool)
        .await
        .expect("insert legacy format rules row");

        initialize_client_table(&pool).await.expect("initialize client table");

        let (connection_mode, transports): (String, String) = sqlx::query_as(&format!(
            "SELECT connection_mode, transports FROM {table} WHERE identifier = ?",
            table = tables::CLIENT,
        ))
        .bind("legacy.rules")
        .fetch_one(&pool)
        .await
        .expect("fetch migrated row");

        assert_eq!(connection_mode, "local_config_detected");
        assert_eq!(transports, legacy_format_rules);
    }
}
