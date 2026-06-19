import type {
  RegistryOfficialMeta,
  RegistryServerEntry,
  ServerSummary,
} from "./types";
import { registryRef } from "./source";

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

function normalizeRegistryCandidate(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function collectRegistryCandidates(values: Array<string | null | undefined>): Set<string> {
  const candidates = new Set<string>();
  for (const value of values) {
    const normalized = normalizeRegistryCandidate(value);
    if (normalized) {
      candidates.add(normalized);
    }
  }
  return candidates;
}

function normalizeUrlCandidate(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  if (!trimmed) {
    return null;
  }

  try {
    const normalized = new URL(trimmed);
    normalized.hash = "";
    if ((normalized.protocol === "http:" || normalized.protocol === "https:") && normalized.pathname !== "/") {
      normalized.pathname = normalized.pathname.replace(/\/+$/, "") || "/";
    }
    if (!normalized.search) {
      normalized.search = "";
    }
    return normalized.toString().replace(/\/$/, "");
  } catch {
    return trimmed.replace(/\/$/, "");
  }
}

export function matchesInstalledRegistryServer(
  registryServer: RegistryServerEntry,
  installedServer: ServerSummary,
): boolean {
  const registryCandidates = collectRegistryCandidates([
    registryServer.name,
    getCanonicalRegistryServerId(registryServer),
  ]);

  if (registryCandidates.size === 0) {
    return false;
  }

  const installedOfficialMeta = installedServer.meta?._meta?.["io.modelcontextprotocol.registry/official"] as
    | RegistryOfficialMeta
    | undefined;
  const installedCandidates = collectRegistryCandidates([
    registryRef(installedServer.source),
    installedServer.name,
    installedOfficialMeta?.serverId,
  ]);

  for (const candidate of installedCandidates) {
    if (registryCandidates.has(candidate)) {
      return true;
    }
  }

  const installedUrl = normalizeUrlCandidate(installedServer.url);
  if (!installedUrl) {
    return false;
  }

  const registryRemoteUrls = (registryServer.remotes ?? [])
    .map((remote) => normalizeUrlCandidate(remote.url))
    .filter((value): value is string => Boolean(value));

  return registryRemoteUrls.includes(installedUrl);
}
