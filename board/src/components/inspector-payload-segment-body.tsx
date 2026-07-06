import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import {
	extractMcpProtocolEnvelopeBody,
	type InspectorMcpResponseViewMode,
} from "../lib/inspector-mcp-response-view";
import {
	extractInspectorResponseText,
} from "../lib/inspector-response-preview";
import { cn } from "../lib/utils";
import {
	InspectorJsonOutline,
	INSPECTOR_PAYLOAD_SURFACE_CLASSNAME,
	InspectorResponsePreview,
} from "./inspector-response-preview";
import { JsonCodeBlock } from "./json-code-block";

const FILL_SURFACE_CLASSNAME =
	"min-h-0 flex-1 overflow-y-auto overflow-x-auto";

const COMPACT_SURFACE_CLASSNAME = "max-h-[min(28vh,240px)] shrink-0";

export function InspectorPayloadSegmentBody({
	mode,
	value,
	kind = "tool",
	fill = false,
	compact = false,
}: {
	mode: InspectorMcpResponseViewMode;
	value: unknown;
	kind?: InspectorCapabilityKind;
	fill?: boolean;
	compact?: boolean;
}) {
	const payload = extractMcpProtocolEnvelopeBody(value);
	const rawText = extractInspectorResponseText(payload, kind);
	const surfaceClassName = cn(
		fill ? "min-h-0 flex-1" : compact ? COMPACT_SURFACE_CLASSNAME : undefined,
	);

	if (mode === "raw") {
		if (rawText == null) {
			return null;
		}
		return (
			<pre
				className={cn(
					INSPECTOR_PAYLOAD_SURFACE_CLASSNAME,
					"m-0 w-full whitespace-pre-wrap break-words p-2 text-slate-700 dark:text-slate-200",
					fill && FILL_SURFACE_CLASSNAME,
					surfaceClassName,
				)}
			>
				{rawText}
			</pre>
		);
	}

	if (mode === "outline") {
		return (
			<InspectorJsonOutline
				value={value}
				className={cn(fill && FILL_SURFACE_CLASSNAME, surfaceClassName)}
			/>
		);
	}

	if (mode === "preview" || mode === "markdown" || mode === "image") {
		return (
			<InspectorResponsePreview
				result={payload}
				kind={kind}
				className={cn(fill && "min-h-0 flex-1", surfaceClassName)}
			/>
		);
	}

	return (
		<JsonCodeBlock
			code={JSON.stringify(value, null, 2)}
			className={cn(
				INSPECTOR_PAYLOAD_SURFACE_CLASSNAME,
				fill && FILL_SURFACE_CLASSNAME,
				fill && "h-full",
				surfaceClassName,
			)}
		/>
	);
}

export function inspectorPayloadSegmentShowsCopyButton(
	mode: InspectorMcpResponseViewMode,
): boolean {
	return mode === "json" || mode === "raw";
}
