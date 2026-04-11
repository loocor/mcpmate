import type {
	ApiResponse,
	AuditEventDetailsResp,
	AuditListResp,
	AuditPolicyResp,
	AuditPolicySetReq,
	BatchOperationResponse,
	CapabilitiesKeysResponse,
	CapabilitiesMetricsStats,
	CapabilitiesStatsResponse,
	CapabilitiesStorageStats,
	ClearCacheResponse,
	ClientBackupActionResp,
	ClientBackupListResp,
	ClientBackupPolicyResp,
	ClientBackupPolicySetReq,
	ClientCapabilityConfigReq,
	ClientCapabilityConfigResp,
	ClientCheckResp,
	ClientConfigImportData,
	ClientConfigImportReq,
	ClientConfigResp,
	ClientConfigRestoreReq,
	ClientConfigUpdateReq,
	ClientConfigUpdateResp,
	ClientManageAction,
	ClientManageResp,
	ClientRecordLifecycleResp,
	ClientRecordReviewReq,
	ConfigPreset,
	ConfigSuit,
	ConfigSuitListResponse,
	ConfigSuitPrompt,
	ConfigSuitPromptsResponse,
	ConfigSuitResource,
	ConfigSuitResourceTemplate,
	ConfigSuitResourceTemplatesResponse,
	ConfigSuitResourcesResponse,
	ConfigSuitServer,
	ConfigSuitServersResponse,
	ConfigSuitTool,
	ConfigSuitToolsResponse,
	CreateConfigSuitRequest,
	InspectorSessionCloseData,
	InspectorSessionOpenData,
	InspectorToolCallCancelData,
	InspectorToolCallStartData,
	InstallResponse,
	InstallRuntimeRequest,
	InstanceDetail,
	InstanceDetailsResp,
	InstanceHealth,
	InstanceHealthResp,
	InstanceListResp,
	InstanceSummary,
	MCPConfig,
	MCPServerConfig,
	OperationResponseResp,
	RegistryMetaPayload,
	RegistryRepositoryInfo,
	RuntimeCacheResponse,
	RuntimeStatusResponse,
	ServerCapabilityResp,
	ServerCapabilitySummary,
	ServerDetail,
	ServerDetailsResp,
	ServerIcon,
	ServerListResp,
	ServerListResponse,
	ServerMetaInfo,
	OAuthCallbackRequest,
	OAuthConfigRequest,
	OAuthInitiateResponse,
	OAuthStatus,
	ServerSummary,
	ServersImportData,
	SkippedServer,
	DefaultClientPolicyUpdateReq,
	SystemMetrics,
	SystemStatus,
	CapabilityTokenLedgerResponse,
	TokenEstimateResponse,
	ToolDetail,
	UpdateConfigSuitRequest,
} from "./types";
import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";

// Base API configuration
// Prefer VITE_API_BASE_URL; otherwise infer from runtime context with sane fallbacks.
// For desktop (Tauri), allow runtime override so Settings can change ports without full reload.
const API_BASE_OVERRIDE_KEY = "mcpmate.api_base_override";
export const API_BASE_CHANGED_EVENT = "mcpmate:api-base-changed";

const isDesktopShellEnvironment = (): boolean => {
	if (typeof window === "undefined") {
		return false;
	}
	const w = window as unknown as {
		__TAURI__?: unknown;
		__TAURI_IPC__?: unknown;
		__TAURI_INTERNALS__?: unknown;
		__TAURI_METADATA__?: unknown;
		__MCPMATE_IS_TAURI__?: unknown;
	};
	const protocol = window.location?.protocol?.toLowerCase() ?? "";
	const ua = typeof navigator !== "undefined" ? navigator.userAgent || "" : "";
	return (
		protocol === "tauri:" ||
		protocol === "app:" ||
		protocol === "file:" ||
		w.__TAURI__ !== undefined ||
		w.__TAURI_IPC__ !== undefined ||
		w.__TAURI_INTERNALS__ !== undefined ||
		w.__TAURI_METADATA__ !== undefined ||
		w.__MCPMATE_IS_TAURI__ !== undefined ||
		ua.includes("Tauri") ||
		ua.includes("MCPMate")
	);
};

const resolveApiBaseUrl = (): string => {
	const envBase =
		typeof import.meta !== "undefined" ? import.meta.env?.VITE_API_BASE_URL : undefined;

	if (typeof envBase === "string" && envBase.trim().length > 0) {
		return envBase.trim();
	}

	try {
		if (isDesktopShellEnvironment() && typeof window !== "undefined" && window.localStorage) {
			const override = window.localStorage.getItem(API_BASE_OVERRIDE_KEY);
			if (override && override.trim().length > 0) {
				return override.trim();
			}
		}
	} catch {
		// ignore storage access issues
	}

	if (typeof window !== "undefined" && typeof window.location !== "undefined") {
		const protocol = window.location.protocol.toLowerCase();

		// Tauri / desktop shells use a custom protocol (e.g. tauri://localhost)
		if (protocol === "tauri:" || protocol === "app:" || protocol === "file:") {
			// default desktop port (may be overridden at runtime via localStorage)
			return "http://127.0.0.1:8080";
		}

		// In web dev/prod environments stick to same-origin relative requests so Vite/Tauri proxies work.
		return "";
	}

	// Server-side fall back to local proxy port
	return "http://127.0.0.1:8080";
};

// Mutable API base URL with runtime setter for desktop shells
export let API_BASE_URL = resolveApiBaseUrl();
export function setApiBaseUrl(newBase: string | null | undefined) {
	const candidate = (newBase ?? "").trim();
	const shouldPersistOverride = isDesktopShellEnvironment();
	if (candidate.length > 0) {
		API_BASE_URL = candidate;
		try {
			if (shouldPersistOverride) {
				window.localStorage?.setItem(API_BASE_OVERRIDE_KEY, candidate);
			} else {
				window.localStorage?.removeItem(API_BASE_OVERRIDE_KEY);
			}
			window.dispatchEvent(
				new CustomEvent(API_BASE_CHANGED_EVENT, { detail: { apiBaseUrl: candidate } }),
			);
		} catch {
			// ignore persistence errors
		}
		return;
	}
	// Clear override and recompute
	try {
		window.localStorage?.removeItem(API_BASE_OVERRIDE_KEY);
	} catch {
		// LocalStorage access may fail in restricted environments
	}
	API_BASE_URL = resolveApiBaseUrl();
	try {
		window.dispatchEvent(
			new CustomEvent(API_BASE_CHANGED_EVENT, {
				detail: { apiBaseUrl: API_BASE_URL },
			}),
		);
	} catch {
		// ignore event dispatch errors
	}
}

const resolveWebSocketUrl = (): string => {
	if (typeof window === "undefined") {
		return "";
	}

	const baseCandidate = API_BASE_URL || window.location.origin;

	try {
		const parsed = new URL(baseCandidate);
		const protocol = parsed.protocol === "https:" ? "wss:" : "ws:";
		const host = parsed.host || parsed.hostname;
		return `${protocol}//${host}/ws`;
	} catch (error) {
		console.warn(
			"Failed to derive WebSocket URL from API base, falling back to window location",
			error,
		);
		const fallbackProtocol =
			window.location.protocol === "https:" ? "wss:" : "ws:";
		return `${fallbackProtocol}//${window.location.host}/ws`;
	}
};

// Utility types
interface ApiWrapper<T> {
	success: boolean;
	data?: T;
	error?: unknown;
}

interface SuitTool {
	id: string;
	name?: string;
	tool_name?: string;
	server_name: string;
	description?: string;
	enabled?: boolean;
	is_enabled?: boolean;
}

/** Tools/resources/prompts list payloads from server capability endpoints */
type CapabilityListResult = {
	items: unknown[];
	meta?: unknown;
	state?: string;
};

/** Row shape from `/api/mcp/profile/list` */
interface ProfileApiRow {
	id: string;
	name: string;
	description?: string | null;
	profile_type?: string;
	multi_select?: boolean;
	priority?: number;
	is_active?: boolean;
	is_default?: boolean;
	role?: string | null;
	allowed_operations?: string[];
}

function profileRowToConfigSuit(p: ProfileApiRow): ConfigSuit {
	return {
		id: p.id,
		name: p.name,
		description: p.description ?? undefined,
		suit_type: p.profile_type ?? "",
		multi_select: p.multi_select ?? false,
		priority: p.priority ?? 0,
		is_active: p.is_active ?? false,
		is_default: p.is_default ?? false,
		role: p.role ?? undefined,
		allowed_operations: p.allowed_operations ?? [],
	};
}

function asOptionalString(v: unknown): string | undefined {
	return typeof v === "string" ? v : undefined;
}

function asStringOrNull(v: unknown): string | null | undefined {
	if (v === null) return null;
	return typeof v === "string" ? v : undefined;
}

/** Single server entry for import fallback JSON */
interface McpImportServerEntry {
	type: string;
	command?: string;
	args?: string[];
	env?: Record<string, string>;
	url?: string;
}

interface NotificationData {
	[key: string]: unknown;
	event?: string;
}

// Enhanced error handling utility
function createApiError(response: Response, parsed?: unknown): Error {
	if (parsed && typeof parsed === "object") {
		const obj = parsed as Record<string, unknown>;
		if (typeof obj.message === "string") {
			return new Error(obj.message);
		}
		if (obj.error && typeof obj.error === "object") {
			const errorObj = obj.error as { message?: unknown };
			if (typeof errorObj.message === "string") {
				return new Error(errorObj.message);
			}
		}
	}
	return new Error(`API Error: ${response.status} ${response.statusText}`);
}

/** True when fetchApi failed with an HTTP 404 (e.g. older backend without a route). */
export function isApiNotFoundError(error: unknown): boolean {
	if (!(error instanceof Error)) {
		return false;
	}
	return /\b404\b/.test(error.message);
}

const toTrimmedString = (value: unknown): string | undefined => {
	if (typeof value !== "string") return undefined;
	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : undefined;
};

const normalizeServerIcon = (icon: unknown): ServerIcon | null => {
	if (!icon || typeof icon !== "object") return null;
	const o = icon as Record<string, unknown>;
	const src =
		toTrimmedString(o.src) ||
		toTrimmedString(o.url) ||
		toTrimmedString(o.href);
	if (!src) return null;
	const mimeType =
		toTrimmedString(o.mime_type) ||
		toTrimmedString(o.mimeType) ||
		undefined;
	const sizes =
		toTrimmedString(o.sizes) || toTrimmedString(o.size) || undefined;
	const normalized: ServerIcon = { src };
	if (mimeType) normalized.mimeType = mimeType;
	if (sizes) normalized.sizes = sizes;
	return normalized;
};

const normalizeServerIconList = (icons: unknown): ServerIcon[] => {
	if (!icons) return [];
	const array = Array.isArray(icons) ? icons : [icons];
	return array
		.map((icon) => normalizeServerIcon(icon))
		.filter((icon): icon is ServerIcon => Boolean(icon));
};

const normalizeRepositoryInfo = (repo: unknown): RegistryRepositoryInfo | null => {
	if (!repo) return null;
	if (typeof repo === "string") {
		const url = toTrimmedString(repo);
		return url ? { url } : null;
	}
	if (typeof repo !== "object") return null;
	const r = repo as Record<string, unknown>;
	const info: RegistryRepositoryInfo = {};
	const url = toTrimmedString(r.url);
	const source = toTrimmedString(r.source);
	const subfolder = toTrimmedString(r.subfolder);
	const id = toTrimmedString(r.id);
	if (url) info.url = url;
	if (source) info.source = source;
	if (subfolder) info.subfolder = subfolder;
	if (id) info.id = id;
	return Object.keys(info).length > 0 ? info : null;
};

export const normalizeServerMeta = (meta: unknown): ServerMetaInfo | undefined => {
	if (!meta || typeof meta !== "object") return undefined;
	const m = meta as Record<string, unknown>;
	const normalized: ServerMetaInfo = {};
	const description = toTrimmedString(m.description);
	const version = toTrimmedString(m.version);
	const websiteUrl =
		toTrimmedString(m.websiteUrl) ||
		toTrimmedString(m.website_url) ||
		toTrimmedString(m.website);
	const repository = normalizeRepositoryInfo(m.repository);
	const icons = normalizeServerIconList(m.icons);

	if (description) normalized.description = description;
	if (version) normalized.version = version;
	if (websiteUrl) normalized.websiteUrl = websiteUrl;
	if (repository) normalized.repository = repository;
	if (icons.length) normalized.icons = icons;

	if (m._meta && typeof m._meta === "object") {
		normalized._meta = m._meta as RegistryMetaPayload;
	}

	if (m.extras && typeof m.extras === "object") {
		normalized.extras = m.extras as Record<string, unknown>;
	}

	const extrasObj =
		m.extras && typeof m.extras === "object"
			? (m.extras as Record<string, unknown>)
			: undefined;
	const legacyFromExtras =
		extrasObj && "legacy" in extrasObj ? extrasObj.legacy : undefined;
	const legacy = m.legacy ?? legacyFromExtras;
	if (!normalized.extras && legacy && typeof legacy === "object") {
		normalized.extras = { legacy };
	}

	return Object.keys(normalized).length > 0 ? normalized : undefined;
};

const serializeRepositoryForApi = (
	repo: RegistryRepositoryInfo | null | undefined,
): Record<string, string> | undefined => {
	if (!repo || typeof repo !== "object") return undefined;
	const payload: Record<string, string> = {};
	const assign = (key: string, value?: string | null) => {
		const trimmed = toTrimmedString(value);
		if (trimmed) payload[key] = trimmed;
	};
	assign("url", repo.url ?? undefined);
	assign("source", repo.source ?? undefined);
	assign("subfolder", repo.subfolder ?? undefined);
	assign("id", repo.id ?? undefined);
	return Object.keys(payload).length > 0 ? payload : undefined;
};

export const serializeMetaForApi = (
	meta: ServerMetaInfo | null | undefined,
): Record<string, unknown> | undefined => {
	if (!meta || typeof meta !== "object") return undefined;
	const payload: Record<string, unknown> = {};

	const assignString = (key: string, value: string | null | undefined) => {
		const trimmed = toTrimmedString(value);
		if (trimmed) {
			payload[key] = trimmed;
		}
	};

	assignString("description", meta.description ?? undefined);
	assignString("version", meta.version ?? undefined);
	assignString("websiteUrl", meta.websiteUrl ?? undefined);

	const repositoryPayload = serializeRepositoryForApi(meta.repository);
	if (repositoryPayload) {
		payload.repository = repositoryPayload;
	}

	if (meta._meta && typeof meta._meta === "object") {
		payload._meta = meta._meta;
	}

	if (meta.extras && typeof meta.extras === "object") {
		payload.extras = meta.extras;
	}

	if (Array.isArray(meta.icons) && meta.icons.length > 0) {
		payload.icons = meta.icons.map((icon) => {
			const normalized: Record<string, string> = { src: icon.src };
			if (icon.mimeType) normalized.mimeType = icon.mimeType;
			if (icon.sizes) normalized.sizes = icon.sizes;
			return normalized;
		});
	}

	return Object.keys(payload).length > 0 ? payload : undefined;
};

const toBoolean = (value: unknown): boolean => {
	if (typeof value === "boolean") return value;
	if (typeof value === "number") return value !== 0;
	if (typeof value === "string") {
		const normalized = value.trim().toLowerCase();
		if (!normalized) return false;
		return ["true", "yes", "1", "y", "on"].includes(normalized);
	}
	return false;
};

const toCount = (value: unknown): number => {
	if (typeof value === "number" && Number.isFinite(value)) {
		return Math.max(0, Math.round(value));
	}
	if (typeof value === "string") {
		const parsed = Number(value);
		if (Number.isFinite(parsed)) {
			return Math.max(0, Math.round(parsed));
		}
	}
	return 0;
};

const normalizeCapabilitySummary = (
	capability: unknown,
): ServerCapabilitySummary | undefined => {
	if (!capability || typeof capability !== "object") return undefined;

	const source = capability as Record<string, unknown>;
	return {
		supports_tools: toBoolean(
			source.supports_tools ?? source.supportsTools ?? source.tools_supported,
		),
		supports_prompts: toBoolean(
			source.supports_prompts ??
				source.supportsPrompts ??
				source.prompts_supported,
		),
		supports_resources: toBoolean(
			source.supports_resources ??
				source.supportsResources ??
				source.resources_supported,
		),
		tools_count: toCount(
			source.tools_count ?? source.toolsCount ?? source.tools,
		),
		prompts_count: toCount(
			source.prompts_count ?? source.promptsCount ?? source.prompts,
		),
		resources_count: toCount(
			source.resources_count ?? source.resourcesCount ?? source.resources,
		),
		resource_templates_count: toCount(
			source.resource_templates_count ??
				source.resourceTemplatesCount ??
				source.templates,
		),
	};
};

const uniqBySrc = (icons: ServerIcon[]): ServerIcon[] => {
	const seen = new Set<string>();
	const result: ServerIcon[] = [];
	for (const icon of icons) {
		if (!seen.has(icon.src)) {
			seen.add(icon.src);
			result.push(icon);
		}
	}
	return result;
};

const enrichServerRecord = <T extends Record<string, unknown>>(server: T) => {
	const base: Record<string, unknown> = { ...server };
	const meta = normalizeServerMeta(server.meta);
	const directIcons = normalizeServerIconList(server.icons);
	const combinedIcons = uniqBySrc([...(meta?.icons ?? []), ...directIcons]);
	const capability = normalizeCapabilitySummary(
		server.capability ?? server.capabilities,
	);

	if (meta || combinedIcons.length) {
		base.meta = {
			...(meta ?? {}),
			...(combinedIcons.length ? { icons: combinedIcons } : {}),
		} as ServerMetaInfo;
	} else {
		delete base.meta;
	}

	if (combinedIcons.length) {
		base.icons = combinedIcons;
	} else {
		delete base.icons;
	}

	if (capability) {
		base.capability = capability;
		base.capabilities = capability;
	} else {
		delete base.capability;
		delete base.capabilities;
	}

	return base as T & {
		meta?: ServerMetaInfo;
		icons?: ServerIcon[];
		capability?: ServerCapabilitySummary;
		capabilities?: ServerCapabilitySummary;
	};
};

// Core API request function
async function fetchApi<T>(endpoint: string, options?: RequestInit): Promise<T> {
	const isRelative = !API_BASE_URL;
	const url = isRelative ? endpoint : `${API_BASE_URL}${endpoint}`;
	const headers =
		options?.headers instanceof Headers
			? options.headers
			: new Headers(options?.headers ?? {});
	// Avoid forcing JSON header on GET requests to keep caching friendly.
	if (!headers.has("content-type") && options?.body) {
		headers.set("content-type", "application/json");
	}
	try {
		const requestInit: RequestInit = {
			credentials: "include",
			...options,
		};
		requestInit.headers = headers;
		const response = await fetch(url, requestInit);

		if (!response.ok) {
			const errorText = await response.text();
			let parsed: unknown;
			try {
				parsed = JSON.parse(errorText);
				console.error(`API Error (${response.status}):`, parsed);
			} catch {
				console.error(`API Error (${response.status}):`, errorText);
			}
			throw createApiError(response, parsed);
		}

		if (response.status === 204) {
			return undefined as T;
		}

		const text = await response.text();
		if (!text) {
			return undefined as T;
		}
		return JSON.parse(text) as T;
	} catch (error) {
		console.error(`API request failed for ${endpoint}:`, error);
		throw error;
	}
}

// Utility function to extract data from wrapped API responses
function extractApiData<T>(response: ApiWrapper<T>): T {
    if (response.success && response.data) {
        return response.data;
    }
    // Prefer detailed backend message when available, but also preserve non-fatal warnings if any were returned.
    // This function remains focused on data extraction; pages now read warning arrays on success paths.
    throw new Error(
        response.error ? String(response.error) : "API request failed",
    );
}

// Utility function for batch operations
async function executeBatchOperation(
	ids: string[],
	operation: (id: string) => Promise<unknown>,
): Promise<BatchOperationResponse> {
	const results = await Promise.allSettled(ids.map(operation));
	const successful = results.filter((r) => r.status === "fulfilled").length;

	return {
		success_count: successful,
		successful_ids: ids.slice(0, successful),
		failed_ids: Object.fromEntries(
			ids.slice(successful).map((id) => [id, "Batch operation failed"]),
		),
	};
}

// Server Management API
export const serversApi = {
	getAll: async (): Promise<ServerListResponse> => {
		try {
			const resp = await fetchApi<ServerListResp>("/api/mcp/servers/list");
			const rawServers = Array.isArray(resp?.data?.servers)
				? resp.data.servers
				: [];
			const servers = rawServers.map((server): ServerSummary => {
				const rec = server as unknown as Record<string, unknown>;
				const enhanced = enrichServerRecord(rec);
				const er = enhanced as Record<string, unknown>;
				const registryServerId =
					(er.registry_server_id as string | null | undefined) ??
					(er.registryServerId as string | null | undefined) ??
					null;
				const serverType =
					toTrimmedString(er.server_type as string | undefined) ??
					toTrimmedString(er.serverType as string | undefined) ??
					toTrimmedString(er.kind as string | undefined);
				return {
					...enhanced,
					server_type: serverType,
					registry_server_id: registryServerId,
					auth_mode: asOptionalString(er.auth_mode) ?? null,
					oauth_status: asOptionalString(er.oauth_status) ?? null,
				} as ServerSummary;
			});
			return { servers };
		} catch (error) {
			console.error("Failed to fetch servers:", error);
			return { servers: [] };
		}
	},

	getOAuthStatus: async (id: string): Promise<OAuthStatus | null> => {
		try {
			const q = new URLSearchParams({ id });
			const resp = await fetchApi<ApiWrapper<OAuthStatus>>(
				`/api/mcp/servers/oauth/status?${q}`,
			);
			return resp?.data ?? null;
		} catch (error) {
			console.error("Failed to fetch OAuth status:", error);
			return null;
		}
	},

	prepareOAuth: async (
		id: string,
		payload: { redirect_uri: string; scopes?: string },
	): Promise<OAuthStatus> => {
		const resp = await fetchApi<ApiWrapper<OAuthStatus>>(
			"/api/mcp/servers/oauth/prepare",
			{
				method: "POST",
				body: JSON.stringify({ server_id: id, ...payload }),
			},
		);
		return extractApiData(resp);
	},

	saveOAuthConfig: async (
		id: string,
		config: Omit<OAuthConfigRequest, 'server_id'>,
	): Promise<OAuthStatus> => {
		const resp = await fetchApi<ApiWrapper<OAuthStatus>>(
			"/api/mcp/servers/oauth/config",
			{
			method: "POST",
			body: JSON.stringify({ server_id: id, ...config }),
		},
		);
		return extractApiData(resp);
	},

	initiateOAuth: async (id: string): Promise<OAuthInitiateResponse> => {
		const resp = await fetchApi<ApiWrapper<OAuthInitiateResponse>>(
			"/api/mcp/servers/oauth/initiate",
			{
			method: "POST",
			body: JSON.stringify({ server_id: id }),
		},
		);
		return extractApiData(resp);
	},

	revokeOAuth: async (id: string): Promise<OAuthStatus> => {
		const resp = await fetchApi<ApiWrapper<OAuthStatus>>(
			"/api/mcp/servers/oauth/revoke",
			{
			method: "POST",
			body: JSON.stringify({ server_id: id }),
		},
		);
		return extractApiData(resp);
	},

	handleOAuthCallback: async (
		payload: OAuthCallbackRequest,
	): Promise<OAuthStatus> => {
		const resp = await fetchApi<ApiWrapper<OAuthStatus>>(
			"/api/mcp/servers/oauth/callback",
			{
			method: "POST",
			body: JSON.stringify(payload),
		},
		);
		return extractApiData(resp);
	},

	getServer: async (id: string): Promise<ServerDetail> => {
		try {
			const q = new URLSearchParams({ id });
			const resp = await fetchApi<ServerDetailsResp>(
				`/api/mcp/servers/details?${q}`,
			);
			const data = (resp?.data ?? {}) as Record<string, unknown>;
			const enhanced = enrichServerRecord(data);
			const detailRecord = enhanced as Record<string, unknown>;
			const enabledValue =
				typeof enhanced?.enabled === "boolean"
					? enhanced.enabled
					: typeof enhanced?.globally_enabled === "boolean"
						? enhanced.globally_enabled
						: undefined;

			const instances = Array.isArray(enhanced?.instances)
				? (enhanced.instances as InstanceSummary[])
				: [];

			const metaRecord =
				enhanced.meta && typeof enhanced.meta === "object"
					? (enhanced.meta as Record<string, unknown>)
					: undefined;
			const rawStatus = (
				enhanced.status ??
				enhanced.state ??
				enhanced.runtime_status ??
				metaRecord?.state ??
				""
			)
				.toString()
				.toLowerCase();

			const statusMap: Record<string, string> = {
				ready: "ready",
				running: "ready",
				connected: "ready",
				busy: "busy",
				active: "ready",
				healthy: "ready",
				idle: "idle",
				unload: enabledValue ? "idle" : "disabled",
				unloaded: enabledValue ? "idle" : "disabled",
				disabled: "disabled",
				offline: "shutdown",
				stopped: "stopped",
				stopping: "stopped",
				error: "error",
				failed: "error",
				unknown: "unknown",
			};

			const hasActiveInstance = instances.some((instance) =>
				[
					"ready",
					"running",
					"connected",
					"busy",
					"active",
					"healthy",
					"thinking",
					"fetch",
				].includes((instance.status || "").toLowerCase()),
			);
			const hasInitializingInstance = instances.some((instance) =>
				["initializing", "starting", "connecting"].includes(
					(instance.status || "").toLowerCase(),
				),
			);
			const hasErrorInstance = instances.some((instance) =>
				["error", "unhealthy", "stopped", "failed"].includes(
					(instance.status || "").toLowerCase(),
				),
			);

			let normalizedStatus = statusMap[rawStatus] ?? rawStatus;
			if (!normalizedStatus) {
				if (hasActiveInstance) normalizedStatus = "ready";
				else if (hasInitializingInstance) normalizedStatus = "initializing";
				else if (hasErrorInstance) normalizedStatus = "error";
				else normalizedStatus = enabledValue ? "idle" : "disabled";
			}

			const erDetail = enhanced as unknown as Record<string, unknown>;
			const registryServerId = asStringOrNull(
				erDetail.registry_server_id ?? erDetail.registryServerId,
			);

			const serverType =
				toTrimmedString(erDetail.server_type as string | undefined) ??
				toTrimmedString(erDetail.kind as string | undefined);

			const displayName = asOptionalString(erDetail.name) ?? id;

			return {
				id: asOptionalString(erDetail.id) ?? id,
				name: displayName,
				status: normalizedStatus,
				server_type: serverType,
				registry_server_id: registryServerId,
				enabled: enabledValue,
				unify_direct_exposure_eligible:
					typeof enhanced?.unify_direct_exposure_eligible === "boolean"
						? enhanced.unify_direct_exposure_eligible
						: undefined,
				globally_enabled:
					typeof enhanced?.globally_enabled === "boolean"
						? enhanced.globally_enabled
						: undefined,
				enabled_in_suits:
					typeof enhanced?.enabled_in_suits === "boolean"
						? enhanced.enabled_in_suits
						: undefined,
				enabled_in_profile:
					typeof enhanced?.enabled_in_profile === "boolean"
						? enhanced.enabled_in_profile
						: undefined,
				instances,
				command: asOptionalString(erDetail.command),
				args: Array.isArray(enhanced?.args)
					? (enhanced.args as string[])
					: undefined,
				env:
					typeof enhanced?.env === "object" && enhanced?.env !== null
						? (enhanced.env as Record<string, string>)
						: undefined,
				url: typeof enhanced?.url === "string" ? enhanced.url : undefined,
				auth_mode: asOptionalString(detailRecord.auth_mode) ?? null,
				oauth_status: asOptionalString(detailRecord.oauth_status) ?? null,
				headers:
					typeof enhanced?.headers === "object" && enhanced?.headers !== null
						? (enhanced.headers as Record<string, string>)
						: undefined,
				meta: enhanced?.meta,
				icons: enhanced?.icons,
			};
		} catch (error) {
			console.error(`Error fetching server details for ${id}:`, error);
			return {
				id,
				name: id,
				status: "error",
				server_type: "unknown",
				instances: [],
			};
		}
	},

	refreshRegistryMetadata: async (id: string): Promise<ServerDetail> => {
		const resp = await fetchApi<ServerDetailsResp>(
			`/api/mcp/registry/servers/refresh`,
			{
				method: "POST",
				body: JSON.stringify({ id }),
			},
		);
		const data = extractApiData(resp as unknown as ApiWrapper<ServerDetail>) as unknown as Record<string, unknown>;
		const enhanced = enrichServerRecord(data);
		const enabledValue =
			typeof enhanced?.enabled === "boolean"
				? enhanced.enabled
				: typeof enhanced?.globally_enabled === "boolean"
					? enhanced.globally_enabled
					: undefined;
		const instances = Array.isArray(enhanced?.instances)
			? (enhanced.instances as InstanceSummary[])
			: [];
		const detailRecord = enhanced as unknown as Record<string, unknown>;
		const registryServerId = asStringOrNull(
			detailRecord.registry_server_id ?? detailRecord.registryServerId,
		);
		const serverType =
			toTrimmedString(detailRecord.server_type as string | undefined) ??
			toTrimmedString(detailRecord.kind as string | undefined);
		return {
			id: asOptionalString(detailRecord.id) ?? id,
			name: asOptionalString(detailRecord.name) ?? id,
			status:
				(typeof enhanced?.status === "string" && enhanced.status) || "unknown",
			server_type: serverType,
			registry_server_id: registryServerId,
			enabled: enabledValue,
			unify_direct_exposure_eligible:
				typeof enhanced?.unify_direct_exposure_eligible === "boolean"
					? enhanced.unify_direct_exposure_eligible
					: undefined,
			globally_enabled:
				typeof enhanced?.globally_enabled === "boolean"
					? enhanced.globally_enabled
					: undefined,
			enabled_in_suits:
				typeof enhanced?.enabled_in_suits === "boolean"
					? enhanced.enabled_in_suits
					: undefined,
			enabled_in_profile:
				typeof enhanced?.enabled_in_profile === "boolean"
					? enhanced.enabled_in_profile
					: undefined,
			instances,
			command: asOptionalString(detailRecord.command),
			args: Array.isArray(enhanced?.args)
				? (enhanced.args as string[])
				: undefined,
			env:
				typeof enhanced?.env === "object" && enhanced?.env !== null
					? (enhanced.env as Record<string, string>)
					: undefined,
			url: typeof enhanced?.url === "string" ? enhanced.url : undefined,
			auth_mode: asOptionalString(detailRecord.auth_mode) ?? null,
			oauth_status: asOptionalString(detailRecord.oauth_status) ?? null,
			headers:
				typeof enhanced?.headers === "object" && enhanced?.headers !== null
					? (enhanced.headers as Record<string, string>)
					: undefined,
			meta: enhanced?.meta,
			icons: enhanced?.icons,
		};
	},

	getInstances: async (serverId: string) => {
		const q = new URLSearchParams({ id: serverId });
		const resp = await fetchApi<InstanceListResp>(
			`/api/mcp/servers/instances/list?${q}`,
		);
		return (resp?.data?.instances as InstanceSummary[]) ?? [];
	},

	getInstance: async (
		serverId: string,
		instanceId: string,
	): Promise<InstanceDetail> => {
		try {
			const q = new URLSearchParams({ server: serverId, instance: instanceId });
			const resp = await fetchApi<InstanceDetailsResp>(
				`/api/mcp/servers/instances/details?${q}`,
			);
			const data = resp?.data;
			return {
				id: data?.id ?? instanceId,
				name: data?.name ?? instanceId,
				server_name: data?.server_name ?? serverId,
				status: data?.status ?? "unknown",
				allowed_operations: data?.allowed_operations ?? [],
				details: data?.details ?? {
					connection_attempts: 0,
					tools_count: 0,
					server_type: "unknown",
				},
			};
		} catch (error) {
			console.error(
				`Error fetching instance details for ${serverId}/${instanceId}:`,
				error,
			);
			return {
				id: instanceId,
				name: instanceId,
				server_name: serverId,
				status: "error",
				allowed_operations: [],
				details: {
					connection_attempts: 0,
					tools_count: 0,
					server_type: "unknown",
					error_message: error instanceof Error ? error.message : String(error),
				},
			};
		}
	},

	getInstanceHealth: async (serverId: string, instanceId: string) => {
		const q = new URLSearchParams({ server: serverId, instance: instanceId });
		const resp = await fetchApi<InstanceHealthResp>(
			`/api/mcp/servers/instances/health?${q}`,
		);
		return (
			(resp?.data as InstanceHealth) ?? {
				id: instanceId,
				name: instanceId,
				healthy: false,
				message: "No data",
				status: "unknown",
				checked_at: new Date().toISOString(),
			}
		);
	},

	// Instance management operations
	_manageInstance: (serverId: string, instanceId: string, action: string) =>
		fetchApi<OperationResponseResp>("/api/mcp/servers/instances/manage", {
			method: "POST",
			body: JSON.stringify({ server: serverId, instance: instanceId, action }),
		}),

	disconnectInstance: (serverId: string, instanceId: string) =>
		serversApi._manageInstance(serverId, instanceId, "Disconnect"),

	forceDisconnectInstance: (serverId: string, instanceId: string) =>
		serversApi._manageInstance(serverId, instanceId, "ForceDisconnect"),

	reconnectInstance: (serverId: string, instanceId: string) =>
		serversApi._manageInstance(serverId, instanceId, "Reconnect"),

	resetAndReconnectInstance: (serverId: string, instanceId: string) =>
		serversApi._manageInstance(serverId, instanceId, "ResetReconnect"),

	cancelInstance: (serverId: string, instanceId: string) =>
		serversApi._manageInstance(serverId, instanceId, "Cancel"),

	// Server management operations
	_manageServer: async (serverId: string, action: string, sync = false) => {
		try {
			return await fetchApi<ApiResponse<null>>("/api/mcp/servers/manage", {
				method: "POST",
				body: JSON.stringify({ id: serverId, action, sync }),
			});
		} catch (error) {
			console.warn("API not available, using mock implementation:", error);
			return {
				status: "success",
				message: `Server ${serverId} ${action.toLowerCase()}d successfully (mock)${sync ? " with sync" : ""}`,
			};
		}
	},

	enableServer: (serverId: string, sync?: boolean) =>
		serversApi._manageServer(serverId, "Enable", sync),

	disableServer: (serverId: string, sync?: boolean) =>
		serversApi._manageServer(serverId, "Disable", sync),

	reconnectAllInstances: (serverId: string) =>
		serversApi._manageServer(serverId, "Reconnect"),

	// Server start/stop operations (mapped to enable/disable)
	startServer: (serverId: string, sync?: boolean) =>
		serversApi._manageServer(serverId, "Enable", sync),

	stopServer: (serverId: string, sync?: boolean) =>
		serversApi._manageServer(serverId, "Disable", sync),

	// CRUD operations
	createServer: async (serverConfig: Partial<MCPServerConfig>) => {
		const sc = serverConfig as {
			url?: string;
			enabled?: boolean;
			unify_direct_exposure_eligible?: boolean;
		};
		const serverType = (serverConfig.kind || "stdio") as string;
		const base: Record<string, unknown> = {
			name: serverConfig.name,
			server_type: serverType,
			enabled: sc.enabled ?? undefined,
			unify_direct_exposure_eligible:
				sc.unify_direct_exposure_eligible ?? undefined,
			profile_ids: serverConfig.profile_ids ?? undefined,
		};
		if (serverType === "stdio") {
			base.command = serverConfig.command ?? undefined;
			if (Array.isArray(serverConfig.args)) base.args = serverConfig.args;
			if (serverConfig.env && typeof serverConfig.env === "object")
				base.env = serverConfig.env as Record<string, string>;
		} else {
			base.url = sc.url ?? serverConfig.command ?? undefined;
		}
		if (serverConfig.pending_import !== undefined) {
			base.pending_import = serverConfig.pending_import;
		}
		// Add meta information if present
		const metaPayload = serializeMetaForApi(serverConfig.meta ?? undefined);
		if (metaPayload) {
			base.meta = metaPayload;
		}
		try {
			return await fetchApi<ServerDetailsResp>("/api/mcp/servers/create", {
				method: "POST",
				body: JSON.stringify(base),
			});
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e);
			// Fallback to import when the create endpoint rejects (schema/DB constraints)
			if (
				serverConfig.pending_import !== true &&
				/Check constraint violation|Unprocessable Entity|Invalid server type|missing field `server_type`/i.test(
					msg,
				)
			) {
				const name = String(serverConfig.name || "").trim();
				const importBody: { mcpServers: Record<string, McpImportServerEntry> } =
					{ mcpServers: {} };
				const cfg: McpImportServerEntry = { type: serverType };
				if (serverType === "stdio") {
					if (serverConfig.command) cfg.command = serverConfig.command;
					if (Array.isArray(serverConfig.args)) cfg.args = serverConfig.args;
					if (serverConfig.env) cfg.env = serverConfig.env;
				} else {
					if (sc.url) cfg.url = sc.url;
					else if (serverConfig.command) cfg.url = serverConfig.command;
				}
				importBody.mcpServers[name] = cfg;
				await fetchApi<ApiWrapper<ServersImportData>>("/api/mcp/servers/import", {
					method: "POST",
					body: JSON.stringify(importBody),
				});
				// Return a minimal compatible response; list refetch will normalize
				return {
					success: true,
					data: {
						id: name,
						name,
						status: "unknown",
						server_type: serverType,
						instances: [],
						command: serverConfig.command,
						args: serverConfig.args,
						env: serverConfig.env,
					},
					error: null,
				} as unknown as ServerDetailsResp;
			}
			throw e;
		}
	},

	updateServer: async (
		serverId: string,
		serverConfig: Partial<MCPServerConfig>,
	) => {
		const sc = serverConfig as {
			url?: string;
			enabled?: boolean;
			unify_direct_exposure_eligible?: boolean;
		};
		const serverType = serverConfig.kind as string | undefined;
		const body: Record<string, unknown> = {
			id: serverId,
			kind: serverConfig.kind ?? undefined,
			args: serverConfig.args ?? undefined,
			env: serverConfig.env ?? undefined,
			headers: serverConfig.headers ?? undefined,
			enabled: sc.enabled ?? undefined,
			unify_direct_exposure_eligible:
				sc.unify_direct_exposure_eligible ?? undefined,
			profile_ids: serverConfig.profile_ids ?? undefined,
		};
		if (serverConfig.pending_import !== undefined) {
			body.pending_import = serverConfig.pending_import;
		}
		if (serverType === "stdio" || !serverType) {
			body.command = serverConfig.command ?? undefined;
			body.url = sc.url ?? undefined;
		} else {
			body.url = sc.url ?? serverConfig.command ?? undefined;
			body.command = undefined;
		}
		// Add meta information if present
		const metaPayload = serializeMetaForApi(serverConfig.meta ?? undefined);
		if (metaPayload) {
			body.meta = metaPayload;
		}
		return fetchApi<ServerDetailsResp>("/api/mcp/servers/update", {
			method: "POST",
			body: JSON.stringify(body),
		});
	},

	deleteServer: (serverId: string) =>
		fetchApi<ServerDetailsResp>("/api/mcp/servers/delete", {
			method: "DELETE",
			body: JSON.stringify({ id: serverId }),
		}),

	// Server capabilities listing
	listTools: async (
		serverId: string,
		refresh: "auto" | "force" | "cache" = "auto",
	): Promise<CapabilityListResult> => {
		const q = new URLSearchParams({ id: serverId, refresh });
		const resp = await fetchApi<{
			success: boolean;
			data?: CapabilityListResult | null;
			error?: unknown | null;
		}>(`/api/mcp/servers/tools?${q}`);
		return resp?.data ?? { items: [] };
	},
	listResources: async (
		serverId: string,
		refresh: "auto" | "force" | "cache" = "auto",
	): Promise<CapabilityListResult> => {
		const q = new URLSearchParams({ id: serverId, refresh });
		const resp = await fetchApi<{
			success: boolean;
			data?: CapabilityListResult | null;
			error?: unknown | null;
		}>(`/api/mcp/servers/resources?${q}`);
		return resp?.data ?? { items: [] };
	},
	listPrompts: async (
		serverId: string,
		refresh: "auto" | "force" | "cache" = "auto",
	): Promise<CapabilityListResult> => {
		const q = new URLSearchParams({ id: serverId, refresh });
		const resp = await fetchApi<{
			success: boolean;
			data?: CapabilityListResult | null;
			error?: unknown | null;
		}>(`/api/mcp/servers/prompts?${q}`);
		return resp?.data ?? { items: [] };
	},
	listResourceTemplates: async (
		serverId: string,
		refresh: "auto" | "force" | "cache" = "auto",
	): Promise<CapabilityListResult> => {
		const q = new URLSearchParams({ id: serverId, refresh });
		const resp = await fetchApi<{
			success: boolean;
			data?: CapabilityListResult | null;
			error?: unknown | null;
		}>(`/api/mcp/servers/resources/templates?${q}`);
		return resp?.data ?? { items: [] };
	},

	// Import servers from JSON-like configuration object
	importServers: async (payload: {
		mcpServers: Record<string, unknown>;
		target_profile_id?: string | null;
		dry_run?: boolean;
	}): Promise<ApiWrapper<ServersImportData>> => {
		return await fetchApi(`/api/mcp/servers/import`, {
			method: "POST",
			body: JSON.stringify(payload),
		});
	},

	// Preview capabilities for proposed server configs without importing
	previewServers: async (payload: {
		include_details?: boolean | null;
		timeout_ms?: number | null;
		servers: Array<{
			name: string;
			server_id?: string | null;
			kind: string;
			command?: string | null;
			args?: string[] | null;
			env?: Record<string, string> | null;
			url?: string | null;
		}>;
	}): Promise<{
		success: boolean;
		data?: { items: unknown[] } | null;
		error?: unknown | null;
	}> => {
		return await fetchApi(`/api/mcp/servers/preview`, {
			method: "POST",
			body: JSON.stringify(payload),
		});
	},
};

export interface ImportStats {
	importedCount: number;
	importedServers: string[];
	skippedCount: number;
	skippedServers: string[];
	skippedDetails: SkippedServer[];
	failedCount: number;
	failedServers: string[];
	errorDetails?: Record<string, string> | null;
}

export function extractImportStats(
	response:
		| ApiWrapper<ServersImportData>
		| ServersImportData
		| null
		| undefined,
): ImportStats {
	let payload: ServersImportData | null = null;
	if (response && typeof response === "object") {
		if ("data" in response && response.data) {
			payload = response.data as ServersImportData;
		} else if ("imported_count" in response) {
			payload = response as ServersImportData;
		}
	}

	const importedServers = Array.isArray(payload?.imported_servers)
		? payload!.imported_servers
		: [];
	const importedCount =
		typeof payload?.imported_count === "number"
			? payload.imported_count
			: importedServers.length;
	const skippedDetails = Array.isArray(payload?.skipped_servers)
		? payload!.skipped_servers
		: [];
	const skippedServers = skippedDetails.map((item) => item.name);
	const skippedCount =
		typeof payload?.skipped_count === "number"
			? payload.skipped_count
			: skippedServers.length;
	const failedServers = Array.isArray(payload?.failed_servers)
		? payload!.failed_servers
		: [];
	const failedCount =
		typeof payload?.failed_count === "number"
			? payload.failed_count
			: failedServers.length;
	return {
		importedCount,
		importedServers,
		skippedCount,
		skippedServers,
		skippedDetails,
		failedCount,
		failedServers,
		errorDetails: payload?.error_details ?? null,
	};
}

// Inspector API
export const inspectorApi = {
	toolsList: (q: {
		server_id?: string;
		server_name?: string;
		mode?: "proxy" | "native";
		refresh?: boolean;
	}) => {
		const qs = new URLSearchParams();
		if (q.server_id) qs.set("server_id", q.server_id);
		if (q.server_name) qs.set("server_name", q.server_name);
		if (q.mode) qs.set("mode", q.mode);
		if (q.refresh != null) qs.set("refresh", String(q.refresh));
		return fetchApi(`/api/mcp/inspector/tool/list?${qs}`);
	},
	toolCall: (req: {
		tool: string;
		server_id?: string;
		server_name?: string;
		arguments?: Record<string, unknown>;
		mode?: "proxy" | "native";
		timeout_ms?: number;
	}) =>
		fetchApi(`/api/mcp/inspector/tool/call`, {
			method: "POST",
			body: JSON.stringify(req),
		}),
	toolCallStart: async (req: {
		tool: string;
		server_id?: string;
		server_name?: string;
		arguments?: Record<string, unknown>;
		mode?: "proxy" | "native";
		timeout_ms?: number;
		session_id?: string;
	}) =>
		fetchApi<ApiWrapper<InspectorToolCallStartData>>(
			`/api/mcp/inspector/tool/call/start`,
			{
				method: "POST",
				body: JSON.stringify(req),
			},
		),
	toolCallCancel: async (req: { call_id: string; reason?: string }) =>
		fetchApi<ApiWrapper<InspectorToolCallCancelData>>(
			`/api/mcp/inspector/tool/call/cancel`,
			{
				method: "POST",
				body: JSON.stringify(req),
			},
		),
	sessionOpen: async (req: {
		mode: "proxy" | "native";
		server_id?: string;
		server_name?: string;
	}) =>
		fetchApi<ApiWrapper<InspectorSessionOpenData>>(
			`/api/mcp/inspector/session/open`,
			{
				method: "POST",
				body: JSON.stringify(req),
			},
		),
	sessionClose: async (req: { session_id: string }) =>
		fetchApi<ApiWrapper<InspectorSessionCloseData>>(
			`/api/mcp/inspector/session/close`,
			{
				method: "POST",
				body: JSON.stringify(req),
			},
		),
	toolCallEventsWsUrl: (callId: string) => {
		const wsBase = resolveWebSocketUrl();
		return `${wsBase}/inspector/events?call_id=${encodeURIComponent(callId)}`;
	},
	resourcesList: (q: {
		server_id?: string;
		server_name?: string;
		mode?: "proxy" | "native";
		refresh?: boolean;
	}) => {
		const qs = new URLSearchParams();
		if (q.server_id) qs.set("server_id", q.server_id);
		if (q.server_name) qs.set("server_name", q.server_name);
		if (q.mode) qs.set("mode", q.mode);
		if (q.refresh != null) qs.set("refresh", String(q.refresh));
		return fetchApi(`/api/mcp/inspector/resource/list?${qs}`);
	},
	resourceRead: (q: {
		uri: string;
		server_id?: string;
		server_name?: string;
		mode?: "proxy" | "native";
	}) => {
		const qs = new URLSearchParams({ uri: q.uri });
		if (q.server_id) qs.set("server_id", q.server_id);
		if (q.server_name) qs.set("server_name", q.server_name);
		if (q.mode) qs.set("mode", q.mode);
		return fetchApi(`/api/mcp/inspector/resource/read?${qs}`);
	},
	promptsList: (q: {
		server_id?: string;
		server_name?: string;
		mode?: "proxy" | "native";
		refresh?: boolean;
	}) => {
		const qs = new URLSearchParams();
		if (q.server_id) qs.set("server_id", q.server_id);
		if (q.server_name) qs.set("server_name", q.server_name);
		if (q.mode) qs.set("mode", q.mode);
		if (q.refresh != null) qs.set("refresh", String(q.refresh));
		return fetchApi(`/api/mcp/inspector/prompt/list?${qs}`);
	},
	promptGet: (req: {
		name: string;
		server_id?: string;
		server_name?: string;
		arguments?: Record<string, unknown>;
		mode?: "proxy" | "native";
	}) =>
		fetchApi(`/api/mcp/inspector/prompt/get`, {
			method: "POST",
			body: JSON.stringify(req),
		}),
	templatesList: (q: {
		server_id?: string;
		server_name?: string;
		mode?: "proxy" | "native";
		refresh?: boolean;
	}) => {
		const qs = new URLSearchParams();
		if (q.server_id) qs.set("server_id", q.server_id);
		if (q.server_name) qs.set("server_name", q.server_name);
		if (q.mode) qs.set("mode", q.mode);
		if (q.refresh != null) qs.set("refresh", String(q.refresh));
		return fetchApi(`/api/mcp/inspector/template/list?${qs}`);
	},
};

export const auditApi = {
	list: async (query?: {
		cursor?: string;
		limit?: number;
		category?: string;
		action?: string;
		status?: string;
		client_id?: string;
		profile_id?: string;
		server_id?: string;
		session_id?: string;
	}) => {
		const qs = new URLSearchParams();
		if (query?.cursor) qs.set("cursor", query.cursor);
		if (query?.limit != null) qs.set("limit", String(query.limit));
		if (query?.category) qs.set("category", query.category);
		if (query?.action) qs.set("action", query.action);
		if (query?.status) qs.set("status", query.status);
		if (query?.client_id) qs.set("client_id", query.client_id);
		if (query?.profile_id) qs.set("profile_id", query.profile_id);
		if (query?.server_id) qs.set("server_id", query.server_id);
		if (query?.session_id) qs.set("session_id", query.session_id);
		const resp = await fetchApi<AuditListResp>(`/api/audit/events?${qs}`);
		return extractApiData(resp);
	},
	details: async (id: number) => {
		const qs = new URLSearchParams({ id: String(id) });
		const resp = await fetchApi<AuditEventDetailsResp>(`/api/audit/events/details?${qs}`);
		const event = resp.data?.event;
		if (!event) {
			throw new Error(`Audit event details response missing event for id ${id}`);
		}
		return event;
	},
	eventsWsUrl: () => {
		const wsBase = resolveWebSocketUrl();
		return `${wsBase}/audit/events`;
	},
	getPolicy: async () => {
		const resp = await fetchApi<AuditPolicyResp>("/api/audit/policy");
		return extractApiData(resp);
	},
	setPolicy: async (payload: AuditPolicySetReq) => {
		const resp = await fetchApi<AuditPolicyResp>("/api/audit/policy", {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(payload),
		});
		return extractApiData(resp);
	},
};

// Tools Management API
export const toolsApi = {
	getAll: async () => {
		try {
			// Try to get tools from active profile first
			const suitsResponse = await configSuitsApi.getAll();
			const active = suitsResponse?.suits?.find((s) => s.is_active);
			if (active) {
				try {
					const q = new URLSearchParams({ profile_id: active.id });
					const resp = await fetchApi<
						ApiWrapper<{
							profile_id: string;
							profile_name: string;
							tools: SuitTool[];
						}>
					>(`/api/mcp/profile/tools/list?${q}`);
					const data = extractApiData(resp);
					if (data?.tools) {
						const tools = data.tools.map((tool: SuitTool) => ({
							tool_name: tool.tool_name || tool.name || "",
							server_name: tool.server_name || "",
							is_enabled: tool.is_enabled ?? tool.enabled ?? true,
							description: tool.description || "",
							tool_id: tool.id || "",
						}));
						return { tools };
					}
				} catch (e) {
					console.error("Failed to fetch profile tools:", e);
				}
			}

			// Fallback to specs endpoint
			const response = await fetchApi<
				Array<{
					name: string;
					tool_name?: string;
					description?: string;
					id?: string;
					server_name?: string;
				}>
			>("/api/mcp/specs/tools");

			const tools = response.map((tool) => {
				let serverName = tool.server_name;
				if (!serverName && tool.description?.includes("server '")) {
					serverName = tool.description.split("server '")[1].split("'")[0];
				}
				const toolName = tool.tool_name || tool.name || "";
				const toolId = tool.id || `${serverName}_${toolName}`;

				return {
					tool_name: toolName,
					server_name: serverName || "",
					is_enabled: true,
					description: tool.description || "",
					tool_id: toolId,
				};
			});

			return { tools };
		} catch (error) {
			console.error("Failed to fetch tools:", error);
			return { tools: [] };
		}
	},

	// Deprecated helpers removed; use configSuitsApi instead

	getTool: (serverId: string, toolName: string) =>
		fetchApi<ToolDetail>(`/api/mcp/specs/tools/${serverId}/${toolName}`),

	updateTool: (
		serverId: string,
		toolName: string,
		config: Partial<ToolDetail>,
	) =>
		fetchApi<ApiResponse<ToolDetail>>(
			`/api/mcp/specs/tools/${serverId}/${toolName}`,
			{
				method: "POST",
				body: JSON.stringify(config),
			},
		),

	// Tool management operations
	_manageTool: async (
		profileId: string,
		toolId: string,
		action: "enable" | "disable",
	) =>
		fetchApi<ApiResponse<null>>("/api/mcp/profile/tools/manage", {
			method: "POST",
			body: JSON.stringify({
				profile_id: profileId,
				component_ids: [toolId],
				action,
			}),
		}),

	enableTool: (profileId: string, toolId: string) =>
		toolsApi._manageTool(profileId, toolId, "enable"),

	disableTool: (profileId: string, toolId: string) =>
		toolsApi._manageTool(profileId, toolId, "disable"),
};

// System Management API
export const systemApi = {
	getStatus: () => fetchApi<SystemStatus>("/api/system/status"),
	getMetrics: () => fetchApi<SystemMetrics>("/api/system/metrics"),
	getDefaultClientMode: async (): Promise<{
		default_config_mode: "unify" | "hosted" | "transparent";
	}> => {
		const response = await fetchApi<
			ApiWrapper<{
				default_config_mode: "unify" | "hosted" | "transparent";
			}>
		>("/api/system/client-default-mode");
		return extractApiData(response);
	},
	setDefaultClientMode: async (default_config_mode: "unify" | "hosted" | "transparent") => {
		const response = await fetchApi<
			ApiWrapper<{
				default_config_mode: "unify" | "hosted" | "transparent";
			}>
		>("/api/system/client-default-mode", {
			method: "POST",
			body: JSON.stringify({ default_config_mode }),
		});
		return extractApiData(response);
	},
	getFirstContactBehavior: async (): Promise<{
		behavior: "deny" | "review" | "allow" | string;
	}> => {
		const response = await fetchApi<
			ApiWrapper<{
				behavior: "deny" | "review" | "allow" | string;
			}>
		>("/api/system/settings/first-contact-behavior");
		return extractApiData(response);
	},
	setFirstContactBehavior: async (behavior: "deny" | "review" | "allow" | string) => {
		const response = await fetchApi<
			ApiWrapper<{
				behavior: "deny" | "review" | "allow" | string;
			}>
		>("/api/system/settings/first-contact-behavior", {
			method: "POST",
			body: JSON.stringify({ behavior }),
		});
		return extractApiData(response);
	},

	shutdown: () =>
		fetchApi<{ status: string; message?: string }>("/api/system/shutdown", {
			method: "POST",
		}),

	restart: () =>
		fetchApi<{ status: string; message?: string; mcp_port?: number }>(
			"/api/system/restart",
			{ method: "POST" },
		),
};

// Runtime Management API
export const runtimeApi = {
	getStatus: async (): Promise<RuntimeStatusResponse> => {
		const response = await fetchApi<ApiWrapper<RuntimeStatusResponse>>(
			"/api/runtime/status",
		);
		return extractApiData(response);
	},

	getCache: async (): Promise<RuntimeCacheResponse> => {
		const response =
			await fetchApi<ApiWrapper<RuntimeCacheResponse>>("/api/runtime/cache");
		return extractApiData(response);
	},

	resetCache: (cacheType?: "all" | "uv" | "bun") =>
		fetchApi<ClearCacheResponse>("/api/runtime/cache/reset", {
			method: "POST",
			body: JSON.stringify({ cache_type: cacheType || "all" }),
		}),

	install: (req: InstallRuntimeRequest) =>
		fetchApi<InstallResponse>("/api/runtime/install", {
			method: "POST",
			body: JSON.stringify(req),
		}),
};

// Capabilities Cache API
export const capabilitiesApi = {
	getStats: async (): Promise<CapabilitiesStatsResponse> => {
		const response = await fetchApi<
			ApiWrapper<{
				storage: CapabilitiesStorageStats;
				metrics: CapabilitiesMetricsStats;
				generated_at: string;
			}>
		>("/api/mcp/servers/cache/detail?view=stats");

		const data = extractApiData(response);
		return {
			storage: data.storage,
			metrics: data.metrics,
			generatedAt: data.generated_at,
		};
	},

	getKeys: (params?: { limit?: number; offset?: number; search?: string }) => {
		const q = new URLSearchParams({ view: "keys" });
		if (params?.limit != null) q.set("limit", String(params.limit));
		if (params?.offset != null) q.set("offset", String(params.offset));
		if (params?.search) q.set("search", params.search);
		return fetchApi<CapabilitiesKeysResponse>(
			`/api/mcp/servers/cache/detail?${q}`,
		);
	},

	reset: () =>
		fetchApi<ClearCacheResponse>("/api/mcp/servers/cache/reset", {
			method: "POST",
		}),
};

// Configuration Management API
export const configApi = {
	getCurrentConfig: async (): Promise<MCPConfig> => {
		try {
			return await fetchApi<MCPConfig>("/api/config/current");
		} catch (error) {
			console.warn(
				"Config API not available, building from available data:",
				error,
			);

			// Build config from available APIs
			const config: MCPConfig = {
				servers: [],
				tools: [],
				global_settings: {
					max_concurrent_connections: 10,
					request_timeout_ms: 30000,
					enable_metrics: true,
					log_level: "info",
				},
			};

			// Try to populate with real data
			try {
				const serversResponse = await serversApi.getAll();
				if (Array.isArray(serversResponse?.servers)) {
					config.servers = serversResponse.servers.map((server) => ({
						name: server.name,
						kind:
							server.server_type === "stdio"
								? "stdio"
								: "streamable_http",
						command: "",
						command_path: undefined,
						args: [],
						env: {},
						max_instances: 1,
						retry_policy: {
							max_attempts: 3,
							initial_delay_ms: 1000,
							max_delay_ms: 10000,
							backoff_multiplier: 1.5,
						},
					}));
				}
			} catch (e) {
				console.error("Failed to fetch servers for config:", e);
			}

			try {
				const toolsResponse = await toolsApi.getAll();
				if (Array.isArray(toolsResponse?.tools)) {
					config.tools = toolsResponse.tools.map((tool) => ({
						name: tool.tool_name,
						server_name: tool.server_name ?? "",
						is_enabled: tool.is_enabled,
						settings: {},
					}));
				}
			} catch (e) {
				console.error("Failed to fetch tools for config:", e);
			}

			return config;
		}
	},

	updateConfig: async (config: MCPConfig) => {
		try {
			return await fetchApi<ApiResponse<MCPConfig>>("/api/config", {
				method: "POST",
				body: JSON.stringify(config),
			});
		} catch (error) {
			console.warn("Config API not available, using mock response:", error);
			return {
				status: "success",
				message: "Configuration updated (mock)",
				data: config,
			};
		}
	},

	// Preset management with simplified mock fallback
	getPresets: async () => {
		try {
			return await fetchApi<ConfigPreset[]>("/api/config/presets");
		} catch (error) {
			console.warn("Config API not available, using mock data:", error);
			return [
				{
					id: "default",
					name: "Default Configuration",
					description: "Default system configuration",
					created_at: new Date().toISOString(),
					updated_at: new Date().toISOString(),
					is_active: true,
					config: await configApi.getCurrentConfig(),
				},
			];
		}
	},

	getPreset: async (id: string) => {
		try {
			return await fetchApi<ConfigPreset>(`/api/config/presets/${id}`);
		} catch (error) {
			console.warn("Config API not available, using mock data:", error);
			const presets = await configApi.getPresets();
			const preset = presets.find((p) => p.id === id);
			if (!preset) throw new Error(`Preset with ID ${id} not found`);
			return preset;
		}
	},

	createPreset: (
		preset: Omit<ConfigPreset, "id" | "created_at" | "updated_at">,
	) =>
		fetchApi<ApiResponse<ConfigPreset>>("/api/config/presets", {
			method: "POST",
			body: JSON.stringify(preset),
		}),

	updatePreset: (id: string, preset: Partial<ConfigPreset>) =>
		fetchApi<ApiResponse<ConfigPreset>>(`/api/config/presets/${id}`, {
			method: "PUT",
			body: JSON.stringify(preset),
		}),

	deletePreset: (id: string) =>
		fetchApi<ApiResponse<null>>(`/api/config/presets/${id}`, {
			method: "DELETE",
		}),

	applyPreset: (id: string) =>
		fetchApi<ApiResponse<null>>(`/api/config/presets/${id}/apply`, {
			method: "POST",
		}),
};

// Config Suits Management API
export const configSuitsApi = {
	getAll: async (): Promise<ConfigSuitListResponse> => {
		try {
			const response = await fetchApi<
				ApiWrapper<{
					profile: ProfileApiRow[];
					total: number;
					timestamp: string;
				}>
			>("/api/mcp/profile/list");
			const data = extractApiData(response);
			const suits: ConfigSuit[] = (data.profile || []).map(profileRowToConfigSuit);
			return { suits };
		} catch (error) {
			console.error("Failed to fetch config suits:", error);
			return { suits: [] };
		}
	},
	// Server capabilities
	listTools: async (serverId: string, refresh?: "auto" | "force" | "cache") => {
		const q = new URLSearchParams({ id: serverId });
		if (refresh) q.set("refresh", refresh);
		const resp = await fetchApi<ServerCapabilityResp>(
			`/api/mcp/servers/tools?${q}`,
		);
		return extractApiData(resp);
	},
	listResources: async (
		serverId: string,
		refresh?: "auto" | "force" | "cache",
	) => {
		const q = new URLSearchParams({ id: serverId });
		if (refresh) q.set("refresh", refresh);
		const resp = await fetchApi<ServerCapabilityResp>(
			`/api/mcp/servers/resources?${q}`,
		);
		return extractApiData(resp);
	},
	listPrompts: async (
		serverId: string,
		refresh?: "auto" | "force" | "cache",
	) => {
		const q = new URLSearchParams({ id: serverId });
		if (refresh) q.set("refresh", refresh);
		const resp = await fetchApi<ServerCapabilityResp>(
			`/api/mcp/servers/prompts?${q}`,
		);
		return extractApiData(resp);
	},
	listResourceTemplates: async (
		serverId: string,
		refresh?: "auto" | "force" | "cache",
	) => {
		const q = new URLSearchParams({ id: serverId });
		if (refresh) q.set("refresh", refresh);
		const resp = await fetchApi<ServerCapabilityResp>(
			`/api/mcp/servers/resources/templates?${q}`,
		);
		return extractApiData(resp);
	},

	getSuit: async (id: string): Promise<ConfigSuit> => {
		const q = new URLSearchParams({ id });
		const response = await fetchApi<ApiWrapper<{ profile: ProfileApiRow }>>(
			`/api/mcp/profile/details?${q}`,
		);
		const data = extractApiData(response);
		if (!data.profile) {
			throw new Error("Profile details response missing profile");
		}
		return profileRowToConfigSuit(data.profile);
	},

	createSuit: async (
		data: CreateConfigSuitRequest,
	): Promise<ApiResponse<ConfigSuit>> => {
		const payload = {
			name: data.name,
			description: data.description ?? null,
			profile_type: data.suit_type,
			multi_select: data.multi_select ?? null,
			priority: data.priority ?? null,
			is_active: data.is_active ?? null,
			is_default: data.is_default ?? null,
			clone_from_id: data.clone_from_id ?? null,
		};
		const response = await fetchApi<ApiWrapper<ProfileApiRow>>(
			"/api/mcp/profile/create",
			{
				method: "POST",
				body: JSON.stringify(payload),
			},
		);
		const p = extractApiData(response);
		const result = profileRowToConfigSuit(p);
		return { status: "success", message: "Profile created", data: result };
	},

	updateSuit: async (
		id: string,
		data: UpdateConfigSuitRequest,
	): Promise<ApiResponse<ConfigSuit>> => {
		const payload = {
			id,
			name: data.name ?? null,
			description: data.description ?? null,
			profile_type: data.suit_type ?? null,
			multi_select: data.multi_select ?? null,
			priority: data.priority ?? null,
			is_active: data.is_active ?? null,
			is_default: data.is_default ?? null,
		};
		const response = await fetchApi<ApiWrapper<ProfileApiRow>>(
			`/api/mcp/profile/update`,
			{
				method: "POST",
				body: JSON.stringify(payload),
			},
		);
		const p = extractApiData(response);
		const result = profileRowToConfigSuit(p);
		return { status: "success", message: "Profile updated", data: result };
	},

	deleteSuit: async (id: string): Promise<ApiResponse<void>> => {
		const response = await fetchApi<ApiWrapper<void>>(
			`/api/mcp/profile/delete`,
			{
				method: "DELETE",
				body: JSON.stringify({ id }),
			},
		);
		extractApiData(response);
		return {
			status: "success",
			message: "Config suit deleted successfully",
		};
	},

	// Suit management operations
	_manageSuit: async (id: string, action: "activate" | "deactivate") => {
		const response = await fetchApi<ApiWrapper<unknown>>(
			"/api/mcp/profile/manage",
			{
				method: "POST",
				body: JSON.stringify({ ids: [id], action }),
			},
		);
		extractApiData(response);
		return {
			status: "success",
			message: `Config suit ${action}d successfully`,
			data: null,
		};
	},

	activateSuit: (id: string) => configSuitsApi._manageSuit(id, "activate"),
	deactivateSuit: (id: string) => configSuitsApi._manageSuit(id, "deactivate"),

	// Batch operations
	batchActivate: (ids: string[]) =>
		executeBatchOperation(ids, configSuitsApi.activateSuit),
	batchDeactivate: (ids: string[]) =>
		executeBatchOperation(ids, configSuitsApi.deactivateSuit),

	// Suit content management
	getServers: async (suitId: string): Promise<ConfigSuitServersResponse> => {
		const q = new URLSearchParams({ profile_id: suitId });
		const response = await fetchApi<
			ApiWrapper<{
				profile_id: string;
				profile_name: string;
				servers: ConfigSuitServer[];
			}>
		>(`/api/mcp/profile/servers/list?${q}`);
		const data = extractApiData(response);
		return {
			suit_id: data.profile_id,
			suit_name: data.profile_name,
			servers: data.servers || [],
		};
	},

	getTools: async (suitId: string): Promise<ConfigSuitToolsResponse> => {
		const q = new URLSearchParams({ profile_id: suitId });
		const response = await fetchApi<
			ApiWrapper<{
				profile_id: string;
				profile_name: string;
				tools: ConfigSuitTool[];
			}>
		>(`/api/mcp/profile/tools/list?${q}`);
		const data = extractApiData(response);
		return {
			suit_id: data.profile_id,
			suit_name: data.profile_name,
			tools: data.tools || [],
		};
	},

	getResources: async (
		suitId: string,
	): Promise<ConfigSuitResourcesResponse> => {
		const q = new URLSearchParams({ profile_id: suitId });
		const response = await fetchApi<
			ApiWrapper<{
				profile_id: string;
				profile_name: string;
				resources: ConfigSuitResource[];
			}>
		>(`/api/mcp/profile/resources/list?${q}`);
		const data = extractApiData(response);
		return {
			suit_id: data.profile_id,
			suit_name: data.profile_name,
			resources: data.resources || [],
		};
	},

	getPrompts: async (suitId: string): Promise<ConfigSuitPromptsResponse> => {
		const q = new URLSearchParams({ profile_id: suitId });
		const response = await fetchApi<
			ApiWrapper<{
				profile_id: string;
				profile_name: string;
				prompts: ConfigSuitPrompt[];
			}>
		>(`/api/mcp/profile/prompts/list?${q}`);
		const data = extractApiData(response);
		return {
			suit_id: data.profile_id,
			suit_name: data.profile_name,
			prompts: data.prompts || [],
		};
	},

	getResourceTemplates: async (
		suitId: string,
	): Promise<ConfigSuitResourceTemplatesResponse> => {
		const q = new URLSearchParams({ profile_id: suitId });
		const response = await fetchApi<
			ApiWrapper<{
				profile_id: string;
				profile_name: string;
				templates: ConfigSuitResourceTemplate[];
			}>
		>(`/api/mcp/profile/resource-templates/list?${q}`);
		const data = extractApiData(response);
		return {
			suit_id: data.profile_id,
			suit_name: data.profile_name,
			templates: data.templates || [],
		};
	},

	// Component management operations
	_manageComponent: (
		endpoint: "servers" | "tools" | "resources" | "prompts" | "resource-templates",
		suitId: string,
		componentId: string,
		action: "enable" | "disable" | "remove",
	) =>
		fetchApi<ApiResponse<null>>(`/api/mcp/profile/${endpoint}/manage`, {
			method: "POST",
			body: JSON.stringify({
				profile_id: suitId,
				component_ids: [componentId],
				action,
			}),
		}),

	// Batch manage multiple component ids in one request for reliability/perf
	_manageComponentsBatch: (
		endpoint: "servers" | "tools" | "resources" | "prompts" | "resource-templates",
		suitId: string,
		componentIds: string[],
		action: "enable" | "disable",
	) =>
		fetchApi<ApiResponse<null>>(`/api/mcp/profile/${endpoint}/manage`, {
			method: "POST",
			body: JSON.stringify({
				profile_id: suitId,
				component_ids: componentIds,
				action,
			}),
		}),

	enableServer: (suitId: string, serverId: string) =>
		configSuitsApi._manageComponent("servers", suitId, serverId, "enable"),
	disableServer: (suitId: string, serverId: string) =>
		configSuitsApi._manageComponent("servers", suitId, serverId, "disable"),
	removeServer: (suitId: string, serverId: string) =>
		configSuitsApi._manageComponent("servers", suitId, serverId, "remove"),
	// Some backends only support single-id manage for servers; do per-id with best-effort batching
	bulkServers: async (
		suitId: string,
		ids: string[],
		action: "enable" | "disable" | "remove",
	) =>
		executeBatchOperation(ids, (id) =>
			configSuitsApi._manageComponent("servers", suitId, id, action),
		),
	enableTool: (suitId: string, toolId: string) =>
		configSuitsApi._manageComponent("tools", suitId, toolId, "enable"),
	disableTool: (suitId: string, toolId: string) =>
		configSuitsApi._manageComponent("tools", suitId, toolId, "disable"),
	bulkTools: (suitId: string, ids: string[], action: "enable" | "disable") =>
		configSuitsApi._manageComponentsBatch("tools", suitId, ids, action),
	enableResource: (suitId: string, resourceId: string) =>
		configSuitsApi._manageComponent("resources", suitId, resourceId, "enable"),
	disableResource: (suitId: string, resourceId: string) =>
		configSuitsApi._manageComponent("resources", suitId, resourceId, "disable"),
	bulkResources: (
		suitId: string,
		ids: string[],
		action: "enable" | "disable",
	) => configSuitsApi._manageComponentsBatch("resources", suitId, ids, action),
	getResourceTemplatesForSuit: (suitId: string) =>
		configSuitsApi.getResourceTemplates(suitId),
	enableResourceTemplate: (suitId: string, id: string) =>
		configSuitsApi._manageComponent("resource-templates", suitId, id, "enable"),
	disableResourceTemplate: (suitId: string, id: string) =>
		configSuitsApi._manageComponent("resource-templates", suitId, id, "disable"),
	bulkResourceTemplates: (
		suitId: string,
		ids: string[],
		action: "enable" | "disable",
	) =>
		configSuitsApi._manageComponentsBatch(
			"resource-templates",
			suitId,
			ids,
			action,
		),
	enablePrompt: (suitId: string, promptId: string) =>
		configSuitsApi._manageComponent("prompts", suitId, promptId, "enable"),
	disablePrompt: (suitId: string, promptId: string) =>
		configSuitsApi._manageComponent("prompts", suitId, promptId, "disable"),
	bulkPrompts: (suitId: string, ids: string[], action: "enable" | "disable") =>
		configSuitsApi._manageComponentsBatch("prompts", suitId, ids, action),
};

// WebSocket Notifications Service
export class NotificationsService {
	private ws: WebSocket | null = null;
	private listeners: Map<string, ((data: NotificationData) => void)[]> =
		new Map();
	private reconnectAttempts = 0;
	private readonly maxReconnectAttempts = 5;
	private readonly reconnectDelay = 5000;
	/** When true, the next `onclose` reconnects immediately to the current `API_BASE_URL` (no backoff). */
	private reconnectToNewBasePending = false;

	connect() {
		if (this.ws) return;

		const wsUrl = resolveWebSocketUrl();
		if (!wsUrl) {
			console.warn(
				"WebSocket URL could not be resolved; skipping connection attempt",
			);
			return;
		}
		console.log(`Connecting to WebSocket at ${wsUrl}`);

		try {
			this.ws = new WebSocket(wsUrl);

			this.ws.onopen = () => {
				console.log("WebSocket connection established");
				this.reconnectAttempts = 0;
			};

			this.ws.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data) as NotificationData;
					if (data.event) {
						const eventListeners = this.listeners.get(data.event) || [];
						eventListeners.forEach((listener) => listener(data));
					}
				} catch (error) {
					console.error("Error parsing WebSocket message:", error);
				}
			};

			this.ws.onerror = (error) => {
				console.error("WebSocket error:", error);
			};

			this.ws.onclose = (event) => {
				console.log(
					`WebSocket connection closed: ${event.code} ${event.reason}`,
				);
				this.ws = null;

				if (this.reconnectToNewBasePending) {
					this.reconnectToNewBasePending = false;
					this.reconnectAttempts = 0;
					queueMicrotask(() => this.connect());
					return;
				}

				if (this.reconnectAttempts < this.maxReconnectAttempts) {
					const delay = this.reconnectDelay * 1.5 ** this.reconnectAttempts;
					this.reconnectAttempts++;
					console.log(
						`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`,
					);
					setTimeout(() => this.connect(), delay);
				} else {
					console.error(
						`Failed to reconnect after ${this.maxReconnectAttempts} attempts`,
					);
				}
			};
		} catch (error) {
			console.error("Error creating WebSocket connection:", error);
		}
	}

	subscribe(event: string, callback: (data: NotificationData) => void) {
		let list = this.listeners.get(event);
		if (!list) {
			list = [];
			this.listeners.set(event, list);
		}
		list.push(callback);

		if (!this.ws) {
			this.connect();
		}

		return () => {
			const eventListeners = this.listeners.get(event) || [];
			const index = eventListeners.indexOf(callback);
			if (index !== -1) {
				eventListeners.splice(index, 1);
			}
		};
	}

	disconnect() {
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
	}

	/**
	 * Use after `setApiBaseUrl` (e.g. desktop Settings port change) so notifications use the new host
	 * without waiting for reconnect backoff or being blocked by `if (this.ws) return`.
	 */
	reconnectAfterApiBaseChanged() {
		if (this.ws) {
			this.reconnectToNewBasePending = true;
			this.ws.close();
			return;
		}
		this.reconnectAttempts = 0;
		this.connect();
	}
}

export const notificationsService = new NotificationsService();

/** Maps legacy API values to the simplified deny | review | allow model. */
function normalizeFirstContactBehavior(raw: string): "deny" | "review" | "allow" {
	if (raw === "pending_review" || raw === "allow_then_review") {
		return "review";
	}
	if (raw === "deny" || raw === "review" || raw === "allow") {
		return raw;
	}
	return "allow";
}

// Clients Management API
export const clientsApi = {
	getDefaultPolicy: async () => {
		const [mode, firstContact] = await Promise.all([
			systemApi.getDefaultClientMode(),
			systemApi.getFirstContactBehavior(),
		]);
		return {
			config_mode: mode.default_config_mode,
			capability_source: "activated" as const,
			first_contact_behavior: normalizeFirstContactBehavior(firstContact.behavior),
		};
	},
	updateDefaultPolicy: async (
		payload: DefaultClientPolicyUpdateReq,
		before?: { config_mode: string; first_contact_behavior: string } | null,
	) => {
		const modeT = payload.config_mode as "unify" | "hosted" | "transparent";
		const modeChanged = !before || before.config_mode !== payload.config_mode;
		const prevFc = before
			? normalizeFirstContactBehavior(before.first_contact_behavior)
			: null;
		const nextFc = normalizeFirstContactBehavior(String(payload.first_contact_behavior));
		const fcChanged = !before || prevFc !== nextFc;

		if (!modeChanged && !fcChanged) {
			return {
				config_mode: payload.config_mode,
				capability_source: payload.capability_source,
				first_contact_behavior: nextFc,
			};
		}

		let mode: { default_config_mode: "unify" | "hosted" | "transparent" };
		let firstContact: { behavior: string };

		if (modeChanged && fcChanged) {
			[mode, firstContact] = await Promise.all([
				systemApi.setDefaultClientMode(modeT),
				systemApi.setFirstContactBehavior(payload.first_contact_behavior),
			]);
		} else if (modeChanged) {
			mode = await systemApi.setDefaultClientMode(modeT);
			firstContact = { behavior: before!.first_contact_behavior };
		} else {
			firstContact = await systemApi.setFirstContactBehavior(payload.first_contact_behavior);
			mode = { default_config_mode: modeT };
		}

		return {
			config_mode: mode.default_config_mode,
			capability_source: payload.capability_source,
			first_contact_behavior: normalizeFirstContactBehavior(firstContact.behavior),
		};
	},
	setDefaultPolicy: async (
		payload: DefaultClientPolicyUpdateReq,
		before?: { config_mode: string; first_contact_behavior: string } | null,
	) => {
		return clientsApi.updateDefaultPolicy(payload, before);
	},
	getCapabilityConfig: async (identifier: string) => {
		const q = new URLSearchParams({ identifier });
		const resp = await fetchApi<ClientCapabilityConfigResp>(
			`/api/client/capability-config?${q}`,
		);
		return extractApiData(resp);
	},

	updateCapabilityConfig: async (payload: ClientCapabilityConfigReq) => {
		const resp = await fetchApi<ClientCapabilityConfigResp>(
			"/api/client/capability-config",
			{
				method: "POST",
				body: JSON.stringify(payload),
			},
		);
		return extractApiData(resp);
	},

	list: async (refresh = false) => {
		const q = new URLSearchParams({ refresh: String(refresh) });
		const resp = await fetchApi<ClientCheckResp>(`/api/client/list?${q}`);
		return extractApiData(resp);
	},

	manage: async (identifier: string, action: ClientManageAction) => {
		const resp = await fetchApi<ClientManageResp>("/api/client/manage", {
			method: "POST",
			body: JSON.stringify({ identifier, action }),
		});
		return extractApiData(resp);
	},

	approveRecord: async (payload: ClientRecordReviewReq) => {
		return fetchApi<ClientRecordLifecycleResp>("/api/client/manage/approve", {
			method: "POST",
			body: JSON.stringify(payload),
		});
	},

	rejectRecord: async (payload: ClientRecordReviewReq) => {
		return fetchApi<ClientRecordLifecycleResp>("/api/client/manage/reject", {
			method: "POST",
			body: JSON.stringify(payload),
		});
	},

	suspendRecord: async (payload: ClientRecordReviewReq) => {
		return fetchApi<ClientRecordLifecycleResp>("/api/client/manage/suspend", {
			method: "POST",
			body: JSON.stringify(payload),
		});
	},

	configDetails: async (identifier: string, doImport = false) => {
		const q = new URLSearchParams({ identifier, import: String(doImport) });
		const resp = await fetchApi<ClientConfigResp>(
			`/api/client/config/details?${q}`,
		);
		return extractApiData(resp);
	},

	update: async (payload: {
		identifier: string;
		display_name?: string;
		config_mode?: string;
		transport?: string;
		connection_mode?: string;
		config_path?: string;
		client_version?: string;
		supported_transports?: string[];
        description?: string;
        homepage_url?: string;
        docs_url?: string;
        support_url?: string;
        logo_url?: string;
	}) => {
		const resp = await fetchApi<
			{ success: boolean } & ApiWrapper<Record<string, unknown>>
		>(`/api/client/update`, {
			method: "POST",
			body: JSON.stringify(payload),
		});
		return resp;
	},

	applyConfig: async (payload: ClientConfigUpdateReq) => {
		const resp = await fetchApi<ClientConfigUpdateResp>(
			`/api/client/config/apply`,
			{
				method: "POST",
				body: JSON.stringify(payload),
			},
		);
		return extractApiData(resp);
	},

	restoreConfig: async (payload: ClientConfigRestoreReq) => {
		const resp = await fetchApi<ClientBackupActionResp>(
			`/api/client/config/restore`,
			{
				method: "POST",
				body: JSON.stringify(payload),
			},
		);
		return extractApiData(resp);
	},

	listBackups: async (identifier?: string) => {
		const q = new URLSearchParams();
		if (identifier) q.set("identifier", identifier);
		const resp = await fetchApi<ClientBackupListResp>(
			`/api/client/backups/list?${q}`,
		);
		return extractApiData(resp);
	},

	deleteBackup: async (identifier: string, backup: string) => {
		const resp = await fetchApi<ClientBackupActionResp>(
			`/api/client/backups/delete`,
			{
				method: "POST",
				body: JSON.stringify({ identifier, backup }),
			},
		);
		return extractApiData(resp);
	},

	getBackupPolicy: async (identifier: string) => {
		const q = new URLSearchParams({ identifier });
		const resp = await fetchApi<ClientBackupPolicyResp>(
			`/api/client/backups/policy?${q}`,
		);
		return extractApiData(resp);
	},

	setBackupPolicy: async (payload: ClientBackupPolicySetReq) => {
		const resp = await fetchApi<ClientBackupPolicyResp>(
			`/api/client/backups/policy`,
			{
				method: "POST",
				body: JSON.stringify(payload),
			},
		);
		return extractApiData(resp);
	},

	// Import servers from an existing client configuration (preview or apply)
	importFromClient: async (
		identifier: string,
		options?: { preview?: boolean; profile_id?: string | null },
	): Promise<ClientConfigImportData> => {
		const body: ClientConfigImportReq = { identifier };
		if (options && typeof options.preview === "boolean") {
			body.preview = options.preview;
		}
		if (options && "profile_id" in options) {
			body.profile_id = options.profile_id ?? null;
		}
		const resp = await fetchApi<ApiWrapper<ClientConfigImportData>>(
			`/api/client/config/import`,
			{
				method: "POST",
				body: JSON.stringify(body),
			},
		);
		return extractApiData(resp);
	},
};

// Token Estimate API
export const tokenEstimateApi = {
	getEstimate: async (profileId: string, capabilitySource?: string): Promise<TokenEstimateResponse> => {
		const params = new URLSearchParams({ profile_id: profileId });
		if (capabilitySource) {
			params.append("capability_source", capabilitySource);
		}
		const url = `/api/mcp/profile/token-estimate?${params}`;
		return fetchApi<TokenEstimateResponse>(url);
	},
};

/** Full capability JSON per profile row for client-side cl100k counting. */
export const capabilityTokenLedgerApi = {
	get: async (profileId: string): Promise<CapabilityTokenLedgerResponse> => {
		const params = new URLSearchParams({ profile_id: profileId });
		return fetchApi<CapabilityTokenLedgerResponse>(
			`/api/mcp/profile/capability-token-ledger?${params}`,
		);
	},
};

// React Query hook for token estimates
export function useTokenEstimate(
	profileId: string | undefined,
	capabilitySource?: string,
	refreshKey?: string,
) {
	return useQuery({
		queryKey: ["tokenEstimate", profileId, capabilitySource, refreshKey],
		queryFn: () => {
			if (!profileId) return Promise.resolve(null);
			return tokenEstimateApi.getEstimate(profileId, capabilitySource);
		},
		enabled: !!profileId,
		// Profile header / detail must reflect enable toggles immediately; server is cheap and authoritative.
		staleTime: 0,
		retry: 1,
	});
}

/**
 * Ledger JSON for client tokenizer; if the route is missing (404, older proxy), falls back to
 * GET token-estimate keyed by capability enable fingerprint so toggles still refresh the chart.
 */
export function useProfileTokenChartSource(
	profileId: string | undefined,
	enabledByComponentId: ReadonlyMap<string, boolean>,
) {
	const capabilityFingerprint = useMemo(
		() =>
			[...enabledByComponentId.entries()]
				.sort(([a], [b]) => a.localeCompare(b))
				.map(([id, on]) => `${id}:${on ? 1 : 0}`)
				.join("|"),
		[enabledByComponentId],
	);

	const ledgerQuery = useQuery({
		queryKey: ["capabilityTokenLedger", profileId],
		queryFn: async () => {
			if (!profileId) return null;
			return capabilityTokenLedgerApi.get(profileId);
		},
		enabled: !!profileId,
		staleTime: 5 * 60 * 1000,
		refetchOnWindowFocus: false,
		retry: (failureCount, error) => {
			if (isApiNotFoundError(error)) {
				return false;
			}
			return failureCount < 1;
		},
	});

	const ledgerMissingRoute =
		ledgerQuery.isError && isApiNotFoundError(ledgerQuery.error);

	const estimateQuery = useQuery({
		queryKey: ["profileChartTokenEstimate", profileId, capabilityFingerprint],
		queryFn: async () => {
			if (!profileId) return null;
			return tokenEstimateApi.getEstimate(profileId);
		},
		enabled: !!profileId && ledgerMissingRoute,
		staleTime: 0,
		retry: 1,
	});

	const isLoading =
		ledgerQuery.isPending || (ledgerMissingRoute && estimateQuery.isPending);
	const isError =
		(ledgerQuery.isError && !ledgerMissingRoute) ||
		(ledgerMissingRoute && estimateQuery.isError);

	const ledgerItems = ledgerQuery.data?.items;
	const fallbackEstimate = ledgerMissingRoute ? (estimateQuery.data ?? null) : null;

	return {
		ledgerItems,
		fallbackEstimate,
		isLoading,
		isError,
	};
}
