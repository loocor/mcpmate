import type { InspectorCapabilityKind } from "./inspector-capability";
import type { InspectorLogEventEntry } from "./inspector-event-log";
import {
	extractInspectorResponseText,
	hasInspectorResponsePreview,
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

export type InspectorPayloadSegmentMode = "preview" | "outline" | "json" | "raw";

export type InspectorMcpResponseSegmentMode = InspectorPayloadSegmentMode;

export const INSPECTOR_PAYLOAD_SEGMENT_OPTIONS: InspectorPayloadSegmentMode[] = [
	"preview",
	"outline",
	"json",
	"raw",
];

export const INSPECTOR_MCP_RESPONSE_SEGMENT_OPTIONS = INSPECTOR_PAYLOAD_SEGMENT_OPTIONS;

const RICH_PREVIEW_MODES: InspectorMcpResponseViewMode[] = ["markdown", "preview", "image"];

/** Preferred display order: rich preview first, then outline/json, raw last. */
export const INSPECTOR_MCP_RESPONSE_VIEW_FALLBACK_ORDER: InspectorMcpResponseViewMode[] = [
	"markdown",
	"preview",
	"image",
	"outline",
	"json",
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
	if (mode === "raw") {
		return "raw";
	}
	if (mode === "outline") {
		return "outline";
	}
	if (mode === "json") {
		return "json";
	}
	return "preview";
}

export function inspectorPayloadSegmentHasContent(
	segment: InspectorPayloadSegmentMode,
	value: unknown,
	kind: InspectorCapabilityKind = "tool",
): boolean {
	if (value === undefined || value === null) {
		return false;
	}

	const payload = extractMcpProtocolEnvelopeBody(value);

	switch (segment) {
		case "preview":
			return hasInspectorResponsePreview(payload, kind);
		case "outline":
		case "json":
			return true;
		case "raw":
			return extractInspectorResponseText(payload, kind) != null;
	}
}

export function resolveAvailablePayloadSegments(
	value: unknown,
	kind: InspectorCapabilityKind = "tool",
): InspectorPayloadSegmentMode[] {
	return INSPECTOR_PAYLOAD_SEGMENT_OPTIONS.filter((segment) =>
		inspectorPayloadSegmentHasContent(segment, value, kind),
	);
}

export function coerceInspectorPayloadSegmentMode(
	preferred: InspectorPayloadSegmentMode,
	value: unknown,
	kind: InspectorCapabilityKind = "tool",
): InspectorPayloadSegmentMode {
	const available = resolveAvailablePayloadSegments(value, kind);
	if (available.includes(preferred)) {
		return preferred;
	}
	return pickDefaultInspectorPayloadSegmentMode(value, kind);
}

export function mapInspectorMcpResponseSegmentToLabel(
	segment: InspectorPayloadSegmentMode,
): string {
	switch (segment) {
		case "preview":
			return "Preview";
		case "outline":
			return "Outline";
		case "json":
			return "JSON";
		case "raw":
			return "Raw";
	}
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
			return richMode ?? "preview";
		}
		case "outline":
			return "outline";
		case "json":
			return "json";
		case "raw":
			return "raw";
	}
}

export function resolveGenericPayloadSegmentView(
	segment: InspectorPayloadSegmentMode,
	value: unknown,
	kind: InspectorCapabilityKind = "tool",
): InspectorMcpResponseViewMode {
	const activeSegment = coerceInspectorPayloadSegmentMode(segment, value, kind);
	return resolveInspectorMcpResponseViewModeForSegment(activeSegment, value, kind);
}

export function pickDefaultInspectorPayloadSegmentMode(
	value?: unknown,
	kind: InspectorCapabilityKind = "tool",
): InspectorPayloadSegmentMode {
	const available = resolveAvailablePayloadSegments(value, kind);
	return available[0] ?? "outline";
}

export function pickDefaultInspectorMcpResponseSegmentMode(
	response: unknown,
	kind: InspectorCapabilityKind,
): InspectorMcpResponseSegmentMode {
	return pickDefaultInspectorPayloadSegmentMode(response, kind);
}

export function resolveActiveInspectorMcpResponseSegmentMode(
	response: unknown,
	kind: InspectorCapabilityKind,
	preferredSegment: InspectorMcpResponseSegmentMode,
): InspectorMcpResponseSegmentMode {
	return coerceInspectorPayloadSegmentMode(preferredSegment, response, kind);
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
	const activeSegment = coerceInspectorPayloadSegmentMode(preferredSegment, response, kind);
	return resolveInspectorMcpResponseViewModeForSegment(activeSegment, response, kind);
}

/** Small segment control for standalone inspector drawer payload headers. */
export const INSPECTOR_COMPACT_SEGMENT_CLASSNAME =
	"w-auto shrink-0 [&_[role=tablist]]:h-7 [&_[role=tablist]]:w-auto [&_[role=tablist]]:gap-0.5 [&_[role=tablist]]:p-0.5 [&_[role=tablist]]:items-center [&_button]:h-6 [&_button]:self-center [&_button]:px-2 [&_button]:py-0 [&_button]:text-[10px] [&_button]:font-medium [&_button]:leading-none";

/**
 * Floating payload action button with three explicit opacity states on the button itself:
 * 1. default — hidden
 * 2. container hover/focus-within — 25%
 * 3. button hover while container is active — 100%
 *
 * Use chained `group-hover/payload:hover:` so button-hover wins over container-hover
 * (plain `hover:` loses to `group-hover/payload:` in generated CSS order).
 */
export const INSPECTOR_PAYLOAD_FLOATING_ACTION_CLASSNAME =
	"h-7 w-7 shadow-sm transition-opacity pointer-events-none opacity-0 group-hover/payload:pointer-events-auto group-hover/payload:opacity-25 group-focus-within/payload:pointer-events-auto group-focus-within/payload:opacity-25 group-hover/payload:hover:opacity-100 focus-visible:pointer-events-auto focus-visible:opacity-100";

export const INSPECTOR_PAYLOAD_FLOATING_ACTIONS_CONTAINER_CLASSNAME =
	"absolute right-2 top-2 z-10 flex items-center gap-1";
