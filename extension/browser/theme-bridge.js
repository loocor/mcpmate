/**
 * MCPMate theme bridge content script.
 * Detects OS color scheme and writes to chrome.storage so the background
 * service worker can update the toolbar icon — even when the worker is dormant.
 */
(function () {
	if (typeof chrome === "undefined" || !chrome.storage?.local) return;

	const mq = window.matchMedia("(prefers-color-scheme: dark)");

	function sync(isDark) {
		try {
			chrome.storage.local.set({ "mcpmate.toolbarTheme": isDark ? "dark" : "light" });
		} catch {}
	}

	sync(mq.matches);
	mq.addEventListener("change", (e) => sync(e.matches));
})();
