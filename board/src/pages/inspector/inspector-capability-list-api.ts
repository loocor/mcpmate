import { inspectorApi } from "../../lib/api";
import type { InspectorNativeTargetRequest } from "../../lib/hooks/use-inspector-native-session";
import type { CapabilityRecord } from "../../types/capabilities";
import type {
	InspectorCapabilityFamily,
	InspectorCapabilityListItem,
} from "./inspector-feature-config";

type InspectorListResponse = {
	success?: boolean;
	data?: {
		tools?: unknown[];
		prompts?: unknown[];
		resources?: unknown[];
		templates?: unknown[];
		tasks?: unknown[];
	} | null;
	error?: unknown;
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

const toCapabilityRecord = (value: unknown): CapabilityRecord | null =>
	isRecord(value) ? (value as CapabilityRecord) : null;

const toStringValue = (value: unknown): string | undefined =>
	typeof value === "string" && value.trim().length > 0 ? value : undefined;

function toSchemaObject(value: unknown): Record<string, unknown> | undefined {
	if (!isRecord(value)) return undefined;
	if (isRecord(value.schema)) {
		return value.schema;
	}
	return value;
}

function resolveInputSchema(
	record: CapabilityRecord,
): Record<string, unknown> | undefined {
	for (const candidate of [record.input_schema, record.inputSchema, record.schema]) {
		const schema = toSchemaObject(candidate);
		if (schema && Object.keys(schema).length > 0) {
			return schema;
		}
	}
	return undefined;
}

function resolveOutputSchema(
	record: CapabilityRecord,
): Record<string, unknown> | undefined {
	for (const candidate of [record.output_schema, record.outputSchema]) {
		const schema = toSchemaObject(candidate);
		if (schema && Object.keys(schema).length > 0) {
			return schema;
		}
	}
	return undefined;
}

export function capabilityFamilyListMethod(family: InspectorCapabilityFamily): string {
	const methods: Record<InspectorCapabilityFamily, string> = {
		tools: "tools/list",
		prompts: "prompts/list",
		resources: "resources/list",
		resource_templates: "resources/templates/list",
		apps: "apps/list",
		tasks: "tasks/list",
		extensions: "extensions/list",
	};
	return methods[family];
}

function capabilityRecordKey(
	record: CapabilityRecord,
	family: InspectorCapabilityFamily,
): string {
	switch (family) {
		case "tools":
			return (
				toStringValue(record.tool_name) ??
				toStringValue(record.name) ??
				toStringValue(record.unique_name) ??
				""
			);
		case "prompts":
			return (
				toStringValue(record.prompt_name) ??
				toStringValue(record.name) ??
				toStringValue(record.unique_name) ??
				""
			);
		case "resources":
			return (
				toStringValue(record.resource_uri) ??
				toStringValue(record.uri) ??
				toStringValue(record.name) ??
				""
			);
		case "resource_templates":
			return (
				toStringValue(record.uriTemplate) ??
				toStringValue(record.uri_template) ??
				toStringValue(record.name) ??
				""
			);
		case "tasks":
			return (
				toStringValue(record.taskId) ??
				toStringValue(record.task_id) ??
				toStringValue(record.name) ??
				""
			);
		default:
			return toStringValue(record.unique_name) ?? toStringValue(record.name) ?? "";
	}
}

function capabilityRecordTitle(
	record: CapabilityRecord,
	family: InspectorCapabilityFamily,
): string {
	const key = capabilityRecordKey(record, family);
	switch (family) {
		case "tools":
			return (
				toStringValue(record.tool_name) ??
				toStringValue(record.name) ??
				toStringValue(record.unique_name) ??
				(key || "Untitled Tool")
			);
		case "prompts":
			return (
				toStringValue(record.prompt_name) ??
				toStringValue(record.name) ??
				toStringValue(record.unique_name) ??
				(key || "Untitled Prompt")
			);
		case "resources":
			return (
				toStringValue(record.name) ??
				toStringValue(record.resource_uri) ??
				toStringValue(record.uri) ??
				(key || "Resource")
			);
		case "resource_templates":
			return (
				toStringValue(record.name) ??
				toStringValue(record.uriTemplate) ??
				toStringValue(record.uri_template) ??
				(key || "Template")
			);
		case "tasks":
			return (
				toStringValue(record.name) ??
				toStringValue(record.taskId) ??
				toStringValue(record.task_id) ??
				(key || "Task")
			);
		default:
			return toStringValue(record.name) ?? (key || "Capability");
	}
}

export function capabilityRecordToListItem(
	record: CapabilityRecord,
	family: InspectorCapabilityFamily,
): InspectorCapabilityListItem | null {
	const key = capabilityRecordKey(record, family);
	if (!key) return null;

	return {
		key,
		title: capabilityRecordTitle(record, family),
		description: toStringValue(record.description),
		inputSchema: resolveInputSchema(record),
		outputSchema: resolveOutputSchema(record),
	};
}

function capabilityRecordMatchesFamily(
	record: CapabilityRecord,
	family: InspectorCapabilityFamily,
): boolean {
	const uri = toStringValue(record.resource_uri) ?? toStringValue(record.uri);
	const template =
		toStringValue(record.uriTemplate) ?? toStringValue(record.uri_template);
	const promptName = toStringValue(record.prompt_name);
	const toolName = toStringValue(record.tool_name);
	const name = toStringValue(record.name);

	switch (family) {
		case "tools":
			if (uri || template) return false;
			if (promptName && !toolName) return false;
			return Boolean(toolName ?? name);
		case "prompts":
			if (uri || template) return false;
			return Boolean(promptName ?? name);
		case "resources":
			if (template) return false;
			return Boolean(uri ?? name);
		case "resource_templates":
			return Boolean(template ?? name);
		case "tasks":
			return Boolean(
				toStringValue(record.taskId) ?? toStringValue(record.task_id) ?? name,
			);
		default:
			return false;
	}
}

function normalizeCapabilityRecords(
	response: InspectorListResponse,
	family: InspectorCapabilityFamily,
): CapabilityRecord[] {
	const data = response.data;
	const rawList =
		family === "tools"
			? data?.tools
			: family === "prompts"
				? data?.prompts
				: family === "resources"
					? data?.resources
					: family === "resource_templates"
						? data?.templates
						: family === "tasks"
							? data?.tasks
							: undefined;

	if (!Array.isArray(rawList)) {
		return [];
	}

	return rawList
		.map((entry) => toCapabilityRecord(entry))
		.filter((entry): entry is CapabilityRecord => entry !== null)
		.filter((entry) => capabilityRecordMatchesFamily(entry, family));
}

export async function fetchInspectorCapabilityList(params: {
	family: InspectorCapabilityFamily;
	targetRequest: InspectorNativeTargetRequest;
	sessionId: string;
	refresh?: boolean;
}): Promise<InspectorCapabilityListItem[]> {
	const { family, targetRequest, sessionId, refresh = true } = params;

	if (
		family !== "tools" &&
		family !== "prompts" &&
		family !== "resources" &&
		family !== "resource_templates" &&
		family !== "tasks"
	) {
		throw new Error(`Capability list is not available for ${family} yet.`);
	}

	const payload = {
		...targetRequest,
		session_id: sessionId,
		refresh,
	};

	let response: InspectorListResponse;
	switch (family) {
		case "tools":
			response = (await inspectorApi.toolsList(payload)) as InspectorListResponse;
			break;
		case "prompts":
			response = (await inspectorApi.promptsList(payload)) as InspectorListResponse;
			break;
		case "resources":
			response = (await inspectorApi.resourcesList(payload)) as InspectorListResponse;
			break;
		case "resource_templates":
			response = (await inspectorApi.templatesList(payload)) as InspectorListResponse;
			break;
		case "tasks":
			response = (await inspectorApi.tasksList(payload)) as InspectorListResponse;
			break;
	}

	if (!response?.success) {
		throw new Error(
			response?.error ? String(response.error) : "Inspector capability list failed",
		);
	}

	const records = normalizeCapabilityRecords(response, family);
	const items = records
		.map((record) => capabilityRecordToListItem(record, family))
		.filter((item): item is InspectorCapabilityListItem => item !== null);

	return items.sort((left, right) => left.title.localeCompare(right.title));
}
