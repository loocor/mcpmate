# MCPMate Desktop Alpha (Tauri Shell)

This crate provides an early Tauri wrapper around the existing MCPMate backend so that we can validate desktop data-directory handling, network permissions, and database access on Windows and Linux before the fully native clients ship.

## Prerequisites

- Rust toolchain from the repository root: `rustup target add` steps are unchanged; run commands from `backend/tauri` for the Tauri shell.
- Node.js tooling for the dashboard (`board/`): ensure `npm install` has been executed in `board/` at least once.
- Tauri CLI 2.x (installed via `cargo install tauri-cli --locked`).

## Building the Dashboard Assets

The desktop shell loads the compiled dashboard bundle. Build it once before `tauri dev` or `tauri build`:

```bash
npm --prefix ../../board run build
```

During development you can rely on the live dev server instead:

```bash
npm --prefix ../../board run dev -- --host 127.0.0.1 --port 5173
```

The Tauri config ensures the correct hooks (`beforeDevCommand`, `beforeBuildCommand`) fire automatically, so running `cargo tauri dev` also spins up the dashboard dev server.

## Running the Desktop Shell

From `backend/tauri/src-tauri`:

```bash
cargo tauri dev
```

or release build:

```bash
cargo tauri build
```

## Data Directory and Environment Overrides

On startup the shell resolves the per-app data directory through Tauri's `PathResolver::app_data_dir()` and passes that path into MCPMate's runtime (via `MCPMATE_DATA_DIR`). This avoids REDB/SQLite contention with existing CLI instances.

You can override runtime ports or modes without recompiling by exporting the following variables before launching:

| Variable                  | Purpose                                      | Default   |
| ------------------------- | -------------------------------------------- | --------- |
| `MCPMATE_TAURI_API_PORT`  | REST API port served by the embedded backend | `8080`    |
| `MCPMATE_TAURI_MCP_PORT`  | Embedded MCP server port                     | `8000`    |
| `MCPMATE_TAURI_LOG`       | Log level if `RUST_LOG` is unset             | `info`    |
| `MCPMATE_TAURI_TRANSPORT` | MCP transport mode                           | `uni`     |
| `MCPMATE_TAURI_PROFILE`   | Comma-delimited profile IDs to preload       | *(empty)* |
| `MCPMATE_TAURI_MINIMAL`   | Set to `true` / `1` to skip profile loading  | `false`   |

## Known Limitations

- The existing `cargo test` suite still reports doctest failures that predate this integration (`aide_wrapper_*` macros). Track and resolve separately.
- Icon assets are placeholders; replace `src-tauri/icons/icon.png` before branding.

## Release & Update Resources

- Desktop release & updater workflow: `docs/desktop-release-guide.md`
- Automation helpers: `script/build-tauri-release.sh`, `script/generate-update-manifest.sh`

## Next Steps

- Wire the Inspector bundle once the backend API schema stabilises.
- Extend shutdown handling if we add dedicated background worker threads beyond the current proxy/API servers.
