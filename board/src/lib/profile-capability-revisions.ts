import { ProfileSyncClientError } from "./profile-sync-error";
import type { CatalogRevisionSet } from "./types";

export interface CapabilityRevisionSource {
	selectedIds: string[];
	items: Array<{ id: string; server_id: string }>;
	sourceRevisionSet: CatalogRevisionSet | undefined;
}

export function requireSelectedCapabilityRevisionSet(
	sources: CapabilityRevisionSource[],
): CatalogRevisionSet {
	if (sources.length === 0) {
		throw new ProfileSyncClientError("catalog_snapshot_missing");
	}
	const selectedRevisions: CatalogRevisionSet = {};
	for (const { selectedIds, items, sourceRevisionSet } of sources) {
		if (!sourceRevisionSet) {
			throw new ProfileSyncClientError("catalog_snapshot_missing");
		}
		const serverIdByCapabilityId = new Map(
			items.map((item) => [item.id, item.server_id]),
		);
		for (const capabilityId of selectedIds) {
			const serverId = serverIdByCapabilityId.get(capabilityId);
			const revision = serverId ? sourceRevisionSet[serverId] : undefined;
			if (!serverId || revision === undefined) {
				throw new ProfileSyncClientError("catalog_snapshot_missing");
			}
			const selectedRevision = selectedRevisions[serverId];
			if (selectedRevision !== undefined && selectedRevision !== revision) {
				throw new ProfileSyncClientError("catalog_snapshot_mismatch");
			}
			selectedRevisions[serverId] = revision;
		}
	}
	return selectedRevisions;
}
