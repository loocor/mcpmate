/**
 * MCPMate background service worker.
 * Switches toolbar icon based on OS color scheme.
 * Reads theme from chrome.storage (written by theme-bridge.js and popup.js).
 */

function iconPaths(suffix) {
  return {
    16: `icons/icon${suffix}-16.png`,
    32: `icons/icon${suffix}-32.png`,
    48: `icons/icon${suffix}-48.png`,
    128: `icons/icon${suffix}-128.png`,
  };
}

function applyIconForTheme(isDark) {
  const suffix = isDark ? "-dark" : "";
  chrome.action.setIcon({ path: iconPaths(suffix) });
}

function openFallbackHandoffPage(url) {
  const parsed = new URL(url);
  const extensionOrigin = new URL(chrome.runtime.getURL("/")).origin;
  if (parsed.origin !== extensionOrigin || parsed.pathname !== "/handoff.html") {
    throw new Error("Refusing to open unsupported extension page");
  }
  chrome.tabs.create({ url });
}

// Sync icon on startup from persisted theme
chrome.storage.local.get("mcpmate.toolbarTheme", (result) => {
  applyIconForTheme(result["mcpmate.toolbarTheme"] === "dark");
});

// React to theme changes from theme-bridge.js or popup.js
chrome.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes["mcpmate.toolbarTheme"]) {
    applyIconForTheme(changes["mcpmate.toolbarTheme"].newValue === "dark");
  }
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type !== "mcpmate.openImportFallback") {
    return false;
  }

  try {
    openFallbackHandoffPage(message.url);
    sendResponse({ ok: true });
  } catch (error) {
    sendResponse({
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
  return false;
});
