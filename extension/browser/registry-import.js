/** Classic-script build of registry-import.mjs for content script use. Keep in sync. */
(function (global) {
	const TRANSPORT_RANK = {
		stdio: 1,
		sse: 2,
		streamable_http: 3,
	};

	function normalizeRegistryTransport(value, fallback = null) {
		const token = typeof value === "string" ? value.trim().toLowerCase() : "";
		if (!token) {
			return fallback;
		}
		switch (token) {
			case "stdio":
				return "stdio";
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
				return null;
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

	function chooseHigherRankedCandidate(candidates, name, entry, kind) {
		const current = candidates.get(name);
		if (!current || TRANSPORT_RANK[kind] > TRANSPORT_RANK[current.kind]) {
			candidates.set(name, { entry, kind });
		}
	}

	function packageEntry(pkg) {
		const entry = { type: "stdio" };

		if (pkg.registryType === "npm") {
			entry.command = "npx";
			entry.args = ["-y", pkg.identifier];
		} else if (pkg.registryType === "pypi") {
			entry.command = pkg.runtimeHint || "uvx";
			entry.args = [pkg.identifier];
		} else if (pkg.runtimeHint) {
			entry.command = pkg.runtimeHint;
			entry.args = [pkg.identifier];
		} else {
			entry.command = "npx";
			entry.args = ["-y", pkg.identifier];
		}

		if (
			Array.isArray(pkg.environmentVariables) &&
			pkg.environmentVariables.length > 0
		) {
			entry.env = {};
			for (const envVar of pkg.environmentVariables) {
				entry.env[envVar.name] = "";
			}
		}

		return entry;
	}

	function convertRegistryToMcpMate(server) {
		const name = server.name || "unknown";
		const candidates = new Map();

		for (const pkg of Array.isArray(server.packages) ? server.packages : []) {
			const serverName = pkg.identifier || name;
			chooseHigherRankedCandidate(
				candidates,
				serverName,
				packageEntry(pkg),
				"stdio",
			);
		}

		for (const remote of Array.isArray(server.remotes) ? server.remotes : []) {
			if (!remote?.url) continue;
			const kind = normalizeRegistryTransport(
				remote.type,
				transportFromUrl(remote.url) ?? "streamable_http",
			);
			if (!kind || kind === "stdio") continue;
			const serverName = remote.identifier || name;
			chooseHigherRankedCandidate(
				candidates,
				serverName,
				{ type: kind, url: remote.url },
				kind,
			);
		}

		if (candidates.size === 0) {
			return null;
		}

		const mcpServers = {};
		for (const [serverName, candidate] of candidates) {
			mcpServers[serverName] = candidate.entry;
		}

		return JSON.stringify({ mcpServers });
	}

	global.__MCPMATE_REGISTRY_IMPORT__ = {
		convertRegistryToMcpMate,
		normalizeRegistryTransport,
	};
})(globalThis);
