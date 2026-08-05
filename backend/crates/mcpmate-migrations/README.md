# MCPMate Database Migrations

`mcpmate-migrations` is the sole owner of durable SQLite schema evolution in MCPMate.

## Why this exists

Users upgrade MCPMate by running a newer version, not by locating and executing a separate migration tool. When a database needs an upgrade, the product creates a recovery backup, applies the pending steps exactly once, and refuses to start if that work cannot complete safely.

## Ownership boundary

- Put durable `CREATE`, `ALTER`, `DROP`, indexes, constraints, and one-time historical data rewrites here.
- Keep ordinary business reads and writes in their domain modules.
- Only the physical database initialization path may invoke `prepare_config_database` or `prepare_audit_database`.
- Domain modules may call the read-only verifier for their database target and validate the tables they consume, but they must not create or upgrade durable schema themselves.
- Each physical database has one ordered migration stream. MCPMate currently has `config` and `audit` targets.

This boundary includes tests. Database, transaction, schema, revision, and concurrency contracts belong in this crate's integration tests. Domain tests should prepare their fixture through the real migration entrypoint and then exercise the domain path; they must not reproduce schema with test-only DDL.

## Artifact layout

Each migration remains visible as a versioned source artifact under `src/migrations/<target>/`:

- `vNNNN_<name>.sql` contains declarative SQLite schema changes.
- `vNNNN_<name>.rs` contains schema-aware or data-preserving transactional steps.
- `<target>/mod.rs` is the ordered registry and the only place that appends a migration to a target stream.
- `runner.rs` owns ledger validation, locking, backup creation, transactions, and the public prepare/verify boundary.

Do not delete old artifacts after release. A bounded upgrade window is a future product policy, not permission to remove the historical ledger source. If such a window is introduced, keep enough immutable artifacts to support every advertised source version and reject older versions explicitly.

## Adding a migration

1. Read this document before designing persistent data changes.
2. Add an immutable, strictly increasing version to the target stream.
3. Use `SqlMigration` for simple DDL. Use a `MigrationStep` implementation for a transactional data transformation that needs schema inspection or Rust control flow.
4. Register the actual SQL or Rust artifact as the checksum source. Do not summarize executable logic into a manually maintained checksum string.
5. Give the migration a stable name. Never edit an already released migration; add a corrective migration instead.
6. Test a fresh database, the relevant legacy structure, rerun idempotence, and failure rollback at the storage-contract layer.
7. If a legacy shape is ambiguous, fail closed. Only rebuild or transform data when the preservation rule is explicit and tested.

## SQL placement policy

Durable schema SQL belongs in versioned migration artifacts. Business modules may retain ordinary queries for reads, writes, and explicit business transactions. They may also perform narrow read-only shape checks after migration, but must not contain schema repair SQL such as `CREATE TABLE IF NOT EXISTS`, `ALTER TABLE`, `ensure_column`, or opportunistic data conversion.

When reviewing a new SQL statement, classify it by purpose:

- Schema or one-time historical rewrite: migration artifact.
- Repeatable business read/write: owning domain module.
- Migration ledger, backup, or schema inspection: migration runner.
- Test fixture data: test support, inserted only after the real migration chain prepares the database.

## Runtime guarantees

The runner records target, version, name, checksum, and application time in `mcpmate_schema_migrations`, with a separate state row proving the applied prefix. It rejects gaps, deleted records, unknown records, modified names or checksums, and mismatched state. All pending steps and ledger updates run in one transaction.

For file-backed databases, a sidecar lock serializes the pending check, recovery backup, migration transaction, and ledger update across processes. An existing file with pending work receives a unique timestamped `.migration-*.bak` snapshot before changes begin. A fresh file does not create an empty backup. Failed attempts keep their backups; a prepared database does not create another one. In-memory tests use the same migration chain without filesystem backup.

The current config stream owns server, client, profile authoring, secure-store, and capability-catalog schemas. The audit stream owns audit storage. Legacy structures are accepted only when their preservation rule is explicit in a migration. A migration must stop with a clear error rather than inventing a transformation for data whose meaning or cryptographic material cannot be recovered safely.

## Validation

From `backend/`, run:

```bash
cargo test -p mcpmate-migrations
cargo test -p mcpmate-capability-store --test catalog_contract
cargo test -p mcpmate-secrets
cargo clippy --all-targets --all-features -- -D warnings
```

Also run the affected caller tests. A compile check or a successful fresh install does not prove an upgrade: migration work must include a representative legacy database, rollback evidence for unsafe input, rerun idempotence, and a readable recovery backup for an existing file.
