import type { RegistryServerEntryWrapper, RegistryServerListResponse } from "../../types";
import { getCanonicalRegistryServerId, getOfficialMeta } from "../../registry";
import type {
  CatalogEntry,
  CatalogPage,
  CatalogQuery,
  MarketCatalogProvider,
} from "../types";

const REGISTRY_API_BASE = import.meta.env.DEV
  ? "/registry-api"
  : "https://registry.modelcontextprotocol.io/v0.1";

/**
 * MCP Registry provider — fetches directly from the official MCP Registry.
 *
 * In dev mode the Vite proxy rewrites /registry-api → the real API.
 * In production builds (Tauri desktop etc.) the full URL is used directly.
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

    // Batch-prefetch: request 2× the needed limit upfront to compensate for
    // upstream duplicates, dedup in a single pass, then top up only if needed.
    const BATCH_MULTIPLIER = 2;
    const batchLimit = cappedLimit * BATCH_MULTIPLIER;

    const dedup = new Map<string, CatalogEntry>();
    let nextCursor: string | undefined = cursor;
    let upstreamTotalCount: number | undefined;

    const MAX_REQUESTS = 5;
    let requestsUsed = 0;

    const upsertEntry = (wrapper: RegistryServerEntryWrapper) => {
      const entry: CatalogEntry = {
        ...wrapper.server,
        _meta: wrapper._meta ?? wrapper.server._meta,
      };
      const key = getCanonicalRegistryServerId(entry);
      if (!dedup.has(key)) {
        dedup.set(key, entry);
      } else {
        const existing = dedup.get(key)!;
        const existingTs = getOfficialMeta(existing)?.updatedAt;
        const candidateTs = getOfficialMeta(entry)?.updatedAt;
        if (
          existingTs &&
          candidateTs &&
          Date.parse(candidateTs) > Date.parse(existingTs)
        ) {
          dedup.set(key, entry);
        }
      }
    };

    const doFetch = async (fetchLimit: number): Promise<string | undefined> => {
      const params = new URLSearchParams();
      params.set("limit", fetchLimit.toString());
      params.set("version", "latest");
      if (nextCursor) params.set("cursor", nextCursor);
      if (search?.trim()) params.set("search", search.trim());

      const requestUrl = `${REGISTRY_API_BASE}/servers?${params.toString()}`;
      const response = await fetch(requestUrl, {
        headers: { Accept: "application/json" },
      });

      if (!response.ok) {
        const text = await response.text().catch(() => "");
        throw new Error(
          `Registry request failed (${response.status} ${response.statusText}): ${text}`,
        );
      }

      const result = (await response.json()) as RegistryServerListResponse;
      if (upstreamTotalCount === undefined) {
        upstreamTotalCount = result.metadata?.count;
      }

      for (const wrapper of result.servers ?? []) {
        upsertEntry(wrapper);
      }

      return result.metadata?.nextCursor;
    };

    // First batch: request 2× limit to absorb duplicates in one pass.
    nextCursor = await doFetch(batchLimit);
    requestsUsed++;

    // Top-up loop: only if the first batch didn't yield enough unique entries.
    while (dedup.size < cappedLimit && nextCursor && requestsUsed < MAX_REQUESTS) {
      nextCursor = await doFetch(cappedLimit);
      requestsUsed++;
    }

    const entries = Array.from(dedup.values()).slice(0, cappedLimit);

    return {
      entries,
      nextCursor,
      totalCount: upstreamTotalCount,
    };
  }

  async fetchByKey(key: string): Promise<CatalogEntry | null> {
    const trimmed = key.trim();
    if (!trimmed) return null;

    const requestUrl = `${REGISTRY_API_BASE}/servers/${encodeURIComponent(trimmed)}/versions/latest`;
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

  buildSourceRef(entry: CatalogEntry): string {
    return `registry:${entry.name}`;
  }
}
