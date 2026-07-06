import type { InspectorCapabilityKind } from "./inspector-capability";

export type InspectorPreviewBlock =
	| {
			kind: "text";
			text: string;
			format: "plain" | "markdown";
	  }
	| {
			kind: "image";
			src: string;
			mimeType?: string;
			alt?: string;
	  };

export type InspectorJsonOutlineSummaryMeta =
	| { kind: "keys"; count: number }
	| { kind: "items"; count: number }
	| { kind: "emptyObject" }
	| { kind: "emptyArray" }
	| { kind: "truncatedRows" }
	| { kind: "truncatedDepth" };

export type InspectorJsonOutlineRow = {
	id: string;
	depth: number;
	label: string;
	type:
		| "array"
		| "boolean"
		| "null"
		| "number"
		| "object"
		| "string"
		| "truncated"
		| "undefined"
		| "unknown";
	summary: string;
	hasChildren: boolean;
	summaryMeta?: InspectorJsonOutlineSummaryMeta;
};

export type InspectorJsonOutlineOptions = {
	maxDepth?: number;
	maxRows?: number;
};

const INSPECTOR_JSON_OUTLINE_DEFAULT_MAX_DEPTH = 8;
const INSPECTOR_JSON_OUTLINE_DEFAULT_MAX_ROWS = 300;
const INSPECTOR_JSON_OUTLINE_MAX_SUMMARY_LENGTH = 160;

function isRecord(value: unknown): value is Record<string, unknown> {
	return value != null && typeof value === "object" && !Array.isArray(value);
}

function stringValue(value: unknown): string | null {
	return typeof value === "string" && value.length > 0 ? value : null;
}

function asArray(value: unknown): unknown[] {
	return Array.isArray(value) ? value : [];
}

function isMarkdownContent(entry: Record<string, unknown>, text: string): boolean {
	const mimeType = stringValue(entry.mimeType) ?? stringValue(entry.mime_type);
	if (mimeType?.toLowerCase().includes("markdown")) {
		return true;
	}
	return text.trimStart().startsWith("#") || text.includes("\n```");
}

function imageSource(entry: Record<string, unknown>): string | null {
	const uri = stringValue(entry.uri) ?? stringValue(entry.url);
	if (uri?.startsWith("data:") || uri?.startsWith("http://") || uri?.startsWith("https://")) {
		return uri;
	}

	const data = stringValue(entry.data) ?? stringValue(entry.blob);
	if (!data) {
		return null;
	}
	const mimeType = stringValue(entry.mimeType) ?? stringValue(entry.mime_type) ?? "image/png";
	return data.startsWith("data:") ? data : `data:${mimeType};base64,${data}`;
}

function pushContentBlock(blocks: InspectorPreviewBlock[], entry: unknown): void {
	if (!isRecord(entry)) {
		return;
	}

	const type = stringValue(entry.type)?.toLowerCase();
	if (type === "image") {
		const src = imageSource(entry);
		if (src) {
			blocks.push({
				kind: "image",
				src,
				mimeType: stringValue(entry.mimeType) ?? stringValue(entry.mime_type) ?? undefined,
				alt: stringValue(entry.alt) ?? undefined,
			});
		}
		return;
	}

	const text =
		stringValue(entry.text) ??
		stringValue(entry.content) ??
		stringValue(entry.value) ??
		null;
	if (text) {
		blocks.push({
			kind: "text",
			text,
			format: isMarkdownContent(entry, text) ? "markdown" : "plain",
		});
	}
}

function collectPromptMessageBlocks(blocks: InspectorPreviewBlock[], messages: unknown): void {
	for (const message of asArray(messages)) {
		if (!isRecord(message)) {
			continue;
		}
		const content = message.content;
		if (Array.isArray(content)) {
			for (const entry of content) {
				pushContentBlock(blocks, entry);
			}
		} else {
			pushContentBlock(blocks, content);
		}
	}
}

function collectResourceBlocks(blocks: InspectorPreviewBlock[], contents: unknown): void {
	for (const entry of asArray(contents)) {
		pushContentBlock(blocks, entry);
	}
}

export function parseInspectorResponsePreview(
	response: unknown,
	_kind: InspectorCapabilityKind,
): InspectorPreviewBlock[] {
	if (!isRecord(response)) {
		return [];
	}

	const blocks: InspectorPreviewBlock[] = [];

	for (const entry of asArray(response.content)) {
		pushContentBlock(blocks, entry);
	}
	collectPromptMessageBlocks(blocks, response.messages);
	collectResourceBlocks(blocks, response.contents);

	if (blocks.length === 0) {
		pushContentBlock(blocks, response);
	}

	return blocks;
}

export function extractInspectorResponseText(
	response: unknown,
	kind: InspectorCapabilityKind,
): string | null {
	const textBlocks = parseInspectorResponsePreview(response, kind)
		.filter((block): block is Extract<InspectorPreviewBlock, { kind: "text" }> => block.kind === "text")
		.map((block) => block.text);
	return textBlocks.length ? textBlocks.join("\n\n") : null;
}

export function hasInspectorResponsePreview(
	response: unknown,
	kind: InspectorCapabilityKind,
): boolean {
	return parseInspectorResponsePreview(response, kind).length > 0;
}

function pluralize(count: number, singular: string, plural: string): string {
	return `${count} ${count === 1 ? singular : plural}`;
}

function jsonOutlineType(value: unknown): InspectorJsonOutlineRow["type"] {
	if (value === null) {
		return "null";
	}
	if (Array.isArray(value)) {
		return "array";
	}
	if (isRecord(value)) {
		return "object";
	}
	if (value === undefined) {
		return "undefined";
	}
	const valueType = typeof value;
	if (valueType === "boolean" || valueType === "number" || valueType === "string") {
		return valueType;
	}
	return "unknown";
}

function truncateJsonOutlineSummary(summary: string): string {
	if (summary.length <= INSPECTOR_JSON_OUTLINE_MAX_SUMMARY_LENGTH) {
		return summary;
	}
	return `${summary.slice(0, INSPECTOR_JSON_OUTLINE_MAX_SUMMARY_LENGTH - 3)}...`;
}

function jsonOutlineRowSummary(value: unknown): {
	summary: string;
	summaryMeta?: InspectorJsonOutlineSummaryMeta;
} {
	if (Array.isArray(value)) {
		if (value.length === 0) {
			return { summary: "[]", summaryMeta: { kind: "emptyArray" } };
		}
		return {
			summary: pluralize(value.length, "item", "items"),
			summaryMeta: { kind: "items", count: value.length },
		};
	}
	if (isRecord(value)) {
		const keyCount = Object.keys(value).length;
		if (keyCount === 0) {
			return { summary: "{}", summaryMeta: { kind: "emptyObject" } };
		}
		return {
			summary: pluralize(keyCount, "key", "keys"),
			summaryMeta: { kind: "keys", count: keyCount },
		};
	}
	if (typeof value === "string") {
		return { summary: truncateJsonOutlineSummary(JSON.stringify(value)) };
	}
	if (value === undefined) {
		return { summary: "undefined" };
	}
	return { summary: truncateJsonOutlineSummary(String(value)) };
}

function valueHasNestedEntries(value: unknown): boolean {
	return (Array.isArray(value) && value.length > 0) || (isRecord(value) && Object.keys(value).length > 0);
}

function pathForObjectKey(parentId: string, key: string): string {
	return /^[A-Za-z_$][\w$]*$/.test(key)
		? `${parentId}.${key}`
		: `${parentId}[${JSON.stringify(key)}]`;
}

function pushJsonOutlineRows(
	rows: InspectorJsonOutlineRow[],
	value: unknown,
	label: string,
	id: string,
	depth: number,
	options: Required<InspectorJsonOutlineOptions>,
	state: { truncated: boolean },
): void {
	if (rows.length >= options.maxRows) {
		state.truncated = true;
		return;
	}

	const { summary, summaryMeta } = jsonOutlineRowSummary(value);
	const hasChildren = depth < options.maxDepth && valueHasNestedEntries(value);

	rows.push({
		id,
		depth,
		label,
		type: jsonOutlineType(value),
		summary,
		summaryMeta,
		hasChildren,
	});

	if (depth >= options.maxDepth) {
		if (valueHasNestedEntries(value)) {
			pushJsonOutlineTruncation(rows, `${id}.__maxDepth`, depth + 1, options, state, "truncatedDepth");
		}
		return;
	}

	if (Array.isArray(value)) {
		value.forEach((entry, index) => {
			if (rows.length >= options.maxRows - 1) {
				pushJsonOutlineTruncation(rows, `${id}.__maxRows`, depth + 1, options, state, "truncatedRows");
				return;
			}
			pushJsonOutlineRows(
				rows,
				entry,
				`[${index}]`,
				`${id}[${index}]`,
				depth + 1,
				options,
				state,
			);
		});
		return;
	}

	if (isRecord(value)) {
		Object.entries(value).forEach(([key, entry]) => {
			if (rows.length >= options.maxRows - 1) {
				pushJsonOutlineTruncation(rows, `${id}.__maxRows`, depth + 1, options, state, "truncatedRows");
				return;
			}
			pushJsonOutlineRows(
				rows,
				entry,
				key,
				pathForObjectKey(id, key),
				depth + 1,
				options,
				state,
			);
		});
	}
}

function pushJsonOutlineTruncation(
	rows: InspectorJsonOutlineRow[],
	id: string,
	depth: number,
	options: Required<InspectorJsonOutlineOptions>,
	state: { truncated: boolean },
	kind: "truncatedRows" | "truncatedDepth",
): void {
	if (state.truncated || rows.length >= options.maxRows) {
		state.truncated = true;
		return;
	}
	rows.push({
		id,
		depth,
		label: "...",
		type: "truncated",
		summary:
			kind === "truncatedDepth"
				? "Nested entries hidden after max depth"
				: "Additional entries hidden after max rows",
		summaryMeta: { kind },
		hasChildren: false,
	});
	state.truncated = true;
}

export function buildInspectorJsonOutline(
	value: unknown,
	options: InspectorJsonOutlineOptions = {},
): InspectorJsonOutlineRow[] {
	const rows: InspectorJsonOutlineRow[] = [];
	const resolvedOptions = {
		maxDepth: Math.max(0, options.maxDepth ?? INSPECTOR_JSON_OUTLINE_DEFAULT_MAX_DEPTH),
		maxRows: Math.max(1, options.maxRows ?? INSPECTOR_JSON_OUTLINE_DEFAULT_MAX_ROWS),
	};
	pushJsonOutlineRows(rows, value, "$", "$", 0, resolvedOptions, {
		truncated: false,
	});
	return rows;
}
