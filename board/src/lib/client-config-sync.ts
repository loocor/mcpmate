import type { TFunction } from "i18next";
import { clientsApi } from "./api";
import type {
	ClientBackupPolicyPayload,
	ClientCapabilityConfigData,
	ClientConfigMode,
	ClientConfigSelected,
} from "./types";

const CUSTOM_PROFILE_MISSING_ERROR = "customProfileMissing";

export function resolveClientConfigMode(
	value: string | null | undefined,
): ClientConfigMode | null {
	if (value === "unify" || value === "hosted" || value === "transparent") {
		return value;
	}
	return null;
}

export function buildClientApplySelectedConfig(
	capabilityData: ClientCapabilityConfigData | null,
): ClientConfigSelected {
	if (capabilityData?.capability_source === "custom") {
		if (capabilityData.custom_profile_missing || !capabilityData.custom_profile_id) {
			throw new Error(CUSTOM_PROFILE_MISSING_ERROR);
		}
		return { profile: { profile_id: capabilityData.custom_profile_id } };
	}
	return "default";
}

function isCustomProfileMissingError(error: unknown): boolean {
	if (error instanceof Error) {
		return error.message === CUSTOM_PROFILE_MISSING_ERROR;
	}
	return typeof error === "string" && error === CUSTOM_PROFILE_MISSING_ERROR;
}

function resolveCustomProfileMissingMessage(t: TFunction): string {
	return t("detail.configuration.errors.customProfileMissing", {
		defaultValue:
			"The client-specific custom workspace is missing. Create it again before applying configuration.",
	});
}

export function resolveClientConfigSyncErrorMessage(
	error: unknown,
	t: TFunction,
): string {
	if (isCustomProfileMissingError(error)) {
		return resolveCustomProfileMissingMessage(t);
	}

	if (error instanceof Error && error.message.trim()) {
		return error.message;
	}

	return String(error);
}

export function canApplyClientConfigWithState(input: {
	mode: ClientConfigMode | null;
	writableConfig: boolean | null | undefined;
	approvalStatus?: string | null;
}): boolean {
	if (!input.mode) return false;
	if (input.writableConfig === false) return false;
	return (
		input.approvalStatus !== "pending" &&
		input.approvalStatus !== "rejected" &&
		input.approvalStatus !== "suspended"
	);
}

export async function applyClientConfigWithResolvedSelection(input: {
	identifier: string;
	mode: ClientConfigMode;
	backupPolicy?: ClientBackupPolicyPayload;
	capabilityData?: ClientCapabilityConfigData | null;
}): Promise<void> {
	const capabilityData =
		input.capabilityData ?? (await clientsApi.getCapabilityConfig(input.identifier));
	const selectedConfig =
		input.mode === "unify"
			? "default"
			: buildClientApplySelectedConfig(capabilityData ?? null);

	await clientsApi.applyConfig({
		identifier: input.identifier,
		mode: input.mode,
		selected_config: selectedConfig,
		preview: false,
		backup_policy: input.backupPolicy,
	});
}
