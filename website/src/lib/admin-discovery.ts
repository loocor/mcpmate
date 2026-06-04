import { COMPATIBLE_CLIENTS_FALLBACK } from "../data/compatible-clients-fallback";

const DEFAULT_ADMIN_DISCOVERY_BASE_URL = "https://public.mcp.umate.ai";
const MAX_CLIENT_LIMIT = 50;
const DISCOVERY_FETCH_TIMEOUT_MS = 10_000;
const CLIENT_PRESETS_CACHE_KEY = "mcpmate:website-client-presets";
const CLIENT_PRESETS_CACHE_TTL_MS = 1000 * 60 * 60 * 12;

export const ADMIN_DISCOVERY_BASE_URL = (
	(typeof import.meta !== "undefined" &&
		import.meta.env?.VITE_ADMIN_API_BASE_URL &&
		String(import.meta.env.VITE_ADMIN_API_BASE_URL).trim()) ||
	DEFAULT_ADMIN_DISCOVERY_BASE_URL
).replace(/\/+$/, "");

export interface WebsiteClientPreset {
	identifier: string;
	displayName: string;
	logoUrl: string;
	homepageUrl: string;
}

export interface WebsiteDiscoveryPortal {
	id: string;
	title: string;
	description: string;
	url: string;
	source: string;
	signal: string;
	meta: string;
	iconUrl: string;
}

function recordValue(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function compactString(value: unknown): string | undefined {
	return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function firstCompactString(...values: unknown[]): string {
	for (const value of values) {
		const compact = compactString(value);
		if (compact) return compact;
	}
	return "";
}

function stringArrayValue(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

export function parseClientPresetForDisplay(raw: unknown): WebsiteClientPreset | null {
	const client = recordValue(raw);
	const identifier =
		compactString(client.identifier) ?? compactString(client.id) ?? compactString(client.name);
	if (!identifier) return null;

	const metadata = recordValue(client.metadata);
	const displayName =
		firstCompactString(client.displayName, client.display_name, metadata.display_name) || identifier;
	const icon = recordValue(client.icon);
	const links = recordValue(client.links);
	const logoUrl = firstCompactString(icon.url, client.logoUrl, client.logo_url, metadata.logo_url);
	const homepageUrl = firstCompactString(links.homepage, client.homepageUrl, client.homepage_url, metadata.homepage_url);

	return {
		identifier,
		displayName,
		logoUrl,
		homepageUrl,
	};
}

export type WebsiteClientPresetsSource = "remote" | "cache" | "fallback";

export interface WebsiteClientPresetsResult {
	clients: WebsiteClientPreset[];
	source: WebsiteClientPresetsSource;
}

interface CachedClientPresets {
	storedAt: number;
	clients: WebsiteClientPreset[];
}

function readCachedClientPresets(): WebsiteClientPreset[] | null {
	if (typeof sessionStorage === "undefined") {
		return null;
	}

	try {
		const raw = sessionStorage.getItem(CLIENT_PRESETS_CACHE_KEY);
		if (!raw) {
			return null;
		}

		const parsed = JSON.parse(raw) as CachedClientPresets;
		if (!Array.isArray(parsed.clients) || parsed.clients.length === 0) {
			return null;
		}

		if (Date.now() - parsed.storedAt > CLIENT_PRESETS_CACHE_TTL_MS) {
			return null;
		}

		return parsed.clients;
	} catch {
		return null;
	}
}

function writeCachedClientPresets(clients: WebsiteClientPreset[]): void {
	if (typeof sessionStorage === "undefined") {
		return;
	}

	try {
		const payload: CachedClientPresets = { storedAt: Date.now(), clients };
		sessionStorage.setItem(CLIENT_PRESETS_CACHE_KEY, JSON.stringify(payload));
	} catch {
		// Ignore quota or private-mode storage errors.
	}
}

function normalizeClientList(clients: WebsiteClientPreset[]): WebsiteClientPreset[] {
	return [...clients].sort((left, right) => left.displayName.localeCompare(right.displayName));
}

export async function fetchWebsiteClientPresets(limit = MAX_CLIENT_LIMIT): Promise<WebsiteClientPreset[]> {
	const url = new URL("/discovery/clients", `${ADMIN_DISCOVERY_BASE_URL}/`);
	url.searchParams.set("limit", String(Math.min(limit, MAX_CLIENT_LIMIT)));

	const init: RequestInit = { credentials: "omit" };
	let timeout: ReturnType<typeof window.setTimeout> | undefined;

	if (typeof window !== "undefined") {
		const controller = new AbortController();
		timeout = window.setTimeout(() => controller.abort(), DISCOVERY_FETCH_TIMEOUT_MS);
		init.signal = controller.signal;
	}

	let response: Response;
	try {
		response = await fetch(url.toString(), init);
	} finally {
		if (timeout !== undefined) {
			window.clearTimeout(timeout);
		}
	}

	if (!response.ok) {
		throw new Error(`Admin discovery request failed with HTTP ${response.status}`);
	}

	const envelope = (await response.json()) as unknown;
	const clients = recordValue(envelope).clients;
	if (!Array.isArray(clients)) {
		throw new Error("Admin discovery response is missing clients array");
	}

	const parsed = clients
		.map(parseClientPresetForDisplay)
		.filter((client): client is WebsiteClientPreset => client !== null);

	if (parsed.length === 0) {
		throw new Error("Admin discovery response contained no displayable clients");
	}

	return normalizeClientList(parsed);
}

export function parseDiscoveryPortalForDisplay(raw: unknown): WebsiteDiscoveryPortal | null {
	const portal = recordValue(raw);
	const metadata = recordValue(portal.metadata);
	const discovery = recordValue(
		recordValue(portal._meta)["ai.mcpmate/discovery"] ?? metadata.discovery,
	);
	const links = recordValue(portal.links);
	const icon = recordValue(portal.icon);
	const id = firstCompactString(portal.id, portal.identifier, portal.slug, portal.title, portal.name);
	const title = firstCompactString(portal.title, portal.name, metadata.title);
	const url = firstCompactString(portal.url, links.homepage, links.website, metadata.url);

	if (!id || !title || !url) {
		return null;
	}

	const categories = stringArrayValue(portal.categories ?? metadata.categories ?? discovery.categories);

	return {
		id,
		title,
		description: firstCompactString(portal.description, metadata.description),
		url,
		source: firstCompactString(portal.source, discovery.source),
		signal: firstCompactString(portal.signal, recordValue(discovery.quality).status),
		meta: firstCompactString(portal.meta, discovery.category, metadata.category, categories.slice(0, 2).join(", ")),
		iconUrl: firstCompactString(icon.url, portal.iconUrl, portal.icon_url, metadata.logo_url),
	};
}

export async function fetchWebsiteDiscoveryPortals(): Promise<WebsiteDiscoveryPortal[]> {
	const url = new URL("/discovery/portals", `${ADMIN_DISCOVERY_BASE_URL}/`);
	const response = await fetch(url.toString(), { credentials: "omit" });
	if (!response.ok) {
		throw new Error(`Admin discovery request failed with HTTP ${response.status}`);
	}

	const envelope = (await response.json()) as unknown;
	const portals = recordValue(envelope).portals;
	if (!Array.isArray(portals)) {
		throw new Error("Admin discovery response is missing portals array");
	}

	return portals
		.map(parseDiscoveryPortalForDisplay)
		.filter((portal): portal is WebsiteDiscoveryPortal => portal !== null)
		.sort((left, right) => left.title.localeCompare(right.title));
}

/** Remote catalog with session cache and built-in fallback for the marketing client wall. */
export async function loadWebsiteClientPresets(limit = MAX_CLIENT_LIMIT): Promise<WebsiteClientPresetsResult> {
	try {
		const clients = await fetchWebsiteClientPresets(limit);
		writeCachedClientPresets(clients);
		return { clients, source: "remote" };
	} catch {
		const cached = readCachedClientPresets();
		if (cached) {
			return { clients: normalizeClientList(cached), source: "cache" };
		}

		return { clients: normalizeClientList(COMPATIBLE_CLIENTS_FALLBACK), source: "fallback" };
	}
}
