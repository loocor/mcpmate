export type InspectorMode = "proxy" | "native";

export type InspectorCapabilityKind = "tool" | "resource" | "prompt" | "template";

type CapabilityRecordLike = Record<string, unknown>;

function stringField(record: CapabilityRecordLike, key: string): string | null {
	const value = record[key];
	return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function inspectorString(value: unknown): string | null {
	return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function toInspectorServerTitleCase(value: string): string {
	const trimmed = value.trim();
	if (!trimmed) {
		return trimmed;
	}

	return trimmed
		.split(/\s+/)
		.map((word) =>
			word.length === 0
				? word
				: word.charAt(0).toUpperCase() + word.slice(1).toLowerCase(),
		)
		.join(" ");
}

export function formatInspectorServerLabel(
	serverName?: string | null,
	serverId?: string | null,
): string {
	const raw = inspectorString(serverName) ?? inspectorString(serverId) ?? "";
	return toInspectorServerTitleCase(raw);
}

export function getInspectorRecordKey(
	record: CapabilityRecordLike,
	kind: InspectorCapabilityKind,
): string {
	switch (kind) {
		case "tool":
			return (
				stringField(record, "unique_name") ??
				stringField(record, "tool_name") ??
				stringField(record, "name") ??
				stringField(record, "id") ??
				""
			);
		case "prompt":
			return (
				stringField(record, "unique_name") ??
				stringField(record, "prompt_name") ??
				stringField(record, "name") ??
				stringField(record, "id") ??
				""
			);
		case "resource":
			return (
				stringField(record, "resource_uri") ??
				stringField(record, "uri") ??
				stringField(record, "name") ??
				stringField(record, "id") ??
				""
			);
		case "template":
			return (
				stringField(record, "uriTemplate") ??
				stringField(record, "uri_template") ??
				stringField(record, "name") ??
				stringField(record, "id") ??
				""
			);
	}
}

export function getInspectorInvocationName(
	record: CapabilityRecordLike,
	kind: InspectorCapabilityKind,
): string {
	return getInspectorRecordKey(record, kind);
}

export function getInspectorToolInvocationName(record: CapabilityRecordLike): string {
	return getInspectorRecordKey(record, "tool");
}
