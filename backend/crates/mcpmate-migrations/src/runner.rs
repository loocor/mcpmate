use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use sqlx::{Pool, Sqlite};

use crate::migrations::{self, Migration};

const LEDGER_TABLE: &str = "mcpmate_schema_migrations";
const LEDGER_STATE_TABLE: &str = "mcpmate_schema_migration_state";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatabaseTarget {
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

#[derive(Debug, Clone, Copy)]
pub enum DatabaseSource<'a> {
    InMemory,
    File { path: &'a Path, existed_before_open: bool },
}

pub async fn prepare_config_database(
    pool: &Pool<Sqlite>,
    source: DatabaseSource<'_>,
) -> Result<Option<PathBuf>> {
    prepare_database(pool, DatabaseTarget::Config, source).await
}

pub async fn prepare_audit_database(
    pool: &Pool<Sqlite>,
    source: DatabaseSource<'_>,
) -> Result<Option<PathBuf>> {
    prepare_database(pool, DatabaseTarget::Audit, source).await
}

pub async fn verify_config_database(pool: &Pool<Sqlite>) -> Result<()> {
    verify_database(pool, DatabaseTarget::Config).await
}

pub async fn verify_audit_database(pool: &Pool<Sqlite>) -> Result<()> {
    verify_database(pool, DatabaseTarget::Audit).await
}

async fn prepare_database(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    source: DatabaseSource<'_>,
) -> Result<Option<PathBuf>> {
    let migrations = migrations_for(target);
    match source {
        DatabaseSource::InMemory => {
            run(pool, target, &migrations).await?;
            Ok(None)
        }
        DatabaseSource::File {
            path,
            existed_before_open,
        } => prepare_file_backed(pool, target, &migrations, path, existed_before_open).await,
    }
}

async fn verify_database(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
) -> Result<()> {
    let migrations = migrations_for(target);
    if has_pending(pool, target, &migrations).await? {
        bail!("migration ledger for {} has pending migrations", target.name());
    }
    Ok(())
}

fn migrations_for(target: DatabaseTarget) -> Vec<Migration> {
    match target {
        DatabaseTarget::Config => migrations::config::all(),
        DatabaseTarget::Audit => migrations::audit::all(),
    }
}

async fn run(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    migrations: &[Migration],
) -> Result<()> {
    validate_migration_versions(target, migrations)?;
    let mut transaction = pool.begin().await.context("begin migration transaction")?;
    let ledger_table_exists = table_exists(&mut transaction, LEDGER_TABLE).await?;
    let state_table_exists = table_exists(&mut transaction, LEDGER_STATE_TABLE).await?;
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {LEDGER_TABLE} (target TEXT NOT NULL, version INTEGER NOT NULL, name TEXT NOT NULL, checksum TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY (target, version))"
    ))
    .execute(&mut *transaction)
    .await
    .context("create migration ledger")?;
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {LEDGER_STATE_TABLE} (target TEXT PRIMARY KEY, version INTEGER NOT NULL, checksum TEXT NOT NULL)"
    ))
    .execute(&mut *transaction)
    .await
    .context("create migration ledger state")?;

    let applied = read_applied(&mut transaction, target).await?;
    let state = if state_table_exists {
        read_state(&mut transaction, target).await?
    } else {
        None
    };
    validate_ledger_history(
        target,
        migrations,
        &applied,
        state.as_ref(),
        ledger_table_exists,
        state_table_exists,
    )?;

    for migration in migrations.iter().skip(applied.len()) {
        let checksum = migration.checksum();
        migration.step.apply(&mut transaction).await.map_err(|error| {
            anyhow::anyhow!(
                "apply migration {} ({}) for {}: {error}",
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
    if let Some(last) = migrations.last() {
        sqlx::query(&format!(
            "INSERT INTO {LEDGER_STATE_TABLE} (target, version, checksum) VALUES (?, ?, ?) ON CONFLICT(target) DO UPDATE SET version = excluded.version, checksum = excluded.checksum"
        ))
        .bind(target.name())
        .bind(last.version)
        .bind(last.checksum())
        .execute(&mut *transaction)
        .await
        .context("record migration ledger state")?;
    }
    transaction.commit().await.context("commit migrations")
}

async fn has_pending(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    migrations: &[Migration],
) -> Result<bool> {
    validate_migration_versions(target, migrations)?;
    let ledger_table_exists = pool_table_exists(pool, LEDGER_TABLE).await?;
    if !ledger_table_exists {
        return Ok(!migrations.is_empty());
    }
    let state_table_exists = pool_table_exists(pool, LEDGER_STATE_TABLE).await?;
    let applied: Vec<(i64, String, String)> = sqlx::query_as(&format!(
        "SELECT version, name, checksum FROM {LEDGER_TABLE} WHERE target = ? ORDER BY version"
    ))
    .bind(target.name())
    .fetch_all(pool)
    .await
    .context("read migration ledger")?;
    let state: Option<(i64, String)> = if state_table_exists {
        sqlx::query_as(&format!(
            "SELECT version, checksum FROM {LEDGER_STATE_TABLE} WHERE target = ?"
        ))
        .bind(target.name())
        .fetch_optional(pool)
        .await
        .context("read migration ledger state")?
    } else {
        None
    };
    validate_ledger_history(
        target,
        migrations,
        &applied,
        state.as_ref(),
        ledger_table_exists,
        state_table_exists,
    )?;
    Ok(applied.len() < migrations.len())
}

async fn prepare_file_backed(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    migrations: &[Migration],
    path: &Path,
    existed_before_open: bool,
) -> Result<Option<PathBuf>> {
    let _lock = UpgradeLock::acquire(path).await?;
    let backup = if existed_before_open {
        backup_if_pending(pool, target, migrations, path).await?
    } else {
        None
    };
    run(pool, target, migrations).await?;
    Ok(backup)
}

async fn backup_if_pending(
    pool: &Pool<Sqlite>,
    target: DatabaseTarget,
    migrations: &[Migration],
    path: &Path,
) -> Result<Option<PathBuf>> {
    if !has_pending(pool, target, migrations).await? {
        return Ok(None);
    }
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let backup_path = available_backup_path(path, &timestamp);
    sqlx::query("VACUUM INTO ?")
        .bind(backup_path.to_string_lossy().as_ref())
        .execute(pool)
        .await
        .with_context(|| format!("create migration backup at {}", backup_path.display()))?;
    Ok(Some(backup_path))
}

fn available_backup_path(
    database_path: &Path,
    timestamp: &str,
) -> PathBuf {
    for attempt in 0_u64.. {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        let candidate = PathBuf::from(format!("{}.migration-{timestamp}{suffix}.bak", database_path.display()));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("the backup suffix space is unbounded")
}

fn validate_migration_versions(
    target: DatabaseTarget,
    migrations: &[Migration],
) -> Result<()> {
    for (index, migration) in migrations.iter().enumerate() {
        let expected = index as i64 + 1;
        if migration.version != expected {
            bail!(
                "migration versions for {} must be contiguous from 1; expected {expected}, found {}",
                target.name(),
                migration.version
            );
        }
    }
    Ok(())
}

fn validate_ledger_history(
    target: DatabaseTarget,
    migrations: &[Migration],
    applied: &[(i64, String, String)],
    state: Option<&(i64, String)>,
    ledger_table_exists: bool,
    state_table_exists: bool,
) -> Result<()> {
    for (index, (version, name, checksum)) in applied.iter().enumerate() {
        let expected = migrations
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("migration ledger for {} contains an unknown migration", target.name()))?;
        if *version != expected.version || name != expected.name || checksum != &expected.checksum() {
            bail!("migration ledger for {} is not a valid migration prefix", target.name());
        }
    }
    if !ledger_table_exists && !state_table_exists {
        return Ok(());
    }
    if !state_table_exists {
        bail!("migration ledger state for {} is missing", target.name());
    }
    match (applied.last(), state) {
        (None, None) => Ok(()),
        (Some((version, _, checksum)), Some((state_version, state_checksum)))
            if version == state_version && checksum == state_checksum =>
        {
            Ok(())
        }
        _ => bail!(
            "migration ledger state for {} does not match its history",
            target.name()
        ),
    }
}

async fn table_exists(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    table: &str,
) -> Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)")
        .bind(table)
        .fetch_one(&mut **transaction)
        .await
        .with_context(|| format!("inspect {table}"))
}

async fn pool_table_exists(
    pool: &Pool<Sqlite>,
    table: &str,
) -> Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)")
        .bind(table)
        .fetch_one(pool)
        .await
        .with_context(|| format!("inspect {table}"))
}

async fn read_applied(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    target: DatabaseTarget,
) -> Result<Vec<(i64, String, String)>> {
    sqlx::query_as(&format!(
        "SELECT version, name, checksum FROM {LEDGER_TABLE} WHERE target = ? ORDER BY version"
    ))
    .bind(target.name())
    .fetch_all(&mut **transaction)
    .await
    .context("read migration ledger")
}

async fn read_state(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    target: DatabaseTarget,
) -> Result<Option<(i64, String)>> {
    sqlx::query_as(&format!(
        "SELECT version, checksum FROM {LEDGER_STATE_TABLE} WHERE target = ?"
    ))
    .bind(target.name())
    .fetch_optional(&mut **transaction)
    .await
    .context("read migration ledger state")
}

struct UpgradeLock(File);

impl UpgradeLock {
    async fn acquire(database_path: &Path) -> Result<Self> {
        let lock_path = PathBuf::from(format!("{}.migration.lock", database_path.display()));
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .with_context(|| format!("open migration lock {}", lock_path.display()))?;
            file.lock_exclusive()
                .with_context(|| format!("lock migration path {}", lock_path.display()))?;
            Ok(Self(file))
        })
        .await
        .context("join migration lock task")?
    }
}

impl Drop for UpgradeLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}
