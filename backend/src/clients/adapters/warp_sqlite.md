# Warp SQLite Adapter – Wiring Plan

This document captures the minimal code changes required to enable Warp (SQLite) configuration management in MCPMate without breaking existing flows.

## Goals
- Read/write Warp MCP servers via SQLite using the documented rules.
- Treat Warp as a `custom` storage target selected by template metadata.
- Provide safe backups and straightforward testing.

## Changes

1) Cargo
- Add optional dependency and feature gate:
  ```toml
  [features]
  warp-sqlite = ["dep:rusqlite", "dep:uuid", "dep:chrono"]

  [dependencies]
  rusqlite = { version = "0.31", optional = true, features = ["bundled"] }
  uuid     = { version = "1", optional = true }
  chrono   = { version = "0.4", optional = true, default-features = false, features = ["clock"] }
  ```

2) Module
- Expose the adapter behind the feature flag in `src/clients/mod.rs`:
  ```rust
  #[cfg(feature = "warp-sqlite")]
  pub mod adapters { pub mod warp_sqlite; }
  ```

3) Engine dispatch
- In `src/clients/engine.rs`, when resolving storage for a template with `kind: Custom`, inspect `template.metadata.get("managed_source")`.
  - If `== "warp_sqlite"` and feature is enabled, construct `WarpSqliteStorage::new(db_path)` where `db_path` defaults to macOS path `~/Library/Application Support/dev.warp.Warp-Stable/warp.sqlite` and can be overridden by env `MCPMATE_WARP_DB`.
  - Otherwise return `ConfigError::StorageAdapterMissing("custom")` as today.

  Sketch:
  ```rust
  #[cfg(feature = "warp-sqlite")]
  use crate::clients::adapters::warp_sqlite::impls::WarpSqliteStorage;

  fn custom_storage_for(template: &ClientTemplate) -> Option<DynConfigStorage> {
      let src = template.metadata.get("managed_source").and_then(|v| v.as_str()).unwrap_or("");
      if src == "warp_sqlite" {
          let db = std::env::var("MCPMATE_WARP_DB").ok()
              .unwrap_or_else(|| format!("{}/Library/Application Support/dev.warp.Warp-Stable/warp.sqlite", dirs::home_dir().unwrap().display()));
          #[cfg(feature = "warp-sqlite")] {
            return Some(WarpSqliteStorage::new(db).as_dyn());
          }
      }
      None
  }
  ```

4) Template example
- Provide a `config/client/warp.json5` official template defaulting to `managed_source: "warp_sqlite"` and `storage.kind: "custom"`.

5) Tests
- Add integration tests under `#[cfg(feature = "warp-sqlite")]` that operate on a temp copy of a fixture DB.
- Cover create → list → update → delete.

## Non‑Goals (Phase 1)
- No UI/API for toggling Warp `active_mcp_servers` — it’s optional.
- No cross‑platform detection beyond macOS default path; Windows/Linux paths can be added later.

## Rollout
- Phase 1: feature‑gated adapter and docs only.
- Phase 2: enable in default features and ship the official template once validated.
