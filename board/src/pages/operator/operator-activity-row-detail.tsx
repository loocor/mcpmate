import React from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Button } from "../../components/ui/button";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import type { AuditEventRecord } from "../../lib/types";
import { cn } from "../../lib/utils";
import {
	formatOperatorAuditAction,
	formatOperatorAuditRelativeTime,
	operatorAuditStatusDotClass,
} from "./operator-audit-format";
import {
	operatorNoDragRegionStyle,
	OperatorRowDetailFrame,
	OperatorRowDetailMessage,
} from "./operator-row-detail-shared";

const ACTIVITY_LIST_MAX_HEIGHT_CLASS = "max-h-[208px]";

function auditEventKey(event: AuditEventRecord, index: number): string {
	return String(
		event.id ?? `${event.occurred_at_ms}-${event.action}-${event.category}-${index}`,
	);
}

function OperatorActivityOpenLogsLink({
	isTauriShell,
	label,
	onOpenLogsBoard,
}: {
	isTauriShell: boolean;
	label: string;
	onOpenLogsBoard: () => void;
}) {
	const className =
		"h-7 w-full text-[11px] text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-200";

	if (isTauriShell) {
		return (
			<Button
				type="button"
				variant="ghost"
				size="sm"
				className={className}
				style={operatorNoDragRegionStyle}
				onClick={(event) => {
					event.stopPropagation();
					onOpenLogsBoard();
				}}
			>
				{label}
			</Button>
		);
	}

	return (
		<Button asChild variant="ghost" size="sm" className={className} style={operatorNoDragRegionStyle}>
			<Link to="/audit" onClick={(event) => event.stopPropagation()}>
				{label}
			</Link>
		</Button>
	);
}

function OperatorActivityEventRow({
	event,
	locale,
}: {
	event: AuditEventRecord;
	locale: string;
}) {
	const { t } = useTranslation();
	const actionLabel = formatOperatorAuditAction(event.action, t);
	const timeLabel = formatOperatorAuditRelativeTime(event.occurred_at_ms, locale);

	return (
		<li className="grid grid-cols-[12px_minmax(0,1fr)] items-start gap-x-2 py-1.5">
			<span className="flex h-3.5 shrink-0 items-center justify-center">
				<span
					className={cn(
						"h-1.5 w-1.5 rounded-full",
						operatorAuditStatusDotClass(event.status),
					)}
					aria-hidden
				/>
			</span>
			<div className="min-w-0">
				<p
					className="truncate text-[11px] font-medium leading-3.5 text-slate-800 dark:text-slate-100"
					title={actionLabel}
				>
					{actionLabel}
				</p>
				<p className="mt-0.5 truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">
					{timeLabel}
				</p>
			</div>
		</li>
	);
}

export function OperatorActivityRowDetail({
	detailId,
	events,
	isError,
	isLoading,
	isTauriShell,
	onOpenLogsBoard,
}: {
	detailId: string;
	events: AuditEventRecord[];
	isError: boolean;
	isLoading: boolean;
	isTauriShell: boolean;
	onOpenLogsBoard: () => void;
}) {
	usePageTranslations("audit");
	const { t, i18n } = useTranslation();
	const openLogsLabel = t("operator:detail.activity.openLogs", {
		defaultValue: "Open Logs in Full Board",
	});

	return (
		<OperatorRowDetailFrame detailId={detailId}>
			{isLoading ? (
				<OperatorRowDetailMessage>
					{t("operator:rows.activity.loading", { defaultValue: "Loading activity" })}
				</OperatorRowDetailMessage>
			) : isError ? (
				<OperatorRowDetailMessage tone="error">
					{t("operator:rows.activity.error", { defaultValue: "Activity is unavailable" })}
				</OperatorRowDetailMessage>
			) : events.length === 0 ? (
				<OperatorRowDetailMessage>
					{t("operator:rows.activity.noEvents", { defaultValue: "No recent activity" })}
				</OperatorRowDetailMessage>
			) : (
				<>
					<div
						className={cn(
							"-mx-3 min-h-0 overflow-y-auto overscroll-y-contain px-3",
							ACTIVITY_LIST_MAX_HEIGHT_CLASS,
							"[scrollbar-width:thin] [&::-webkit-scrollbar]:w-1",
						)}
						data-testid="operator-activity-scroll"
					>
						<ul className="divide-y divide-slate-100 dark:divide-slate-800">
							{events.map((event, index) => (
								<OperatorActivityEventRow
									key={auditEventKey(event, index)}
									event={event}
									locale={i18n.language}
								/>
							))}
						</ul>
					</div>
					<div className="-mx-3 border-t border-slate-100 px-1 pt-0.5 dark:border-slate-800">
						<OperatorActivityOpenLogsLink
							isTauriShell={isTauriShell}
							label={openLogsLabel}
							onOpenLogsBoard={onOpenLogsBoard}
						/>
					</div>
				</>
			)}
		</OperatorRowDetailFrame>
	);
}
