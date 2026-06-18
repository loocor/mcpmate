import type { RegistryServerEntry } from "../types";

/** Canonical catalog entry type — all providers must return this shape. */
export type CatalogEntry = RegistryServerEntry;

/** Opaque pagination cursor. */
export type CatalogCursor = string;

/** Query parameters for catalog browsing. */
export interface CatalogQuery {
  search?: string;
  limit?: number;
  cursor?: CatalogCursor;
}

/** Paginated catalog response. */
export interface CatalogPage {
  entries: CatalogEntry[];
  nextCursor?: CatalogCursor;
  totalCount?: number;
  lastSyncedAt?: string;
}

/** Provider metadata for UI rendering. */
export interface CatalogProviderMeta {
  id: string;
  displayName: string;
  description?: string;
  supportsSync?: boolean;
}

/**
 * MarketCatalogProvider interface.
 *
 * Each catalog source (MCP Registry, admin catalog, etc.) implements this.
 * The market page and install wizard operate only on CatalogEntry.
 */
export interface MarketCatalogProvider {
  readonly meta: CatalogProviderMeta;

  /** Fetch a page of catalog entries. */
  fetchPage(query: CatalogQuery): Promise<CatalogPage>;

  /** Fetch a single entry by its catalog-local key. */
  fetchByKey(key: string): Promise<CatalogEntry | null>;

  /** Optional: trigger a background sync. */
  sync?(): Promise<void>;

  /**
   * Build a source_ref string for a catalog entry.
   * Default pattern: "{providerId}:{entry.name}"
   */
  buildSourceRef(entry: CatalogEntry): string;
}
