/** Classic-script build of cursor-deeplink.mjs for content script injection. Keep in sync. */
(function (global) {
	/** Split merged stdio command strings when `args` are absent (cursor.directory quirk).
	 *  Keep behavior aligned with `board/src/lib/install-normalizer.ts`. */
	function splitMergedStdioCommand(command, args) {
		const trimmed = typeof command === "string" ? command.trim() : "";
		if (!trimmed || (Array.isArray(args) && args.length > 0)) {
			return { command: trimmed || undefined, args };
		}
		const parts = trimmed.split(/\s+/).filter(Boolean);
		if (parts.length <= 1) {
			return { command: trimmed, args };
		}
		return { command: parts[0], args: parts.slice(1) };
	}

	/** @param {string} href Cursor MCP install deep link URL. */
	function parseCursorMcpInstallLink(href) {
		try {
			const url = new URL(href);
			const name = url.searchParams.get("name") || "unknown";
			const configB64 = url.searchParams.get("config");
			if (!configB64) return null;

			const cursorConfig = JSON.parse(atob(configB64));
			const cursorArgs = Array.isArray(cursorConfig.args)
				? cursorConfig.args
				: undefined;
			const entry = {};

			if (cursorConfig.url) {
				entry.url = cursorConfig.url;
			}
			if (cursorConfig.command) {
				const { command, args } = splitMergedStdioCommand(
					cursorConfig.command,
					cursorArgs,
				);
				if (command) entry.command = command;
				if (args?.length) entry.args = args;
			} else if (cursorArgs?.length) {
				entry.args = cursorArgs;
			}
			if (cursorConfig.env && typeof cursorConfig.env === "object") {
				entry.env = cursorConfig.env;
			}
			if (cursorConfig.transport && typeof cursorConfig.transport === "string") {
				entry.type = cursorConfig.transport;
			}
			if (!entry.url && !entry.command) return null;

			return JSON.stringify({ mcpServers: { [name]: entry } });
		} catch {
			return null;
		}
	}

	global.__MCPMATE_CURSOR_DEEPLINK__ = { parseCursorMcpInstallLink };
})(globalThis);
