import type { ReactNode } from "react";
import { cn } from "../lib/utils";

/**
 * Pixel-locked outer height for dashboard trend charts (Metrics + Token Savings).
 * Single breakpoint-free value keeps the two cards visually identical; min/max avoid flex shrink.
 */
export const DASHBOARD_CHART_VIEWPORT_CLASS =
	"h-[240px] min-h-[240px] max-h-[240px] w-full shrink-0 overflow-hidden";

/**
 * Fixed legend band inside Recharts so 2-line vs 4-line series names do not change plot height.
 */
export const DASHBOARD_CHART_LEGEND_WRAPPER_CLASS =
	"flex h-10 w-full max-w-full flex-shrink-0 flex-wrap items-center justify-center gap-x-3 gap-y-0.5 overflow-hidden px-2 text-xs leading-tight";

/** Shared LineChart margins for Metrics and Token Savings (keeps plot box aligned). */
export const DASHBOARD_LINE_CHART_MARGIN = {
	top: 10,
	right: 24,
	left: 10,
	bottom: 8,
} as const;

export function DashboardChartSkeleton({ className }: { className?: string }) {
	return (
		<div
			className={cn(
				DASHBOARD_CHART_VIEWPORT_CLASS,
				"w-full animate-pulse rounded-md bg-slate-200 dark:bg-slate-800",
				className,
			)}
			aria-hidden
		/>
	);
}

type DashboardChartPlaceholderProps = {
	children: ReactNode;
	className?: string;
};

/**
 * Same outer size as the Recharts container so empty / waiting states do not collapse to a single text line.
 */
export function DashboardChartPlaceholder({
	children,
	className,
}: DashboardChartPlaceholderProps) {
	return (
		<div
			className={cn(
				DASHBOARD_CHART_VIEWPORT_CLASS,
				"flex w-full flex-col items-center justify-center gap-2 rounded-md border border-dashed border-slate-200 bg-slate-50/90 px-4 text-center dark:border-slate-600 dark:bg-slate-800/50",
				className,
			)}
		>
			{children}
		</div>
	);
}
