import type { RegistryServerListResponse } from "../../types";
import { getCanonicalRegistryServerId } from "../../registry";
import type {
  CatalogEntry,
  CatalogPage,
  CatalogQuery,
  MarketCatalogProvider,
} from "../types";

/**
 * MCP Registry provider — fetches directly from the official MCP Registry.
 *
 * No backend proxy needed. The Vite dev server proxies /registry-api to
 * avoid CORS issues; production deployments use an nginx/CF Worker proxy.
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
    const params = new URLSearchParams();
    // Official registry caps at 100; higher values return empty results.
    params.set("limit", Math.max(1, Math.min(limit, 100)).toString());
    if (cursor) params.set("cursor", cursor);
    if (search?.trim()) params.set("search", search.trim());

    const requestUrl = `/registry-api/servers?${params.toString()}`;
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

    const entries: CatalogEntry[] = (result.servers ?? []).map((wrapper) => ({
      ...wrapper.server,
      _meta: wrapper._meta ?? wrapper.server._meta,
    }));

    return {
      entries,
      nextCursor: result.metadata?.nextCursor,
      totalCount: result.metadata?.count,
    };
  }

  async fetchByKey(key: string): Promise<CatalogEntry | null> {
    const trimmed = key.trim();
    if (!trimmed) return null;

    // Paginate through search results to find the exact match.
    // The official registry caps at 100 per page.
    let cursor: string | undefined;
    for (let page = 0; page < 10; page++) {
      const result = await this.fetchPage({
        search: trimmed,
        limit: 100,
        cursor,
      });

      const match = result.entries.find(
        (entry) =>
          getCanonicalRegistryServerId(entry) === trimmed ||
          entry.name === trimmed,
      );
      if (match) return match;
      if (!result.nextCursor) break;
      cursor = result.nextCursor;
    }

    return null;
  }

  buildSourceRef(entry: CatalogEntry): string {
    return `registry:${entry.name}`;
  }
}
