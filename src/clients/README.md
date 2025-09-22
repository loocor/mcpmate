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
  - `"kv"`, `"custom"` – reserved for future adapters.
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
  container_key: "context_servers",
  container_type: "object_map",
  merge_strategy: "deep_merge",
  keep_original_config: true,
  managed_endpoint: { ... },
  format_rules: { ... }
}
```

- `container_key` – dotted path inside the client config where MCP entries live (use `""` for root).
- `container_type` – controls how fragments are merged:
  - `"object_map"` – treat the container as a JSON object keyed by server name.
  - `"array"` – treat it as an array; items will be upserted by `name`.
  - `"mixed"` – nested path (VS Code style); merge happens on the subtree only.
- `merge_strategy` – when a fragment already exists:
  - `"replace"` – overwrite the subtree with the new fragment.
  - `"deep_merge"` – recursively merge objects/arrays.
- `keep_original_config` – whether untouched parts of the original file should be preserved.

#### Managed Endpoint

Defines how a “hosted/managed” profile should emit a single proxy server.  The engine infers supported transports from `format_rules` keys and applies a fixed global priority when choosing what to render:

- Priority: `streamable_http` → `sse` → `stdio`.
- If `stdio` is chosen, MCPMate automatically uses the co-located bridge binary to expose its upstream endpoint over stdio, emitting a `command/args/env` entry the client understands. There is no separate `stdio_bridge` transport in templates.
- `managed_source` – optional descriptor of where metadata comes from (e.g. `"profile"`).
  - Legacy: older templates may use `managed_endpoint: { source: "profile" }`; both形式兼容，但推荐使用扁平字段 `managed_source`。

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

- `keep_last` – maintain a single backup alongside the current file (default).
- `keep_n` – retain `n` most recent backups under `~/.mcpmate/backups/client/…`.
- `off` – write without creating or pruning backups.

Policy changes are surfaced through `/api/client/backups/policy` and drive retention for both automated renders and manual restores.

## Supporting Types

All enums/structs are defined in `src/clients/models.rs`.  Key type mappings:

- `ContainerType` – `object_map | array | mixed`.
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

1. `ClientConfigService::bootstrap` seeds official templates under `~/.mcpmate/client/official/` and builds an in-memory index.
2. API handlers (`src/api/handlers/client/handlers.rs`) call the service to list templates, render configs, and apply changes.
3. Storage adapters resolve target paths using detection rules and the path service; writes are performed atomically with backups.

## Management & Backup APIs

- `/api/client/manage` (POST) toggles whether MCPMate manages a client (`enable` / `disable`).  Disabled clients are skipped during update calls and surface with `managed: false` in responses.
- `/api/client/backups/list` (GET) lists stored backups, optionally filtered by `identifier`.
- `/api/client/backups/restore` / `/api/client/backups/delete` (POST) restore or remove a backup snapshot.
- `/api/client/backups/policy` (GET/POST) reads or updates the retention policy described above.

All state is persisted in the lightweight `client` table so that backups and management preferences survive restarts without reintroducing the legacy config tables.
