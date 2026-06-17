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
