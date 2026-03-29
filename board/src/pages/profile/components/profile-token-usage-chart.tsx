import { useMemo } from "react";
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
	estimateMethod: ProfileTokenEstimateMethod;
	className?: string;
	/** Donut + center percent only (e.g. profile list cards); omits the left legend column. */
	layout?: "default" | "chartOnly";
	/**
	 * When defined and `0`, shows a gray full ring and "-" in the center (no token data).
	 * Omit or leave undefined while profile server list is still loading.
	 */
	profileServerCount?: number;
}

/**
 * Shared profile token donut: used on the Profiles grid (`layout="chartOnly"`) and the profile detail header
 * (`layout="default"`). Token sums come from ledger payloads × live enable map, or legacy token-estimate on 404.
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
}: ProfileTokenUsageChartProps) {
	const { t, i18n } = useTranslation();

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
		width: CHART_SIZE,
		height: CHART_SIZE,
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
			</PieChart>
			<div
				className="pointer-events-none absolute inset-0 flex items-center justify-center"
				aria-hidden
			>
				{noServersInProfile ? (
					<span className="text-[11px] font-semibold tabular-nums leading-none tracking-tight text-muted-foreground">
						-
					</span>
				) : (
					<span className="text-[11px] font-semibold tabular-nums leading-none tracking-tight text-foreground">
						{visiblePercent}%
					</span>
				)}
			</div>
		</div>
	);

	return (
		<div
			className={cn(
				"inline-flex max-w-full shrink-0 items-center gap-2 sm:gap-3",
				layout === "chartOnly" && "gap-0",
				className,
			)}
		>
			{noServersInProfile ? (
				<>
					{donutBlock}
					<span className="sr-only">{noServersAria}</span>
				</>
			) : isLoading ? (
				<Spinner size="sm" />
			) : isError ? (
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
			) : chartData.length > 0 ? (
				<>
					{legendBlock}
					{donutBlock}
					<span className="sr-only">{percentLabel}</span>
				</>
			) : (
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
			)}
		</div>
	);
}
