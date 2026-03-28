import { useQueries, useQuery } from "@tanstack/react-query";
import { useMemo, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from "recharts";
import { Info, Loader2, Sparkles, TrendingUp } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "./ui/card";
import {
  Tooltip as UiTooltip,
  TooltipArrow,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";
import {
  DASHBOARD_CHART_LEGEND_WRAPPER_CLASS,
  DASHBOARD_CHART_VIEWPORT_CLASS,
  DASHBOARD_LINE_CHART_MARGIN,
  DashboardChartPlaceholder,
  DashboardChartSkeleton,
} from "./dashboard-chart-area";
import { auditApi, capabilityTokenLedgerApi, configSuitsApi } from "../lib/api";
import { computeProfileLedgerTokens } from "../lib/profile-token-ledger";
import {
  formatTokenCount,
} from "../lib/token-utils";

const LINE_COLORS = {
  before: "#3b82f6",
  after: "#22c55e",
};

const HISTORY_KEY = "mcp_token_savings_history_v2";
const MAX_HISTORY_POINTS = 60;
const DASHBOARD_REFRESH_INTERVAL_MS = 60_000;
const TOKEN_USAGE_ACTIONS = new Set([
  "tools_list",
  "resources_list",
  "prompts_list",
  "tools_call",
  "resources_read",
  "prompts_get",
]);

interface TokenSavingsTrendCardProps {
  className?: string;
}

interface GlobalSavingsStats {
  totalAvailableTokens: number;
  visibleTokens: number;
  savedTokens: number;
  savedPercentage: number;
  totalCalls: number;
  cumulativeSavings: number;
  profileCount: number;
  activeProfileCount: number;
}

interface HistoryPoint {
  time: string;
  beforeFiltering: number;
  afterFiltering: number;
  savedTokens: number;
  totalCalls: number;
}

interface ProfileTokenSnapshot {
  profileId: string;
  totalTokens: number;
  visibleTokens: number;
  savedTokens: number;
}

function parseStoredHistory(raw: string | null): HistoryPoint[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((entry: unknown): entry is HistoryPoint => {
      if (!entry || typeof entry !== "object") return false;
      const candidate = entry as Record<string, unknown>;
      return (
        typeof candidate.time === "string" &&
        typeof candidate.beforeFiltering === "number" &&
        typeof candidate.afterFiltering === "number" &&
        typeof candidate.savedTokens === "number" &&
        typeof candidate.totalCalls === "number"
      );
    });
  } catch {
    return [];
  }
}

function TokenLegend({ payload }: { payload?: Array<{ value?: string; color?: string; dataKey?: string }> }) {
  if (!payload || payload.length === 0) return null;
  return (
    <div className={DASHBOARD_CHART_LEGEND_WRAPPER_CLASS}>
      {payload.map((entry, index) => {
        const key = entry.dataKey ?? entry.value ?? index;
        const color = entry.color ?? "#9ca3af";
        const displayName = entry.value ?? "";
        const isBeforeFiltering = entry.dataKey === "beforeFiltering";
        return (
          <div key={key} className="flex items-center gap-1">
            <svg
              width={20}
              height={8}
              className="shrink-0"
              aria-hidden
            >
              <line
                x1="0"
                y1="4"
                x2="20"
                y2="4"
                stroke={color}
                strokeWidth={2}
                strokeLinecap="round"
                strokeDasharray={isBeforeFiltering ? "5 4" : undefined}
              />
            </svg>
            <span style={{ color }}>{displayName}</span>
          </div>
        );
      })}
    </div>
  );
}

export function TokenSavingsTrendCard({ className }: TokenSavingsTrendCardProps) {
  const { t } = useTranslation("dashboard");

  const [history, setHistory] = useState<HistoryPoint[]>(() => {
    if (typeof window === "undefined") return [];
    return parseStoredHistory(window.localStorage.getItem(HISTORY_KEY));
  });

  const { data: profilesResponse, isLoading: isLoadingProfiles } = useQuery({
    queryKey: ["configSuits", "dashboard"],
    queryFn: configSuitsApi.getAll,
    refetchInterval: DASHBOARD_REFRESH_INTERVAL_MS,
    retry: false,
    refetchOnWindowFocus: false,
  });

  const { data: auditData } = useQuery({
    queryKey: ["audit", "mcp-calls-stats"],
    queryFn: () => auditApi.list({ limit: 1000 }),
    refetchInterval: DASHBOARD_REFRESH_INTERVAL_MS,
    retry: false,
    refetchOnWindowFocus: false,
  });

  const activeProfiles = useMemo(
    () => (profilesResponse?.suits ?? []).filter((profile) => profile.is_active),
    [profilesResponse],
  );

  const profileLedgerQueries = useQueries({
    queries: activeProfiles.map((profile) => ({
      queryKey: ["dashboardCapabilityTokenLedger", profile.id],
      queryFn: () => capabilityTokenLedgerApi.get(profile.id),
      refetchInterval: DASHBOARD_REFRESH_INTERVAL_MS,
      retry: false,
      refetchOnWindowFocus: false,
      staleTime: 0,
    })),
  });

  const profileTokenSnapshots = useMemo<ProfileTokenSnapshot[]>(
    () =>
      activeProfiles.map((profile, index) => {
        const ledgerItems = profileLedgerQueries[index]?.data?.items;
        const { totalTokens, visibleTokens } = computeProfileLedgerTokens(ledgerItems);
        return {
          profileId: profile.id,
          totalTokens,
          visibleTokens,
          savedTokens: Math.max(0, totalTokens - visibleTokens),
        };
      }),
    [activeProfiles, profileLedgerQueries],
  );

  const savingsStats = useMemo<GlobalSavingsStats | null>(() => {
    const profiles = profilesResponse?.suits ?? [];

    if (profiles.length === 0 || activeProfiles.length === 0) {
      return null;
    }

    const totalAvailableTokens = profileTokenSnapshots.reduce(
      (sum, snapshot) => sum + snapshot.totalTokens,
      0,
    );
    const visibleTokens = profileTokenSnapshots.reduce(
      (sum, snapshot) => sum + snapshot.visibleTokens,
      0,
    );
    const savingsByProfileId = new Map(
      profileTokenSnapshots.map((snapshot) => [snapshot.profileId, snapshot.savedTokens]),
    );

    let totalCalls = 0;
    let cumulativeSavings = 0;

    for (const event of auditData?.events ?? []) {
      if (event.status !== "success" || !TOKEN_USAGE_ACTIONS.has(event.action)) {
        continue;
      }
      if (!event.profile_id) {
        continue;
      }
      const profileSavings = savingsByProfileId.get(event.profile_id);
      if (profileSavings === undefined) {
        continue;
      }
      totalCalls += 1;
      cumulativeSavings += profileSavings;
    }

    const savedTokens = Math.max(0, totalAvailableTokens - visibleTokens);
    const savedPercentage =
      totalAvailableTokens > 0 ? Math.round((savedTokens / totalAvailableTokens) * 100) : 0;

    return {
      totalAvailableTokens,
      visibleTokens,
      savedTokens,
      savedPercentage,
      totalCalls,
      cumulativeSavings,
      profileCount: profiles.length,
      activeProfileCount: activeProfiles.length,
    };
  }, [profilesResponse, activeProfiles.length, profileTokenSnapshots, auditData]);

  useEffect(() => {
    if (!savingsStats || typeof window === "undefined") return;

    const now = new Date();
    const timeStr = now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

    const newPoint: HistoryPoint = {
      time: timeStr,
      beforeFiltering: savingsStats.totalAvailableTokens,
      afterFiltering: savingsStats.visibleTokens,
      savedTokens: savingsStats.savedTokens,
      totalCalls: savingsStats.totalCalls,
    };

    setHistory((prev) => {
      if (prev.length > 0 && prev[prev.length - 1].time === timeStr) {
        return prev;
      }
      const next = [...prev, newPoint].slice(-MAX_HISTORY_POINTS);
      try {
        window.localStorage.setItem(HISTORY_KEY, JSON.stringify(next));
      } catch {
        /* noop */
      }
      return next;
    });
  }, [savingsStats]);

  const hasPendingLedgerQuery = profileLedgerQueries.some((query) => query.isPending);
  const isStatsPending = isLoadingProfiles || hasPendingLedgerQuery;
  const hasCachedSeries = history.length > 1;
  const isEmptyAfterLoad = !isStatsPending && savingsStats === null;

  const renderTooltip = ({ active, payload, label }: { active?: boolean; payload?: Array<{ value?: number; name?: string; color?: string; dataKey?: string; payload?: HistoryPoint }>; label?: string }) => {
    if (!active || !payload || payload.length === 0) return null;

    const dataPoint = payload[0]?.payload;

    return (
      <div className="rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-xs text-slate-100 shadow-lg">
        {label && <div className="mb-1.5 text-[11px] text-slate-400 font-medium">{label}</div>}
        <div className="space-y-1">
          {payload.map((entry, index) => {
            if (typeof entry.value !== "number") return null;
            const color = entry.color ?? "#9ca3af";
            const displayName = entry.name ?? "";
            return (
              <div key={`${entry.dataKey}-${index}`} className="flex items-center justify-between gap-4">
                <div className="flex items-center gap-2 text-[11px] text-slate-300">
                  <span className="inline-block h-2 w-2 rounded-full" style={{ backgroundColor: color }} />
                  <span>{displayName}</span>
                </div>
                <span className="min-w-[48px] text-right text-[11px] font-semibold text-slate-50">
                  {formatTokenCount(entry.value)}
                </span>
              </div>
            );
          })}
          {dataPoint && (
            <>
              <div className="border-t border-slate-700 my-1.5" />
              <div className="flex items-center justify-between gap-4 text-[11px]">
                <span className="text-slate-400">{t("tokenSavings.savedPerCall", { defaultValue: "Saved per call" })}</span>
                <span className="text-emerald-400 font-medium">{formatTokenCount(dataPoint.savedTokens)}</span>
              </div>
              <div className="flex items-center justify-between gap-4 text-[11px]">
                <span className="text-slate-400">{t("tokenSavings.calls", { defaultValue: "Total calls" })}</span>
                <span className="text-slate-200 font-medium">{dataPoint.totalCalls.toLocaleString()}</span>
              </div>
            </>
          )}
        </div>
      </div>
    );
  };

  const totalSavedDisplay =
    savingsStats &&
    (savingsStats.cumulativeSavings > 0
      ? formatTokenCount(savingsStats.cumulativeSavings)
      : formatTokenCount(savingsStats.savedTokens));

  const infoLines = [
    t("tokenSavings.infoLine1", {
      defaultValue: "Current values are recalculated from active profiles using tokenizer-based capability payloads.",
    }),
    t("tokenSavings.infoLine2", {
      defaultValue: "Each successful MCP list or call event in audit logs is matched to its profile and contributes that profile's current savings.",
    }),
    t("tokenSavings.infoLine3", {
      defaultValue: "This is not a frozen historical ledger yet: when profile configuration changes, earlier totals can be recomputed.",
    }),
    t("tokenSavings.infoLine4", {
      defaultValue:
        "That keeps the logic dynamic and closer to real usage, while finer time-slice reconstruction is still being improved.",
    }),
  ];

  const chartBody = (() => {
    if (hasCachedSeries) {
      return (
        <div className={DASHBOARD_CHART_VIEWPORT_CLASS}>
          <ResponsiveContainer width="100%" height="100%">
            <LineChart
              data={history}
              margin={DASHBOARD_LINE_CHART_MARGIN}
            >
              <CartesianGrid strokeDasharray="3 3" stroke="rgba(148, 163, 184, 0.25)" />
              <XAxis
                dataKey="time"
                stroke="#9ca3af"
                fontSize={11}
                height={26}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                stroke="#9ca3af"
                fontSize={11}
                tickLine={false}
                axisLine={false}
                tickFormatter={formatTokenCount}
                width={52}
              />
              <Tooltip content={renderTooltip} />
              <Legend content={(props) => <TokenLegend payload={props.payload} />} />
              <Line
                type="monotone"
                dataKey="beforeFiltering"
                name={t("tokenSavings.beforeFiltering", { defaultValue: "Before Filtering" })}
                stroke={LINE_COLORS.before}
                strokeWidth={2}
                strokeDasharray="6 4"
                dot={false}
                activeDot={{ r: 5, strokeWidth: 0 }}
                isAnimationActive={false}
              />
              <Line
                type="monotone"
                dataKey="afterFiltering"
                name={t("tokenSavings.afterFiltering", { defaultValue: "After Filtering" })}
                stroke={LINE_COLORS.after}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 5, strokeWidth: 0 }}
                isAnimationActive={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      );
    }
    if (isStatsPending) {
      return <DashboardChartSkeleton />;
    }
    if (isEmptyAfterLoad) {
      return (
        <DashboardChartPlaceholder>
          <Sparkles className="h-8 w-8 text-amber-500/80" aria-hidden />
          <p className="text-sm font-medium text-slate-700 dark:text-slate-200">
            {t("tokenSavings.emptyOrg", {
              defaultValue:
                "Add a server or profile to estimate token savings from capability filtering.",
            })}
          </p>
        </DashboardChartPlaceholder>
      );
    }
    return (
      <DashboardChartPlaceholder>
        <Loader2
          className="h-7 w-7 animate-spin text-amber-500/90"
          aria-hidden
        />
        <p className="text-sm font-medium text-slate-700 dark:text-slate-200">
          {t("tokenSavings.collectingData", { defaultValue: "Collecting data..." })}
        </p>
        <p className="max-w-sm text-xs text-slate-500 dark:text-slate-400">
          {t("tokenSavings.collectingDataHint", {
            defaultValue:
              "Estimates appear once servers and profiles finish loading.",
          })}
        </p>
      </DashboardChartPlaceholder>
    );
  })();

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-amber-500" />
            <div className="flex items-center gap-1.5">
              <CardTitle className="text-base">
                {t("tokenSavings.title", { defaultValue: "Token Savings" })}
              </CardTitle>
              <TooltipProvider delayDuration={150}>
                <UiTooltip>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      aria-label={t("tokenSavings.infoLabel", {
                        defaultValue: "How token savings are estimated",
                      })}
                      className="inline-flex h-5 w-5 items-center justify-center rounded-full border border-slate-200/80 text-slate-400 transition-colors hover:border-amber-300 hover:bg-amber-50 hover:text-amber-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber-400 dark:border-slate-700 dark:text-slate-500 dark:hover:border-amber-500/50 dark:hover:bg-amber-500/10 dark:hover:text-amber-300"
                    >
                      <Info className="h-3.5 w-3.5" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent
                    side="bottom"
                    align="start"
                    className="max-w-[23rem] border-slate-700 bg-slate-950/95 px-3 py-2 text-slate-100 shadow-xl backdrop-blur"
                  >
                    <div className="space-y-1 text-[11px] leading-relaxed">
                      {infoLines.map((line) => (
                        <p key={line}>{line}</p>
                      ))}
                    </div>
                    <TooltipArrow className="fill-slate-950/95" />
                  </TooltipContent>
                </UiTooltip>
              </TooltipProvider>
            </div>
          </div>
          {savingsStats ? (
            <div className="flex shrink-0 items-center gap-1.5 text-xs">
              <TrendingUp className="h-3.5 w-3.5 text-emerald-500" />
              <span className="font-medium text-emerald-600 dark:text-emerald-400">
                {totalSavedDisplay}
              </span>
              <span className="text-muted-foreground">
                {t("tokenSavings.saved", { defaultValue: "saved" })}
              </span>
            </div>
          ) : isStatsPending ? (
            <div
              className="h-4 w-28 shrink-0 animate-pulse rounded bg-slate-200 dark:bg-slate-700"
              aria-hidden
            />
          ) : null}
        </div>
        <CardDescription className="text-xs">
          {t("tokenSavings.description", {
            defaultValue: "Estimated context savings from profile filtering",
          })}
        </CardDescription>
      </CardHeader>
      <CardContent>{chartBody}</CardContent>
    </Card>
  );
}
