import type { ServerSummary } from "./types";

export function formatServerEndpoint(endpoint?: string | null): string | null {
  const value = endpoint?.trim();
  if (!value) {
    return null;
  }

  try {
    const url = new URL(value);
    url.username = "";
    url.password = "";
    url.search = "";
    url.hash = "";
    return url.toString().replace(/\/$/, url.pathname === "/" ? "" : "/");
  } catch {
    return null;
  }
}

export function formatServerNamespaceTitle(namespace: string): string {
  return namespace
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

export function getServerDisplayName(server: ServerSummary): string {
  const upstreamTitle = server.server_info?.title?.trim();
  if (upstreamTitle) {
    return upstreamTitle;
  }

  return formatServerNamespaceTitle(server.name);
}
