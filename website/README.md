MCPMate Website

The `website/` project contains:
- Public marketing pages
- Embedded product documentation (`src/docs/`)
- Release changelog pages used by `mcp.umate.ai`

Local development
- Install deps: `bun install` (or `npm install`)
- Run dev server: `bun run dev` (or `npm run dev`)
- Build: `bun run build` (or `npm run build`)

Documentation authoring
- Docs routes and sidebar: `src/docs/nav.ts`
- English pages: `src/docs/pages/en/`
- Chinese pages: `src/docs/pages/zh/`
- Changelog data: `src/docs/changelog/en.json`, `src/docs/changelog/zh.json`
- Keep EN/ZH docs aligned when adding or updating product capabilities

Current product highlights to keep documented
- Core server + UI separated operation mode (backend decoupled from dashboard shell)
- Integrated desktop mode (Tauri bundles backend + board)
- Audit logs and operational traceability workflows
- Streamable HTTP aligned transport behavior and legacy SSE input normalization in install/import flows
- Upstream OAuth support for Streamable HTTP servers (prepare + callback-based authorization from install flow)

Preview download configuration
- Copy `.env.example` to `.env`
- Set macOS dmg URLs: `VITE_MAC_ARM64_URL`, `VITE_MAC_X64_URL`.
  - Use the current Preview tag release assets:
    - `https://github.com/mcpmate/mcpmate/releases/download/preview/mcpmate_preview_aarch64.dmg`
    - `https://github.com/mcpmate/mcpmate/releases/download/preview/mcpmate_preview_x64.dmg`
- Pause public downloads by setting `VITE_PREVIEW_SUSPENDED=true` (buttons become disabled and the note explains how to request access).
- Optionally set checksums: `VITE_MAC_ARM64_SHA256`, `VITE_MAC_X64_SHA256` (update when you replace the DMGs)
- Optionally set `VITE_PREVIEW_VERSION`, `VITE_PREVIEW_EXPIRES_AT` (default 2025-11-01)
- Optionally set `VITE_DOCS_URL`, `VITE_INSTALL_URL`, `VITE_WIN_URL`, `VITE_LINUX_URL`

Notes
- Waitlist section is removed; navigation includes "Download Preview" and a Download section with macOS variants.
- If URLs are not set, buttons render as "Coming Soon".
