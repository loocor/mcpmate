import { useTranslation } from "react-i18next";
import type { InstanceSummary } from "../lib/types";
import { getStatusVariant } from "../lib/utils";
import { Badge } from "./ui/badge";

interface StatusBadgeProps {
	status?: string;
	instances?: InstanceSummary[];
	showLabel?: boolean;
	className?: string;
	blinkOnError?: boolean;
	isServerEnabled?: boolean;
}

export function StatusBadge({
	status = "unknown",
	instances = [],
	showLabel = true,
	className = "",
	blinkOnError = true,
	isServerEnabled = false,
}: StatusBadgeProps) {
	const { t } = useTranslation();
	// If instances array is provided, determine overall status based on instance statuses
	let statusStr = status?.toString().toLowerCase() || "unknown";
	let shouldBlink = false;

	if (instances && instances.length > 0) {
		// Check if any instance is in ready or busy state
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

		// Check if any instance is in error state
		const hasErrorInstance = instances.some((instance) =>
			["error", "unhealthy", "stopped", "failed"].includes(
				(instance.status || "").toLowerCase(),
			),
		);

		// Check if any instance is initializing
		const hasInitializingInstance = instances.some((instance) =>
			["initializing", "starting", "connecting"].includes(
				(instance.status || "").toLowerCase(),
			),
		);

		// 状态优先级：如果有任何实例是活跃的，整个服务就是活跃的
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
	} else {
		// Handle case where no instances but we know server status
		if (instances.length === 0 && isServerEnabled) {
			statusStr = "idle"; // Server is enabled but not running (no instances)
		} else if (
			["error", "unhealthy", "stopped", "failed"].includes(statusStr) &&
			blinkOnError
		) {
			shouldBlink = true;
		}
	}

	const variant = getStatusVariant(statusStr);

	// Determine display text based on status
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

	return (
		<Badge
			variant={variant}
			className={`${className} ${shouldBlink ? "animate-pulse" : ""}`}
		>
			<span className="flex items-center">
				<span
					className={`mr-1 h-2 w-2 rounded-full ${
						variant === "success"
							? "bg-emerald-400"
							: variant === "warning"
								? "bg-amber-400"
								: variant === "destructive"
									? "bg-red-400"
									: variant === "secondary"
										? "bg-slate-400"
										: "bg-slate-400"
					}`}
				/>
				{showLabel && displayText}
			</span>
		</Badge>
	);
}
