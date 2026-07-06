import { Copy } from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { InspectorMcpResponseViewer } from "./inspector-mcp-response-viewer";
import { InspectorContextPropertyPanel } from "./inspector-context-property-panel";
import { InspectorDrawerCollapsibleSection } from "./inspector-drawer-collapsible-section";
import {
	InspectorRecordPropertyPanel,
	isFlatInspectorRecord,
} from "./inspector-record-property-panel";
import {
	InspectorPayloadSegmentBody,
	inspectorPayloadSegmentShowsCopyButton,
} from "./inspector-payload-segment-body";
import { useInspectorPayloadSegmentOptions } from "./use-inspector-payload-segment-options";
import { Button } from "./ui/button";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { writeClipboardText } from "../lib/clipboard";
import {
	buildInspectorEventProtocolView,
	type InspectorEventProtocolView,
} from "../lib/inspector-event-protocol-view";
import {
	formatInspectorEventAction,
	type InspectorLogEventEntry,
	type InspectorLogTranslate,
} from "../lib/inspector-event-log";
import {
	INSPECTOR_COMPACT_SEGMENT_CLASSNAME,
	INSPECTOR_PAYLOAD_FLOATING_ACTION_CLASSNAME,
	coerceInspectorPayloadSegmentMode,
	inferInspectorCapabilityKindFromEntry,
	pickDefaultInspectorPayloadSegmentMode,
	resolveGenericPayloadSegmentView,
	type InspectorPayloadSegmentMode,
} from "../lib/inspector-mcp-response-view";
import { notifyError, notifySuccess } from "../lib/notify";
import { cn, formatLocalDateTime } from "../lib/utils";
import { Segment } from "./ui/segment";

function serializePayloadForClipboard(value: unknown): string {
	if (typeof value === "string") {
		return value;
	}
	return JSON.stringify(value, null, 2) ?? String(value);
}

function PayloadSection({
	title,
	value,
	fill = false,
	copyLabel,
	copyMessage,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	title: string;
	value: unknown;
	fill?: boolean;
	copyLabel?: string;
	copyMessage?: string;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	const { t } = useTranslation("inspector");
	const payloadDisplayOptions = useInspectorPayloadSegmentOptions(value);
	const [displayMode, setDisplayMode] = useState<InspectorPayloadSegmentMode>(() =>
		pickDefaultInspectorPayloadSegmentMode(value),
	);

	useEffect(() => {
		setDisplayMode(pickDefaultInspectorPayloadSegmentMode(value));
	}, [value]);

	const activeDisplayMode = useMemo(
		() => coerceInspectorPayloadSegmentMode(displayMode, value),
		[displayMode, value],
	);

	const serializedValue = useMemo(() => serializePayloadForClipboard(value), [value]);
	const effectiveMode = useMemo(
		() => resolveGenericPayloadSegmentView(activeDisplayMode, value),
		[activeDisplayMode, value],
	);
	const showPropertyPanel = isFlatInspectorRecord(value) && !fill;

	const handleCopy = useCallback(async () => {
		try {
			await writeClipboardText(serializedValue);
			notifySuccess(
				t("notifications.payloadCopySuccess", { defaultValue: "Payload copied" }),
				copyMessage ??
				t("notifications.payloadCopySuccessMessage", {
					defaultValue: "Payload copied to clipboard.",
				}),
			);
		} catch (error) {
			notifyError(
				t("notifications.copyFailed", { defaultValue: "Copy failed" }),
				error instanceof Error ? error.message : String(error),
			);
		}
	}, [copyMessage, serializedValue, t]);

	if (value === undefined) {
		return null;
	}

	const segmentControl =
		!showPropertyPanel && payloadDisplayOptions.length > 1 ? (
			<Segment
				value={activeDisplayMode}
				onValueChange={(nextMode) =>
					setDisplayMode(nextMode as InspectorPayloadSegmentMode)
				}
				options={payloadDisplayOptions}
				showDots={false}
				className={INSPECTOR_COMPACT_SEGMENT_CLASSNAME}
			/>
		) : null;

	return (
		<InspectorDrawerCollapsibleSection
			title={title}
			fill={fill}
			collapsible={showPropertyPanel}
			headerActions={segmentControl}
		>
			{showPropertyPanel ? (
				<InspectorRecordPropertyPanel
					record={value}
					t={t}
					onFilterByServerId={onFilterByServerId}
					onFilterBySessionId={onFilterBySessionId}
				/>
			) : (
				<div
					className={cn(
						"group/payload relative min-w-0",
						fill && "flex min-h-0 flex-1 flex-col overflow-hidden",
					)}
				>
					{inspectorPayloadSegmentShowsCopyButton(effectiveMode) ? (
						<Button
							type="button"
							variant="outline"
							size="icon"
							className={cn(
								INSPECTOR_PAYLOAD_FLOATING_ACTION_CLASSNAME,
								"absolute right-2 top-2 z-10",
							)}
							aria-label={copyLabel ?? t("actions.copyPayload", { defaultValue: "Copy payload" })}
							onClick={() => void handleCopy()}
						>
							<Copy className="h-3.5 w-3.5" />
						</Button>
					) : null}
					<InspectorPayloadSegmentBody
						mode={effectiveMode}
						value={value}
						fill={fill}
						compact={!fill}
					/>
				</div>
			)}
		</InspectorDrawerCollapsibleSection>
	);
}

function hasMcpProtocolPayload(protocolView: InspectorEventProtocolView): boolean {
	return (
		protocolView.request !== undefined ||
		protocolView.response !== undefined ||
		protocolView.notification !== undefined
	);
}

function InspectorContextCard({
	protocolView,
	t,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	protocolView: InspectorEventProtocolView;
	t: (key: string, options?: Record<string, unknown>) => string;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	return (
		<InspectorDrawerCollapsibleSection
			title={t("activity.drawer.inspectorContext", {
				defaultValue: "Inspector context",
			})}
		>
			<InspectorContextPropertyPanel
				context={protocolView.context}
				t={t}
				onFilterByServerId={onFilterByServerId}
				onFilterBySessionId={onFilterBySessionId}
			/>
		</InspectorDrawerCollapsibleSection>
	);
}

function McpProtocolContent({
	protocolView,
	entry,
	t,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	protocolView: InspectorEventProtocolView;
	entry: InspectorLogEventEntry;
	t: (key: string, options?: Record<string, unknown>) => string;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	const capabilityKind = inferInspectorCapabilityKindFromEntry(entry);
	const hasProtocolPayload = hasMcpProtocolPayload(protocolView);
	const hasContext = Object.keys(protocolView.context).length > 0;

	const payloadSections = useMemo(() => {
		if (!hasProtocolPayload) {
			return [];
		}

		const sections: Array<{ key: string; node: (fill: boolean) => ReactNode }> = [];

		if (protocolView.request !== undefined) {
			sections.push({
				key: "request",
				node: (fill) => (
					<PayloadSection
						fill={fill}
						title={t("activity.drawer.mcpRequest", {
							defaultValue: "MCP request",
						})}
						value={protocolView.request}
						copyLabel={t("actions.copyRequest", { defaultValue: "Copy request" })}
						copyMessage={t("notifications.requestCopySuccessMessage", {
							defaultValue: "Request payload copied to clipboard.",
						})}
						onFilterByServerId={onFilterByServerId}
						onFilterBySessionId={onFilterBySessionId}
					/>
				),
			});
		}

		if (protocolView.response !== undefined) {
			sections.push({
				key: "response",
				node: (fill) => (
					<InspectorMcpResponseViewer
						fill={fill}
						response={protocolView.response}
						kind={capabilityKind}
						title={t("activity.drawer.mcpResponse", {
							defaultValue: "MCP response",
						})}
					/>
				),
			});
		}

		if (protocolView.notification !== undefined) {
			sections.push({
				key: "notification",
				node: (fill) => (
					<PayloadSection
						fill={fill}
						title={t("activity.drawer.mcpNotification", {
							defaultValue: "MCP notification",
						})}
						value={protocolView.notification}
						copyLabel={t("actions.copyNotification", {
							defaultValue: "Copy notification",
						})}
						copyMessage={t("notifications.notificationCopySuccessMessage", {
							defaultValue: "Notification payload copied to clipboard.",
						})}
						onFilterByServerId={onFilterByServerId}
						onFilterBySessionId={onFilterBySessionId}
					/>
				),
			});
		}

		return sections;
	}, [capabilityKind, hasProtocolPayload, onFilterByServerId, onFilterBySessionId, protocolView, t]);

	const lastPayloadIndex = payloadSections.length - 1;

	if (!hasContext && payloadSections.length === 0) {
		return (
			<div className="text-sm text-muted-foreground">
				{t("activity.drawer.mcpEmpty", {
					defaultValue: "No MCP request, response, or notification payload for this activity entry.",
				})}
			</div>
		);
	}

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
			{hasContext ? (
				<InspectorContextCard
					protocolView={protocolView}
					t={t}
					onFilterByServerId={onFilterByServerId}
					onFilterBySessionId={onFilterBySessionId}
				/>
			) : null}
			{payloadSections.length > 0 ? (
				<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
					{payloadSections.map((section, index) => (
						<div
							key={section.key}
							className={cn(
								"min-h-0 min-w-0",
								index === lastPayloadIndex && "flex flex-1 flex-col",
							)}
						>
							{section.node(index === lastPayloadIndex)}
						</div>
					))}
				</div>
			) : null}
		</div>
	);
}

function FallbackActivityContent({
	entry,
	t,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	entry: InspectorLogEventEntry;
	t: (key: string, options?: Record<string, unknown>) => string;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	const capabilityKind = inferInspectorCapabilityKindFromEntry(entry);

	const payloadSections = useMemo(() => {
		const sections: Array<{ key: string; node: (fill: boolean) => ReactNode }> = [];

		if (entry.data !== undefined) {
			sections.push({
				key: "activity",
				node: (fill) => (
					<PayloadSection
						fill={fill}
						title={t("activity.drawer.activity", { defaultValue: "Activity" })}
						value={entry.data}
						copyLabel={t("actions.copyActivity", { defaultValue: "Copy activity" })}
						copyMessage={t("notifications.activityCopySuccessMessage", {
							defaultValue: "Activity payload copied to clipboard.",
						})}
						onFilterByServerId={onFilterByServerId}
						onFilterBySessionId={onFilterBySessionId}
					/>
				),
			});
		}

		if (entry.request !== undefined) {
			sections.push({
				key: "request",
				node: (fill) => (
					<PayloadSection
						fill={fill}
						title={t("activity.drawer.request", { defaultValue: "Request" })}
						value={entry.request}
						copyLabel={t("actions.copyRequest", { defaultValue: "Copy request" })}
						copyMessage={t("notifications.requestCopySuccessMessage", {
							defaultValue: "Request payload copied to clipboard.",
						})}
						onFilterByServerId={onFilterByServerId}
						onFilterBySessionId={onFilterBySessionId}
					/>
				),
			});
		}

		if (entry.response !== undefined) {
			sections.push({
				key: "response",
				node: (fill) => (
					<InspectorMcpResponseViewer
						fill={fill}
						response={entry.response}
						kind={capabilityKind}
						title={t("activity.drawer.response", { defaultValue: "Response" })}
					/>
				),
			});
		}

		return sections;
	}, [capabilityKind, entry, onFilterByServerId, onFilterBySessionId, t]);

	const lastPayloadIndex = payloadSections.length - 1;

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
			{payloadSections.map((section, index) => (
				<div
					key={section.key}
					className={cn(
						"min-h-0 min-w-0",
						index === lastPayloadIndex && "flex flex-1 flex-col",
					)}
				>
					{section.node(index === lastPayloadIndex)}
				</div>
			))}
		</div>
	);
}

export function InspectorEventDetailDrawer({
	open,
	onOpenChange,
	entry,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	entry: InspectorLogEventEntry | null;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	const { t } = useTranslation("inspector");
	const translateEvent = useCallback<InspectorLogTranslate>(
		(key, options) => {
			if (key.startsWith("inspector:")) {
				return t(key.replace(/^inspector:/, ""), options);
			}
			return t(key, options);
		},
		[t],
	);
	const actionLabel = entry ? formatInspectorEventAction(entry, translateEvent) : "";
	const description = useMemo(() => {
		if (!entry) {
			return t("activity.drawer.subtitle", {
				defaultValue: "Inspect the full request and response payload",
			});
		}
		return `${actionLabel} · ${formatLocalDateTime(entry.timestamp)}`;
	}, [actionLabel, entry, t]);

	const protocolView = useMemo(
		() => (entry ? buildInspectorEventProtocolView(entry) : null),
		[entry],
	);

	const handleFilterByServerId = useCallback(
		(serverId: string) => {
			onFilterByServerId?.(serverId);
			onOpenChange(false);
		},
		[onFilterByServerId, onOpenChange],
	);

	const handleFilterBySessionId = useCallback(
		(sessionId: string) => {
			onFilterBySessionId?.(sessionId);
			onOpenChange(false);
		},
		[onFilterBySessionId, onOpenChange],
	);

	return (
		<Drawer open={open} onOpenChange={onOpenChange} direction="right">
			<DrawerContent className="flex h-full flex-col overflow-hidden">
				<DrawerHeader className="shrink-0">
					<div className="min-w-0">
						<DrawerTitle>
							{t("activity.drawer.title", { defaultValue: "Activity details" })}
						</DrawerTitle>
						<DrawerDescription>{description}</DrawerDescription>
					</div>
				</DrawerHeader>
				<div className="flex min-h-0 flex-1 flex-col overflow-hidden px-4 pb-3 pt-0">
					{entry ? (
						protocolView ? (
							<McpProtocolContent
								protocolView={protocolView}
								entry={entry}
								t={t}
								onFilterByServerId={handleFilterByServerId}
								onFilterBySessionId={handleFilterBySessionId}
							/>
						) : (
							<FallbackActivityContent
								entry={entry}
								t={t}
								onFilterByServerId={handleFilterByServerId}
								onFilterBySessionId={handleFilterBySessionId}
							/>
						)
					) : (
						<div className="text-sm text-muted-foreground">
							{t("activity.drawer.empty", { defaultValue: "Select an activity entry to inspect." })}
						</div>
					)}
				</div>
			</DrawerContent>
		</Drawer>
	);
}
