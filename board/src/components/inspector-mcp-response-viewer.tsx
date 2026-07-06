import { Copy } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import { writeClipboardText } from "../lib/clipboard";
import {
	INSPECTOR_COMPACT_SEGMENT_CLASSNAME,
	INSPECTOR_PAYLOAD_FLOATING_ACTION_CLASSNAME,
	pickDefaultInspectorMcpResponseSegmentMode,
	resolveActiveInspectorMcpResponseSegmentMode,
	resolveEffectiveInspectorMcpResponseViewMode,
	type InspectorMcpResponseSegmentMode,
} from "../lib/inspector-mcp-response-view";
import { cn } from "../lib/utils";
import {
	InspectorPayloadSegmentBody,
	inspectorPayloadSegmentShowsCopyButton,
} from "./inspector-payload-segment-body";
import { InspectorDrawerCollapsibleSection } from "./inspector-drawer-collapsible-section";
import { useInspectorPayloadSegmentOptions } from "./use-inspector-payload-segment-options";
import { notifyError, notifySuccess } from "../lib/notify";
import { Button } from "./ui/button";
import { Segment } from "./ui/segment";

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
	const segmentOptions = useInspectorPayloadSegmentOptions(response, kind);
	const [preferredSegment, setPreferredSegment] = useState<InspectorMcpResponseSegmentMode>(
		() => pickDefaultInspectorMcpResponseSegmentMode(response, kind),
	);

	useEffect(() => {
		setPreferredSegment(pickDefaultInspectorMcpResponseSegmentMode(response, kind));
	}, [response, kind]);

	const activeSegment = useMemo(
		() => resolveActiveInspectorMcpResponseSegmentMode(response, kind, preferredSegment),
		[response, kind, preferredSegment],
	);

	const effectiveMode = useMemo(
		() => resolveEffectiveInspectorMcpResponseViewMode(response, kind, activeSegment),
		[response, kind, activeSegment],
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

	const segmentControl =
		segmentOptions.length > 1 ? (
			<Segment
				value={activeSegment}
				onValueChange={(value) =>
					setPreferredSegment(value as InspectorMcpResponseSegmentMode)
				}
				options={segmentOptions}
				showDots={false}
				className={INSPECTOR_COMPACT_SEGMENT_CLASSNAME}
			/>
		) : null;

	const showCopyButton = inspectorPayloadSegmentShowsCopyButton(effectiveMode);

	return (
		<InspectorDrawerCollapsibleSection
			title={title ?? t("activity.drawer.mcpResponse", { defaultValue: "MCP response" })}
			fill={fill}
			collapsible={false}
			className={className}
			headerActions={segmentControl}
		>
			<div
				className={cn(
					"group/payload relative min-w-0",
					fill && "flex min-h-0 flex-1 flex-col overflow-hidden",
				)}
			>
				{showCopyButton ? (
					<Button
						type="button"
						variant="outline"
						size="icon"
						className={cn(
							INSPECTOR_PAYLOAD_FLOATING_ACTION_CLASSNAME,
							"absolute right-2 top-2 z-10",
						)}
						aria-label={t("actions.copyResponse", { defaultValue: "Copy response" })}
						onClick={() => void handleCopy()}
					>
						<Copy className="h-3.5 w-3.5" />
					</Button>
				) : null}
				<InspectorPayloadSegmentBody
					mode={effectiveMode}
					value={response}
					kind={kind}
					fill={fill}
					compact={!fill}
				/>
			</div>
		</InspectorDrawerCollapsibleSection>
	);
}
