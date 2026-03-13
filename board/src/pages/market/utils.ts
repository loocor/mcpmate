import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { getOfficialMeta } from "../../lib/registry";
import type { RegistryServerEntry } from "../../lib/types";
import type { RemoteOption } from "./types";

export function useDebouncedValue<T>(value: T, delay = 300) {
	const [debounced, setDebounced] = useState(value);
	useEffect(() => {
		const handle = window.setTimeout(() => setDebounced(value), delay);
		return () => window.clearTimeout(handle);
	}, [value, delay]);
	return debounced;
}

export function hasPreviewableOption(
	server: RegistryServerEntry | null,
): boolean {
	if (!server) return false;
	const hasRemote = (server.remotes ?? []).some(
		(remote) =>
			Boolean(normalizeRemoteKind(remote.type)) && Boolean(remote.url),
	);
	const hasPackage = (server.packages ?? []).some((pkg) =>
		Boolean(normalizeRemoteKind(pkg.transport?.type)),
	);
	return hasRemote || hasPackage;
}

export function formatServerName(raw: string) {
	if (!raw) return "Unknown";
	const segments = raw.split("/").filter(Boolean);
	const target = segments[segments.length - 1] ?? raw;
	return target
		.replace(/[-_]+/g, " ")
		.split(" ")
		.filter(Boolean)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ");
}

export function getRegistryIdentity(server: RegistryServerEntry): string {
	const official = getOfficialMeta(server);
	if (official?.serverId) {
		return official.serverId;
	}
	return `${server.name}@${server.version}`;
}

// Market mode helper functions
export function normalizeRemoteKind(value?: string | null): string | null {
	if (!value) return null;
	const lower = value.toLowerCase();
	if (lower === "sse") return "sse";
	if (lower === "stdio") return "stdio";
	if (
		lower === "streamable-http" ||
		lower === "streamable_http" ||
		lower === "streamablehttp" ||
		lower === "http" ||
		lower === "http_stream" ||
		lower === "http-stream" ||
		lower === "httpstream"
	)
		return "streamable_http";
	return null;
}

export function getRemoteTypeLabel(type: string): string {
	switch (type.toLowerCase()) {
		case "sse":
			return "SSE";
		case "streamable_http":
		case "streamable-http":
			return "Streamable HTTP";
		case "stdio":
			return "Stdio";
		default:
			return type;
	}
}

export function slugifyForConfig(value: string): string {
	const slug = value
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-+|-+$/g, "");
	return slug || "registry-server";
}

export function buildDraftFromRemoteOption(
	option: RemoteOption,
	fallbackName: string,
): ServerInstallDraft {
	const descriptors =
		option.source === "package"
			? (option.envVars ?? [])
			: (option.headers ?? []);

	const env: Record<string, string> = {};
	descriptors.forEach((descriptor) => {
		env[descriptor.name] = "";
	});

	if (option.source === "package") {
		// For packages, we need to build command and args
		const identifier = option.packageIdentifier || "";
		const registryType =
			(
				option.packageMeta as { registryType?: string }
			)?.registryType?.toLowerCase() || "";

		let command = "";
		let args: string[] = [];

		if (
			registryType === "pip" ||
			registryType === "pypi" ||
			registryType === "python"
		) {
			command = "uvx";
			args = [identifier];
		} else if (registryType === "bun" || registryType === "bunx") {
			command = "bunx";
			args = ["-y", identifier];
		} else {
			command = "npx";
			args = ["-y", identifier];
		}

		return {
			name: fallbackName,
			kind: option.kind as "stdio" | "sse" | "streamable_http",
			url: undefined,
			command,
			args,
			env,
		};
	}

	return {
		name: fallbackName,
		kind: option.kind as "stdio" | "sse" | "streamable_http",
		url: option.url || "",
		command: "",
		args: [],
		env,
	};
}

// Import React hooks for the debounced value function
import { useEffect, useState } from "react";
