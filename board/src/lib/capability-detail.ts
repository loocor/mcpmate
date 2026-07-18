import type { CapabilityRecord } from "../types/capabilities";

export function mergeCapabilityInspectorItem<T extends CapabilityRecord>(
	managementItem: T,
	protocolDetail: CapabilityRecord | null | undefined,
): T & CapabilityRecord {
	return {
		...managementItem,
		...(protocolDetail ?? {}),
	};
}

export function resolveCapabilityRawPayload<T>(
	managementItem: T,
	protocolDetail: CapabilityRecord | null | undefined,
	usesLazyDetails: boolean,
): T | CapabilityRecord | undefined {
	if (usesLazyDetails) {
		return protocolDetail ?? undefined;
	}
	return managementItem;
}
