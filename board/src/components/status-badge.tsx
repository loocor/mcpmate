import { useTranslation } from "react-i18next";
import type { InstanceSummary } from "../lib/types";
import { getStatusVariant } from "../lib/utils";
import { Badge } from "./ui/badge";

interface StatusBadgeProps {
	status?: string;
	statusLabel?: string;
	instances?: InstanceSummary[];
	showLabel?: boolean;
	className?: string;
	blinkOnError?: boolean;
	isServerEnabled?: boolean;
	appearance?: "badge" | "plain";
}

function getStatusTextClass(
	variant: ReturnType<typeof getStatusVariant>,
): string {
	switch (variant) {
		case "success":
			return "text-emerald-700 dark:text-emerald-300";
		case "warning":
			return "text-amber-600 dark:text-amber-400";
		case "destructive":
			return "text-red-600 dark:text-red-400";
		default:
			return "text-slate-600 dark:text-slate-300";
	}
}

function getStatusDotClass(variant: ReturnType<typeof getStatusVariant>): string {
	switch (variant) {
		case "success":
			return "bg-emerald-400";
		case "warning":
			return "bg-amber-400";
		case "destructive":
			return "bg-red-400";
		case "secondary":
			return "bg-slate-400";
		default:
			return "bg-slate-400";
	}
}

export function StatusBadge({
	status = "unknown",
	statusLabel,
	instances = [],
	showLabel = true,
	className = "",
	blinkOnError = true,
	isServerEnabled = false,
	appearance = "badge",
}: StatusBadgeProps) {
	const { t } = useTranslation();
	let statusStr = status?.toString().toLowerCase() || "unknown";
	let shouldBlink = false;

	if (instances.length > 0) {
		const hasActiveInstance = instances.some((instance) =>
			[
				"ready",
				"busy",
				"running",
				"connected",
				"active",
				"healthy",
				"thinking",
				"fetch",
			].includes((instance.status || "").toLowerCase()),
		);

		const hasErrorInstance = instances.some((instance) =>
			["error", "unhealthy", "stopped", "failed"].includes(
				(instance.status || "").toLowerCase(),
			),
		);

		const hasInitializingInstance = instances.some((instance) =>
			["initializing", "starting", "connecting"].includes(
				(instance.status || "").toLowerCase(),
			),
		);

		if (hasActiveInstance) {
			statusStr = "ready";
		} else if (hasInitializingInstance) {
			statusStr = "initializing";
		} else if (hasErrorInstance) {
			statusStr = "error";
			shouldBlink = blinkOnError;
		} else {
			statusStr = "shutdown";
		}
	} else if (isServerEnabled) {
		statusStr = "idle";
	} else if (
		["error", "unhealthy", "stopped", "failed"].includes(statusStr) &&
		blinkOnError
	) {
		shouldBlink = true;
	}

	const variant = getStatusVariant(statusStr);
	const dotClass = getStatusDotClass(variant);

	let displayText = statusStr;
	if (
		[
			"ready",
			"running",
			"connected",
			"busy",
			"active",
			"healthy",
			"thinking",
			"fetch",
		].includes(statusStr)
	) {
		displayText = t("status.ready");
	} else if (["error", "unhealthy", "failed"].includes(statusStr)) {
		displayText = t("status.error");
	} else if (statusStr === "offline") {
		displayText = t("status.disconnected");
	} else if (
		["shutdown", "disconnected", "stopped", "disabled"].includes(statusStr)
	) {
		displayText = t("status.disconnected");
	} else if (["initializing", "starting", "connecting"].includes(statusStr)) {
		displayText = t("status.initializing");
	} else if (statusStr === "idle") {
		displayText = t("status.idle");
	} else {
		displayText = t("status.unknown");
	}

	if (statusLabel != null && statusLabel.trim().length > 0) {
		displayText = statusLabel.trim();
	}

	if (appearance === "plain") {
		if (!showLabel) {
			return null;
		}

		return (
			<span
				className={`text-sm ${getStatusTextClass(variant)} ${className} ${shouldBlink ? "animate-pulse" : ""}`}
			>
				{displayText}
			</span>
		);
	}

	return (
		<Badge
			variant={variant}
			className={`${className} ${shouldBlink ? "animate-pulse" : ""}`}
		>
			<span className="flex items-center">
				<span
					className={`mr-1 h-2 w-2 rounded-full ${dotClass}`}
				/>
				{showLabel && displayText}
			</span>
		</Badge>
	);
}
