//! The sole owner of durable SQLite schema evolution in MCPMate.

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite, Transaction};
use std::path::{Path, PathBuf};

const LEDGER_TABLE: &str = "mcpmate_schema_migrations";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseTarget {
    Config,
    Audit,
}

impl DatabaseTarget {
    const fn name(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Audit => "audit",
        }
    }
}

#[async_trait]
pub trait MigrationStep: Send + Sync {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()>;
}

pub struct SqlMigration {
    sql: &'static str,
}

impl SqlMigration {
    pub const fn new(sql: &'static str) -> Self {
        Self { sql }
    }
}

#[async_trait]
impl MigrationStep for SqlMigration {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        for statement in self.sql.split(";\n").filter(|statement| !statement.trim().is_empty()) {
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .context("execute SQL migration statement")?;
        }
        Ok(())
    }
}

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub checksum_source: &'static str,
    pub step: Box<dyn MigrationStep>,
}

impl Migration {
    pub fn checksum(&self) -> String {
        format!("{:x}", Sha256::digest(self.checksum_source.as_bytes()))
    }
}

pub async fn run(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    migrations: Vec<Migration>,
) -> Result<()> {
    let mut transaction = pool.begin().await.context("begin migration transaction")?;
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {LEDGER_TABLE} (target TEXT NOT NULL, version INTEGER NOT NULL, name TEXT NOT NULL, checksum TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY (target, version))"
    ))
    .execute(&mut *transaction)
    .await
    .context("create migration ledger")?;

    let mut previous = 0;
    for migration in migrations {
        if migration.version <= previous {
            bail!("migration versions for {} are not strictly increasing", target.name());
        }
        previous = migration.version;
        let checksum = migration.checksum();
        let applied: Option<(String, String)> = sqlx::query_as(&format!(
            "SELECT name, checksum FROM {LEDGER_TABLE} WHERE target = ? AND version = ?"
        ))
        .bind(target.name())
        .bind(migration.version)
        .fetch_optional(&mut *transaction)
        .await
        .context("read migration ledger")?;
        if let Some((name, existing_checksum)) = applied {
            if name != migration.name || existing_checksum != checksum {
                bail!(
                    "migration {} for {} was modified after being applied",
                    migration.version,
                    target.name()
                );
            }
            continue;
        }
        migration.step.apply(&mut transaction).await.with_context(|| {
            format!(
                "apply migration {} ({}) for {}",
                migration.version,
                migration.name,
                target.name()
            )
        })?;
        sqlx::query(&format!(
            "INSERT INTO {LEDGER_TABLE} (target, version, name, checksum) VALUES (?, ?, ?, ?)"
        ))
        .bind(target.name())
        .bind(migration.version)
        .bind(migration.name)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .context("record applied migration")?;
    }
    transaction.commit().await.context("commit migrations")
}

pub async fn migrate_audit(pool: &Pool<Sqlite>) -> Result<()> {
    run(pool, DatabaseTarget::Audit, audit_migrations()).await
}

pub async fn audit_has_pending(pool: &Pool<Sqlite>) -> Result<bool> {
    has_pending(pool, DatabaseTarget::Audit, audit_migrations()).await
}

fn audit_migrations() -> Vec<Migration> {
    vec![Migration {
            version: 1,
            name: "create audit storage",
            checksum_source: AUDIT_INITIAL_SCHEMA,
            step: Box::new(SqlMigration::new(AUDIT_INITIAL_SCHEMA)),
        }]
}

pub async fn migrate_config(pool: &Pool<Sqlite>) -> Result<()> {
    run(pool, DatabaseTarget::Config, config_migrations()).await
}

pub async fn config_has_pending(pool: &Pool<Sqlite>) -> Result<bool> {
    has_pending(pool, DatabaseTarget::Config, config_migrations()).await
}

pub async fn backup_pending_config(pool: &Pool<Sqlite>, path: &Path) -> Result<Option<PathBuf>> {
    backup_if_pending(pool, DatabaseTarget::Config, config_migrations(), path).await
}

pub async fn backup_pending_audit(pool: &Pool<Sqlite>, path: &Path) -> Result<Option<PathBuf>> {
    backup_if_pending(pool, DatabaseTarget::Audit, audit_migrations(), path).await
}

async fn backup_if_pending(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    migrations: Vec<Migration>,
    path: &Path,
) -> Result<Option<PathBuf>> {
    if !has_pending(pool, target, migrations).await? {
        return Ok(None);
    }
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup_path = PathBuf::from(format!("{}.migration-{timestamp}.bak", path.display()));
    sqlx::query("VACUUM INTO ?")
        .bind(backup_path.to_string_lossy().as_ref())
        .execute(pool)
        .await
        .with_context(|| format!("create migration backup at {}", backup_path.display()))?;
    Ok(Some(backup_path))
}

fn config_migrations() -> Vec<Migration> {
    vec![
            Migration {
                version: 1,
                name: "create llm provider",
                checksum_source: LLM_PROVIDER_INITIAL_SCHEMA,
                step: Box::new(SqlMigration::new(LLM_PROVIDER_INITIAL_SCHEMA)),
            },
            Migration {
                version: 2,
                name: "add llm provider default flag",
                checksum_source: "add llm_provider.is_default when absent",
                step: Box::new(AddLlmDefaultColumn),
            },
            Migration {
                version: 3,
                name: "create server configuration",
                checksum_source: SERVER_INITIAL_SCHEMA,
                step: Box::new(SqlMigration::new(SERVER_INITIAL_SCHEMA)),
            },
            Migration {
                version: 4,
                name: "upgrade server configuration columns",
                checksum_source: "upgrade legacy server_config and server_meta columns",
                step: Box::new(UpgradeServerColumns),
            },
            Migration {
                version: 5,
                name: "create profile authoring storage",
                checksum_source: PROFILE_INITIAL_SCHEMA,
                step: Box::new(SqlMigration::new(PROFILE_INITIAL_SCHEMA)),
            },
    ]
}

async fn has_pending(pool: &Pool<Sqlite>, target: DatabaseTarget, migrations: Vec<Migration>) -> Result<bool> {
    let ledger_exists: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{LEDGER_TABLE}')"
    ))
    .fetch_one(pool)
    .await
    .context("inspect migration ledger")?;
    if !ledger_exists {
        return Ok(!migrations.is_empty());
    }
    for migration in migrations {
        let applied: Option<(String, String)> = sqlx::query_as(&format!(
            "SELECT name, checksum FROM {LEDGER_TABLE} WHERE target = ? AND version = ?"
        ))
        .bind(target.name())
        .bind(migration.version)
        .fetch_optional(pool)
        .await
        .context("read migration ledger")?;
        match applied {
            None => return Ok(true),
            Some((name, checksum)) if name == migration.name && checksum == migration.checksum() => {}
            Some(_) => bail!("migration {} for {} was modified after being applied", migration.version, target.name()),
        }
    }
    Ok(false)
}

struct UpgradeServerColumns;

#[async_trait]
impl MigrationStep for UpgradeServerColumns {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        ensure_columns(
            transaction,
            "server_config",
            &[
                ("pending_import", "BOOLEAN NOT NULL DEFAULT 0"),
                ("unify_direct_exposure_eligible", "BOOLEAN NOT NULL DEFAULT 0"),
                ("source", "TEXT"),
            ],
        )
        .await?;
        ensure_columns(
            transaction,
            "server_meta",
            &[
                ("registry_version", "TEXT"),
                ("registry_meta_json", "TEXT"),
                ("extras_json", "TEXT"),
                ("upstream_name", "TEXT"),
                ("upstream_title", "TEXT"),
            ],
        )
        .await
    }
}

async fn ensure_columns(
    transaction: &mut Transaction<'_, Sqlite>,
    table: &str,
    columns: &[(&str, &str)],
) -> Result<()> {
    let existing: Vec<String> = sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .fetch_all(&mut **transaction)
        .await
        .with_context(|| format!("inspect {table} schema"))?;
    for (column, definition) in columns {
        if !existing.iter().any(|existing_column| existing_column == column) {
            sqlx::query(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))
                .execute(&mut **transaction)
                .await
                .with_context(|| format!("add {table}.{column}"))?;
        }
    }
    Ok(())
}

struct AddLlmDefaultColumn;

#[async_trait]
impl MigrationStep for AddLlmDefaultColumn {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('llm_provider')")
            .fetch_all(&mut **transaction)
            .await
            .context("inspect llm_provider schema")?;
        if !columns.iter().any(|column| column == "is_default") {
            sqlx::query("ALTER TABLE llm_provider ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT 0")
                .execute(&mut **transaction)
                .await
                .context("add llm_provider.is_default")?;
        }
        Ok(())
    }
}

const LLM_PROVIDER_INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS llm_provider (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, provider_type TEXT NOT NULL,
    base_url TEXT NOT NULL, model_id TEXT NOT NULL, secret_alias TEXT,
    default_params_json TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"#;

const SERVER_INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS server_config (
    id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE,
    server_type TEXT NOT NULL CHECK (server_type IN ('stdio', 'sse', 'streamable_http')),
    command TEXT, url TEXT, source TEXT, enabled BOOLEAN NOT NULL DEFAULT 1,
    unify_direct_exposure_eligible BOOLEAN NOT NULL DEFAULT 0,
    pending_import BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS server_args (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL,
    arg_index INTEGER NOT NULL, arg_value TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, arg_index)
);
CREATE TABLE IF NOT EXISTS server_env (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL,
    env_key TEXT NOT NULL, env_value TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, env_key)
);
CREATE TABLE IF NOT EXISTS server_headers (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, header_key TEXT NOT NULL,
    header_value TEXT NOT NULL, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, header_key)
);
CREATE TABLE IF NOT EXISTS server_meta (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL,
    author TEXT, category TEXT, description TEXT, extras_json TEXT, icons_json TEXT,
    protocol_version TEXT, rating INTEGER, recommended_scenario TEXT, registry_meta_json TEXT,
    registry_version TEXT, repository TEXT, upstream_name TEXT, upstream_title TEXT,
    server_version TEXT, website TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id)
);
CREATE TABLE IF NOT EXISTS server_namespace_issue (
    server_id TEXT PRIMARY KEY, issue_kind TEXT NOT NULL, capability_kind TEXT,
    external_identifier TEXT, upstream_value TEXT, conflicting_server_id TEXT,
    conflicting_upstream_value TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    FOREIGN KEY (conflicting_server_id) REFERENCES server_config (id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS server_oauth_config (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL UNIQUE, authorization_endpoint TEXT NOT NULL,
    token_endpoint TEXT NOT NULL, client_id TEXT NOT NULL, client_secret TEXT, scopes TEXT,
    redirect_uri TEXT NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS server_oauth_tokens (
    id TEXT PRIMARY KEY, server_id TEXT NOT NULL UNIQUE, access_token TEXT NOT NULL,
    refresh_token TEXT, token_type TEXT NOT NULL DEFAULT 'bearer', expires_at TEXT, scope TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE
);
"#;

const PROFILE_INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS profile (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT, type TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'user', multi_select BOOLEAN NOT NULL DEFAULT 0, priority INTEGER NOT NULL DEFAULT 0, is_active BOOLEAN NOT NULL DEFAULT 0, is_default BOOLEAN NOT NULL DEFAULT 0, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS profile_server_relationships (profile_id TEXT NOT NULL, server_id TEXT NOT NULL, enabled BOOLEAN NOT NULL DEFAULT 1, new_ref_policy TEXT NOT NULL CHECK (new_ref_policy IN ('follow', 'review')), FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE, PRIMARY KEY(profile_id, server_id));
CREATE TABLE IF NOT EXISTS server_tools (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, tool_name TEXT NOT NULL, unique_name TEXT NOT NULL, description TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id, tool_name), UNIQUE(unique_name));
CREATE INDEX IF NOT EXISTS idx_server_tools_lookup ON server_tools(server_id, tool_name);
CREATE INDEX IF NOT EXISTS idx_server_tools_unique_name ON server_tools(unique_name);
CREATE INDEX IF NOT EXISTS idx_server_tools_server_name ON server_tools(server_name);
CREATE TABLE IF NOT EXISTS server_prompts (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, prompt_name TEXT NOT NULL, unique_name TEXT NOT NULL, description TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id, prompt_name), UNIQUE(unique_name));
CREATE INDEX IF NOT EXISTS idx_server_prompts_lookup ON server_prompts(server_id, prompt_name);
CREATE INDEX IF NOT EXISTS idx_server_prompts_unique_name ON server_prompts(unique_name);
CREATE INDEX IF NOT EXISTS idx_server_prompts_server_name ON server_prompts(server_name);
CREATE TABLE IF NOT EXISTS server_resources (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, resource_uri TEXT NOT NULL, unique_uri TEXT NOT NULL, name TEXT, description TEXT, mime_type TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id, resource_uri), UNIQUE(unique_uri));
CREATE INDEX IF NOT EXISTS idx_server_resources_lookup ON server_resources(server_id, resource_uri);
CREATE INDEX IF NOT EXISTS idx_server_resources_unique_uri ON server_resources(unique_uri);
CREATE INDEX IF NOT EXISTS idx_server_resources_server_name ON server_resources(server_name);
CREATE TABLE IF NOT EXISTS server_resource_templates (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, uri_template TEXT NOT NULL, unique_name TEXT NOT NULL, route_uri TEXT, name TEXT NOT NULL, description TEXT, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id, uri_template), UNIQUE(unique_name), UNIQUE(route_uri));
CREATE INDEX IF NOT EXISTS idx_server_resource_templates_lookup ON server_resource_templates(server_id, uri_template);
CREATE INDEX IF NOT EXISTS idx_server_resource_templates_unique_name ON server_resource_templates(unique_name);
CREATE INDEX IF NOT EXISTS idx_server_resource_templates_route_uri ON server_resource_templates(route_uri);
CREATE INDEX IF NOT EXISTS idx_server_resource_templates_server_name ON server_resource_templates(server_name);
CREATE TABLE IF NOT EXISTS server_issued_resources (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, resource_uri TEXT NOT NULL, unique_uri TEXT NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE, UNIQUE(server_id, resource_uri), UNIQUE(unique_uri));
CREATE INDEX IF NOT EXISTS idx_server_issued_resources_lookup ON server_issued_resources(server_id, resource_uri);
CREATE INDEX IF NOT EXISTS idx_server_issued_resources_unique_uri ON server_issued_resources(unique_uri);
CREATE TABLE IF NOT EXISTS profile_capability_refs (profile_id TEXT NOT NULL, ref_id TEXT NOT NULL, enabled BOOLEAN NOT NULL, FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE, FOREIGN KEY (ref_id) REFERENCES capability_refs (ref_id) ON DELETE CASCADE, PRIMARY KEY(profile_id, ref_id));
CREATE INDEX IF NOT EXISTS idx_profile_capability_refs_ref ON profile_capability_refs(ref_id);
CREATE TABLE IF NOT EXISTS direct_exposure_refs (consumer_id TEXT NOT NULL, ref_id TEXT NOT NULL, enabled BOOLEAN NOT NULL, FOREIGN KEY (consumer_id) REFERENCES client (identifier) ON DELETE CASCADE, FOREIGN KEY (ref_id) REFERENCES capability_refs (ref_id) ON DELETE CASCADE, PRIMARY KEY(consumer_id, ref_id));
CREATE INDEX IF NOT EXISTS idx_direct_exposure_refs_ref ON direct_exposure_refs(ref_id);
CREATE TABLE IF NOT EXISTS direct_exposure_servers (consumer_id TEXT NOT NULL, server_id TEXT NOT NULL, new_ref_policy TEXT NOT NULL CHECK (new_ref_policy IN ('follow', 'review')), FOREIGN KEY (consumer_id) REFERENCES client (identifier) ON DELETE CASCADE, PRIMARY KEY(consumer_id, server_id));
"#;

const AUDIT_INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL, action TEXT NOT NULL, status TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL, actor TEXT, request_id TEXT, client_id TEXT,
    profile_id TEXT, server_id TEXT, session_id TEXT, protocol_version TEXT,
    http_method TEXT, route TEXT, mcp_method TEXT, target TEXT, direction TEXT,
    error_code TEXT, error_message TEXT, detail TEXT, duration_ms INTEGER,
    data_json TEXT, task_id TEXT, related_task_id TEXT, progress_token TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_events_occurred_at ON audit_events (occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_category_action ON audit_events (category, action, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_status ON audit_events (status, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_server_id ON audit_events (server_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_profile_id ON audit_events (profile_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_client_id ON audit_events (client_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_session_id ON audit_events (session_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_task_id ON audit_events (task_id, occurred_at_ms DESC, id DESC);
CREATE TABLE IF NOT EXISTS audit_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1), policy TEXT NOT NULL,
    sweep_interval_secs INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn applies_once_and_detects_mutated_history() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        run(
            &pool,
            DatabaseTarget::Config,
            vec![Migration {
                version: 1,
                name: "create example",
                checksum_source: "create example v1",
                step: Box::new(SqlMigration::new("CREATE TABLE example (id INTEGER PRIMARY KEY);\n")),
            }],
        )
        .await
        .unwrap();
        run(
            &pool,
            DatabaseTarget::Config,
            vec![Migration {
                version: 1,
                name: "create example",
                checksum_source: "create example v1",
                step: Box::new(SqlMigration::new("CREATE TABLE example (id INTEGER PRIMARY KEY);\n")),
            }],
        )
        .await
        .unwrap();
        let error = run(
            &pool,
            DatabaseTarget::Config,
            vec![Migration {
                version: 1,
                name: "create example",
                checksum_source: "changed",
                step: Box::new(SqlMigration::new("SELECT 1;\n")),
            }],
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("modified"));
    }

    #[tokio::test]
    async fn creates_audit_schema_through_the_ledger() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        migrate_audit(&pool).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mcpmate_schema_migrations WHERE target = 'audit'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
        sqlx::query(
            "INSERT INTO audit_policy (id, policy, sweep_interval_secs, updated_at_ms) VALUES (1, 'keep', 1, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn upgrades_legacy_llm_provider_schema() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE llm_provider (id TEXT PRIMARY KEY, name TEXT NOT NULL, provider_type TEXT NOT NULL, base_url TEXT NOT NULL, model_id TEXT NOT NULL, secret_alias TEXT, default_params_json TEXT, created_at TIMESTAMP NOT NULL, updated_at TIMESTAMP NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        migrate_config(&pool).await.unwrap();
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('llm_provider')")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(columns.iter().any(|column| column == "is_default"));
    }
}
