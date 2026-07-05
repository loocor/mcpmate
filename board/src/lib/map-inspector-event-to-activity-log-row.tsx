import type { ReactNode } from "react";
import { Badge } from "../components/ui/badge";
import type { ActivityLogRow } from "./activity-log-row";
import {
	formatInspectorEventAction,
	formatInspectorEventDetails,
	formatInspectorEventMeta,
	inspectorEventCategory,
	inspectorEventDurationMs,
	inspectorEventHasPayload,
	inspectorEventRowKey,
	inspectorEventStatus,
	inspectorEventTarget,
	resolveInspectorEventCategoryKind,
	type InspectorLogEventEntry,
	type InspectorLogTranslate,
} from "./inspector-event-log";

function renderInspectorCategoryBadge(
	entry: InspectorLogEventEntry,
	t: InspectorLogTranslate,
): ReactNode {
	const kind = resolveInspectorEventCategoryKind(entry);
	const label = inspectorEventCategory(entry, t);
	let variant: "secondary" | "default" | "outline";
	switch (kind) {
		case "platform":
			variant = "outline";
			break;
		case "ai":
			variant = "secondary";
			break;
		case "mcp":
			variant = "default";
			break;
	}
	return (
		<Badge variant={variant} className="max-w-full truncate font-normal">
			{label}
		</Badge>
	);
}

function renderInspectorStatusBadge(
	status: ReturnType<typeof inspectorEventStatus>,
	t: InspectorLogTranslate,
): ReactNode {
	if (status === "active") {
		return (
			<Badge variant="secondary">
				{t("inspector:activity.status.active", { defaultValue: "Active" })}
			</Badge>
		);
	}
	let variant: "success" | "destructive" | "warning";
	switch (status) {
		case "success":
			variant = "success";
			break;
		case "failed":
			variant = "destructive";
			break;
		default:
			variant = "warning";
	}
	return (
		<Badge variant={variant}>
			{t(`audit:statusValues.${status}`, { defaultValue: status })}
		</Badge>
	);
}

function renderInspectorEventDetails(
	entry: InspectorLogEventEntry,
	t: InspectorLogTranslate,
): ReactNode | undefined {
	const detail = formatInspectorEventDetails(entry, t);
	const meta = formatInspectorEventMeta(entry, t);
	if (!detail && meta.length === 0) {
		return undefined;
	}
	return (
		<div className="space-y-2">
			{detail ? (
				<pre className="whitespace-pre-wrap break-words text-muted-foreground">{detail}</pre>
			) : null}
			{meta.length > 0 ? (
				<div className="text-xs text-muted-foreground">{meta.join(" · ")}</div>
			) : null}
		</div>
	);
}

export function mapInspectorEventToActivityLogRow(
	entry: InspectorLogEventEntry,
	index: number,
	t: InspectorLogTranslate,
): ActivityLogRow {
	const details = renderInspectorEventDetails(entry, t);
	return {
		key: inspectorEventRowKey(entry, index),
		eventId: entry.id,
		timestampMs: entry.timestamp,
		action: formatInspectorEventAction(entry, t),
		category: renderInspectorCategoryBadge(entry, t),
		status: renderInspectorStatusBadge(inspectorEventStatus(entry), t),
		target: inspectorEventTarget(entry),
		durationMs: inspectorEventDurationMs(entry),
		details,
		expandable: inspectorEventHasPayload(entry) || details != null,
	};
}
