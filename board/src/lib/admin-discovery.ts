import type { ClientConfigFileParse, ClientConfigFileState, TransportRuleData } from "./types";

export type AdminDiscoverySurface = "onboarding" | "extension";
export type AdminDiscoveryPlatform = "macos" | "windows" | "linux";

const MAX_ADMIN_DISCOVERY_LIMIT = 50;
const MAX_ADMIN_DISCOVERY_RANDOM = 12;
const DEFAULT_ADMIN_DISCOVERY_BASE_URL = "https://public.mcp.umate.ai";

export const ADMIN_DISCOVERY_BASE_URL = trimTrailingSlash(
	(typeof import.meta !== "undefined" &&
		import.meta.env?.VITE_ADMIN_API_BASE_URL &&
		String(import.meta.env.VITE_ADMIN_API_BASE_URL).trim()) ||
		DEFAULT_ADMIN_DISCOVERY_BASE_URL,
);

export interface AdminDiscoveryQuery {
	surface?: AdminDiscoverySurface;
	limit?: number;
	offset?: number;
	random?: number;
	platform?: AdminDiscoveryPlatform;
}

export interface AdminDiscoveryClientCandidate {
	identifier: string;
	displayName: string;
	configFileChoice: ClientConfigFileState;
	configPath: string;
	configFileParseFormat: string;
	configFileParseContainerType: "standard" | "array";
	configFileParseContainerKeysText: string;
	description: string;
	homepageUrl: string;
	docsUrl: string;
	supportUrl: string;
	logoUrl: string;
	supportedTransports: string[];
	transports: Record<string, TransportRuleData>;
}

export interface AdminClientUpdatePayload {
	identifier: string;
	display_name: string;
	config_file_state: ClientConfigFileState;
	config_path?: string;
	description?: string;
	homepage_url?: string;
	docs_url?: string;
	support_url?: string;
	logo_url?: string;
	config_file_parse?: ClientConfigFileParse;
	clear_config_file_parse: boolean;
	transports?: Record<string, TransportRuleData>;
	clear_transports: boolean;
}

export interface AdminDiscoveryServerCandidate {
	key: string;
	name: string;
	kind: string;
	command?: string;
	args: string[];
	env: Record<string, string>;
	url?: string;
	source_clients: string[];
	source_client_ids: string[];
	import_config: Record<string, unknown>;
}

function trimTrailingSlash(url: string): string {
	return url.replace(/\/+$/, "");
}

function compactString(value: unknown): string | undefined {
	return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function firstCompactString(...values: unknown[]): string | undefined {
	return values.find((value): value is string => Boolean(compactString(value)))?.trim();
}

function recordValue(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function stringArrayValue(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function stringRecordValue(value: unknown): Record<string, string> {
	return Object.fromEntries(
		Object.entries(recordValue(value)).filter((entry): entry is [string, string] => typeof entry[1] === "string"),
	);
}

function cappedCount(value: unknown, max: number): number | undefined {
	if (typeof value !== "number" || !Number.isFinite(value)) return undefined;
	return Math.max(0, Math.min(max, Math.round(value)));
}

export function buildAdminDiscoveryUrl(
	path: "/discovery/clients" | "/discovery/servers",
	options: AdminDiscoveryQuery = {},
	baseUrl = ADMIN_DISCOVERY_BASE_URL,
): string {
	const url = new URL(path, `${trimTrailingSlash(baseUrl)}/`);
	if (options.surface) {
		url.searchParams.set("surface", options.surface);
	}
	const random = cappedCount(options.random, MAX_ADMIN_DISCOVERY_RANDOM);
	if (typeof random === "number") {
		url.searchParams.set("random", String(random));
		return url.toString();
	}
	const limit = cappedCount(options.limit, MAX_ADMIN_DISCOVERY_LIMIT);
	if (typeof limit === "number") {
		url.searchParams.set("limit", String(limit));
	}
	if (typeof options.offset === "number" && Number.isFinite(options.offset)) {
		url.searchParams.set("offset", String(Math.max(0, Math.round(options.offset))));
	}
	return url.toString();
}

async function fetchAdminDiscoveryEnvelope(path: "/discovery/clients" | "/discovery/servers", options: AdminDiscoveryQuery) {
	const response = await fetch(buildAdminDiscoveryUrl(path, options), {
		credentials: "omit",
	});
	if (!response.ok) {
		throw new Error(`Admin discovery request failed with HTTP ${response.status}`);
	}
	return response.json() as Promise<unknown>;
}

function discoveryItems(envelope: unknown, key: "clients" | "servers"): unknown[] {
	const record = recordValue(envelope);
	const items = record[key];
	if (!Array.isArray(items)) {
		throw new Error(`Admin discovery response is missing '${key}' array`);
	}
	return items;
}

export async function fetchAdminDiscoveryClients(options: AdminDiscoveryQuery): Promise<AdminDiscoveryClientCandidate[]> {
	const envelope = await fetchAdminDiscoveryEnvelope("/discovery/clients", options);
	return discoveryItems(envelope, "clients").flatMap((item) => {
		const candidate = adminDiscoveryClientToCandidate(item, { platform: options.platform });
		return candidate ? [candidate] : [];
	});
}

export async function fetchAdminDiscoveryServers(options: AdminDiscoveryQuery): Promise<AdminDiscoveryServerCandidate[]> {
	const envelope = await fetchAdminDiscoveryEnvelope("/discovery/servers", options);
	return discoveryItems(envelope, "servers").flatMap((item) => {
		const candidate = adminDiscoveryServerToOnboardingCandidate(item);
		return candidate ? [candidate] : [];
	});
}

function metadataRecord(client: Record<string, unknown>): Record<string, unknown> {
	return recordValue(client.metadata);
}

function configPathFromDiscoveryClient(
	file: Record<string, unknown>,
	platform?: AdminDiscoveryPlatform,
): string {
	if (!platform) return "";
	return compactString(recordValue(file.paths)[platform]) ?? "";
}

function adminConfigFileParse(file: Record<string, unknown>): {
	format: string;
	containerType: "standard" | "array";
	containerKeys: string[];
} {
	const container = recordValue(file.container);
	return {
		format: compactString(file.format) ?? "json",
		containerType:
			container.type === "array" || file.containerType === "array" || file.container_type === "array"
				? "array"
				: "standard",
		containerKeys: stringArrayValue(container.keys ?? file.containerKeys ?? file.container_keys),
	};
}

function adminTransports(config: Record<string, unknown>): Record<string, TransportRuleData> {
	const transports = recordValue(config.transports);
	return Object.fromEntries(
		Object.entries(transports).filter((entry): entry is [string, TransportRuleData] => {
			const value = entry[1];
			return Boolean(value && typeof value === "object" && !Array.isArray(value));
		}),
	);
}

export function adminDiscoveryClientToCandidate(
	raw: unknown,
	options?: { platform?: AdminDiscoveryPlatform },
): AdminDiscoveryClientCandidate | null {
	const client = recordValue(raw);
	const metadata = metadataRecord(client);
	const config = recordValue(client.config);
	const file = adminConfigFileParse(recordValue(config.file));
	const transports = adminTransports(config);
	const identifier = compactString(client.identifier) ?? compactString(client.id) ?? compactString(client.name);
	if (!identifier) return null;
	const displayName =
		firstCompactString(client.displayName, client.display_name, metadata.display_name) ?? identifier;
	const fileRecord = recordValue(config.file);
	const configPath = configPathFromDiscoveryClient(fileRecord, options?.platform);
	const configKind = compactString(config.kind);
	const hasConfigFileKind = configKind === "file";
	const hasWritableConfig = Boolean(configPath && hasConfigFileKind);
	const configFileChoice: ClientConfigFileState = hasWritableConfig ? "with_config_file" : "without_config_file";
	const links = recordValue(client.links);
	const icon = recordValue(client.icon);

	return {
		identifier,
		displayName,
		configFileChoice,
		configPath: configFileChoice === "with_config_file" ? configPath : "",
		configFileParseFormat: file.format,
		configFileParseContainerType: file.containerType,
		configFileParseContainerKeysText: file.containerKeys.join(", "),
		description: compactString(client.description) ?? compactString(metadata.description) ?? "",
		homepageUrl: firstCompactString(links.homepage, client.homepageUrl, client.homepage_url, metadata.homepage_url) ?? "",
		docsUrl: firstCompactString(links.docs, client.docsUrl, client.docs_url, metadata.docs_url) ?? "",
		supportUrl: firstCompactString(links.support, client.supportUrl, client.support_url, metadata.support_url) ?? "",
		logoUrl: firstCompactString(icon.url, client.logoUrl, client.logo_url, metadata.logo_url) ?? "",
		supportedTransports: Object.keys(transports),
		transports,
	};
}

function resolvedAdminDiscoveryClientCandidate(raw: unknown): AdminDiscoveryClientCandidate | null {
	const candidate = recordValue(raw);
	const identifier = compactString(candidate.identifier);
	const displayName = compactString(candidate.displayName);
	if (!identifier || !displayName) return null;
	if (candidate.configFileChoice !== "with_config_file" && candidate.configFileChoice !== "without_config_file") {
		return null;
	}
	return {
		identifier,
		displayName,
		configFileChoice: candidate.configFileChoice,
		configPath: compactString(candidate.configPath) ?? "",
		configFileParseFormat: compactString(candidate.configFileParseFormat) ?? "json",
		configFileParseContainerType: candidate.configFileParseContainerType === "array" ? "array" : "standard",
		configFileParseContainerKeysText: compactString(candidate.configFileParseContainerKeysText) ?? "",
		description: compactString(candidate.description) ?? "",
		homepageUrl: compactString(candidate.homepageUrl) ?? "",
		docsUrl: compactString(candidate.docsUrl) ?? "",
		supportUrl: compactString(candidate.supportUrl) ?? "",
		logoUrl: compactString(candidate.logoUrl) ?? "",
		supportedTransports: stringArrayValue(candidate.supportedTransports),
		transports: recordValue(candidate.transports) as Record<string, TransportRuleData>,
	};
}

export function adminDiscoveryClientToUpdatePayload(
	raw: unknown,
	options?: { configPath?: string; forceWithoutConfigFile?: boolean },
): AdminClientUpdatePayload {
	const candidate = resolvedAdminDiscoveryClientCandidate(raw) ?? adminDiscoveryClientToCandidate(raw);
	if (!candidate) {
		throw new Error("Admin discovery client is missing a usable identifier.");
	}
	const configPath = options?.configPath?.trim() ?? candidate.configPath.trim();
	const hasConfigFile = !options?.forceWithoutConfigFile && candidate.configFileChoice === "with_config_file" && Boolean(configPath);
	const configFileParse: ClientConfigFileParse = {
		format: candidate.configFileParseFormat,
		container_type: candidate.configFileParseContainerType,
		container_keys: candidate.configFileParseContainerKeysText
			.split(",")
			.map((key) => key.trim())
			.filter(Boolean),
	};
	return {
		identifier: candidate.identifier,
		display_name: candidate.displayName,
		config_file_state: hasConfigFile ? "with_config_file" : "without_config_file",
		config_path: hasConfigFile ? configPath : undefined,
		description: candidate.description || undefined,
		homepage_url: candidate.homepageUrl || undefined,
		docs_url: candidate.docsUrl || undefined,
		support_url: candidate.supportUrl || undefined,
		logo_url: candidate.logoUrl || undefined,
		config_file_parse: hasConfigFile ? configFileParse : undefined,
		clear_config_file_parse: !hasConfigFile,
		transports: hasConfigFile ? candidate.transports : undefined,
		clear_transports: !hasConfigFile,
	};
}

export function adminDiscoveryServerToOnboardingCandidate(raw: unknown): AdminDiscoveryServerCandidate | null {
	const server = recordValue(raw);
	const official = recordValue(server.official);
	const curated = recordValue(server.curated);
	const runtime = recordValue(server.runtime);
	const installConfig = recordValue(runtime.install_config ?? server.install_config);
	const registryId =
		compactString(server.id) ?? compactString(official.name) ?? compactString(official.title) ?? "admin-server";
	const displayName = compactString(curated.displayName) ?? compactString(curated.display_name) ?? compactString(official.title) ?? registryId;
	const kind = compactString(installConfig.server_type) ?? compactString(installConfig.type) ?? "stdio";
	const command = compactString(installConfig.command);
	const url = compactString(installConfig.url);
	if (kind === "stdio" && !command) return null;
	if (kind !== "stdio" && !url) return null;
	const args = stringArrayValue(installConfig.args);
	const env = stringRecordValue(installConfig.env);
	const headers = recordValue(installConfig.headers);
	const importConfig: Record<string, unknown> = {
		type: kind,
		registry_server_id: registryId,
	};

	if (command) importConfig.command = command;
	if (args.length > 0) importConfig.args = args;
	if (Object.keys(env).length > 0) importConfig.env = env;
	if (url) importConfig.url = url;
	if (Object.keys(headers).length > 0) importConfig.headers = headers;

	return {
		key: `admin:${registryId}`,
		name: displayName,
		kind,
		command,
		args,
		env,
		url,
		source_clients: ["MCPMate Admin"],
		source_client_ids: [],
		import_config: importConfig,
	};
}
