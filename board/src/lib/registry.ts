import { API_BASE_URL } from "./api";
import type {
  RegistryOfficialMeta,
  RegistryServerEntry,
  RegistryServerListResponse,
} from "./types";

export interface RegistryQueryOptions {
  limit?: number;
  cursor?: string;
  search?: string;
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

export function getOfficialMeta(
  server: RegistryServerEntry,
): RegistryOfficialMeta | undefined {
  return server?._meta?.["io.modelcontextprotocol.registry/official"];
}

export function buildRegistryServerKey(server: RegistryServerEntry): string {
  const official = getOfficialMeta(server);
  if (official?.serverId) return official.serverId;
  return `${server.name}@${server.version}`;
}
