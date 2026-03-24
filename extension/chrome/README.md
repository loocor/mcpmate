# MCPMate Server Import (Chrome)

The MCPMate Server Import extension detects MCP configuration snippets on web pages and sends them to the MCPMate desktop app in one click.

- **Product**: MCPMate
- **Website**: [https://mcp.umate.ai](https://mcp.umate.ai)
- **Desktop deep link**: `mcpmate://import/server?p=<base64url(JSON)>`

The payload JSON matches desktop handling in `deep_link.rs`:

```json
{ "text": "...", "format": "json|toml|...", "source": "https://..." }
```

## Features

- Detects likely MCP snippets that contain `mcpServers` in `pre`/`code` blocks.
- Injects a compact bar on each block: **MCPMate logo** (inline SVG, same mark as `icons/logo.svg`) by default—avoids page CSP blocking `chrome-extension://` images; on hover/focus it expands to **Add to MCPMate** (English UI; i18n later).
- Sends source URL along with snippet text for auditability.
- Extension toolbar uses an **inline copy** of the logo in `content.js` (`MCPMATE_LOGO_SVG`); keep it in sync with `icons/logo.svg` / `website/public/logo.svg` when the brand mark changes. **`manifest.json` icons** use PNGs (`icons/icon-{16,32,48,128}.png`) because Chrome’s extension UI does not reliably show SVG there; regenerate those PNGs from `desktop/tauri/src-tauri/icons/icon.png` when the app icon changes (e.g. `sips -z <size> <size> icon.png --out icons/icon-<size>.png` on macOS).

## Install (unpacked)

1. Install/build MCPMate desktop first (registers `mcpmate://` URL scheme).
2. Open Chrome/Edge → **Extensions** → enable **Developer mode**.
3. Click **Load unpacked** and select `extension/chrome/`.
4. Visit a page with MCP config snippets and click inside the snippet area.
5. Click **Add to MCPMate** to open desktop import flow.

## Product links

- Homepage: [https://mcp.umate.ai](https://mcp.umate.ai)
- Repository: [https://github.com/Loocor/MCPMate](https://github.com/Loocor/MCPMate)

## Notes and limits

- After updating the extension, reload it on `chrome://extensions` and refresh the tab (GitHub loads README after first paint; the script re-scans on DOM changes and scroll).
- Very large snippets may exceed URL limits; extension warns above ~48k characters.
- If MCPMate desktop is not installed, browser may show no handler for `mcpmate:`.
- Chrome Web Store expects PNGs at fixed sizes; this folder already ships `icons/icon-*.png` derived from the desktop app icon.
