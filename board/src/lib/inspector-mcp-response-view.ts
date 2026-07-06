import type { InspectorCapabilityKind } from "./inspector-capability";
import type { InspectorLogEventEntry } from "./inspector-event-log";
import {
	extractInspectorResponseText,
	parseInspectorResponsePreview,
	type InspectorPreviewBlock,
} from "./inspector-response-preview";

export type InspectorMcpResponseViewMode =
	| "preview"
	| "raw"
	| "markdown"
	| "image"
	| "outline"
	| "json";

export type InspectorMcpResponseSegmentMode = "preview" | "json" | "outline" | "raw";

export const INSPECTOR_MCP_RESPONSE_SEGMENT_OPTIONS: InspectorMcpResponseSegmentMode[] = [
	"preview",
	"json",
	"outline",
	"raw",
];

const RICH_PREVIEW_MODES: InspectorMcpResponseViewMode[] = ["markdown", "preview", "image"];

/** Preferred display order: rich content first, structured JSON next, raw text last. */
export const INSPECTOR_MCP_RESPONSE_VIEW_FALLBACK_ORDER: InspectorMcpResponseViewMode[] = [
	"markdown",
	"preview",
	"image",
	"json",
	"outline",
	"raw",
];

function isRecord(value: unknown): value is Record<string, unknown> {
	return value != null && typeof value === "object" && !Array.isArray(value);
}

export function extractMcpProtocolEnvelopeBody(response: unknown): unknown {
	if (!isRecord(response)) {
		return response;
	}
	if ("result" in response) {
		return response.result;
	}
	if ("error" in response) {
		return response.error;
	}
	return response;
}

export function inferInspectorCapabilityKindFromEntry(
	entry: InspectorLogEventEntry,
): InspectorCapabilityKind {
	const { data } = entry;
	switch (data.event) {
		case "prompt_get":
			return "prompt";
		case "resource_read":
			return "resource";
		case "mcp_exchange":
			if (data.method.includes("prompt")) {
				return "prompt";
			}
			if (data.method.includes("resource")) {
				return "resource";
			}
			return "tool";
		case "ephemeral_invoke":
			if (data.operation === "prompt") {
				return "prompt";
			}
			if (data.operation === "resource") {
				return "resource";
			}
			return "tool";
		default:
			return "tool";
	}
}

export function firstInspectorPreviewMarkdownBlock(
	blocks: InspectorPreviewBlock[],
): Extract<InspectorPreviewBlock, { kind: "text" }> | null {
	for (const block of blocks) {
		if (block.kind === "text" && block.format === "markdown") {
			return block;
		}
	}
	return null;
}

export function firstInspectorPreviewImageBlock(
	blocks: InspectorPreviewBlock[],
): Extract<InspectorPreviewBlock, { kind: "image" }> | null {
	for (const block of blocks) {
		if (block.kind === "image") {
			return block;
		}
	}
	return null;
}

export function inspectorMcpResponseModeHasContent(
	mode: InspectorMcpResponseViewMode,
	response: unknown,
	kind: InspectorCapabilityKind,
): boolean {
	if (response === undefined || response === null) {
		return false;
	}

	const payload = extractMcpProtocolEnvelopeBody(response);
	const blocks = parseInspectorResponsePreview(payload, kind);

	switch (mode) {
		case "markdown":
			return firstInspectorPreviewMarkdownBlock(blocks) != null;
		case "preview":
			return blocks.some((block) => block.kind === "text");
		case "image":
			return firstInspectorPreviewImageBlock(blocks) != null;
		case "json":
		case "outline":
			return true;
		case "raw":
			return extractInspectorResponseText(payload, kind) != null;
	}
}

export function mapInspectorMcpResponseModeToSegment(
	mode: InspectorMcpResponseViewMode,
): InspectorMcpResponseSegmentMode {
	if (mode === "json") {
		return "json";
	}
	if (mode === "outline") {
		return "outline";
	}
	if (mode === "raw") {
		return "raw";
	}
	return "preview";
}

export function pickRichInspectorMcpResponseViewMode(
	response: unknown,
	kind: InspectorCapabilityKind,
): InspectorMcpResponseViewMode | null {
	for (const mode of RICH_PREVIEW_MODES) {
		if (inspectorMcpResponseModeHasContent(mode, response, kind)) {
			return mode;
		}
	}
	return null;
}

export function resolveInspectorMcpResponseViewModeForSegment(
	segment: InspectorMcpResponseSegmentMode,
	response: unknown,
	kind: InspectorCapabilityKind,
): InspectorMcpResponseViewMode {
	switch (segment) {
		case "preview": {
			const richMode = pickRichInspectorMcpResponseViewMode(response, kind);
			if (richMode) {
				return richMode;
			}
			if (inspectorMcpResponseModeHasContent("json", response, kind)) {
				return "json";
			}
			if (inspectorMcpResponseModeHasContent("raw", response, kind)) {
				return "raw";
			}
			return "json";
		}
		case "json":
			return "json";
		case "outline":
			return "outline";
		case "raw":
			if (inspectorMcpResponseModeHasContent("raw", response, kind)) {
				return "raw";
			}
			if (inspectorMcpResponseModeHasContent("json", response, kind)) {
				return "json";
			}
			return pickRichInspectorMcpResponseViewMode(response, kind) ?? "json";
	}
}

export function pickDefaultInspectorMcpResponseSegmentMode(
	response: unknown,
	kind: InspectorCapabilityKind,
): InspectorMcpResponseSegmentMode {
	return mapInspectorMcpResponseModeToSegment(
		pickDefaultInspectorMcpResponseViewMode(response, kind),
	);
}

export function resolveActiveInspectorMcpResponseSegmentMode(
	response: unknown,
	kind: InspectorCapabilityKind,
	preferredSegment: InspectorMcpResponseSegmentMode,
): InspectorMcpResponseSegmentMode {
	const effectiveMode = resolveInspectorMcpResponseViewModeForSegment(
		preferredSegment,
		response,
		kind,
	);
	return mapInspectorMcpResponseModeToSegment(effectiveMode);
}

export function resolveInspectorMcpResponseViewModes(
	response: unknown,
	kind: InspectorCapabilityKind,
): InspectorMcpResponseViewMode[] {
	if (response === undefined || response === null) {
		return [];
	}

	const modes = INSPECTOR_MCP_RESPONSE_VIEW_FALLBACK_ORDER.filter((mode) =>
		inspectorMcpResponseModeHasContent(mode, response, kind),
	);

	if (!modes.includes("json")) {
		modes.push("json");
	}

	return modes;
}

export function pickDefaultInspectorMcpResponseViewMode(
	response: unknown,
	kind: InspectorCapabilityKind,
): InspectorMcpResponseViewMode {
	const modes = resolveInspectorMcpResponseViewModes(response, kind);
	for (const mode of INSPECTOR_MCP_RESPONSE_VIEW_FALLBACK_ORDER) {
		if (modes.includes(mode)) {
			return mode;
		}
	}
	return "json";
}

export function resolveEffectiveInspectorMcpResponseViewMode(
	response: unknown,
	kind: InspectorCapabilityKind,
	preferredSegment: InspectorMcpResponseSegmentMode,
): InspectorMcpResponseViewMode {
	return resolveInspectorMcpResponseViewModeForSegment(
		preferredSegment,
		response,
		kind,
	);
}

export const INSPECTOR_COMPACT_SEGMENT_CLASSNAME =
	"w-auto shrink-0 [&_[role=tablist]]:min-h-8 [&_[role=tablist]]:h-8 [&_[role=tablist]]:w-auto [&_[role=tablist]]:p-0.5 [&_button]:px-2.5 [&_button]:py-1 [&_button]:text-xs [&_button]:font-medium";
