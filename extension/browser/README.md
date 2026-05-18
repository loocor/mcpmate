# MCPMate Browser Extension

The MCPMate browser extension shows curated MCP portals, servers, and clients inside the browser toolbar popup, and it can still hand detected MCP snippets off to the local MCPMate desktop app.

- **Product**: MCPMate
- **Website**: [https://mcp.umate.ai](https://mcp.umate.ai)
- **Account API origin**: `https://auth.mcp.umate.ai`
- **Current discovery mode**: `mock`

The import handoff payload JSON matches desktop handling in `deep_link.rs`:

```json
{ "text": "...", "format": "json|toml|...", "source": "https://..." }
```

## Features

- Popup discovery tabs for **Portals**, **Servers**, and **Clients**.
- Discovery data currently comes from packaged mock JSON while the Account discovery APIs are not published yet. Switch `config.js` to `discoveryMode: "account"` when those APIs are ready.
- Optional language and theme preferences live inside the toolbar popup settings panel.
- Popup styling mirrors the shadcn Dashboard visual language with lightweight static HTML/CSS/JS, avoiding a React bundle inside the extension.
- Optional icon metadata is supported when Account catalog entries provide it.
- Uses `config.js` as the extension deployment config. Update `accountApiOrigin` there if the account API origin changes.
- The snippet-to-desktop import path remains enabled through `content.js` and `mcpmate://import/server`.
- What remains disabled is telemetry-style import submission to Account APIs. The extension does not upload import events or usage analytics to Account in the current phase.
- `manifest.json` icons use PNGs (`icons/icon-{16,32,48,128}.png`) because Chromium extension UIs do not reliably show SVG there. The popup also switches to `icons/icon-dark-{16,32,48,128}.png` for dark mode so the toolbar mark remains legible.

## Install (store)

Status: Available on Chromium-based browser extension stores, including Chrome Web Store and Microsoft Edge Add-ons.

- Chrome Web Store: https://chromewebstore.google.com/detail/mcpmate-server-import/jngogcgclencgillbmeeimkcjjnobidf
- Microsoft Edge Add-ons: https://microsoftedge.microsoft.com/addons/detail/mcpmate-server-import/nbpdfanhajcjghegoocfmjkpaklidckn

## Install (unpacked)

1. Install the extension in a supported Chromium-based browser such as Chrome or Edge.
2. Open the browser's extensions page and enable **Developer mode**.
3. Click **Load unpacked** and select `extension/browser/`.
4. Open the MCPMate toolbar popup.
5. Use the popup settings panel to choose the extension language and theme.
6. Browse discovery sections backed by the configured discovery source.
7. Visit any page with an MCP server snippet to use the in-page **Add to MCPMate** handoff.

## Product links

- Homepage: [https://mcp.umate.ai](https://mcp.umate.ai)
- Repository: [https://github.com/Loocor/MCPMate](https://github.com/Loocor/MCPMate)

## Notes and limits

- Reload the extension from your browser's extensions page after local changes.
- `mock` mode is an intentional substitute for missing upstream discovery APIs, not a silent fallback. Change `config.js` when switching environments.
- Chromium-based extension stores expect PNGs at fixed sizes; this folder already ships `icons/icon-*.png` derived from the desktop app icon.
- Discovery data is owned by the separate MCPMate account service, not by the marketing website.
