import type { QueryClient } from "@tanstack/react-query";

import { addServersToProfile, serversApi } from "../../lib/api";
import { notifyCapabilityDiscoveryFailure } from "../../lib/capability-discovery-notice";
import { ProfileSyncClientError } from "../../lib/profile-sync-error";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { draftToServerConfig } from "./draft-to-server-config";

type PendingPublishInstallPipeline = {
	setImportResult: (result: {
		success: true;
		summary: { imported_count: number; skipped_count: number };
		servers: Record<string, { id: string; status: "success" }>;
	}) => void;
};

type Translate = (
	key: string,
	options?: { defaultValue?: string; message?: string },
) => string;

export async function finalizePendingPublishImport({
	draft,
	publishedServerId,
	targetProfileId,
	queryClient,
	installPipeline,
	t,
}: {
	draft: ServerInstallDraft;
	publishedServerId: string;
	targetProfileId: string | null;
	queryClient: QueryClient;
	installPipeline: PendingPublishInstallPipeline;
	t: Translate;
}) {
	const updateResponse = await serversApi.updateServer(
		publishedServerId,
		draftToServerConfig(draft, {
			pending_import: false,
		}),
	);
	notifyCapabilityDiscoveryFailure(updateResponse.data?.capability_discovery, t);
	const publishedServer = await serversApi.getServer(publishedServerId);
	const sourceRevisionSet = publishedServer.source_revision_set;
	if (!sourceRevisionSet) {
		throw new ProfileSyncClientError("capability_snapshot_missing");
	}
	await serversApi.enableServer(publishedServerId, sourceRevisionSet);
	if (targetProfileId) {
		await addServersToProfile(targetProfileId, [publishedServerId]);
	}
	await queryClient.invalidateQueries({ queryKey: ["servers"] });
	if (targetProfileId) {
		await queryClient.invalidateQueries({
			queryKey: ["configSuits"],
		});
	}
	installPipeline.setImportResult({
		success: true,
		summary: {
			imported_count: 1,
			skipped_count: 0,
		},
		servers: {
			[draft.name]: {
				id: publishedServerId,
				status: "success",
			},
		},
	});
}
