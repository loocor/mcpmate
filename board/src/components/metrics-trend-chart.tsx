import React from "react";
import { Activity } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { LegendProps, TooltipProps } from "recharts";
import {
	CartesianGrid,
	Legend,
	Line,
	LineChart,
	ResponsiveContainer,
	Tooltip,
	XAxis,
	YAxis,
} from "recharts";
import {
	computeMetricsChartYAxisMax,
	formatMetricsYAxisTick,
	useIsDarkMode,
	useMetricsHistory,
} from "../hooks/use-metrics-history";
import {
	DASHBOARD_CHART_LEGEND_WRAPPER_CLASS,
	DASHBOARD_CHART_VIEWPORT_CLASS,
	DASHBOARD_LINE_CHART_MARGIN,
	DashboardChartPlaceholder,
	DashboardChartSkeleton,
	OPERATOR_CAROUSEL_CHART_VIEWPORT_CLASS,
	OPERATOR_CAROUSEL_LINE_CHART_MARGIN,
} from "./dashboard-chart-area";
import { cn } from "../lib/utils";

type MetricsTooltipContentProps = TooltipProps<number, string> & {
	formatValue: (
		value: number,
		name: string,
		item: NonNullable<TooltipProps<number, string>["payload"]>[number],
	) => [string | number, string];
};

function MetricsTooltipContent({
	active,
	payload,
	label,
	formatValue,
}: MetricsTooltipContentProps) {
	if (!active || !payload || payload.length === 0) {
		return null;
	}
	return (
		<div className="rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-xs text-slate-100 shadow-lg">
			{label ? <div className="mb-1 text-[11px] text-slate-400">{label}</div> : null}
			<div className="space-y-1">
				{payload.map((item, index) => {
					if (typeof item.value !== "number") {
						return null;
					}
					const [valueLabel] = formatValue(item.value, item.name ?? "", item);
					const displayName = item.name ?? item.dataKey ?? "";
					const color = item.color ?? "#9ca3af";
					const key = `${String(item.dataKey ?? displayName)}-${index}`;
					const valueDisplay =
						typeof valueLabel === "number" ? valueLabel.toString() : String(valueLabel);
					return (
						<div key={key} className="flex items-center justify-between gap-4">
							<div className="flex items-center gap-2 text-[11px] text-slate-300">
								<span
									className="inline-block h-2 w-2 rounded-full"
									style={{ backgroundColor: color }}
								/>
								<span style={{ color }}>{displayName}</span>
							</div>
							<span className="min-w-[48px] text-right text-[11px] font-semibold text-slate-50">
								{valueDisplay}
							</span>
						</div>
					);
				})}
			</div>
		</div>
	);
}

function MetricsLegend({ payload }: { payload?: LegendProps["payload"] }) {
	if (!payload || payload.length === 0) {
		return null;
	}
	return (
		<div className={DASHBOARD_CHART_LEGEND_WRAPPER_CLASS}>
			{payload.map((entry) => {
				const key =
					typeof entry.dataKey === "string" && entry.dataKey.length > 0
						? entry.dataKey
						: String(entry.value);
				const swatchColor = entry.color ?? "#9ca3af";
				return (
					<div key={key} className="flex items-center gap-1">
						<span
							className="inline-block h-2 w-2 rounded-full"
							style={{ backgroundColor: swatchColor }}
						/>
						<span style={{ color: swatchColor }}>{entry.value}</span>
					</div>
				);
			})}
		</div>
	);
}

export function MetricsTrendChart({
	className,
	variant = "default",
}: {
	className?: string;
	variant?: "default" | "compact";
}) {
	const { t } = useTranslation();
	const { history, isLoading } = useMetricsHistory();
	const isDarkMode = useIsDarkMode();
	const isCompact = variant === "compact";

	const gridStroke = isDarkMode
		? "rgba(55, 65, 81, 0.6)"
		: "rgba(148, 163, 184, 0.25)";

	const metricsChartYAxisMax = React.useMemo(
		() => computeMetricsChartYAxisMax(history),
		[history],
	);

	const formatMegabytes = React.useCallback((value?: number) => {
		if (typeof value !== "number" || Number.isNaN(value)) {
			return "—";
		}
		if (value >= 1024) {
			return `${(value / 1024).toFixed(1)} GB`;
		}
		return `${value.toFixed(1)} MB`;
	}, []);

	const metricsTooltipFormatter = React.useCallback(
		(
			value: number,
			name: string,
			item: NonNullable<TooltipProps<number, string>["payload"]>[number],
		): [string | number, string] => {
			const dataKey = typeof item.dataKey === "string" ? item.dataKey : name;
			const percentLabel = `${value.toFixed(1)}%`;
			const payload = item.payload as Partial<{
				mcpmateMemoryMb: number;
			}>;
			if (dataKey === "mcpmateMemoryPercent") {
				return [`${percentLabel} • ${formatMegabytes(payload.mcpmateMemoryMb)}`, name];
			}
			return [percentLabel, name];
		},
		[formatMegabytes],
	);

	const renderTooltip = React.useCallback(
		(props: TooltipProps<number, string>) => (
			<MetricsTooltipContent {...props} formatValue={metricsTooltipFormatter} />
		),
		[metricsTooltipFormatter],
	);

	const metricsYAxisTickFormatter = React.useCallback(
		(value: number) => formatMetricsYAxisTick(value, metricsChartYAxisMax),
		[metricsChartYAxisMax],
	);

	const viewportClass = isCompact
		? OPERATOR_CAROUSEL_CHART_VIEWPORT_CLASS
		: DASHBOARD_CHART_VIEWPORT_CLASS;
	const chartMargin = isCompact ? OPERATOR_CAROUSEL_LINE_CHART_MARGIN : DASHBOARD_LINE_CHART_MARGIN;

	if (isLoading) {
		return <DashboardChartSkeleton className={cn(viewportClass, className)} />;
	}

	if (history.length === 0) {
		return (
			<DashboardChartPlaceholder className={cn(viewportClass, className)}>
				<Activity className={cn("text-sky-500/80", isCompact ? "h-5 w-5" : "h-8 w-8")} aria-hidden />
				<p
					className={cn(
						"font-medium text-slate-700 dark:text-slate-200",
						isCompact ? "text-xs" : "text-sm",
					)}
				>
					{t("dashboard:metrics.noData", {
						defaultValue: "No metrics have been reported yet.",
					})}
				</p>
			</DashboardChartPlaceholder>
		);
	}

	return (
		<div className={cn(viewportClass, className)}>
			<ResponsiveContainer width="100%" height="100%">
				<LineChart data={history} margin={chartMargin}>
					<CartesianGrid strokeDasharray="3 3" stroke={gridStroke} />
					<XAxis
						dataKey="time"
						stroke="#9ca3af"
						fontSize={isCompact ? 10 : 11}
						height={isCompact ? 18 : 26}
						axisLine={false}
						tickLine={false}
					/>
					<YAxis
						domain={[0, metricsChartYAxisMax]}
						stroke="#9ca3af"
						fontSize={isCompact ? 10 : 11}
						tickFormatter={metricsYAxisTickFormatter}
						width={isCompact ? 36 : 52}
					/>
					<Tooltip content={renderTooltip} />
					{!isCompact ? (
						<Legend content={(legendProps) => <MetricsLegend {...legendProps} />} />
					) : null}
					<Line
						type="monotone"
						dataKey="mcpmateCpuPercent"
						name={t("dashboard:metrics.mcpmateCpu", {
							defaultValue: "CPU (%)",
						})}
						stroke="#3b82f6"
						strokeWidth={2}
						dot={false}
						activeDot={{ r: 4, strokeWidth: 0 }}
						isAnimationActive={false}
					/>
					<Line
						type="monotone"
						dataKey="mcpmateMemoryPercent"
						name={t("dashboard:metrics.mcpmateMemory", {
							defaultValue: "Memory (%)",
						})}
						stroke="#10b981"
						strokeWidth={2}
						strokeDasharray="6 4"
						dot={false}
						activeDot={{ r: 4, strokeWidth: 0 }}
						isAnimationActive={false}
					/>
				</LineChart>
			</ResponsiveContainer>
		</div>
	);
}
