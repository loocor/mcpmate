# MCPMate Database Migrations

`mcpmate-migrations` is the sole owner of durable SQLite schema evolution in MCPMate.

## Why this exists

Users upgrade MCPMate by running a newer version, not by locating and executing a separate migration tool. When a database needs an upgrade, the product creates a recovery backup, applies the pending steps exactly once, and refuses to start if that work cannot complete safely.

## Ownership boundary

- Put durable `CREATE`, `ALTER`, `DROP`, indexes, constraints, and one-time historical data rewrites here.
- Keep ordinary business reads and writes in their domain modules.
- A domain crate may declare the schema version it needs, but it must not create or upgrade durable schema itself.
- Each physical database has one ordered migration stream. MCPMate currently has `config` and `audit` targets.

## Adding a migration

1. Read this document before designing persistent data changes.
2. Add an immutable, strictly increasing version to the target stream.
3. Use `SqlMigration` for simple DDL. Use a `MigrationStep` implementation for a transactional data transformation that needs schema inspection or Rust control flow.
4. Give the migration stable name and checksum source. Never edit an already released migration; add a corrective migration instead.
5. Test a fresh database, the relevant legacy structure, rerun idempotence, and failure rollback.

## Runtime guarantees

The runner records target, version, name, checksum, and application time in `mcpmate_schema_migrations`. It rejects modified history and runs pending steps in one transaction. File-backed startup backup and target migration registration belong to the product composition root; in-memory tests use the same migration chain without filesystem backup.
