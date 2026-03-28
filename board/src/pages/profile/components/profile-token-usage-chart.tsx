import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Cell, Pie, PieChart, Tooltip } from "recharts";
import { Spinner } from "../../../components/ui/spinner";
import { computeProfileTrimTokens } from "../../../lib/profile-token-ledger";
import type { CapabilityTokenLedgerRow, TokenEstimateResponse } from "../../../lib/types";

const CHART_COLORS = {
	visible: "#22c55e",
	disabled: "#3b82f6",
};

/** Fixed pixel size — avoids ResponsiveContainer stretching flex parents. */
const CHART_SIZE = 64;
const INNER_RADIUS = 17;
const OUTER_RADIUS = 28;

interface ProfileTokenUsageChartProps {
	ledgerItems: CapabilityTokenLedgerRow[] | undefined;
	/** Older backends without capability-token-ledger: server aggregate (chars/4); toggles refresh via query key. */
	fallbackEstimate: TokenEstimateResponse | null;
	isLoading: boolean;
	isError: boolean;
	enabledByComponentId: ReadonlyMap<string, boolean>;
	className?: string;
}

/**
 * Header donut: cl100k sums from ledger payloads × live enable map, or token-estimate fallback on 404.
 */
export function ProfileTokenUsageChart({
	ledgerItems,
	fallbackEstimate,
	isLoading,
	isError,
	enabledByComponentId,
	className,
}: ProfileTokenUsageChartProps) {
	const { t, i18n } = useTranslation();

	const { totalTokens, visibleTokens } = useMemo(() => {
		if (fallbackEstimate) {
			return {
				totalTokens: fallbackEstimate.total_available_tokens,
				visibleTokens: fallbackEstimate.visible_tokens,
			};
		}
		return computeProfileTrimTokens(ledgerItems, enabledByComponentId);
	}, [fallbackEstimate, ledgerItems, enabledByComponentId]);

	const disabledTokens = Math.max(0, totalTokens - visibleTokens);
	const visiblePercent =
		totalTokens > 0 ? Math.round((visibleTokens / totalTokens) * 100) : 0;

	const formatNumber = (value: number) =>
		new Intl.NumberFormat(i18n.language).format(value);

	const chartData = useMemo(() => {
		if (totalTokens <= 0) {
			return [];
		}
		return [
			{
				name: t("profiles:detail.tokenSavings.visible", {
					defaultValue: "Currently Exposed",
				}),
				value: visibleTokens,
				fill: CHART_COLORS.visible,
			},
			{
				name: t("profiles:detail.tokenSavings.saved", {
					defaultValue: "Filtered Out",
				}),
				value: disabledTokens,
				fill: CHART_COLORS.disabled,
			},
		].filter((item) => item.value > 0);
	}, [totalTokens, visibleTokens, disabledTokens, t, i18n.language]);

	const percentLabel = t("profiles:detail.tokenSavings.exposedPercentAria", {
		defaultValue: "{{percent}}% of estimated context is currently exposed",
		percent: visiblePercent,
	});

	const shellClass = [
		"relative inline-flex shrink-0 items-center justify-center",
		className,
	]
		.filter(Boolean)
		.join(" ");

	const shellStyle = {
		width: CHART_SIZE,
		height: CHART_SIZE,
	} as const;

	return (
		<div className={shellClass} style={shellStyle}>
			{isLoading ? (
				<Spinner size="sm" />
			) : isError ? (
				<span className="max-w-[4.5rem] text-center text-[9px] leading-tight text-destructive">
					{t("profiles:detail.tokenSavings.error", {
						defaultValue: "Failed to load token estimate.",
					})}
				</span>
			) : chartData.length > 0 ? (
				<>
					<PieChart
						width={CHART_SIZE}
						height={CHART_SIZE}
						margin={{ top: 0, right: 0, bottom: 0, left: 0 }}
						className="shrink-0"
					>
						<Pie
							data={chartData}
							dataKey="value"
							cx="50%"
							cy="50%"
							innerRadius={INNER_RADIUS}
							outerRadius={OUTER_RADIUS}
							strokeWidth={0}
							paddingAngle={chartData.length > 1 ? 2 : 0}
							isAnimationActive={false}
						>
							{chartData.map((entry) => (
								<Cell key={entry.name} fill={entry.fill} />
							))}
						</Pie>
						<Tooltip
							formatter={(value: number) => formatNumber(value)}
							contentStyle={{
								backgroundColor: "hsl(var(--popover))",
								border: "1px solid hsl(var(--border))",
								borderRadius: "8px",
								fontSize: "12px",
							}}
						/>
					</PieChart>
					<div
						className="pointer-events-none absolute inset-0 flex items-center justify-center"
						aria-hidden
					>
						<span
							className="text-[11px] font-semibold tabular-nums leading-none tracking-tight text-foreground"
							title={percentLabel}
						>
							{visiblePercent}%
						</span>
					</div>
					<span className="sr-only">{percentLabel}</span>
				</>
			) : (
				<div className="flex h-full w-full items-center justify-center px-0.5 text-center text-[9px] leading-tight text-muted-foreground">
					{t("profiles:detail.tokenSavings.noSavings", {
						defaultValue: "No savings — all capabilities enabled",
					})}
				</div>
			)}
		</div>
	);
}
