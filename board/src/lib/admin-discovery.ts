import type { ClientConfigFileParse, ClientConfigFileState, TransportRuleData } from "./types";

export type AdminDiscoverySurface = "onboarding" | "extension";
type AdminDiscoveryPlatform = "macos" | "windows" | "linux";

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

function currentAdminDiscoveryPlatform(): AdminDiscoveryPlatform | undefined {
	const platform = typeof navigator === "undefined" ? "" : navigator.platform.toLowerCase();
	const userAgent = typeof navigator === "undefined" ? "" : navigator.userAgent.toLowerCase();
	const value = `${platform} ${userAgent}`;
	if (value.includes("mac")) return "macos";
	if (value.includes("win")) return "windows";
	if (value.includes("linux")) return "linux";
	return undefined;
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
		const candidate = adminDiscoveryClientToCandidate(item);
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

function detectionRules(client: Record<string, unknown>, platform = currentAdminDiscoveryPlatform()): unknown[] {
	const detectionRecord = recordValue(client.detection);
	if (platform && Array.isArray(detectionRecord[platform])) {
		return detectionRecord[platform];
	}
	return Object.values(detectionRecord).flatMap((value) => (Array.isArray(value) ? value : []));
}

function detectionRuleConfigPath(rule: unknown): string | undefined {
	const record = recordValue(rule);
	return compactString(record.config_path) ?? (record.method === "config_path" ? compactString(record.value) : undefined);
}

function configPathFromDiscoveryClient(client: Record<string, unknown>): string {
	for (const rule of detectionRules(client)) {
		const path = detectionRuleConfigPath(rule);
		if (path) return path;
	}
	return compactString(client.config_path) ?? "";
}

function adminConfigFileParse(file: Record<string, unknown>): {
	format: string;
	containerType: "standard" | "array";
	containerKeys: string[];
} {
	return {
		format: compactString(file.format) ?? "json",
		containerType:
			file.containerType === "array" || file.container_type === "array" ? "array" : "standard",
		containerKeys: stringArrayValue(file.containerKeys ?? file.container_keys),
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
		compactString(client.displayName) ??
		compactString(client.display_name) ??
		compactString(metadata.display_name) ??
		identifier;
	const configPath = options?.platform
		? configPathFromDiscoveryClient({ ...client, detection: { [options.platform]: detectionRules(client, options.platform) } })
		: configPathFromDiscoveryClient(client);
	const hasWritableConfig = Boolean(
		configPath && (config.kind === "has_config_file" || config.file || Object.keys(transports).length > 0),
	);
	const configFileChoice: ClientConfigFileState = hasWritableConfig ? "with_config_file" : "without_config_file";

	return {
		identifier,
		displayName,
		configFileChoice,
		configPath: configFileChoice === "with_config_file" ? configPath : "",
		configFileParseFormat: file.format,
		configFileParseContainerType: file.containerType,
		configFileParseContainerKeysText: file.containerKeys.join(", "),
		description: compactString(client.description) ?? compactString(metadata.description) ?? "",
		homepageUrl:
			compactString(client.homepageUrl) ?? compactString(client.homepage_url) ?? compactString(metadata.homepage_url) ?? "",
		docsUrl: compactString(client.docsUrl) ?? compactString(client.docs_url) ?? compactString(metadata.docs_url) ?? "",
		supportUrl:
			compactString(client.supportUrl) ?? compactString(client.support_url) ?? compactString(metadata.support_url) ?? "",
		logoUrl: compactString(client.logoUrl) ?? compactString(client.logo_url) ?? compactString(metadata.logo_url) ?? "",
		supportedTransports: Object.keys(transports),
		transports,
	};
}

export function adminDiscoveryClientToUpdatePayload(
	raw: unknown,
	options?: { configPath?: string; forceWithoutConfigFile?: boolean },
): AdminClientUpdatePayload {
	const candidate = adminDiscoveryClientToCandidate(raw);
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
