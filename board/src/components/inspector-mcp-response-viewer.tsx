import { Copy } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import { writeClipboardText } from "../lib/clipboard";
import {
	extractMcpProtocolEnvelopeBody,
	firstInspectorPreviewImageBlock,
	firstInspectorPreviewMarkdownBlock,
	INSPECTOR_COMPACT_SEGMENT_CLASSNAME,
	INSPECTOR_MCP_RESPONSE_SEGMENT_OPTIONS,
	pickDefaultInspectorMcpResponseSegmentMode,
	resolveActiveInspectorMcpResponseSegmentMode,
	resolveEffectiveInspectorMcpResponseViewMode,
	type InspectorMcpResponseSegmentMode,
	type InspectorMcpResponseViewMode,
} from "../lib/inspector-mcp-response-view";
import {
	extractInspectorResponseText,
	parseInspectorResponsePreview,
} from "../lib/inspector-response-preview";
import { cn } from "../lib/utils";
import {
	InspectorJsonOutline,
	InspectorResponsePreview,
} from "./inspector-response-preview";
import { JsonCodeBlock } from "./json-code-block";
import { LazyImage } from "./lazy-image";
import { notifyError, notifySuccess } from "../lib/notify";
import { Button } from "./ui/button";
import { Segment, type SegmentOption } from "./ui/segment";

const FILL_SURFACE_CLASSNAME =
	"min-h-0 flex-1 overflow-y-auto overflow-x-auto";

const SEGMENT_LABELS: Record<InspectorMcpResponseSegmentMode, string> = {
	preview: "Preview",
	json: "JSON",
	outline: "Outline",
	raw: "Raw",
};

function InspectorMcpResponseBody({
	mode,
	response,
	kind,
	fill,
}: {
	mode: InspectorMcpResponseViewMode;
	response: unknown;
	kind: InspectorCapabilityKind;
	fill?: boolean;
}) {
	const { t } = useTranslation("inspector");
	const payload = useMemo(() => extractMcpProtocolEnvelopeBody(response), [response]);
	const blocks = useMemo(() => parseInspectorResponsePreview(payload, kind), [payload, kind]);
	const markdownBlock = firstInspectorPreviewMarkdownBlock(blocks);
	const imageBlock = firstInspectorPreviewImageBlock(blocks);
	const rawText = extractInspectorResponseText(payload, kind);

	if (mode === "preview") {
		return (
			<InspectorResponsePreview
				result={payload}
				kind={kind}
				className={fill ? "min-h-0 flex-1" : undefined}
			/>
		);
	}

	if (mode === "raw") {
		if (!rawText) {
			return (
				<div className="text-sm text-muted-foreground">
					{t("response.previewUnavailable", {
						defaultValue:
							"No visual preview is available for this response. Switch to JSON view.",
					})}
				</div>
			);
		}
		return (
			<pre
				className={cn(
					"m-0 w-full whitespace-pre-wrap break-words rounded-md bg-slate-50 p-2 font-mono text-xs text-slate-700 dark:bg-slate-900 dark:text-slate-200",
					fill && FILL_SURFACE_CLASSNAME,
				)}
			>
				{rawText}
			</pre>
		);
	}

	if (mode === "markdown") {
		if (!markdownBlock) {
			return null;
		}
		return (
			<JsonCodeBlock
				code={markdownBlock.text}
				language="markdown"
				className={fill ? FILL_SURFACE_CLASSNAME : undefined}
			/>
		);
	}

	if (mode === "image") {
		if (!imageBlock) {
			return null;
		}
		return (
			<div
				className={cn(
					"rounded-md bg-slate-50 p-2 dark:bg-slate-900",
					fill && "flex min-h-0 flex-1 flex-col overflow-y-auto",
				)}
			>
				<LazyImage
					src={imageBlock.src}
					alt={imageBlock.alt ?? ""}
					cacheKey={imageBlock.src.slice(0, 128)}
					className="block max-w-full"
					imgClassName="block h-auto max-w-full rounded-md border border-slate-200 object-contain dark:border-slate-800"
					fallback={
						<div className="rounded-md border border-dashed border-slate-300 px-3 py-6 text-center text-xs text-muted-foreground dark:border-slate-700">
							{imageBlock.mimeType ?? "image"}
						</div>
					}
				/>
			</div>
		);
	}

	if (mode === "outline") {
		return (
			<InspectorJsonOutline
				value={response}
				className={fill ? FILL_SURFACE_CLASSNAME : undefined}
			/>
		);
	}

	return (
		<JsonCodeBlock
			code={JSON.stringify(response, null, 2)}
			className={fill ? FILL_SURFACE_CLASSNAME : undefined}
		/>
	);
}

export function InspectorMcpResponseViewer({
	response,
	kind,
	fill = false,
	title,
	className,
}: {
	response: unknown;
	kind: InspectorCapabilityKind;
	fill?: boolean;
	title?: string;
	className?: string;
}) {
	const { t } = useTranslation("inspector");
	const [preferredSegment, setPreferredSegment] =
		useState<InspectorMcpResponseSegmentMode>("json");

	useEffect(() => {
		setPreferredSegment(pickDefaultInspectorMcpResponseSegmentMode(response, kind));
	}, [response, kind]);

	const activeSegment = useMemo(
		() => resolveActiveInspectorMcpResponseSegmentMode(response, kind, preferredSegment),
		[response, kind, preferredSegment],
	);

	const effectiveMode = useMemo(
		() => resolveEffectiveInspectorMcpResponseViewMode(response, kind, preferredSegment),
		[response, kind, preferredSegment],
	);
	const serializedResponse = useMemo(
		() => JSON.stringify(response, null, 2) ?? String(response),
		[response],
	);

	const handleCopy = useCallback(async () => {
		try {
			await writeClipboardText(serializedResponse);
			notifySuccess(
				t("notifications.payloadCopySuccess", { defaultValue: "Payload copied" }),
				t("notifications.responseCopySuccessMessage", {
					defaultValue: "Response payload copied to clipboard.",
				}),
			);
		} catch (error) {
			notifyError(
				t("notifications.copyFailed", { defaultValue: "Copy failed" }),
				error instanceof Error ? error.message : String(error),
			);
		}
	}, [serializedResponse, t]);

	const segmentOptions = useMemo<SegmentOption[]>(
		() =>
			INSPECTOR_MCP_RESPONSE_SEGMENT_OPTIONS.map((mode) => ({
				value: mode,
				label: t(`response.format.${mode}`, {
					defaultValue: SEGMENT_LABELS[mode],
				}),
			})),
		[t],
	);

	const segmentControl = (
		<Segment
			value={activeSegment}
			onValueChange={(value) =>
				setPreferredSegment(value as InspectorMcpResponseSegmentMode)
			}
			options={segmentOptions}
			showDots={false}
			className={INSPECTOR_COMPACT_SEGMENT_CLASSNAME}
		/>
	);

	return (
		<section
			className={cn(
				"flex min-h-0 min-w-0 flex-col gap-2",
				fill && "flex-1",
				className,
			)}
		>
			{title ? (
				<div className="flex shrink-0 items-center justify-between gap-2">
					<h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
						{title}
					</h3>
					{segmentControl}
				</div>
			) : (
				segmentControl
			)}
			<div
				className={cn(
					"group/payload relative min-w-0",
					fill && "flex min-h-0 flex-1 flex-col overflow-hidden",
				)}
			>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className="absolute right-2 top-2 z-10 h-7 w-7 opacity-0 shadow-sm transition-opacity group-hover/payload:opacity-100 group-focus-within/payload:opacity-100"
					aria-label={t("actions.copyResponse", { defaultValue: "Copy response" })}
					onClick={() => void handleCopy()}
				>
					<Copy className="h-3.5 w-3.5" />
				</Button>
				<InspectorMcpResponseBody
					mode={effectiveMode}
					response={response}
					kind={kind}
					fill={fill}
				/>
			</div>
		</section>
	);
}
