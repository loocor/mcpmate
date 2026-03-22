# MCPMate Desktop Beta (Tauri Shell)

This crate provides an early Tauri wrapper around the existing MCPMate backend so that we can validate desktop data-directory handling, network permissions, and database access on Windows and Linux before the fully native clients ship.

## Prerequisites

- Rust toolchain from the repository root: `rustup target add` steps are unchanged; run commands from `desktop/tauri` for the Tauri shell.
- Node.js tooling for the dashboard (`board/`): ensure `npm install` has been executed in `board/` at least once.
- Tauri CLI 2.x (installed via `cargo install tauri-cli --locked`).

## Building the Dashboard Assets

The desktop shell loads the compiled dashboard bundle. `tauri.conf.json` runs **`beforeDevCommand` / `beforeBuildCommand`** from `board/` (prefers **Bun**, falls back to **npm**).

For a **packaged** macOS build, the **bridge sidecar** must exist at `backend/target/sidecars/bridge` (`tauri.conf.json` `externalBin`). Use the release script (board + notices + bridge + Tauri in one go):

```bash
cd desktop/tauri
./script/macos-build-tauri-release.sh --targets aarch64-apple-darwin --skip-notarize
```

Adjust `--targets` for Intel (`x86_64-apple-darwin`) or both. Signing/notarization can wait; see the same script when you enable them.

**OAuth acceptance (Mac app + Worker logs):** see `auth/README.md` → *QA acceptance: macOS OAuth + Worker visibility*.

Manual board build (optional):

```bash
npm --prefix ../../board run build
```

During development you can rely on the live dev server instead:

```bash
npm --prefix ../../board run dev -- --host 127.0.0.1 --port 5173
```

The Tauri config ensures the correct hooks (`beforeDevCommand`, `beforeBuildCommand`) fire automatically, so running `cargo tauri dev` also spins up the dashboard dev server.

## Running the Desktop Shell

From `desktop/tauri/src-tauri`:

```bash
cargo tauri dev
```

or release build:

```bash
cargo tauri build
```

## One-Time Signing Setup (Minimal)

For macOS DMG distribution (not on the App Store), use Apple ID notarization
with a Developer ID Application certificate. Put your credentials in `.env`:

```bash
cp .env.example .env
# Fill these four values:
#  - APPLE_SIGNING_IDENTITY
#  - APPLE_ID
#  - APPLE_PASSWORD (app-specific)
#  - APPLE_TEAM_ID
```

Then run the release helper (no extra flags needed):

```bash
./script/macos-build-tauri-release.sh
```

The script reads `.env` / `.env.local`, configures codesign & notarization,
builds the bridge sidecar, and outputs a notarized DMG to `$HOME/Downloads`.

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
 - Market pages in the dashboard are served via a custom URI scheme `mcpmate://localhost/market-proxy/*` implemented in `src-tauri/src/market_proxy.rs`. This replaces Vite's dev-time middleware so the Market (MCP Market / MCP.so) works in the packaged app without a local dev server. If remote portals change their HTML significantly, adjust the lightweight HTML rewriter and the injected shim accordingly.

## Diagnostics (Market pages)

To help diagnose white screen/garbled text cases when embedding third‑party portals, the desktop shell
provides built‑in diagnostics covering both the network proxy and the in‑page runtime:

- Help → Enable Market Diagnostics: toggles diagnostics at runtime.
- Help → Export Market Diagnostics…: copies the log file to your Desktop.
- Log file path: `/tmp/mcpmate-market-diag.log` (macOS; `std::env::temp_dir()` on other OSes).

Build‑time default (for zero‑touch debug builds):

- Set env `MCPMATE_TAURI_MARKET_DIAG_DEFAULT=1` or pass `--diag-default` to the release script
  to ship a build where diagnostics are enabled on launch. The front‑end will also forward
  runtime events (e.g., `market-ready`, errors, unhandled rejections) into the same log.

## Release & Update Resources

- Desktop release & updater workflow: `docs/desktop-release-guide.md`
- Automation helpers: `script/macos-build-tauri-release.sh`, `script/generate-update-manifest.sh`
- Auto-updater plugin is compiled in but disabled by default (`plugins.updater.active = false`). Replace the placeholder Minisign public key in `tauri.conf.json` and point to real endpoints before turning it on.

## Next Steps

- Wire the Inspector bundle once the backend API schema stabilises.
- Extend shutdown handling if we add dedicated background worker threads beyond the current proxy/API servers.
