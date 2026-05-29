import type { ClientConfigFileParse, ClientConfigFileState, TransportRuleData } from "./types";

export type AdminDiscoverySurface = "onboarding" | "extension";
export type AdminDiscoveryPlatform = "macos" | "windows" | "linux";

const MAX_ADMIN_DISCOVERY_LIMIT = 50;
const MAX_ADMIN_DISCOVERY_RANDOM = 12;
const DEFAULT_ADMIN_DISCOVERY_BASE_URL = "https://public.mcp.umate.ai";
const CANONICAL_TRANSPORT_KEYS = ["streamable_http", "sse", "stdio"] as const;
const CONFIG_PARSE_FORMATS = ["json", "json5", "toml", "yaml"] as const;

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

export interface AdminDiscoveryDiagnostic {
	identifier?: string;
	reason: string;
}

export interface AdminDiscoveryClientCatalog {
	clients: AdminDiscoveryClientCandidate[];
	diagnostics: AdminDiscoveryDiagnostic[];
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

function optionalRecordValue(value: unknown): Record<string, unknown> | null {
	if (value === undefined) return {};
	if (value === null) return null;
	return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
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
	const catalog = await fetchAdminDiscoveryClientCatalog(options);
	if (catalog.diagnostics.length > 0) {
		console.warn("Skipped invalid Admin discovery clients.", catalog.diagnostics);
	}
	return catalog.clients;
}

export async function fetchAdminDiscoveryClientCatalog(options: AdminDiscoveryQuery): Promise<AdminDiscoveryClientCatalog> {
	const envelope = await fetchAdminDiscoveryEnvelope("/discovery/clients", options);
	const clients: AdminDiscoveryClientCandidate[] = [];
	const diagnostics: AdminDiscoveryDiagnostic[] = [];
	for (const item of discoveryItems(envelope, "clients")) {
		const candidate = adminDiscoveryClientToCandidate(item, { platform: options.platform });
		if (candidate) {
			clients.push(candidate);
		} else {
			diagnostics.push({
				identifier: adminDiscoveryClientIdentifier(item),
				reason: "Invalid Admin discovery client contract.",
			});
		}
	}
	return { clients, diagnostics };
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

function adminDiscoveryClientIdentifier(raw: unknown): string | undefined {
	const client = recordValue(raw);
	return compactString(client.identifier) ?? compactString(client.id) ?? compactString(client.name);
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
} | null {
	const format = compactString(file.format) ?? "json";
	if (!(CONFIG_PARSE_FORMATS as readonly string[]).includes(format)) return null;
	const container = recordValue(file.container);
	const containerType = compactString(container.type ?? file.containerType ?? file.container_type);
	if (containerType && !["standard", "object_map", "array"].includes(containerType)) return null;
	const containerKeys = stringArrayValue(container.keys ?? file.containerKeys ?? file.container_keys);
	if (containerKeys.length === 0) return null;
	return {
		format,
		containerType: containerType === "array" ? "array" : "standard",
		containerKeys,
	};
}

function isCanonicalTransportKey(value: string): boolean {
	return (CANONICAL_TRANSPORT_KEYS as readonly string[]).includes(value);
}

function hasCompactString(value: unknown): boolean {
	return typeof value === "string" && value.trim().length > 0;
}

function isOptionalCompactString(value: unknown): boolean {
	return value === undefined || value === null || hasCompactString(value);
}

function isOptionalBoolean(value: unknown): boolean {
	return value === undefined || value === null || typeof value === "boolean";
}

function isOptionalRecord(value: unknown): boolean {
	return value === undefined || value === null || (typeof value === "object" && value !== null && !Array.isArray(value));
}

function isValidTransportRule(key: string, rule: Record<string, unknown>): boolean {
	for (const field of Object.keys(rule)) {
		if (!isTransportRuleField(field)) return false;
	}
	if (Object.prototype.hasOwnProperty.call(rule, "requires_type_field")) return false;
	for (const field of ["command_field", "args_field", "env_field", "type_value", "url_field", "headers_field"]) {
		if (!isOptionalCompactString(rule[field])) return false;
	}
	if (!isOptionalBoolean(rule.include_type) || !isOptionalBoolean(rule.selected)) return false;
	if (!isOptionalRecord(rule.extra_fields)) return false;
	if (rule.include_type === true && !hasCompactString(rule.type_value)) return false;
	if (key === "stdio") return hasCompactString(rule.command_field);
	return hasCompactString(rule.url_field);
}

function isTransportRuleField(field: string): boolean {
	return [
		"template",
		"command_field",
		"args_field",
		"env_field",
		"include_type",
		"type_value",
		"url_field",
		"headers_field",
		"extra_fields",
		"selected",
	].includes(field);
}

function adminTransports(config: Record<string, unknown>): Record<string, TransportRuleData> | null {
	const transports = optionalRecordValue(config.transports);
	if (!transports) return null;
	const entries: Array<[string, TransportRuleData]> = [];

	for (const [key, value] of Object.entries(transports)) {
		if (!isCanonicalTransportKey(key)) return null;
		const rule = optionalRecordValue(value);
		if (!rule) return null;
		if (!isValidTransportRule(key, rule)) return null;
		entries.push([key, rule as TransportRuleData]);
	}

	return Object.fromEntries(entries);
}

export function adminDiscoveryClientToCandidate(
	raw: unknown,
	options?: { platform?: AdminDiscoveryPlatform },
): AdminDiscoveryClientCandidate | null {
	const client = recordValue(raw);
	const metadata = metadataRecord(client);
	const config = recordValue(client.config);
	const transports = adminTransports(config);
	if (!transports) return null;
	const identifier = compactString(client.identifier) ?? compactString(client.id) ?? compactString(client.name);
	if (!identifier) return null;
	const displayName =
		firstCompactString(client.displayName, client.display_name, metadata.display_name) ?? identifier;
	const fileRecord = recordValue(config.file);
	const configPath = configPathFromDiscoveryClient(fileRecord, options?.platform);
	const configKind = compactString(config.kind);
	const hasConfigFileKind = configKind === "file";
	const hasWritableConfig = Boolean(configPath && hasConfigFileKind);
	const file = hasWritableConfig ? adminConfigFileParse(fileRecord) : null;
	if (hasWritableConfig && !file) return null;
	const configFileChoice: ClientConfigFileState = hasWritableConfig ? "with_config_file" : "without_config_file";
	const links = recordValue(client.links);
	const icon = recordValue(client.icon);

	return {
		identifier,
		displayName,
		configFileChoice,
		configPath: configFileChoice === "with_config_file" ? configPath : "",
		configFileParseFormat: file?.format ?? "json",
		configFileParseContainerType: file?.containerType ?? "standard",
		configFileParseContainerKeysText: file?.containerKeys.join(", ") ?? "",
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
	const supportedTransports = stringArrayValue(candidate.supportedTransports);
	if (supportedTransports.some((transport) => !isCanonicalTransportKey(transport))) return null;
	const transports = adminTransports({ transports: candidate.transports });
	if (!transports) return null;
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
		supportedTransports,
		transports,
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
		source_clients: ["MCPMate"],
		source_client_ids: [],
		import_config: importConfig,
	};
}
