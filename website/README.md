MCPMate Website

Local development
- Install deps: `bun install` (or `npm install`)
- Run dev server: `bun run dev` (or `npm run dev`)
- Build: `bun run build` (or `npm run build`)

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
