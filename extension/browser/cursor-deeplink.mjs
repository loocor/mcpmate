/** Split merged stdio command strings when `args` are absent (cursor.directory quirk).
 *  Keep behavior aligned with `board/src/lib/install-normalizer.ts`. */
export function splitMergedStdioCommand(command, args) {
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

function normalizeCursorTransport(value, fallback) {
	const token = typeof value === "string" ? value.trim().toLowerCase() : "";
	switch (token) {
		case "sse":
		case "server-sent-events":
			return "sse";
		case "streamable-http":
		case "streamable_http":
		case "streamablehttp":
		case "http":
		case "http_stream":
		case "http-stream":
		case "httpstream":
			return "streamable_http";
		default:
			return fallback;
	}
}

function transportFromUrl(url) {
	try {
		const parsed = new URL(url);
		const segments = parsed.pathname
			.split("/")
			.map((segment) => segment.trim().toLowerCase())
			.filter(Boolean);
		if (segments.includes("mcp")) {
			return "streamable_http";
		}
		if (segments.includes("sse")) {
			return "sse";
		}
	} catch {
		return null;
	}
	return null;
}

/** @param {string} href Cursor MCP install deep link URL. */
export function parseCursorMcpInstallLink(href) {
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
			entry.type = normalizeCursorTransport(
				cursorConfig.transport,
				transportFromUrl(cursorConfig.url) ?? "streamable_http",
			);
		}
		if (cursorConfig.command) {
			const { command, args } = splitMergedStdioCommand(
				cursorConfig.command,
				cursorArgs,
			);
			entry.type = "stdio";
			if (command) entry.command = command;
			if (args?.length) entry.args = args;
		} else if (cursorArgs?.length) {
			entry.type = "stdio";
			entry.args = cursorArgs;
		}
		if (cursorConfig.env && typeof cursorConfig.env === "object") {
			entry.env = cursorConfig.env;
		}
		if (!entry.url && !entry.command) return null;

		return JSON.stringify({ mcpServers: { [name]: entry } });
	} catch {
		return null;
	}
}
