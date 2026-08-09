import { expect, mock, test } from "bun:test";

import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import * as Api from "../../lib/api";
import * as Notify from "../../lib/notify";
import * as ProfileSyncError from "../../lib/profile-sync-error";

const updateServer = mock(async () => ({
	data: {
		capability_discovery: {
			attempted: true,
			status: "failed" as const,
			error: "Capability discovery failed upstream",
		},
	},
}));
const getServer = mock(async () => ({
	id: "server-a",
	name: "server-a",
	server_type: "stdio",
	status: "idle",
	enabled: true,
	instances: [],
	source_revision_set: { tools: "tools-r1" },
}));
const enableServer = mock(async () => undefined);
const addServersToProfile = mock(async () => undefined);
const notifyError = mock(() => undefined);

mock.module("../../lib/api", () => ({
	...Api,
	addServersToProfile,
	serversApi: {
		...Api.serversApi,
		updateServer,
		getServer,
		enableServer,
	},
}));

mock.module("../../lib/notify", () => ({ ...Notify, notifyError }));

mock.module("../../lib/profile-sync-error", () => ProfileSyncError);

test("pending publish reports failed discovery while completing server and profile setup", async () => {
	const { finalizePendingPublishImport } = await import("./pending-publish");
	const invalidateQueries = mock(async () => undefined);
	const setImportResult = mock(() => undefined);
	const t = (key: string, options?: { defaultValue?: string; message?: string }) =>
		options?.defaultValue?.replace("{{message}}", options.message ?? "") ?? key;

	await finalizePendingPublishImport({
		draft: {
			name: "server-a",
			kind: "stdio",
			command: "node",
			args: [],
			env: {},
		} as ServerInstallDraft,
		publishedServerId: "server-a",
		targetProfileId: "profile-a",
		queryClient: { invalidateQueries } as never,
		installPipeline: { setImportResult },
		t,
	});

	expect(notifyError).toHaveBeenCalledWith(
		"Refresh failed",
		"Unable to refresh server capabilities: Capability discovery failed upstream",
	);
	expect(getServer).toHaveBeenCalledWith("server-a");
	expect(updateServer).toHaveBeenCalledWith(
		"server-a",
		expect.objectContaining({ pending_import: false }),
	);
	expect(enableServer).toHaveBeenCalledWith("server-a", { tools: "tools-r1" });
	expect(addServersToProfile).toHaveBeenCalledWith("profile-a", ["server-a"]);
	expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["servers"] });
	expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["configSuits"] });
	expect(setImportResult).toHaveBeenCalledWith({
		success: true,
		summary: { imported_count: 1, skipped_count: 0 },
		servers: { "server-a": { id: "server-a", status: "success" } },
	});
});
