import type { RegistryServerEntryWrapper, RegistryServerListResponse, ServerSource } from "../../types";
import { getCanonicalRegistryServerId } from "../../registry";
import type {
	CatalogEntry,
	CatalogPage,
	CatalogQuery,
	MarketCatalogProvider,
} from "../types";
import {
	MCP_REGISTRY_MAX_REQUESTS,
	mergeRegistryServerWrappers,
	resolveCatalogPageNextCursor,
	upstreamPageHasUnseenEntries,
	type RegistryUpstreamPage,
} from "./mcp-registry-pagination";

import { isTauriEnvironmentSync } from "../../platform";

const REGISTRY_API_BASE = import.meta.env.DEV
	? "/registry-api"
	: "https://registry.modelcontextprotocol.io/v0.1";

const CONFIGURED_REGISTRY_API_BASE =
	typeof import.meta.env.VITE_MARKET_REGISTRY_BASE_URL === "string" &&
	import.meta.env.VITE_MARKET_REGISTRY_BASE_URL.trim().length > 0
		? import.meta.env.VITE_MARKET_REGISTRY_BASE_URL.trim()
		: null;

const EFFECTIVE_REGISTRY_API_BASE = CONFIGURED_REGISTRY_API_BASE ?? REGISTRY_API_BASE;

function shouldLogRegistryTraffic(): boolean {
	return import.meta.env.DEV || isTauriEnvironmentSync();
}

function logRegistryRequest(requestUrl: string): void {
	if (shouldLogRegistryTraffic()) {
		console.info("[mcp-registry-provider] requestUrl:", requestUrl);
	}
}

/**
 * MCP Registry provider — fetches through the configured registry API base.
 *
 * In dev mode the Vite proxy rewrites /registry-api to the official API.
 * Production web deployments should serve the same path through nginx, Pages,
 * Workers, or a deployment-specific VITE_MARKET_REGISTRY_BASE_URL.
 */
export class McpRegistryProvider implements MarketCatalogProvider {
	readonly meta = {
		id: "mcp-registry",
		displayName: "MCP Registry",
		description: "Official Model Context Protocol server registry",
		supportsSync: false,
	} as const;

	async fetchPage(query: CatalogQuery): Promise<CatalogPage> {
		const { limit = 30, cursor, search } = query;
		const cappedLimit = Math.max(1, Math.min(limit, 100));
		const trimmedSearch = search?.trim() || undefined;

		const dedup = new Map<string, CatalogEntry>();
		let upstreamNextCursor: string | undefined = cursor;
		let upstreamTotalCount: number | undefined;

		for (let attempt = 0; attempt < MCP_REGISTRY_MAX_REQUESTS; attempt++) {
			const result = await this.fetchUpstreamPage({
				cursor: upstreamNextCursor,
				limit: cappedLimit,
				search: trimmedSearch,
			});
			if (upstreamTotalCount === undefined) {
				upstreamTotalCount = result.metadata?.count;
			}

			mergeRegistryServerWrappers(dedup, result.servers ?? []);
			upstreamNextCursor = result.metadata?.nextCursor ?? undefined;
			if (dedup.size >= cappedLimit || !upstreamNextCursor) {
				break;
			}
		}

		const entries = Array.from(dedup.values()).slice(0, cappedLimit);
		const seenKeys = new Set(entries.map((entry) => getCanonicalRegistryServerId(entry)));
		const shouldPeekAhead = entries.length === cappedLimit && Boolean(upstreamNextCursor);
		const hasMoreUniqueEntriesAhead = shouldPeekAhead
			? await this.hasMoreUnseenEntries(
				upstreamNextCursor!,
				seenKeys,
				trimmedSearch,
				cappedLimit,
			)
			: false;

		return {
			entries,
			nextCursor: resolveCatalogPageNextCursor({
				entries,
				cappedLimit,
				upstreamNextCursor,
				hasMoreUniqueEntriesAhead,
			}),
			totalCount: upstreamTotalCount,
		};
	}

	async fetchByKey(key: string): Promise<CatalogEntry | null> {
		const trimmed = key.trim();
		if (!trimmed) return null;

		const requestUrl = `${EFFECTIVE_REGISTRY_API_BASE}/servers/${encodeURIComponent(trimmed)}/versions/latest`;
		logRegistryRequest(requestUrl);
		const response = await fetch(requestUrl, {
			headers: { Accept: "application/json" },
		});

		if (response.status === 404) {
			return null;
		}

		if (!response.ok) {
			const text = await response.text().catch(() => "");
			throw new Error(
				`Registry request failed (${response.status} ${response.statusText}): ${text}`,
			);
		}

		const result = (await response.json()) as RegistryServerEntryWrapper;
		return {
			...result.server,
			_meta: result._meta ?? result.server._meta,
		};
	}

	buildSource(entry: CatalogEntry): ServerSource {
		return { type: "registry", ref: entry.name };
	}

	private async hasMoreUnseenEntries(
		startCursor: string,
		seenKeys: ReadonlySet<string>,
		search: string | undefined,
		limit: number,
	): Promise<boolean> {
		let cursor: string | undefined = startCursor;

		for (let attempt = 0; attempt < MCP_REGISTRY_MAX_REQUESTS; attempt++) {
			const result = await this.fetchUpstreamPage({ cursor, limit, search });
			if (upstreamPageHasUnseenEntries(result, seenKeys)) {
				return true;
			}

			cursor = result.metadata?.nextCursor ?? undefined;
			if (!cursor) {
				return false;
			}
		}

		return false;
	}

	private async fetchUpstreamPage(params: {
		cursor?: string;
		limit: number;
		search?: string;
	}): Promise<RegistryUpstreamPage> {
		const requestParams = new URLSearchParams();
		requestParams.set("limit", params.limit.toString());
		requestParams.set("version", "latest");
		if (params.cursor) {
			requestParams.set("cursor", params.cursor);
		}
		if (params.search) {
			requestParams.set("search", params.search);
		}

		const requestUrl = `${EFFECTIVE_REGISTRY_API_BASE}/servers?${requestParams.toString()}`;
		logRegistryRequest(requestUrl);
		const response = await fetch(requestUrl, {
			headers: { Accept: "application/json" },
		});

		if (!response.ok) {
			const text = await response.text().catch(() => "");
			throw new Error(
				`Registry request failed (${response.status} ${response.statusText}): ${text}`,
			);
		}

		return (await response.json()) as RegistryServerListResponse;
	}
}
