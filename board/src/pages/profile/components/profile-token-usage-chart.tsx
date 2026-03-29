import { useMemo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Cell, Pie, PieChart } from "recharts";
import { Spinner } from "../../../components/ui/spinner";
import type { ProfileTokenEstimateMethod } from "../../../lib/profile-token-estimate-method";
import { computeProfileTrimTokens } from "../../../lib/profile-token-ledger";
import type { CapabilityTokenLedgerRow, TokenEstimateResponse } from "../../../lib/types";
import { cn } from "../../../lib/utils";

const CHART_COLORS = {
	visible: "#22c55e",
	disabled: "#3b82f6",
	/** Full ring when profile has no servers. */
	emptyServers: "#94a3b8",
};

/** Default square size — avoids ResponsiveContainer stretching flex parents. */
const DEFAULT_CHART_SIZE_PX = 64;
const DEFAULT_INNER_RADIUS = 17;
const DEFAULT_OUTER_RADIUS = 28;

function chartRadiiForSize(chartSizePx: number): {
	size: number;
	innerRadius: number;
	outerRadius: number;
} {
	const scale = chartSizePx / DEFAULT_CHART_SIZE_PX;
	return {
		size: chartSizePx,
		innerRadius: Math.max(1, Math.round(DEFAULT_INNER_RADIUS * scale)),
		outerRadius: Math.max(2, Math.round(DEFAULT_OUTER_RADIUS * scale)),
	};
}

interface ProfileTokenUsageChartProps {
	ledgerItems: CapabilityTokenLedgerRow[] | undefined;
	/** Older backends without capability-token-ledger: server aggregate (chars/4); toggles refresh via query key. */
	fallbackEstimate: TokenEstimateResponse | null;
	isLoading: boolean;
	isError: boolean;
	enabledByComponentId: ReadonlyMap<string, boolean>;
	estimateMethod: ProfileTokenEstimateMethod;
	className?: string;
	/** Donut + center percent only (e.g. profile list cards); omits the left legend column. */
	layout?: "default" | "chartOnly";
	/**
	 * When defined and `0`, shows a gray full ring and "-" in the center (no token data).
	 * Omit or leave undefined while profile server list is still loading.
	 */
	profileServerCount?: number;
	/** Square pixel size for the donut (default 64). */
	chartSizePx?: number;
}

/**
 * Shared profile token donut: used on the Profiles grid (`layout="chartOnly"`), client configuration rows
 * (optional `chartSizePx`), and the profile detail header (`layout="default"`). Token sums come from ledger
 * payloads × live enable map, or legacy token-estimate on 404.
 */
export function ProfileTokenUsageChart({
	ledgerItems,
	fallbackEstimate,
	isLoading,
	isError,
	enabledByComponentId,
	estimateMethod,
	className,
	layout = "default",
	profileServerCount,
	chartSizePx = DEFAULT_CHART_SIZE_PX,
}: ProfileTokenUsageChartProps) {
	const { t, i18n } = useTranslation();

	const { size: chartSize, innerRadius, outerRadius } = useMemo(
		() => chartRadiiForSize(chartSizePx),
		[chartSizePx],
	);

	const centerPercentClass =
		chartSizePx <= 56 ? "text-[10px]" : "text-[11px]";

	const noServersInProfile =
		typeof profileServerCount === "number" && profileServerCount === 0;

	const { totalTokens, visibleTokens } = useMemo(() => {
		if (fallbackEstimate) {
			return {
				totalTokens: fallbackEstimate.total_available_tokens,
				visibleTokens: fallbackEstimate.visible_tokens,
			};
		}
		return computeProfileTrimTokens(
			ledgerItems,
			enabledByComponentId,
			estimateMethod,
		);
	}, [fallbackEstimate, ledgerItems, enabledByComponentId, estimateMethod]);

	const disabledTokens = Math.max(0, totalTokens - visibleTokens);
	const visiblePercent =
		totalTokens > 0 ? Math.round((visibleTokens / totalTokens) * 100) : 0;
	const disabledPercent =
		totalTokens > 0 ? Math.round((disabledTokens / totalTokens) * 100) : 0;

	const formatNumber = (value: number) =>
		new Intl.NumberFormat(i18n.language).format(value);

	const chartData = useMemo(() => {
		if (noServersInProfile) {
			return [{ name: "empty", value: 1, fill: CHART_COLORS.emptyServers }];
		}
		if (totalTokens <= 0) {
			return [];
		}
		return [
			{
				name: "visible",
				value: visibleTokens,
				fill: CHART_COLORS.visible,
			},
			{
				name: "saved",
				value: disabledTokens,
				fill: CHART_COLORS.disabled,
			},
		].filter((item) => item.value > 0);
	}, [noServersInProfile, totalTokens, visibleTokens, disabledTokens]);

	const percentLabel = t("profiles:detail.tokenSavings.exposedPercentAria", {
		defaultValue:
			"About {{percent}}% of estimated profile tokens are in use",
		percent: visiblePercent,
	});

	const shellStyle = {
		width: chartSize,
		height: chartSize,
	} as const;

	const visibleLabel = t("profiles:detail.tokenSavings.visibleTokens", {
		defaultValue: "Tokens in use",
	});
	const savedLabel = t("profiles:detail.tokenSavings.savedTokens", {
		defaultValue: "Saved tokens",
	});

	const legendBlock =
		layout === "default" && !noServersInProfile ? (
			<div
				className="flex min-w-0 flex-col gap-1 border-r border-border/60 pr-2 sm:pr-3"
				aria-label={percentLabel}
			>
				<div className="flex min-w-0 items-center gap-1.5 text-[11px] leading-tight">
					<span
						className="h-2 w-2 shrink-0 rounded-full ring-1 ring-border/60"
						style={{ backgroundColor: CHART_COLORS.visible }}
						aria-hidden
					/>
					<span className="min-w-0 truncate text-muted-foreground">
						{visibleLabel}
					</span>
					<span
						className="mx-0.5 hidden h-px min-w-[6px] flex-1 bg-border/80 sm:block"
						aria-hidden
					/>
					<span className="shrink-0 tabular-nums font-medium text-foreground">
						{formatNumber(visibleTokens)}
					</span>
					<span className="shrink-0 tabular-nums text-muted-foreground">
						({visiblePercent}%)
					</span>
				</div>
				{disabledTokens > 0 ? (
					<div className="flex min-w-0 items-center gap-1.5 text-[11px] leading-tight">
						<span
							className="h-2 w-2 shrink-0 rounded-full ring-1 ring-border/60"
							style={{ backgroundColor: CHART_COLORS.disabled }}
							aria-hidden
						/>
						<span className="min-w-0 truncate text-muted-foreground">
							{savedLabel}
						</span>
						<span
							className="mx-0.5 hidden h-px min-w-[6px] flex-1 bg-border/80 sm:block"
							aria-hidden
						/>
						<span className="shrink-0 tabular-nums font-medium text-foreground">
							{formatNumber(disabledTokens)}
						</span>
						<span className="shrink-0 tabular-nums text-muted-foreground">
							({disabledPercent}%)
						</span>
					</div>
				) : null}
			</div>
		) : null;

	const noServersAria = t("profiles:detail.tokenSavings.noServersChartAria", {
		defaultValue: "No servers in profile",
	});

	let donutAriaLabel: string | undefined;
	if (noServersInProfile) {
		donutAriaLabel = noServersAria;
	} else if (layout === "chartOnly") {
		donutAriaLabel = percentLabel;
	}

	const donutBlock = (
		<div
			className="relative shrink-0"
			style={shellStyle}
			aria-label={donutAriaLabel}
		>
			<PieChart
				width={chartSize}
				height={chartSize}
				margin={{ top: 0, right: 0, bottom: 0, left: 0 }}
				className="shrink-0"
			>
				<Pie
					data={chartData}
					dataKey="value"
					cx="50%"
					cy="50%"
					innerRadius={innerRadius}
					outerRadius={outerRadius}
					strokeWidth={0}
					paddingAngle={chartData.length > 1 ? 2 : 0}
					isAnimationActive={false}
				>
					{chartData.map((entry) => (
						<Cell key={entry.name} fill={entry.fill} />
					))}
				</Pie>
			</PieChart>
			<div
				className="pointer-events-none absolute inset-0 flex items-center justify-center"
				aria-hidden
			>
				{noServersInProfile ? (
					<span
						className={cn(
							"font-semibold tabular-nums leading-none tracking-tight text-muted-foreground",
							centerPercentClass,
						)}
					>
						-
					</span>
				) : (
					<span
						className={cn(
							"font-semibold tabular-nums leading-none tracking-tight text-foreground",
							centerPercentClass,
						)}
					>
						{visiblePercent}%
					</span>
				)}
			</div>
		</div>
	);

	function renderMainContent(): ReactNode {
		if (noServersInProfile) {
			return (
				<>
					{donutBlock}
					<span className="sr-only">{noServersAria}</span>
				</>
			);
		}
		if (isLoading) {
			return (
				<div
					className="flex shrink-0 items-center justify-center"
					style={shellStyle}
					aria-hidden
				>
					<Spinner size="sm" />
				</div>
			);
		}
		if (isError) {
			return (
				<span
					className={cn(
						"leading-tight text-destructive",
						layout === "chartOnly" ? "max-w-[4rem] text-[9px]" : "max-w-[10rem] text-[9px]",
					)}
				>
					{t("profiles:detail.tokenSavings.error", {
						defaultValue: "Failed to load token estimate.",
					})}
				</span>
			);
		}
		if (chartData.length > 0) {
			return (
				<>
					{legendBlock}
					{donutBlock}
					<span className="sr-only">{percentLabel}</span>
				</>
			);
		}
		return (
			<div
				className={cn(
					"flex h-full w-full items-center justify-center text-center text-[9px] leading-tight text-muted-foreground",
					layout === "chartOnly" ? "max-w-[4rem] px-0" : "max-w-[12rem] px-0.5",
				)}
			>
				{t("profiles:detail.tokenSavings.noSavings", {
					defaultValue: "No savings — all capabilities enabled",
				})}
			</div>
		);
	}

	return (
		<div
			className={cn(
				"inline-flex max-w-full shrink-0 items-center gap-2 sm:gap-3",
				layout === "chartOnly" && "gap-0",
				className,
			)}
		>
			{renderMainContent()}
		</div>
	);
}
