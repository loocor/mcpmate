#[path = "support/file.rs"]
mod file_support;
#[path = "support/memory.rs"]
mod memory_support;

use std::fs;

use mcpmate_migrations::{DatabaseSource, prepare_config_database};
use tempfile::tempdir;

#[tokio::test]
async fn preserves_client_relationships_during_legacy_normalization() {
    let pool = memory_support::pool().await;
    sqlx::query(
        "CREATE TABLE client (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            identifier TEXT NOT NULL UNIQUE,
            config_mode TEXT NOT NULL DEFAULT 'hosted',
            transport TEXT NOT NULL DEFAULT 'auto',
            client_version TEXT,
            backup_policy TEXT NOT NULL DEFAULT 'keep_n',
            backup_limit INTEGER DEFAULT 5,
            capability_source TEXT NOT NULL DEFAULT 'activated',
            selected_profile_ids TEXT,
            custom_profile_id TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await
    .expect("create legacy client table");
    sqlx::query(
        "CREATE TABLE client_writeback_policy (
            client_identifier TEXT PRIMARY KEY,
            merge_strategy TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (client_identifier) REFERENCES client(identifier) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await
    .expect("create legacy writeback policy table");
    sqlx::query(
        "CREATE TABLE direct_exposure_refs (
            consumer_id TEXT NOT NULL,
            ref_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL,
            FOREIGN KEY (consumer_id) REFERENCES client(identifier) ON DELETE CASCADE,
            PRIMARY KEY (consumer_id, ref_id)
        )",
    )
    .execute(&pool)
    .await
    .expect("create legacy direct exposure reference table");
    sqlx::query(
        "CREATE TABLE direct_exposure_servers (
            consumer_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            new_ref_policy TEXT NOT NULL,
            FOREIGN KEY (consumer_id) REFERENCES client(identifier) ON DELETE CASCADE,
            PRIMARY KEY (consumer_id, server_id)
        )",
    )
    .execute(&pool)
    .await
    .expect("create legacy direct exposure server table");

    sqlx::query("INSERT INTO client (id, name, identifier) VALUES ('client-1', 'Cursor', 'cursor')")
        .execute(&pool)
        .await
        .expect("insert legacy client");
    sqlx::query(
        "INSERT INTO client_writeback_policy (client_identifier, merge_strategy)
         VALUES ('cursor', 'deep_merge')",
    )
    .execute(&pool)
    .await
    .expect("insert legacy writeback policy");
    sqlx::query(
        "INSERT INTO direct_exposure_refs (consumer_id, ref_id, enabled)
         VALUES ('cursor', 'tool:server:lookup', 1)",
    )
    .execute(&pool)
    .await
    .expect("insert legacy direct exposure reference");
    sqlx::query(
        "INSERT INTO direct_exposure_servers (consumer_id, server_id, new_ref_policy)
         VALUES ('cursor', 'server-1', 'follow')",
    )
    .execute(&pool)
    .await
    .expect("insert legacy direct exposure server");

    prepare_config_database(&pool, DatabaseSource::InMemory)
        .await
        .expect("migrate legacy config database");

    let writeback_policy: String =
        sqlx::query_scalar("SELECT merge_strategy FROM client_writeback_policy WHERE client_identifier = 'cursor'")
            .fetch_one(&pool)
            .await
            .expect("load migrated writeback policy");
    let exposed_ref: (String, bool) =
        sqlx::query_as("SELECT ref_id, enabled FROM direct_exposure_refs WHERE consumer_id = 'cursor'")
            .fetch_one(&pool)
            .await
            .expect("load migrated direct exposure reference");
    let exposed_server: (String, String) =
        sqlx::query_as("SELECT server_id, new_ref_policy FROM direct_exposure_servers WHERE consumer_id = 'cursor'")
            .fetch_one(&pool)
            .await
            .expect("load migrated direct exposure server");

    assert_eq!(writeback_policy, "deep_merge");
    assert_eq!(exposed_ref, ("tool:server:lookup".into(), true));
    assert_eq!(exposed_server, ("server-1".into(), "follow".into()));
    let identity: (String, String, String) =
        sqlx::query_as("SELECT name, display_name, connection_mode FROM client WHERE identifier = 'cursor'")
            .fetch_one(&pool)
            .await
            .expect("load normalized client identity");
    assert_eq!(identity, ("Cursor".into(), "Cursor".into(), "manual".into()));
    let foreign_key_errors: Vec<String> = sqlx::query_scalar("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .expect("check migrated foreign keys");
    assert!(foreign_key_errors.is_empty());
}

#[tokio::test]
async fn creates_a_distinct_backup_for_each_failed_upgrade_attempt() {
    let directory = tempdir().expect("create temporary directory");
    let database_path = directory.path().join("config.db");
    let pool = file_support::pool(&database_path).await;
    sqlx::query(
        "CREATE TABLE secure_store_secrets (
            alias TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            encrypted_value TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("create incompatible secure store table");
    sqlx::query(
        "INSERT INTO secure_store_secrets (alias, kind, encrypted_value)
         VALUES ('legacy', 'api_key', 'ciphertext')",
    )
    .execute(&pool)
    .await
    .expect("insert incompatible secure store record");

    for _ in 0..2 {
        let error = prepare_config_database(
            &pool,
            DatabaseSource::File {
                path: &database_path,
                existed_before_open: true,
            },
        )
        .await
        .expect_err("unsafe secure store migration must fail");
        assert!(
            error.to_string().contains("cannot be safely upgraded"),
            "unexpected migration error: {error:#}"
        );
    }

    let backup_count = fs::read_dir(directory.path())
        .expect("read temporary directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("config.db.migration-"))
        .count();
    assert_eq!(backup_count, 2, "each failed attempt needs its own recovery backup");
}

#[tokio::test]
async fn creates_one_readable_backup_for_a_successful_existing_file_upgrade() {
    let directory = tempdir().expect("create temporary directory");
    let database_path = directory.path().join("config.db");
    let pool = file_support::pool(&database_path).await;
    sqlx::query("CREATE TABLE backup_probe (value TEXT NOT NULL)")
        .execute(&pool)
        .await
        .expect("create backup probe table");
    sqlx::query("INSERT INTO backup_probe (value) VALUES ('before-migration')")
        .execute(&pool)
        .await
        .expect("insert backup probe");

    let backup = prepare_config_database(
        &pool,
        DatabaseSource::File {
            path: &database_path,
            existed_before_open: true,
        },
    )
    .await
    .expect("upgrade existing config database")
    .expect("pending upgrade creates a recovery backup");
    let backup_pool = file_support::pool(&backup).await;
    let value: String = sqlx::query_scalar("SELECT value FROM backup_probe")
        .fetch_one(&backup_pool)
        .await
        .expect("read recovery backup");
    assert_eq!(value, "before-migration");

    let second = prepare_config_database(
        &pool,
        DatabaseSource::File {
            path: &database_path,
            existed_before_open: true,
        },
    )
    .await
    .expect("recheck prepared config database");
    assert!(second.is_none());
}

#[tokio::test]
async fn fresh_file_upgrade_does_not_create_a_backup() {
    let directory = tempdir().expect("create temporary directory");
    let database_path = directory.path().join("config.db");
    let pool = file_support::pool(&database_path).await;

    let backup = prepare_config_database(
        &pool,
        DatabaseSource::File {
            path: &database_path,
            existed_before_open: false,
        },
    )
    .await
    .expect("prepare fresh config database");

    assert!(backup.is_none());
    let backup_count = fs::read_dir(directory.path())
        .expect("read temporary directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".migration-"))
        .count();
    assert_eq!(backup_count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serializes_concurrent_file_upgrades() {
    let directory = tempdir().expect("create temporary directory");
    let database_path = directory.path().join("config.db");
    let first_pool = file_support::pool(&database_path).await;
    let second_pool = file_support::pool(&database_path).await;
    let first_source = DatabaseSource::File {
        path: &database_path,
        existed_before_open: true,
    };
    let second_source = DatabaseSource::File {
        path: &database_path,
        existed_before_open: true,
    };

    let (first, second) = tokio::join!(
        prepare_config_database(&first_pool, first_source),
        prepare_config_database(&second_pool, second_source),
    );
    let backups = [first.expect("first upgrade"), second.expect("second upgrade")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(backups.len(), 1);
}
