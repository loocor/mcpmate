import { clientsApi } from "./api";
import type {
	ClientBackupPolicyPayload,
	ClientCapabilityConfigData,
	ClientConfigMode,
	ClientConfigSelected,
} from "./types";

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
	if (
		capabilityData?.capability_source === "custom" &&
		capabilityData.custom_profile_id
	) {
		return { profile: { profile_id: capabilityData.custom_profile_id } };
	}
	return "default";
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
