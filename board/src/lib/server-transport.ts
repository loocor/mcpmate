export type ClassifiedServerTransport =
	| "stdio"
	| "sse"
	| "streamable_http"
	| "http"
	| "unknown";

export type ServerInstallTransportKind = "stdio" | "sse" | "streamable_http";

/**
 * Classify a legacy/compat `server_type` string into a stable transport bucket.
 * Callers that already know the structured draft is unrecognized should not use
 * this alone for user-facing labels — prefer the draft over the projected type.
 */
export function classifyServerTransport(
	serverType?: string | null,
): ClassifiedServerTransport {
	const kind = (serverType ?? "").toLowerCase().trim();
	if (!kind) {
		return "unknown";
	}
	if (kind.includes("stdio") || kind.includes("process")) {
		return "stdio";
	}
	if (kind.includes("streamable")) {
		return "streamable_http";
	}
	if (kind === "sse" || kind.includes("sse")) {
		return "sse";
	}
	if (kind.includes("stream")) {
		return "streamable_http";
	}
	if (kind.includes("http") || kind.includes("rest")) {
		return "http";
	}
	return "unknown";
}

/** Map a legacy/compat server_type into an install/edit draft kind. */
export function inferServerInstallKind(
	serverType?: string | null,
): ServerInstallTransportKind {
	switch (classifyServerTransport(serverType)) {
		case "sse":
			return "sse";
		case "streamable_http":
		case "http":
			return "streamable_http";
		case "stdio":
		case "unknown":
			return "stdio";
	}
}

export function isRemoteHttpTransport(
	serverType?: string | null,
): boolean {
	const transport = classifyServerTransport(serverType);
	return (
		transport === "sse" ||
		transport === "streamable_http" ||
		transport === "http"
	);
}
