import { Copy } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { InspectorMcpResponseViewer } from "./inspector-mcp-response-viewer";
import { Button } from "./ui/button";
import { JsonCodeBlock } from "./json-code-block";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { writeClipboardText } from "../lib/clipboard";
import {
	buildInspectorEventProtocolView,
	type InspectorEventProtocolView,
	serializeInspectorEventEntryForDisplay,
} from "../lib/inspector-event-protocol-view";
import {
	formatInspectorEventAction,
	type InspectorLogEventEntry,
	type InspectorLogTranslate,
} from "../lib/inspector-event-log";
import { inferInspectorCapabilityKindFromEntry } from "../lib/inspector-mcp-response-view";
import { notifyError, notifySuccess } from "../lib/notify";
import { cn, formatLocalDateTime } from "../lib/utils";

function PayloadSection({
	title,
	value,
	fill = false,
}: {
	title: string;
	value: unknown;
	fill?: boolean;
}) {
	if (value === undefined) {
		return null;
	}
	return (
		<section
			className={cn("space-y-2", fill && "flex min-h-0 min-w-0 flex-1 flex-col")}
		>
			<h3 className="shrink-0 text-xs font-medium uppercase tracking-wide text-muted-foreground">
				{title}
			</h3>
			<JsonCodeBlock
				code={JSON.stringify(value, null, 2)}
				className={cn(
					fill
						? "min-h-0 flex-1 overflow-y-auto overflow-x-auto"
						: "max-h-[min(28vh,240px)] shrink-0 overflow-y-auto overflow-x-auto",
				)}
			/>
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

function McpProtocolTabContent({
	protocolView,
	entry,
	t,
}: {
	protocolView: InspectorEventProtocolView;
	entry: InspectorLogEventEntry;
	t: (key: string, options?: Record<string, unknown>) => string;
}) {
	if (!hasMcpProtocolPayload(protocolView)) {
		return (
			<div className="text-sm text-muted-foreground">
				{t("activity.drawer.mcpEmpty", {
					defaultValue: "No MCP request, response, or notification payload for this activity entry.",
				})}
			</div>
		);
	}

	const capabilityKind = inferInspectorCapabilityKindFromEntry(entry);

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-4">
			<PayloadSection
				title={t("activity.drawer.mcpRequest", {
					defaultValue: "MCP request",
				})}
				value={protocolView.request}
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
			/>
		</div>
	);
}

function InspectorContextTabContent({
	protocolView,
	t,
}: {
	protocolView: InspectorEventProtocolView;
	t: (key: string, options?: Record<string, unknown>) => string;
}) {
	if (Object.keys(protocolView.context).length === 0) {
		return (
			<div className="text-sm text-muted-foreground">
				{t("activity.drawer.inspectorEmpty", {
					defaultValue: "No inspector context for this activity entry.",
				})}
			</div>
		);
	}

	return (
		<PayloadSection
			title={t("activity.drawer.inspectorContext", {
				defaultValue: "Inspector context",
			})}
			value={protocolView.context}
		/>
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
	const [detailTab, setDetailTab] = useState<"mcp" | "inspector">("mcp");

	useEffect(() => {
		if (!protocolView) {
			return;
		}
		setDetailTab(hasMcpProtocolPayload(protocolView) ? "mcp" : "inspector");
	}, [entry?.id, protocolView]);

	const handleCopy = useCallback(async () => {
		if (!entry) {
			return;
		}
		try {
			await writeClipboardText(serializeInspectorEventEntryForDisplay(entry));
			notifySuccess(
				t("notifications.activityEntryCopySuccess", { defaultValue: "Activity entry copied" }),
				t("notifications.activityEntryCopySuccessMessage", {
					defaultValue: "Inspector activity payload copied to clipboard.",
				}),
			);
		} catch (error) {
			notifyError(
				t("notifications.copyFailed", { defaultValue: "Copy failed" }),
				error instanceof Error ? error.message : String(error),
			);
		}
	}, [entry, t]);

	return (
		<Drawer open={open} onOpenChange={onOpenChange} direction="right">
			<DrawerContent className="flex h-full flex-col overflow-hidden">
				<DrawerHeader className="shrink-0">
					<div className="flex items-start justify-between gap-3">
						<div className="min-w-0">
							<DrawerTitle>
								{t("activity.drawer.title", { defaultValue: "Activity details" })}
							</DrawerTitle>
							<DrawerDescription>{description}</DrawerDescription>
						</div>
						{entry ? (
							<Button
								type="button"
								variant="outline"
								size="sm"
								className="h-7 shrink-0 gap-1 px-2 text-xs"
								onClick={() => void handleCopy()}
							>
								<Copy className="h-3.5 w-3.5" />
								{t("actions.copy", { defaultValue: "Copy" })}
							</Button>
						) : null}
					</div>
				</DrawerHeader>
				<div className="flex min-h-0 flex-1 flex-col overflow-hidden px-4 py-3">
					{entry ? (
						protocolView ? (
							<Tabs
								value={detailTab}
								onValueChange={(value) => setDetailTab(value as "mcp" | "inspector")}
								className="flex min-h-0 flex-1 flex-col space-y-3"
							>
								<TabsList className="grid w-full grid-cols-2 text-sm">
									<TabsTrigger value="mcp">
										{t("activity.drawer.tabs.mcp", { defaultValue: "MCP" })}
									</TabsTrigger>
									<TabsTrigger value="inspector">
										{t("activity.drawer.tabs.inspector", { defaultValue: "Inspector" })}
									</TabsTrigger>
								</TabsList>
								<TabsContent
									value="mcp"
									className="min-h-0 flex-1 flex-col overflow-hidden data-[state=active]:flex"
								>
									<McpProtocolTabContent
										protocolView={protocolView}
										entry={entry}
										t={t}
									/>
								</TabsContent>
								<TabsContent
									value="inspector"
									className="min-h-0 flex-1 flex-col gap-4 overflow-y-auto data-[state=active]:flex"
								>
									<InspectorContextTabContent protocolView={protocolView} t={t} />
								</TabsContent>
							</Tabs>
						) : (
							<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
								<PayloadSection
									title={t("activity.drawer.activity", { defaultValue: "Activity" })}
									value={entry.data}
								/>
								<PayloadSection
									title={t("activity.drawer.request", { defaultValue: "Request" })}
									value={entry.request}
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
