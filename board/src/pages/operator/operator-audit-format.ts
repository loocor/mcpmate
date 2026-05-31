import type { TFunction } from "i18next";
import { formatRelativeTime } from "../../lib/utils";

export function formatOperatorAuditAction(
	action: string | undefined,
	t: TFunction,
): string {
	if (!action) {
		return "";
	}
	return t(`audit:actionValues.${action}`, {
		defaultValue: action.replaceAll("_", " "),
	});
}

export function formatOperatorAuditRelativeTime(
	occurredAtMs: number,
	locale?: string,
): string {
	return formatRelativeTime(new Date(occurredAtMs).toISOString(), locale);
}

export function operatorAuditStatusDotClass(status: string | undefined): string {
	switch (String(status || "").toLowerCase()) {
		case "success":
			return "bg-emerald-500";
		case "failed":
			return "bg-red-500";
		case "cancelled":
			return "bg-slate-400 dark:bg-slate-500";
		default:
			return "bg-sky-500";
	}
}
