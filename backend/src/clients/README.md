# Clients Module Overview

The `clients` module provides the template-driven client configuration engine.  Templates replace the legacy SQLite rules and describe how to detect clients, how to render configuration fragments, and how to write them back to disk or other storage adapters.

## Template Metadata Structure

Template files live under `config/client/*.json5`.  Each file contains a `ClientTemplate` definition with the following sections:

### Top-level Fields

- `identifier` – stable key used to reference the client.
- `display_name` – human-friendly name shown in UIs.
- `format` – default output format rendered by the engine (`json`, `json5`, `toml`, `yaml`).
- `protocol_revision` – optional MCP standards revision used for future schema validation.
- `metadata` – arbitrary JSON values exposed to tooling (icons, categories, runtime hints, etc.).

### Storage

```json5
storage: {
  kind: "file",
  path_strategy: "config_path",
}
```

- `kind` – which storage adapter to use. Built-ins:
  - `"file"` – read/write a text file via `FileConfigStorage`.
  - `"custom"` – reserved for future adapters.
- `path_strategy` – optional hint to the storage adapter.  For the file adapter we currently support:
  - `"config_path"` – resolve the first `detection` rule that supplies `config_path`/`value` as the target file.
  - `null`/omitted – adapter chooses default behaviour.

### Current Scope Boundary

The backend mainline intentionally ships only file-backed client configuration writes. Earlier Cherry Studio and Warp-specific storage work was tightly coupled to backend build features, but the implementation remained shallow and unreliable across the broader product surface. That coupling made the architecture harder to extend and increased compile-time/build complexity, so those adapters were removed from the default backend path for now.

This does **not** remove MCPMate's hosted or unify management model. Users can still add MCPMate manually as an MCP server in compatible clients and continue using hosted/unify-style management flows through MCPMate itself. The removed part is automatic client-specific config writing for Cherry Studio and Warp, not the higher-level MCPMate management capability.

### Detection Rules

Detection rules tell the engine how to recognise an installed client and which configuration file to use.

```json5
{
  method: "file_path",
  value: "/Applications/Zed.app",
  config_path: "~/.config/zed/settings.json",
}
```

- `method` – one of:
  - `"file_path"` – check whether a file or directory exists at `value`.
  - `"config_path"` – treat `value` (or `config_path`) as the configuration file location.
  - `"bundle_id"` – reserved for macOS bundle detection.
- `value` – method-specific payload (path or bundle id).
- `config_path` – optional override for the resolved config file.
- `priority` – smaller numbers run first (optional).

### Configuration Mapping

```json5
config_mapping: {
  container_keys: ["context_servers"],
  container_type: "object_map",
  merge_strategy: "deep_merge",
  keep_original_config: true,
  managed_endpoint: { ... },
  transports: { ... }
}
```

- `container_keys` – ordered dot paths where MCP entries live (the first entry is used for creation; the rest act as version fallbacks).
- `container_type` – controls how fragments are merged:
  - `"object_map"` – treat the container as a JSON object keyed by server name.
  - `"array"` – treat it as an array; items will be upserted by `name`.
Note: the historical `"mixed"` mode has been folded into `object_map` semantics. Use `container_keys` with dot paths to address subtrees.
- `merge_strategy` – when a fragment already exists:
  - `"replace"` – overwrite the subtree with the new fragment.
  - `"deep_merge"` – recursively merge objects/arrays.
- `keep_original_config` – whether untouched parts of the original file should be preserved.

#### Managed Endpoint

Defines how a “hosted/managed” profile should emit a single proxy server.  The engine infers supported transports from `transports` keys and applies a fixed global priority when choosing what to render:

- Priority: `streamable_http` → `sse` → `stdio`.
- If `stdio` is chosen, MCPMate automatically uses the co-located bridge binary to expose its upstream endpoint over stdio, emitting a `command/args/env` entry the client understands. There is no separate `stdio_bridge` transport in templates.
- `managed_source` – optional descriptor of where metadata comes from (e.g. `"profile"`).
  - Legacy: older templates may use `managed_endpoint: { source: "profile" }`; both forms are supported, but the flattened `managed_source` is preferred.

#### Transports

Describe how to render each transport.  The `template` value is arbitrary JSON/JSON5 with Handlebars placeholders.

For Zed:

```json5
stdio: {
  template: {
    source: "custom",
    enabled: true,
    command: "{{command}}",
    args: "{{{json args}}}",
    env: "{{{json env}}}"
  },
  requires_type_field: false,
},
```

Everything inside `template` becomes part of the rendered client config.  The `source: "custom"` and `enabled: true` keys are exactly what Zed expects in `context_servers` entries; they are not keywords inside MCP Mate.

### Storage Helpers

The file storage adapter also exposes `PathService::atomic_write_with_backup(identifier, policy)` for safe writes.  Each client has a persisted `BackupPolicySetting` (`keep_last`, `keep_n`, or `off`) stored in the SQLite `client` table:

- `keep_last` – maintain a single backup alongside the current file.
- `keep_n` – maintain the last N backups (default: 30), stored under `~/.mcpmate/backups/client/...`.
- `off` – write without creating or pruning backups.

Policy changes are surfaced through `/api/client/backups/policy` and drive retention for both automated renders and manual restores.

## Supporting Types

All enums/structs are defined in `src/clients/models.rs`.  Key type mappings:

- `ContainerType` – `object_map | array`.
- `MergeStrategy` – `replace | deep_merge`.
- `DetectionMethod` – `file_path | bundle_id | config_path`.
- `StorageKind` – `file | kv | custom`.

## Adding a New Template

1. Create a JSON5 file under `config/client/<identifier>.json5`.
2. List detection rules for each platform you want to support.
3. Describe `config_mapping` so the engine knows where to patch data.
4. Register any new transports in `transports`.
5. Optional: add a smoke test similar to those in `clients::service::tests`.

## Runtime Flow

1. `ClientConfigService::bootstrap` seeds official templates into the runtime database and builds an in-memory index.
2. API handlers (`src/api/handlers/client/handlers.rs`) call the service to list templates, render configs, and apply changes.
3. Storage adapters resolve target paths using detection rules and the path service; writes are performed atomically with backups.

## Client Approval Workflow

MCPMate supports explicit approval states for detected clients, enabling better control over which applications can be managed.

### Approval States

Clients can be in one of three approval states:

- **`approved`** — Default state. Client is fully functional and can be configured/managed
- **`pending`** — Client detected but awaiting approval (typically for unknown clients without templates)
- **`suspended`** — Client explicitly disabled from management operations

### Unknown Client Detection

When `ClientConfigService::list_clients()` detects an installed application without a matching template:

1. **Automatic Row Creation** — `ensure_pending_unknown_row()` creates a database record with:
   - `approval_status = 'pending'`
   - `template_id = NULL` (no template binding)

2. **Synthetic Template** — A minimal `ClientTemplate` is generated in-memory with:
   - Empty `detection`, `config_mapping`, and `transports`
   - Identifier matching the detected client
   - Display name from detection results

3. **API Exposure** — The unknown client appears in `/api/client` responses with:
   - `approval_status: "pending"`
   - `template_known: false`
   - `pending_approval: true`

### Approval API Endpoints

- **`POST /api/client/manage/approve`**
  - Sets `approval_status = 'approved'`
  - Enables configuration operations for the client
  
- **`POST /api/client/manage/suspend`**
  - Sets `approval_status = 'suspended'`
  - Disables management without deleting the record

### Operation Guards

Configuration operations (`/api/client/config/apply`, `/api/client/config/restore`) check approval status:

```rust
if state.is_pending_approval() {
    return Err(StatusCode::FORBIDDEN);
}
```

Attempting to configure a pending client returns **403 Forbidden** with a warning log entry.

### Template Binding

Approved clients can be bound to templates later by:

1. Creating or identifying a matching template
2. Updating the `template_id` field in the `client` table
3. Optionally setting `template_version` for version tracking

The `template_id` field decouples record identity (`identifier`) from template binding, allowing flexible evolution of client definitions.

## Governance, Attachment & Backup APIs

- `/api/client/manage/approve` (POST) sets `approval_status = 'approved'` and allows configuration operations.
- `/api/client/manage/suspend` (POST) sets `approval_status = 'suspended'` and blocks configuration operations without deleting MCPMate-side settings.
- `/api/client/detach` (POST) removes MCPMate entries from the external client configuration and marks `attachment_state = 'detached'`.
- `/api/client/backups/list` (GET) lists stored backups, optionally filtered by `identifier`.
- `/api/client/backups/restore` / `/api/client/backups/delete` (POST) restore or remove a backup snapshot.
- `/api/client/backups/policy` (GET/POST) reads or updates the retention policy described above.

All state is persisted in the lightweight `client` table so that backups and management preferences survive restarts without reintroducing the legacy config tables. Transport rules are persisted with canonical transport keys so record data stays authoritative and deterministic.
