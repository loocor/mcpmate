import { ApiRequestError, ProfileAssociationError } from "./api";

export type ProfileSyncClientErrorCode =
	| "profile_authoring_state_missing"
	| "profile_authoring_state_mismatch"
	| "catalog_snapshot_missing"
	| "catalog_snapshot_mismatch"
	| "capability_snapshot_missing";

export class ProfileSyncClientError extends Error {
	readonly code: ProfileSyncClientErrorCode;

	constructor(code: ProfileSyncClientErrorCode) {
		super(code);
		this.name = "ProfileSyncClientError";
		this.code = code;
	}
}

const CLIENT_ERROR_KEYS: Record<ProfileSyncClientErrorCode, string> = {
	profile_authoring_state_missing:
		"profileSyncErrors.profileAuthoringStateMissing",
	profile_authoring_state_mismatch:
		"profileSyncErrors.profileAuthoringStateMismatch",
	catalog_snapshot_missing: "profileSyncErrors.catalogSnapshotMissing",
	catalog_snapshot_mismatch: "profileSyncErrors.catalogSnapshotMismatch",
	capability_snapshot_missing: "profileSyncErrors.capabilitySnapshotMissing",
};

const ASSOCIATION_ERROR_KEYS = {
	imported_server_missing: "profileSyncErrors.importedServerMissing",
	imported_server_ambiguous: "profileSyncErrors.importedServerAmbiguous",
} as const;

const API_ERROR_KEYS: Record<string, string> = {
	profile_authoring_changed: "profileSyncErrors.profileAuthoringChanged",
	catalog_dependency_changed: "profileSyncErrors.catalogDependencyChanged",
	consumer_binding_changed: "profileSyncErrors.consumerBindingChanged",
	invalid_target: "profileSyncErrors.invalidTarget",
};

export function profileSyncErrorTranslationKey(error: unknown): string {
	if (error instanceof ProfileAssociationError) {
		return ASSOCIATION_ERROR_KEYS[error.code];
	}
	if (error instanceof ProfileSyncClientError) {
		return CLIENT_ERROR_KEYS[error.code];
	}
	if (error instanceof ApiRequestError && error.code) {
		return API_ERROR_KEYS[error.code] ?? "profileSyncErrors.unexpected";
	}
	return "profileSyncErrors.unexpected";
}

export type PendingPublishResult =
	| { status: "no_pending" }
	| { status: "success"; pendingImportServerId: string }
	| {
			status: "retryable_failure";
			pendingImportServerId: string;
			notificationKey: string;
	  };

export async function orchestratePendingPublish(
	pendingImportServerId: string | null,
	finalize: (pendingImportServerId: string) => Promise<void>,
): Promise<PendingPublishResult> {
	if (!pendingImportServerId) {
		return { status: "no_pending" };
	}
	try {
		await finalize(pendingImportServerId);
		return { status: "success", pendingImportServerId };
	} catch (error) {
		return {
			status: "retryable_failure",
			pendingImportServerId,
			notificationKey: profileSyncErrorTranslationKey(error),
		};
	}
}
