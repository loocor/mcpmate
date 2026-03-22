# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCPMate Desktop (Tauri) is a cross-platform desktop application that wraps the MCPMate backend and dashboard. It serves as an alpha shell for Windows and Linux before fully native clients ship. The key innovation is a streaming reverse proxy for third-party market portals (like https://mcp.so) embedded in an iframe.

## Architecture

### Core Components

- **Backend Integration**: Embeds the MCPMate Rust backend directly via workspace dependencies
- **Dashboard UI**: Loads the pre-built React dashboard from `board/dist` or dev server
- **Market Proxy**: Streaming reverse proxy for Next.js SSR/RSC apps (e.g., mcp.so)
  - `market_stream.rs`: HTTP streaming proxy with HTML injection
  - `market_proxy.rs`: Custom URI scheme handler (`mcpmate://localhost/market-proxy/*`)
- **Data Isolation**: Uses Tauri's `app_data_dir()` to avoid REDB/SQLite contention with CLI

### Streaming Proxy Design

The market proxy handles Next.js streaming rendering by:
1. Buffering minimal data (512 bytes max, keeping 256 bytes for pattern matching)
2. Detecting `<head>` tag in the HTML stream
3. Injecting config/styles after `<head>` without blocking the stream
4. Passing through all other chunks immediately to preserve SSR hydration timing

Reference implementation: `board/vite.config.ts` (lines 287-349)

## Build Commands

### Development

```bash
# From desktop/tauri directory

# Quick dev (requires board dev server)
cargo tauri dev

# Or manually start board first
npm --prefix ../../board run dev -- --host 127.0.0.1 --port 5173
cargo tauri dev
```

### Production Build

```bash
# Build dashboard assets first
npm --prefix ../../board run build

# macOS builds (arm64 and x86_64)
CI=true cargo tauri build --target aarch64-apple-darwin --bundles dmg
CI=true cargo tauri build --target x86_64-apple-darwin --bundles dmg

# Windows
cargo tauri build --target x86_64-pc-windows-msvc --bundles msi

# Use automation script (recommended)
./script/macos-build-tauri-release.sh --targets aarch64-apple-darwin,x86_64-apple-darwin --bundles dmg
```

### Build Script Options

`script/macos-build-tauri-release.sh` supports:
- `--profile <release|debug>`: Cargo profile (default: release)
- `--targets <list>`: Comma-separated targets (default: aarch64,x86_64)
- `--bundles <list>`: Bundle types (default: dmg)
- `--skip-board`: Skip dashboard rebuild
- `--output-dir <path>`: DMG output location (default: ~/Downloads)
- `--sign-identity <string>`: macOS codesign identity (or set `APPLE_SIGNING_IDENTITY`)
- `--apple-id <email>` / `--apple-password <pass>` / `--apple-team-id <TEAMID>`: Apple ID notarization
- `--apple-api-key <KEYID>` / `--apple-api-issuer <UUID>` / `--apple-api-key-path <path>`: Notary API key mode
- `--skip-notarize`: Force-disable notarization even if credentials are set
- `--diag-default`: Build with Market diagnostics enabled by default (equivalent to env
  `MCPMATE_TAURI_MARKET_DIAG_DEFAULT=1`). The app will:
  - auto-enable network proxy diagnostics,
  - auto-enable front-end runtime logging and persist events into the same log file,
  - expose Help → Enable/Export Market Diagnostics menu items.

Diagnostics log location: `/tmp/mcpmate-market-diag.log` on macOS (otherwise `std::env::temp_dir()`).

### Post-build Checksums

After a successful macOS build, the script computes SHA256 for the produced DMGs (arm64/x64) and updates `website/.env`:
- `VITE_MAC_ARM64_SHA256`
- `VITE_MAC_X64_SHA256`

If `website/.env` does not exist, it will be created (preferring `.env.example` as a base when available).

## Runtime Configuration

Environment variables (optional):

| Variable                  | Purpose                                   | Default |
| ------------------------- | ----------------------------------------- | ------- |
| `MCPMATE_TAURI_API_PORT`  | REST API port                             | 8080    |
| `MCPMATE_TAURI_MCP_PORT`  | MCP server port                           | 8000    |
| `MCPMATE_TAURI_LOG`       | Log level                                 | info    |
| `MCPMATE_TAURI_TRANSPORT` | MCP transport mode                        | uni     |
| `MCPMATE_TAURI_PROFILE`   | Comma-delimited profile IDs to preload    | (empty) |
| `MCPMATE_TAURI_MINIMAL`   | Set to `true`/`1` to skip profile loading | false   |

## Key Files

```
src-tauri/
├── src/
│   ├── lib.rs              # Main app setup, window creation, backend bootstrap
│   ├── main.rs             # Entry point
│   ├── market_stream.rs    # Streaming HTTP proxy for Next.js SSR/RSC
│   └── market_proxy.rs     # Custom URI scheme (mcpmate://) for static assets
├── tauri.conf.json         # Tauri config (window, bundle, updater)
└── Cargo.toml              # Dependencies (includes backend workspace)
```

## Common Tasks

### Testing Market Proxy Streaming

1. Launch dev build: `cargo tauri dev`
2. Navigate to Market tab in dashboard
3. Click on mcp.so portal
4. Verify page loads without blocking (streaming works if initial content appears before full load)
5. Check DevTools console for injection markers (`mcpmate-market-config`, `mcpmate-market-shim`)

### Debugging Backend Issues

- Backend logs appear in the Dashboard page console
- Check port conflicts: `lsof -i :8080` or `lsof -i :8000`
- Verify data directory: `~/Library/Application Support/desktop.mcp.umate.ai/` (macOS)

### Updating Dependencies

```bash
# Update backend
cd ../../backend && cargo update

# Update Tauri
cargo update -p tauri -p tauri-plugin-updater -p tauri-plugin-dialog -p tauri-plugin-opener

# Update dashboard
npm --prefix ../../board update
```

## Release Process

See `docs/desktop-release-guide.md` for full details. Key steps:

1. Update version in `tauri.conf.json` and `Cargo.toml`
2. Build dashboard: `npm --prefix ../../board run build`
3. Run release script: `./script/macos-build-tauri-release.sh`
4. (macOS) Codesign & notarize: provide either Apple ID or API key creds via flags or environment.
   - Example (Apple ID): `./script/macos-build-tauri-release.sh --targets aarch64-apple-darwin,x86_64-apple-darwin --sign-identity "Developer ID Application: Your Org (TEAMID)" --apple-id you@example.com --apple-password abcd-efgh-ijkl-mnop --apple-team-id TEAMID`
   - Example (API key): `./script/macos-build-tauri-release.sh --targets aarch64-apple-darwin,x86_64-apple-darwin --sign-identity "Developer ID Application: Your Org (TEAMID)" --apple-api-key ABCDE12345 --apple-api-issuer 00112233-4455-6677-8899-aabbccddeeff --apple-api-key-path ~/AuthKeys/AuthKey_ABCD12345.p8`
   - The script summarizes detected identities and whether notarization is enabled.
5. Sign bundles for the auto-updater (if enabling updates): `tauri signer sign --private-key key.pem bundle.dmg`
6. Generate update manifest: `./script/generate-update-manifest.sh`

## Known Limitations

- Auto-updater is disabled (`plugins.updater.active = false`) until CDN/signing pipeline is ready
- Placeholder Minisign public key needs replacement before enabling updates
- macOS 26+ requires `CI=true` to skip Finder AppleScript during DMG creation
- Some backend doctests fail (pre-existing, tracked separately)

## Important Notes

### Market Proxy Implementation

The streaming proxy (`market_stream.rs`) is critical for Next.js apps like mcp.so that use:
- Server-Side Rendering (SSR)
- React Server Components (RSC)
- Streaming HTML responses

**Do not** replace streaming logic with buffered HTML rewrites. This breaks hydration timing and causes blank pages or React errors.

### Custom URI Scheme

The `mcpmate://` scheme (`market_proxy.rs`) handles:
- Static assets that escape absolute paths (`/_next/`, `/static/`)
- Embedded market assets (`/scripts/market/*`)
- Fallback for non-streaming content

Both proxies work together: streaming for HTML, URI scheme for assets.
