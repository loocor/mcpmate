# MCPMate Browser Extension

The MCPMate browser extension shows curated MCP portals, servers, and clients inside the browser toolbar popup, and it can still hand detected MCP snippets off to the local MCPMate desktop app.

- **Product**: MCPMate
- **Website**: [https://mcp.umate.ai](https://mcp.umate.ai)
- **Admin API origin**: `https://public.mcp.umate.ai`
- **Current discovery mode**: `account`

The import handoff payload JSON matches desktop handling in `deep_link.rs`:

```json
{ "text": "...", "format": "json|toml|...", "source": "https://..." }
```

## Features

- Popup discovery tabs for **Portals**, **Servers**, and **Clients**.
- Discovery data comes from the published Public Worker discovery APIs backed by MCPMate Admin catalog data.
- Portals, servers, and clients request the `extension` surface with paginated discovery
  queries and load more entries as the popup list is scrolled.
- Use the popup refresh button to reload the active discovery panel; touch
  devices can also pull down from the top of the panel.
- Discovery responses are cached locally for one hour to avoid repeated popup fetches.
- Popup reopen uses session snapshots (scroll position and loaded pages) plus stale-while-revalidate rendering for faster first paint.
- Language defaults to the browser language on first open (`zh` → 中文, `ja` → 日本語, otherwise English). Theme and language preferences live inside the toolbar popup settings panel.
- The footer community button shows Feishu for Chinese and Discord for other languages.
- Popup styling mirrors the shadcn Dashboard visual language with lightweight static HTML/CSS/JS, avoiding a React bundle inside the extension.
- Optional icon metadata is supported when Admin catalog entries provide it.
- Uses `config.js` as the extension deployment config. Update `adminApiOrigin` there if the Admin API origin changes.
- Run `bun extension/browser/scripts/write-build-info.mjs` before packaging to refresh `build-info.js` with the current build date.
- The snippet-to-desktop import path remains enabled through `content.js` and `mcpmate://import/server`.
- What remains disabled is telemetry-style import submission to Admin APIs. The extension does not upload import events or usage analytics to Admin in the current phase.
- `manifest.json` icons use PNGs (`icons/icon-{16,32,48,128}.png`) because Chromium extension UIs do not reliably show SVG there. The popup also switches to `icons/icon-dark-{16,32,48,128}.png` for dark mode so the toolbar mark remains legible.
- **GitHub MCP page integration**: On `github.com/mcp` pages, the extension automatically injects an "Install in MCPMate" option into the Install dropdown menus for each MCP server. This provides a seamless way to import servers directly from GitHub's MCP catalog into MCPMate desktop.

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

## Popup dev preview (Cursor / local browser)

Run the popup UI in a normal browser tab (including Cursor's built-in Simple Browser) without loading the unpacked extension:

```bash
cd extension/browser
bun run dev
```

Then open:

- Mock catalog (offline, default): [http://127.0.0.1:5179/dev/popup.html?mode=mock](http://127.0.0.1:5179/dev/popup.html?mode=mock)
- Live discovery API: [http://127.0.0.1:5179/dev/popup.html?mode=account](http://127.0.0.1:5179/dev/popup.html?mode=account)

The dev page loads `dev/chrome-shim.js` to stub `chrome.runtime`, `chrome.storage`, and `chrome.tabs`, then reuses the production `popup.js` and `popup.css`. Edit those files and refresh the tab to iterate on popup layout and discovery UI.

In Cursor, use **Simple Browser: Show** (`Simple Browser: Show` command) and paste the URL above.

Limits: toolbar icon theming and true extension-popup sizing are approximated in a framed preview. Content-script features (`content.js`, page import buttons) still require an unpacked extension in Chrome/Edge.

## Product links

- Homepage: [https://mcp.umate.ai](https://mcp.umate.ai)
- Repository: [https://github.com/Loocor/MCPMate](https://github.com/Loocor/MCPMate)

## Notes and limits

- Reload the extension from your browser's extensions page after local changes.
- `mock` mode remains available for local UI checks only. Production config uses `account` and does not silently fall back to mock data when the public discovery API is unavailable.
- The paginated discovery UI uses the existing popup, the `storage` permission
  for local settings/cache, and the published public discovery API origin. It does not add
  background network activity, remote code execution, analytics submission, or
  new host/background permissions.
- Mouse and trackpad gestures are not intercepted for pull-to-refresh, so text
  selection in catalog cards stays native.
- Chromium-based extension stores expect PNGs at fixed sizes; this folder already ships `icons/icon-*.png` derived from the desktop app icon.
- Discovery data is owned by the separate MCPMate Admin service, not by the marketing website.
