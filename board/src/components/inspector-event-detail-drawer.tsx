import { ChevronRight, Copy } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { InspectorMcpResponseViewer } from "./inspector-mcp-response-viewer";
import { Button } from "./ui/button";
import { InspectorJsonOutline } from "./inspector-response-preview";
import { JsonCodeBlock } from "./json-code-block";
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
	inferInspectorCapabilityKindFromEntry,
} from "../lib/inspector-mcp-response-view";
import { notifyError, notifySuccess } from "../lib/notify";
import { cn, formatLocalDateTime } from "../lib/utils";
import { Segment, type SegmentOption } from "./ui/segment";

type PayloadDisplayMode = "json" | "outline";

const PAYLOAD_DISPLAY_OPTIONS: SegmentOption[] = [
	{ value: "json", label: "JSON" },
	{ value: "outline", label: "Outline" },
];

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
}: {
	title: string;
	value: unknown;
	fill?: boolean;
	copyLabel?: string;
	copyMessage?: string;
}) {
	const { t } = useTranslation("inspector");
	const [displayMode, setDisplayMode] = useState<PayloadDisplayMode>("json");
	const serializedValue = useMemo(() => serializePayloadForClipboard(value), [value]);

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

	return (
		<section
			className={cn("space-y-2", fill && "flex min-h-0 min-w-0 flex-1 flex-col")}
		>
			<div className="flex shrink-0 items-center justify-between gap-2">
				<h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
					{title}
				</h3>
				<Segment
					value={displayMode}
					onValueChange={(nextMode) => setDisplayMode(nextMode as PayloadDisplayMode)}
					options={PAYLOAD_DISPLAY_OPTIONS}
					showDots={false}
					className={INSPECTOR_COMPACT_SEGMENT_CLASSNAME}
				/>
			</div>
			<div
				className={cn(
					"group/payload relative min-w-0",
					fill && "flex min-h-0 flex-1 flex-col",
				)}
			>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className="absolute right-2 top-2 z-10 h-7 w-7 opacity-0 shadow-sm transition-opacity group-hover/payload:opacity-100 group-focus-within/payload:opacity-100"
					aria-label={copyLabel ?? t("actions.copyPayload", { defaultValue: "Copy payload" })}
					onClick={() => void handleCopy()}
				>
					<Copy className="h-3.5 w-3.5" />
				</Button>
				{displayMode === "outline" ? (
					<InspectorJsonOutline
						value={value}
						className={cn(
							fill
								? "min-h-0 flex-1"
								: "max-h-[min(28vh,240px)] shrink-0",
						)}
					/>
				) : (
					<JsonCodeBlock
						code={JSON.stringify(value, null, 2)}
						className={cn(
							fill
								? "min-h-0 flex-1 overflow-y-auto overflow-x-auto"
								: "max-h-[min(28vh,240px)] shrink-0 overflow-y-auto overflow-x-auto",
						)}
					/>
				)}
			</div>
		</section>
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
}: {
	protocolView: InspectorEventProtocolView;
	t: (key: string, options?: Record<string, unknown>) => string;
}) {
	const [expanded, setExpanded] = useState(false);
	const contextFieldCount = Object.keys(protocolView.context).length;

	if (contextFieldCount === 0) {
		return null;
	}

	return (
		<section className="shrink-0 rounded-md border border-border bg-card text-card-foreground">
			<button
				type="button"
				className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left"
				aria-expanded={expanded}
				onClick={() => setExpanded((current) => !current)}
			>
				<span className="flex min-w-0 items-center gap-2">
					<ChevronRight
						className={cn(
							"h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform",
							expanded && "rotate-90",
						)}
						aria-hidden
					/>
					<span className="truncate text-xs font-medium uppercase tracking-wide text-muted-foreground">
						{t("activity.drawer.inspectorContext", {
							defaultValue: "Inspector context",
						})}
					</span>
				</span>
				<span className="shrink-0 font-mono text-[11px] text-muted-foreground">
					{t("activity.drawer.contextFieldCount", {
						defaultValue: "{{count}} fields",
						count: contextFieldCount,
					})}
				</span>
			</button>
			{expanded ? (
				<div className="border-t border-border p-3">
					<PayloadSection
						title={t("activity.drawer.contextPayload", {
							defaultValue: "Context payload",
						})}
						value={protocolView.context}
						copyLabel={t("actions.copyContext", { defaultValue: "Copy context" })}
						copyMessage={t("notifications.contextCopySuccessMessage", {
							defaultValue: "Inspector context copied to clipboard.",
						})}
					/>
				</div>
			) : null}
		</section>
	);
}

function McpProtocolContent({
	protocolView,
	entry,
	t,
}: {
	protocolView: InspectorEventProtocolView;
	entry: InspectorLogEventEntry;
	t: (key: string, options?: Record<string, unknown>) => string;
}) {
	const capabilityKind = inferInspectorCapabilityKindFromEntry(entry);
	const hasProtocolPayload = hasMcpProtocolPayload(protocolView);

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
			<InspectorContextCard protocolView={protocolView} t={t} />
			{hasProtocolPayload ? (
				<>
					<PayloadSection
						title={t("activity.drawer.mcpRequest", {
							defaultValue: "MCP request",
						})}
						value={protocolView.request}
						copyLabel={t("actions.copyRequest", { defaultValue: "Copy request" })}
						copyMessage={t("notifications.requestCopySuccessMessage", {
							defaultValue: "Request payload copied to clipboard.",
						})}
					/>
					{protocolView.response !== undefined ? (
						<InspectorMcpResponseViewer
							fill
							response={protocolView.response}
							kind={capabilityKind}
							title={t("activity.drawer.mcpResponse", {
								defaultValue: "MCP response",
							})}
						/>
					) : null}
					<PayloadSection
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
					/>
				</>
			) : (
				<div className="text-sm text-muted-foreground">
					{t("activity.drawer.mcpEmpty", {
						defaultValue: "No MCP request, response, or notification payload for this activity entry.",
					})}
				</div>
			)}
		</div>
	);
}

export function InspectorEventDetailDrawer({
	open,
	onOpenChange,
	entry,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	entry: InspectorLogEventEntry | null;
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
				<div className="flex min-h-0 flex-1 flex-col overflow-hidden px-4 py-3">
					{entry ? (
						protocolView ? (
							<McpProtocolContent protocolView={protocolView} entry={entry} t={t} />
						) : (
							<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
								<PayloadSection
									title={t("activity.drawer.activity", { defaultValue: "Activity" })}
									value={entry.data}
									copyLabel={t("actions.copyActivity", { defaultValue: "Copy activity" })}
									copyMessage={t("notifications.activityCopySuccessMessage", {
										defaultValue: "Activity payload copied to clipboard.",
									})}
								/>
								<PayloadSection
									title={t("activity.drawer.request", { defaultValue: "Request" })}
									value={entry.request}
									copyLabel={t("actions.copyRequest", { defaultValue: "Copy request" })}
									copyMessage={t("notifications.requestCopySuccessMessage", {
										defaultValue: "Request payload copied to clipboard.",
									})}
								/>
								{entry.response !== undefined ? (
									<InspectorMcpResponseViewer
										fill
										response={entry.response}
										kind={inferInspectorCapabilityKindFromEntry(entry)}
										title={t("activity.drawer.response", { defaultValue: "Response" })}
									/>
								) : null}
							</div>
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
