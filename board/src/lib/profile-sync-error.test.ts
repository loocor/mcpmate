import { afterEach, describe, expect, test } from "bun:test";

import i18n, { ensureI18n } from "./i18n/index";
import { ApiRequestError, ProfileAssociationError } from "./api";
import {
	ProfileSyncClientError,
	orchestratePendingPublish,
	profileSyncErrorTranslationKey,
} from "./profile-sync-error";

const originalLanguage = i18n.language;

afterEach(async () => {
	await i18n.changeLanguage(originalLanguage);
});

describe("Profile synchronization error copy", () => {
	test("maps stable client and API codes to translation keys", () => {
		expect(
			profileSyncErrorTranslationKey(
				new ProfileAssociationError("imported_server_missing"),
			),
		).toBe("profileSyncErrors.importedServerMissing");
		expect(
			profileSyncErrorTranslationKey(
				new ProfileAssociationError("imported_server_ambiguous"),
			),
		).toBe("profileSyncErrors.importedServerAmbiguous");
		expect(
			profileSyncErrorTranslationKey(
				new ProfileSyncClientError("catalog_snapshot_mismatch"),
			),
		).toBe("profileSyncErrors.catalogSnapshotMismatch");
		expect(
			profileSyncErrorTranslationKey(
				new ApiRequestError(
					"Profile changed",
					409,
					"profile_authoring_changed",
				),
			),
		).toBe("profileSyncErrors.profileAuthoringChanged");
		expect(
			profileSyncErrorTranslationKey(
				new ApiRequestError(
					"Catalog changed",
					409,
					"catalog_dependency_changed",
				),
			),
		).toBe("profileSyncErrors.catalogDependencyChanged");
		expect(profileSyncErrorTranslationKey(new Error("backend English"))).toBe(
			"profileSyncErrors.unexpected",
		);
	});

	test("returns a retryable failure without running success continuation", async () => {
		let finalizerCalls = 0;
		let continuationCalls = 0;
		let successCalls = 0;
		let clearCalls = 0;
		const result = await orchestratePendingPublish(
			"server-pending",
			async () => {
				finalizerCalls += 1;
				throw new ProfileSyncClientError("capability_snapshot_missing");
			},
		);
		if (result.status === "success") {
			successCalls += 1;
			clearCalls += 1;
			continuationCalls += 1;
		}

		expect(result).toEqual({
			status: "retryable_failure",
			pendingImportServerId: "server-pending",
			notificationKey: "profileSyncErrors.capabilitySnapshotMissing",
		});
		expect(finalizerCalls).toBe(1);
		expect(continuationCalls).toBe(0);
		expect(successCalls).toBe(0);
		expect(clearCalls).toBe(0);
	});

	test("returns success or no_pending without duplicate finalization", async () => {
		let finalizerCalls = 0;
		expect(
			await orchestratePendingPublish("server-pending", async () => {
				finalizerCalls += 1;
			}),
		).toEqual({
			status: "success",
			pendingImportServerId: "server-pending",
		});
		expect(
			await orchestratePendingPublish(null, async () => {
				finalizerCalls += 1;
			}),
		).toEqual({ status: "no_pending" });
		expect(finalizerCalls).toBe(1);
	});

	test("provides localized copy for every supported locale", async () => {
		await ensureI18n();
		for (const locale of ["en", "zh-CN", "ja-JP"] as const) {
			await i18n.changeLanguage(locale);
			for (const key of [
				"profileSyncErrors.importedServerMissing",
				"profileSyncErrors.importedServerAmbiguous",
				"profileSyncErrors.catalogSnapshotMismatch",
				"profileSyncErrors.profileAuthoringChanged",
				"profileSyncErrors.unexpected",
			]) {
				expect(i18n.t(key)).not.toBe(key);
			}
		}
	});
});
