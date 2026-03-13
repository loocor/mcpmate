# MCPMate Workspace Progress (2025-10-12)

This document lists the active, near‑term tasks.

## Completed — API request/response logging & client apply diagnostics

Status: Completed (2025-10-12)

Deliverables
- Added TraceLayer for HTTP request/response logging (method/path/status/latency).
  - 5xx at ERROR, 4xx at WARN, 2xx at DEBUG to reduce noise.
- Added contextual error logs to `/client/config/apply` handler.
- Temporary write‑probe for apply path downgraded to DEBUG; used for transport diagnosis.

Notes
- Audit logging will be implemented later as a dedicated layer (structured, redacted, pluggable sinks).

---

## Completed — Cherry adapter: support stdio/sse/streamable_http

Status: Completed (2025-10-12)

Deliverables
- `cherry` crate: ServerRequest/ServerResponse now include optional fields for stdio (command/args/env) and HTTP (baseUrl/headers/longRunning).
- Backend Cherry adapter writes only present fields; no synthetic defaults required.
- Fixed 500s caused by strict deserialization (missing args/command).

---

## Planned — Client settings: config_mode/transport/client_version

Goal: Persist management mode and transport choice to drive UI and write/apply behavior without extra round‑trips.

Deliverables
- Schema: add columns to `client` table
  - `config_mode` TEXT NOT NULL DEFAULT 'hosted'  // 'hosted'|'transparent'
  - `transport` TEXT NULL  // 'streamable_http'|'sse'|'stdio'; NULL means auto‑select then persist
  - `client_version` TEXT NULL
- API:
  - List: `/api/client/list` includes `config_mode`, `transport`, `client_version` in ClientInfo
  - Update: `PATCH /api/client/update` (partial update)
    - Body: `{ identifier, config_mode?, transport?, client_version? }`
    - `transport: null` clears fixed choice → auto‑select next apply and persist
- Apply behavior:
  - If `transport` is set → use it.
  - If NULL → auto‑select by priority (streamable_http → sse → stdio) gated by `client_version`; then persist.

Board (Dashboard)
- Overview
  - Show/hide Re‑apply based on `config_mode` ('transparent' only)
  - Show `Supported transports` (chips) and current `transport`
  - Add `Transport` selector (Auto = clear `transport` → NULL) and inline save (PATCH /api/client/update)
- Configuration
  - Keep Diff/Warnings; no main CTA (trigger from Overview)

Acceptance
- First page render determines Re‑apply visibility from list data (no extra details call).
- Transparent mode supports manual transport selection; Hosted mode hides Re‑apply.
- Apply path respects persisted `transport` or performs auto‑select and persists.

---

## Notes / Parking Lot
- Version/Capability detection can be improved later; for now `client_version` + priority provides a practical baseline.

---

## In Progress — Backend multi-crate: storage bootstrap (M1)

Status: In progress (2025-10-27)

Deliverables
- `mcpmate-storage` owns SQLite bootstrap via new trait-based SPI (`bootstrap.rs`, `sqlite.rs`), wrapping database creation and pool wiring.
- Backend `config::database::Database` now consumes the bootstrapper (`Database::with_config`), preserving initialization/import/event flow through hooks.
- Proxy init exposes `setup_database_with_config` so binaries/tests can supply bootstrap settings without touching storage internals.

Next
- Continue migrating config submodules into `mcpmate-storage` following `backend/docs/refactor/multi_crates.md` (M1 checklist).
- Resolve longstanding doctest macro/import failures before enabling full `cargo test --workspace` gating.

Update (2025-10-27 evening)
- Restored OpenAPI visibility for provider-driven read endpoints; routers now clone shared definitions and `create_router_internal` merges provider specs before serving `/docs`.
- Registry provider keeps runtime handler wiring while manually appending docs to avoid blocking on `OperationHandler` bounds.
- Evidence: `./script/full_test.sh` (2025-10-27T17:37+08, passes clippy/tests/E2E) and `cargo check` w/ providers.

Evidence
- `backend/docs/progress.md#M1-—-storage-bootstrap-extraction-config-→-mcpmate-storage-2025-10-27`
- `cargo check --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace` (doctest macro imports still pending from pre-existing backlog)

---

## In Progress — Backend multi-crate: API extraction to crates (IM4 → M4)

Status: In progress (2025-10-28)

Goal
- Decompose `backend/src/api` into crate-owned modules with clear boundaries.
- `mcpmate-api` owns API aggregation (routers + OpenAPI) and providers. App becomes thin: inject SPI services, mount `/docs`, serve SSE.

Plan (small, verifiable steps)
1) SPI boundaries (read/write split) — 2–3h
   - Define minimal service traits in `mcpmate-types` (or `mcpmate-api::spi`) for: System/RuntimeQuery, ServerQuery/Admin, ProfileQuery/Admin, ClientConfigAdmin, InspectorQuery/Admin (incl. call subscription), CacheAdmin.
   - Provide app-side adapters wrapping existing implementations (Redb/SQLite/connection pool, etc.) behind `Arc<dyn Trait>`; keep old constructors for rollback.
2) Providers/handlers depend on SPI — 3–4h
   - Refactor HTTP handlers to accept `State<Arc<dyn Trait>>` instead of `AppState` and adjust providers to pass trait objects. Keep routes and payloads identical.
   - Move provider modules under `mcpmate-api` once no `AppState` coupling remains.
3) OpenAPI helpers and macros migration — 1–2h
   - Move aide wrapper macros into `mcpmate-api` (context-free), keep tag/description parity. Ensure `/docs` renders unchanged. DTOs move to `mcpmate-types` if referenced.
4) Inspector SSE extraction — 1–2h
   - Introduce `CallEvents` subscription SPI; move SSE endpoint to `mcpmate-api` while app wires implementation. Validate 10× concurrency + health probe.
5) Cleanup and final flip — 1–2h
   - Delete residual `backend/src/api` aggregation/helpers. App only injects services, mounts `(router, spec)` from `mcpmate-api`, and serves `/docs`/`/openapi.json`.

Recent progress (done)
- Aggregation moved to `mcpmate-api` via `build_api_with_docs` (backend/crates/mcpmate-api/src/lib.rs:62).
- OpenAPI merge utilities centralized in `mcpmate-api::spec` (backend/crates/mcpmate-api/src/spec.rs).
- All write endpoints wrapped as Providers (server/client/inspector/profile) to reduce app-level routing noise.

Validation & guardrails
- Each step must pass: `cargo clippy --all-targets --all-features -D warnings`, `cargo test`, and `./script/full_test.sh` (manual for E2E).
- OpenAPI delta: only additions allowed; no breaking changes.
- Concurrency smoke: 10× `tools/list` remains stable; cache TTL/signature invalidation unchanged.

Next action (now)
- Implement Step 1: Add SPI traits to `mcpmate-types` and provide app-side adapters (no call-site switching yet). Then compile and clippy.

---

## Completed — Backend multi-crate: system extraction (IM5)

Status: Completed (2025-10-29)

Deliverables
- New crate `backend/crates/mcpmate-system` owning:
  - Runtime ports configuration (API/MCP) with global accessor.
  - System metrics collector with smoothing and optional background refresh.
  - Path mapper/service with cross-platform template resolution.
  - Client application detection scaffolding (cross-platform rules, macOS path helpers).
- Removed legacy `src/system` and updated `app-mcpmate` to import from `mcpmate_system`.
- No API behavior changes; OpenAPI output unchanged.

Notes
- Transitional: detection still uses `sqlx/sqlite` in `mcpmate-system` to reuse existing rules; this temporarily violates the storage boundary. Plan to move SQL into `mcpmate-storage` behind a SPI (`SystemDetectionRepo`) and inject into `mcpmate-system`.

Evidence
- `cargo check --workspace`, `cargo clippy --all-targets --all-features -D warnings`.
- `./script/full_test.sh` green; Inspector parity unchanged.
