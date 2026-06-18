import type { CatalogEntry } from "../types";
import { getCanonicalRegistryServerId, getOfficialMeta } from "../../registry";

export const MCP_REGISTRY_MAX_REQUESTS = 5;

export interface RegistryUpstreamPage {
	servers: Array<{ server: CatalogEntry; _meta?: CatalogEntry["_meta"] }>;
	metadata?: {
		nextCursor?: string | null;
		count?: number;
	};
}

type RegistryServerWrapper = RegistryUpstreamPage["servers"][number];

function wrapperToCatalogEntry(wrapper: RegistryServerWrapper): CatalogEntry {
	return {
		...wrapper.server,
		_meta: wrapper._meta ?? wrapper.server._meta,
	};
}

export function mergeRegistryServerWrappers(
	dedup: Map<string, CatalogEntry>,
	wrappers: RegistryUpstreamPage["servers"],
): void {
	for (const wrapper of wrappers ?? []) {
		const entry = wrapperToCatalogEntry(wrapper);
		const key = getCanonicalRegistryServerId(entry);
		if (!dedup.has(key)) {
			dedup.set(key, entry);
			continue;
		}

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
}

export function resolveCatalogPageNextCursor(params: {
	entries: CatalogEntry[];
	cappedLimit: number;
	upstreamNextCursor: string | undefined;
	hasMoreUniqueEntriesAhead: boolean;
}): string | undefined {
	const { entries, cappedLimit, upstreamNextCursor, hasMoreUniqueEntriesAhead } = params;

	if (
		entries.length < cappedLimit ||
		!upstreamNextCursor ||
		!hasMoreUniqueEntriesAhead
	) {
		return undefined;
	}

	return upstreamNextCursor;
}

export function upstreamPageHasUnseenEntries(
	page: RegistryUpstreamPage,
	seenKeys: ReadonlySet<string>,
): boolean {
	for (const wrapper of page.servers ?? []) {
		const key = getCanonicalRegistryServerId(wrapperToCatalogEntry(wrapper));
		if (!seenKeys.has(key)) {
			return true;
		}
	}
	return false;
}
