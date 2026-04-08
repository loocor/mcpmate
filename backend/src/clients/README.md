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
  - `"kv"` – key-value backed adapters. Currently supported: `adapter: "cherry_kv"` (Cherry Studio LevelDB). Enabled by default via the `kv-cherry` feature.
  - `"custom"` – reserved for future adapters.
- `path_strategy` – optional hint to the storage adapter.  For the file adapter we currently support:
  - `"config_path"` – resolve the first `detection` rule that supplies `config_path`/`value` as the target file.
  - `null`/omitted – adapter chooses default behaviour.

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
  format_rules: { ... }
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

Defines how a “hosted/managed” profile should emit a single proxy server.  The engine infers supported transports from `format_rules` keys and applies a fixed global priority when choosing what to render:

- Priority: `streamable_http` → `sse` → `stdio`.
- If `stdio` is chosen, MCPMate automatically uses the co-located bridge binary to expose its upstream endpoint over stdio, emitting a `command/args/env` entry the client understands. There is no separate `stdio_bridge` transport in templates.
- `managed_source` – optional descriptor of where metadata comes from (e.g. `"profile"`).
  - Legacy: older templates may use `managed_endpoint: { source: "profile" }`; both forms are supported, but the flattened `managed_source` is preferred.

#### Format Rules

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
4. Register any new transports in `format_rules`.
5. Optional: add a smoke test similar to those in `clients::service::tests`.

## Runtime Flow

1. `ClientConfigService::bootstrap` seeds official templates into the runtime database and builds an in-memory index.
2. API handlers (`src/api/handlers/client/handlers.rs`) call the service to list templates, render configs, and apply changes.
3. Storage adapters resolve target paths using detection rules and the path service; writes are performed atomically with backups.

## Client Approval Workflow

MCPMate supports explicit approval states for detected clients, enabling better control over which applications can be managed.

### Approval States

Clients can be in one of four approval states:

- **`approved`** — Default state. Client is fully functional and can be configured/managed
- **`pending`** — Client detected but awaiting approval (typically for unknown clients without templates)
- **`suspended`** — Client explicitly disabled from management operations
- **`rejected`** — Client explicitly rejected and excluded from lists

### Unknown Client Detection

When `ClientConfigService::list_clients()` detects an installed application without a matching template:

1. **Automatic Row Creation** — `ensure_pending_unknown_row()` creates a database record with:
   - `approval_status = 'pending'`
   - `managed = 0` (disabled by default)
   - `template_id = NULL` (no template binding)

2. **Synthetic Template** — A minimal `ClientTemplate` is generated in-memory with:
   - Empty `detection`, `config_mapping`, and `format_rules`
   - Identifier matching the detected client
   - Display name from detection results

3. **API Exposure** — The unknown client appears in `/api/client` responses with:
   - `approval_status: "pending"`
   - `template_known: false`
   - `pending_approval: true`

### Approval API Endpoints

- **`POST /api/client/manage/approve`**
  - Sets `approval_status = 'approved'` and `managed = 1`
  - Enables configuration operations for the client
  
- **`POST /api/client/manage/suspend`**
  - Sets `approval_status = 'suspended'` and `managed = 0`
  - Disables management without deleting the record

- **`POST /api/client/manage/reject`**
  - Sets `approval_status = 'rejected'`
  - Client is excluded from future listings

### Operation Guards

Configuration operations (`/api/client/config/apply`, `/api/client/config/restore`) check approval status:

```rust
if state.is_pending_unknown() {
    return Err(StatusCode::FORBIDDEN);
}
```

Attempting to configure a pending unknown client returns **403 Forbidden** with a warning log entry.

### Template Binding

Approved clients can be bound to templates later by:

1. Creating or identifying a matching template
2. Updating the `template_id` field in the `client` table
3. Optionally setting `template_version` for version tracking

The `template_id` field decouples record identity (`identifier`) from template binding, allowing flexible evolution of client definitions.

## Management & Backup APIs

- `/api/client/manage` (POST) toggles whether MCPMate manages a client (`enable` / `disable`).  Disabled clients are skipped during update calls and surface with `managed: false` in responses.
- `/api/client/backups/list` (GET) lists stored backups, optionally filtered by `identifier`.
- `/api/client/backups/restore` / `/api/client/backups/delete` (POST) restore or remove a backup snapshot.
- `/api/client/backups/policy` (GET/POST) reads or updates the retention policy described above.

All state is persisted in the lightweight `client` table so that backups and management preferences survive restarts without reintroducing the legacy config tables. Transport alias keymaps are also shipped as built-in code defaults rather than external user files.

## Warp (SQLite) Support

This section documents how Warp stores its MCP server configuration and how MCPMate can read/write it safely.

### Storage Overview

Warp persists MCP servers in a SQLite database at:
- macOS: `~/Library/Application Support/dev.warp.Warp-Stable/warp.sqlite`

Key tables involved:
- `generic_string_objects` – JSON payloads for servers (name, uuid, transports).
- `object_metadata` – tags each row as an MCP server and links it by `shareable_object_id`.
- `mcp_environment_variables` – environment variables keyed by server UUID (as 16‑byte BLOB).
- `active_mcp_servers` – optional, tracks which servers are active (ephemeral; not required for config persistence).

### Data Invariants

- `object_metadata.object_type` must be exactly `GENERIC_STRING_JSON_MCPSERVER` for Warp MCP entries.
- `object_metadata.shareable_object_id = generic_string_objects.id`.
- `generic_string_objects.data` (JSON) must include at least:
  - `name: string`
  - `uuid: string` (canonical UUID with dashes)
  - `transport_type.CLIServer = { command: string, args: string[], cwd_parameter: null|string, static_env_vars: [] }`
- `mcp_environment_variables.mcp_server_uuid` stores the same UUID as a 16‑byte BLOB.
  - Join rule: `upper(replace(json_extract(g.data,'$.uuid'),'-','')) = upper(hex(mv.mcp_server_uuid))`.

### List Servers (with env)

```sql
select
  g.id,
  json_extract(g.data,'$.name')  as name,
  json_extract(g.data,'$.uuid')  as uuid,
  json_extract(g.data,'$.transport_type.CLIServer.command') as command,
  json_extract(g.data,'$.transport_type.CLIServer.args')    as args,
  mv.environment_variables                                 as env_json
from generic_string_objects g
join object_metadata om
  on om.shareable_object_id = g.id
 and om.object_type = 'GENERIC_STRING_JSON_MCPSERVER'
left join mcp_environment_variables mv
  on upper(replace(json_extract(g.data,'$.uuid'),'-','')) = upper(hex(mv.mcp_server_uuid))
order by g.id;
```

### Create Server (transaction)

1) Insert JSON row and capture `id`:
```sql
insert into generic_string_objects(data) values (:data_json);
select last_insert_rowid(); -- => :gso_id
```
2) Tag as MCP server:
```sql
insert into object_metadata(is_pending, object_type, shareable_object_id, retry_count)
values (0, 'GENERIC_STRING_JSON_MCPSERVER', :gso_id, 0);
```
3) Upsert env by UUID (16‑byte BLOB):
```sql
insert into mcp_environment_variables(mcp_server_uuid, environment_variables)
values (:uuid_blob, :env_json)
on conflict(mcp_server_uuid) do update set environment_variables=excluded.environment_variables;
```
4) (Optional) Mark active:
```sql
insert or ignore into active_mcp_servers(mcp_server_uuid) values (:uuid_text);
```

UUID → BLOB mapping: remove dashes and interpret as raw bytes in big‑endian nibble order (no byte reordering). In Rust, `uuid::Uuid::parse_str(uuid).as_bytes()` yields the correct 16‑byte slice.

### Update / Delete

- Update JSON:
```sql
update generic_string_objects set data=:data_json where id=:gso_id;
```
- Update env:
```sql
update mcp_environment_variables set environment_variables=:env_json where mcp_server_uuid=:uuid_blob;
```
- Delete all related rows:
```sql
delete from active_mcp_servers where mcp_server_uuid=:uuid_text;
delete from object_metadata where shareable_object_id=:gso_id and object_type='GENERIC_STRING_JSON_MCPSERVER';
delete from mcp_environment_variables where mcp_server_uuid=:uuid_blob;
delete from generic_string_objects where id=:gso_id;
```

### Adapter Mapping in MCPMate

- Use `StorageKind::Custom` with a dedicated adapter `warp_sqlite` (see `src/clients/adapters/warp_sqlite.rs`).
- Dispatch rule (planned): when a template has `kind: custom` and `metadata.managed_source == "warp_sqlite"`, route to the Warp adapter; otherwise fall back to other custom adapters.
- The adapter implements `ConfigStorage` and interprets `read`/`write_atomic` as the SQL flows above. Backups are implemented via SQLite hot backup into a timestamped file next to `warp.sqlite`.

Template hint (JSON5):
```json5
{
  identifier: "warp.mcp",
  display_name: "Warp MCP (SQLite)",
  format: "json",
  metadata: { managed_source: "warp_sqlite" },
  storage: { kind: "custom" },
  format_rules: {
    // Render the minimal server JSON (goes into generic_string_objects.data)
    custom: {
      template: {
        name: "{{name}}",
        uuid: "{{uuid}}",
        transport_type: {
          CLIServer: { command: "{{command}}", args: {{args}}, cwd_parameter: null, static_env_vars: [] }
        }
      }
    }
  }
}
```

### Safety & Operations

- Prefer `BEGIN IMMEDIATE` transactions for multi‑step writes.
- Back up `warp.sqlite` together with `-wal`/`-shm` if present; or use SQLite Backup API.
- Do not place credentials inside CLI `args`; store them in `mcp_environment_variables.environment_variables` and inject at spawn time.

### Rust UUID ↔ BLOB Notes

```rust
let u = uuid::Uuid::parse_str(uuid_text)?;           // "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
let uuid_blob: &[u8] = u.as_bytes();                 // 16 bytes for binding
// rusqlite example
conn.execute(
    "insert into mcp_environment_variables(mcp_server_uuid, environment_variables) values (?1, ?2)",
    (uuid_blob, env_json_str),
)?;
```
