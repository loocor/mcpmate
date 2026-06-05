import { useEffect, useState } from "react";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { getCanonicalRegistryServerId } from "../../lib/registry";
import type { RegistryPackageArgument, RegistryServerEntry } from "../../lib/types";
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
			Boolean(normalizeRemoteKind(remote.type)) &&
			Boolean(remote.url) &&
			!hasRegistryVariables(remote.variables),
	);
	const hasPackage = (server.packages ?? []).some((pkg) =>
		normalizeRemoteKind(pkg.transport?.type) === "stdio" &&
		Boolean(pkg.identifier?.trim()) &&
		isSupportedRegistryPackageType(pkg.registryType) &&
		!hasUnresolvedRequiredRegistryArguments(pkg.runtimeArguments) &&
		!hasUnresolvedRequiredRegistryArguments(pkg.packageArguments),
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
	return getCanonicalRegistryServerId(server);
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

export function hasRegistryVariables(
	variables?: Record<string, unknown> | null,
): boolean {
	if (!variables || typeof variables !== "object") return false;
	return Object.keys(variables).length > 0;
}

export function getRemoteTypeLabel(type: string): string {
	switch (type.toLowerCase()) {
		case "sse":
			return "SSE (Legacy)";
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

function compactString(value?: string | null): string | null {
	if (typeof value !== "string") return null;
	const trimmed = value.trim();
	return trimmed ? trimmed : null;
}

export function isSupportedRegistryPackageType(value?: string | null): boolean {
	const registryType = compactString(value)?.toLowerCase();
	return registryType === "npm" || registryType === "pypi";
}

export function isKnownUnsupportedRegistryPackageType(
	value?: string | null,
): boolean {
	const registryType = compactString(value)?.toLowerCase();
	return (
		registryType === "nuget" ||
		registryType === "oci" ||
		registryType === "mcpb"
	);
}

export function hasUnsupportedRegistryPackageOption(
	server: RegistryServerEntry | null,
): boolean {
	if (!server) return false;
	return (server.packages ?? []).some((pkg) => {
		const kind = normalizeRemoteKind(pkg.transport?.type);
		return (
			kind === "stdio" &&
			Boolean(pkg.identifier?.trim()) &&
			isKnownUnsupportedRegistryPackageType(pkg.registryType)
		);
	});
}

function registryArgumentValue(argument: RegistryPackageArgument): string | null {
	return compactString(argument.value) ?? compactString(argument.default);
}

export function hasUnresolvedRequiredRegistryArguments(
	argumentsList?: RegistryPackageArgument[] | null,
): boolean {
	return (argumentsList ?? []).some(
		(argument) => Boolean(argument.isRequired) && !registryArgumentValue(argument),
	);
}

function resolveRegistryArguments(
	argumentsList?: RegistryPackageArgument[] | null,
): string[] {
	const resolved: string[] = [];
	for (const argument of argumentsList ?? []) {
		const argumentType = compactString(argument.type)?.toLowerCase() ?? "positional";
		const value = registryArgumentValue(argument);
		if (argument.isRequired && !value) {
			throw new Error("Required package argument is missing a value");
		}
		if (argumentType === "named") {
			const name = compactString(argument.name);
			if (!name) {
				throw new Error("Named package argument requires a name");
			}
			resolved.push(name);
			if (value) {
				resolved.push(value);
			}
			continue;
		}
		if (value) {
			resolved.push(value);
		}
	}
	return resolved;
}

function packageSpecifier(identifier: string, version?: string | null): string {
	const normalizedVersion = compactString(version);
	return normalizedVersion ? `${identifier}@${normalizedVersion}` : identifier;
}

function commandForRegistryPackage(registryType: string): string {
	switch (registryType) {
		case "npm":
			return "npx";
		case "pypi":
			return "uvx";
		default:
			throw new Error(`Unsupported package registry type '${registryType}'`);
	}
}

function argsForRegistryPackage(
	registryType: string,
	identifier: string,
	version: string | null | undefined,
	runtimeArguments?: RegistryPackageArgument[] | null,
	packageArguments?: RegistryPackageArgument[] | null,
): string[] {
	const args = resolveRegistryArguments(runtimeArguments);
	if (registryType === "npm") {
		args.unshift("-y");
	}
	args.push(packageSpecifier(identifier, version));
	args.push(...resolveRegistryArguments(packageArguments));
	return args;
}

export function buildDraftFromRemoteOption(
	option: RemoteOption,
	fallbackName: string,
): ServerInstallDraft {
	const descriptors =
		option.source === "package"
			? (option.envVars ?? [])
			: (option.headers ?? []);

	const inputValues: Record<string, string> = {};
	descriptors.forEach((descriptor) => {
		inputValues[descriptor.name] =
			compactString(descriptor.value) ??
			compactString(descriptor.default) ??
			"";
	});

	if (option.source === "package") {
		if (option.kind !== "stdio") {
			throw new Error("Package transport must be stdio");
		}
		const identifier = option.packageIdentifier || "";
		if (!identifier) {
			throw new Error("Package identifier is required");
		}
		const packageMeta = option.packageMeta as {
			registryType?: string | null;
			version?: string | null;
			packageArguments?: RegistryPackageArgument[] | null;
			runtimeArguments?: RegistryPackageArgument[] | null;
		};
		const registryType = compactString(packageMeta.registryType)?.toLowerCase();
		if (!registryType) {
			throw new Error("Package registry type is required");
		}
		if (!isSupportedRegistryPackageType(registryType)) {
			throw new Error(`Unsupported package registry type '${registryType}'`);
		}

		return {
			name: fallbackName,
			kind: option.kind as ServerInstallDraft["kind"],
			url: undefined,
			command: commandForRegistryPackage(registryType),
			args: argsForRegistryPackage(
				registryType,
				identifier,
				packageMeta.version,
				packageMeta.runtimeArguments,
				packageMeta.packageArguments,
			),
			env: inputValues,
		};
	}

	return {
		name: fallbackName,
		kind: option.kind as ServerInstallDraft["kind"],
		url: option.url || "",
		command: "",
		args: [],
		env: {},
		headers: inputValues,
	};
}
