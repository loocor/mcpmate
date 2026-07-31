import type { CapabilityKind } from "../components/capability-list";

export function capabilityKey(type: string, id: string): string {
	return `${type}:${id}`;
}

export function splitCapabilityKey(key: string): {
	capability_type: CapabilityKind;
	capability_id: string;
} {
	const separator = key.indexOf(":");
	return {
		capability_type: key.slice(0, separator) as CapabilityKind,
		capability_id: key.slice(separator + 1),
	};
}
