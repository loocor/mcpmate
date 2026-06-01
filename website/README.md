# MCPMate Website

Public marketing site and embedded product documentation for [MCPMate](https://mcp.umate.ai).

**Positioning (aligned with landing copy):** your progressive MCP management partner — import MCP once, start simple, then add profiles, per-client tools, and setup modes as your workflow grows.

## What ships here

- Marketing homepage (`src/pages/Homepage.tsx`): Hero, compatible clients, features, how-it-works, setup modes, FAQ, Quick Start download
- Product documentation (`src/docs/`)
- Release changelog pages served on `mcp.umate.ai`

Marketing strings: `src/i18n/en.ts`, `src/i18n/zh.ts`, `src/i18n/ja.ts` (update EN and ZH together; sync JA after primary locales stabilize). See [AGENTS.md](./AGENTS.md) for terminology alignment with Board.

## Local development

```bash
bun install   # or npm install
bun run dev   # or npm run dev
bun run build
```

## Documentation authoring

- Routes and sidebar: `src/docs/nav.ts`
- Pages: `src/docs/pages/en/`, `src/docs/pages/zh/`, `src/docs/pages/ja/`
- Changelog: `src/docs/changelog/{en,zh,ja}.json`
- Keep EN/ZH docs aligned when product capabilities change, then sync JA

## Preview download configuration

Copy `.env.example` to `.env`.

| Variable | Purpose |
| -------- | ------- |
| `VITE_MAC_ARM64_URL`, `VITE_MAC_X64_URL` | macOS DMG URLs (e.g. Preview tag assets on GitHub Releases) |
| `VITE_PREVIEW_SUSPENDED` | Set `true` to disable public download buttons |
| `VITE_MAC_ARM64_SHA256`, `VITE_MAC_X64_SHA256` | Optional checksums when DMGs change |
| `VITE_PREVIEW_VERSION`, `VITE_PREVIEW_EXPIRES_AT` | Optional preview metadata |
| `VITE_DOCS_URL`, `VITE_INSTALL_URL`, `VITE_WIN_URL`, `VITE_LINUX_URL` | Optional outbound links |

Example Preview DMGs:

- `https://github.com/mcpmate/mcpmate/releases/download/preview/mcpmate_preview_aarch64.dmg`
- `https://github.com/mcpmate/mcpmate/releases/download/preview/mcpmate_preview_x64.dmg`

If URLs are unset, download buttons show **Coming Soon**.

## Extension discovery

Extension discovery data is owned by the MCPMate backend service. The website may link to extension docs and privacy pages; it does not host discovery APIs or collect extension import metadata.

## Notes

- Navigation highlights **Quick Start** and **Get MCPMate** (waitlist flow removed).
- Compatible client logos may load from admin discovery with a local fallback list (`src/data/compatible-clients-fallback.ts`).
