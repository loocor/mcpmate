import { API_BASE_URL } from "./api";
import type {
  RegistryOfficialMeta,
  RegistryServerEntry,
  RegistryServerListResponse,
} from "./types";

export interface CachedRegistryServerListResponse
  extends RegistryServerListResponse {
  last_synced_at?: string;
}

export interface RegistrySyncResponse {
  success: boolean;
  updatedCount: number;
  lastSyncedAt: string;
}

export interface RegistryQueryOptions {
  limit?: number;
  cursor?: string;
  search?: string;
}

export async function installServer(serverName: string, version?: string): Promise<{ success: boolean; message?: string }> {
  const response = await fetch(`${API_BASE_URL}/api/mcp/registry/install`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({ name: serverName, version: version ?? "latest" }),
    credentials: "include",
  });

  if (!response.ok) {
    const text = await response.text().catch(() => "");
    throw new Error(`Install failed (${response.status}): ${text}`);
  }

  return (await response.json()) as { success: boolean; message?: string };
}

export async function fetchRegistryServers(
  options: RegistryQueryOptions = {},
): Promise<RegistryServerListResponse> {
  const { limit = 30, cursor, search } = options;
  const params = new URLSearchParams();
  params.set("limit", Math.max(1, Math.min(limit, 100)).toString());
  params.set("version", "latest");
  if (cursor) params.set("cursor", cursor);
  if (search?.trim()) params.set("search", search.trim());

  const requestUrl = `${API_BASE_URL}/api/mcp/registry/servers?${params.toString()}`;

  const response = await fetch(requestUrl, {
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
  });

  if (!response.ok) {
    const text = await response.text().catch(() => "");
    throw new Error(
      `Registry request failed (${response.status} ${response.statusText}): ${text}`,
    );
  }

  return (await response.json()) as RegistryServerListResponse;
}

export async function fetchCachedRegistryServers(
  options: RegistryQueryOptions = {},
): Promise<CachedRegistryServerListResponse> {
  const { limit = 30, cursor, search } = options;
  const params = new URLSearchParams();
  params.set("limit", Math.max(1, Math.min(limit, 100)).toString());
  if (cursor) params.set("cursor", cursor);
  if (search?.trim()) params.set("search", search.trim());

  const requestUrl = `${API_BASE_URL}/api/mcp/registry/servers/cached?${params.toString()}`;

  const response = await fetch(requestUrl, {
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
  });

  if (!response.ok) {
    const text = await response.text().catch(() => "");
    throw new Error(
      `Cached registry request failed (${response.status} ${response.statusText}): ${text}`,
    );
  }

  return (await response.json()) as CachedRegistryServerListResponse;
}

export async function fetchCachedRegistryServerByKey(
  key: string,
): Promise<RegistryServerEntry | null> {
  const trimmed = key.trim();
  if (!trimmed) return null;

  const result = await fetchCachedRegistryServers({ search: trimmed, limit: 50 });
  const match = result.servers
    .map((entry) => ({
      ...entry.server,
      _meta: entry._meta ?? entry.server._meta,
    }))
    .find((server) => buildRegistryServerKey(server) === trimmed || server.name === trimmed);

  return match ?? null;
}

export async function syncRegistry(): Promise<RegistrySyncResponse> {
  const response = await fetch(`${API_BASE_URL}/api/mcp/registry/sync`, {
    method: "POST",
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
  });

  if (!response.ok) {
    const text = await response.text().catch(() => "");
    throw new Error(
      `Registry sync failed (${response.status} ${response.statusText}): ${text}`,
    );
  }

  return (await response.json()) as RegistrySyncResponse;
}

export function getOfficialMeta(
  server: RegistryServerEntry,
): RegistryOfficialMeta | undefined {
  return server?._meta?.["io.modelcontextprotocol.registry/official"];
}

export function getCanonicalRegistryServerId(server: RegistryServerEntry): string {
  const canonicalName = (server.name ?? "").trim();
  const officialServerId = getOfficialMeta(server)?.serverId?.trim();
  if (officialServerId && officialServerId === canonicalName) {
    return officialServerId;
  }
  return canonicalName;
}

export function buildRegistryServerKey(server: RegistryServerEntry): string {
  return getCanonicalRegistryServerId(server);
}
