import { useQuery } from "@tanstack/react-query";
import { Activity, Server, Sliders, Users } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
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
import { StatusBadge } from "../../components/status-badge";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	clientsApi,
	configSuitsApi,
	serversApi,
	systemApi,
} from "../../lib/api";
import type { ClientCheckData } from "../../lib/types";
import { formatUptime } from "../../lib/utils";

const METRICS_HISTORY_STORAGE_KEY = "mcp_metrics_history_v3";

type MetricsHistoryPoint = {
	time: string;
	mcpmateCpuPercent: number;
	mcpmateMemoryPercent: number;
	systemCpuPercent: number;
	systemMemoryPercent: number;
	mcpmateMemoryMb: number;
	systemMemoryMb: number;
};

type MetricsHistory = MetricsHistoryPoint[];

const parseStoredHistory = (raw: string | null): MetricsHistory => {
	if (!raw) {
		return [];
	}
	try {
		const parsed = JSON.parse(raw);
		if (!Array.isArray(parsed)) {
			return [];
		}
		return parsed.filter((entry: unknown): entry is MetricsHistoryPoint => {
			if (!entry || typeof entry !== "object") {
				return false;
			}
			const candidate = entry as Record<string, unknown>;
			return (
				typeof candidate.time === "string" &&
				typeof candidate.mcpmateCpuPercent === "number" &&
				typeof candidate.mcpmateMemoryPercent === "number" &&
				typeof candidate.systemCpuPercent === "number" &&
				typeof candidate.systemMemoryPercent === "number" &&
				typeof candidate.mcpmateMemoryMb === "number" &&
				typeof candidate.systemMemoryMb === "number"
			);
		});
	} catch {
		return [];
	}
};

// Maintain a lightweight metrics history in component state
function useMetricsHistory() {
	const [history, setHistory] = React.useState<MetricsHistory>(() => {
		if (typeof window === "undefined") {
			return [];
		}
		return parseStoredHistory(
			window.localStorage.getItem(METRICS_HISTORY_STORAGE_KEY),
		);
	});

	const metricsQuery = useQuery({
		queryKey: ["systemMetrics"],
		queryFn: systemApi.getMetrics,
		refetchInterval: 10000,
	});

	React.useEffect(() => {
		const metrics = metricsQuery.data;
		if (!metrics || typeof window === "undefined") {
			return;
		}
		const ensureNumber = (value: unknown): number | null =>
			typeof value === "number" && Number.isFinite(value) ? value : null;

		const clampPercent = (value: number): number =>
			Math.min(100, Math.max(0, value));

		const percentOf = (value: number | null, total: number | null): number => {
			if (value === null || total === null || total <= 0) {
				return 0;
			}
			return clampPercent((value / total) * 100);
		};

		const timestamp = metrics.timestamp
			? new Date(metrics.timestamp)
			: new Date();

		const mcpmateCpuPercent = (() => {
			const candidates = [
				ensureNumber(metrics.cpu_usage_percent),
				ensureNumber(metrics.cpu_usage),
				ensureNumber(metrics.system_cpu_usage),
			];
			for (const candidate of candidates) {
				if (candidate !== null) {
					return clampPercent(candidate);
				}
			}
			return 0;
		})();

		const systemCpuPercent = (() => {
			const value = ensureNumber(metrics.system_cpu_usage);
			return value !== null ? clampPercent(value) : mcpmateCpuPercent;
		})();

		const systemMemoryTotalBytes = ensureNumber(metrics.system_memory_total);
		const mcpmateMemoryBytes = (() => {
			const candidates = [
				ensureNumber(metrics.memory_usage),
				ensureNumber(metrics.memory_usage_bytes),
				ensureNumber(metrics.system_memory_usage),
			];
			for (const candidate of candidates) {
				if (candidate !== null) {
					return candidate;
				}
			}
			return null;
		})();

		const systemMemoryUsageBytes =
			ensureNumber(metrics.system_memory_usage) ?? mcpmateMemoryBytes;

		const mcpmateMemoryPercent = percentOf(
			mcpmateMemoryBytes,
			systemMemoryTotalBytes,
		);
		const systemMemoryPercent = percentOf(
			systemMemoryUsageBytes,
			systemMemoryTotalBytes,
		);

		const mcpmateMemoryMb =
			mcpmateMemoryBytes !== null ? mcpmateMemoryBytes / (1024 * 1024) : 0;
		const systemMemoryMb =
			systemMemoryUsageBytes !== null
				? systemMemoryUsageBytes / (1024 * 1024)
				: mcpmateMemoryMb;

		const point: MetricsHistoryPoint = {
			time: timestamp.toLocaleTimeString([], {
				hour: "2-digit",
				minute: "2-digit",
			}),
			mcpmateCpuPercent,
			mcpmateMemoryPercent,
			systemCpuPercent,
			systemMemoryPercent,
			mcpmateMemoryMb,
			systemMemoryMb,
		};
		setHistory((prev) => {
			const next = [...prev, point];
			const trimmed = next.slice(-60);
			try {
				window.localStorage.setItem(
					METRICS_HISTORY_STORAGE_KEY,
					JSON.stringify(trimmed),
				);
			} catch {
				/* noop */
			}
			return trimmed;
		});
		try {
			window.localStorage.removeItem("mcp_metrics_history");
			window.localStorage.removeItem("mcp_metrics_history_v2");
		} catch {
			/* noop */
		}
	}, [metricsQuery.data]);

	const latestPoint = history.length > 0 ? history[history.length - 1] : null;
	const isLoading = metricsQuery.isLoading && history.length === 0;

	return { history, latestPoint, isLoading };
}

function useIsDarkMode() {
	const [isDarkMode, setIsDarkMode] = React.useState(() => {
		if (typeof document === "undefined") {
			return false;
		}
		return document.documentElement.classList.contains("dark");
	});

	React.useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}
		const media = window.matchMedia("(prefers-color-scheme: dark)");
		const update = () => {
			setIsDarkMode(
				document.documentElement.classList.contains("dark") || media.matches,
			);
		};
		update();
		const listener = (event: MediaQueryListEvent) => {
			setIsDarkMode(event.matches);
		};
		media.addEventListener("change", listener);
		return () => media.removeEventListener("change", listener);
	}, []);

	return isDarkMode;
}

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
			{label ? (
				<div className="mb-1 text-[11px] text-slate-400">{label}</div>
			) : null}
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
						typeof valueLabel === "number"
							? valueLabel.toString()
							: String(valueLabel);
					return (
						<div key={key} className="flex items-center justify-between gap-4">
							<div className="flex items-center gap-2 text-[11px] text-slate-300">
								<span
									className="inline-block h-2 w-2 rounded-full"
									style={{ backgroundColor: color }}
								></span>
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
		<div className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 px-2 pb-1 text-xs">
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
						></span>
						<span style={{ color: swatchColor }}>{entry.value}</span>
					</div>
				);
			})}
		</div>
	);
}

export function DashboardPage() {
	usePageTranslations("dashboard");
	const { t } = useTranslation();
	const { data: systemStatus, isLoading: isLoadingSystem } = useQuery({
		queryKey: ["systemStatus"],
		queryFn: systemApi.getStatus,
		refetchInterval: 30000,
	});

	const { data: servers, isLoading: isLoadingServers } = useQuery({
		queryKey: ["servers"],
		queryFn: serversApi.getAll,
		refetchInterval: 30000,
	});

	const { data: clientsData, isLoading: isLoadingClients } =
		useQuery<ClientCheckData | null>({
			queryKey: ["clients", "dashboard"],
			queryFn: () => clientsApi.list(false),
			refetchInterval: 30000,
		});

	// Runtime card removed on Dashboard

	// Metrics history for chart
	const { history: metricsHistory, isLoading: isLoadingMetrics } =
		useMetricsHistory();
	const isDarkMode = useIsDarkMode();

	const gridStroke = isDarkMode
		? "rgba(55, 65, 81, 0.6)"
		: "rgba(148, 163, 184, 0.25)";

	const createDot = React.useCallback(
		(color: string) => ({ r: 2.5, fill: color, strokeWidth: 0 }),
		[],
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
			if (typeof value !== "number") {
				return [value, name];
			}
			const percentLabel = `${value.toFixed(1)}%`;
			const dataKey =
				typeof item.dataKey === "string" ? item.dataKey : undefined;
			const payload = item.payload as Partial<MetricsHistoryPoint>;
			if (dataKey === "mcpmateMemoryPercent") {
				return [
					`${percentLabel} • ${formatMegabytes(payload.mcpmateMemoryMb)}`,
					name,
				];
			}
			if (dataKey === "systemMemoryPercent") {
				return [
					`${percentLabel} • ${formatMegabytes(payload.systemMemoryMb)}`,
					name,
				];
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

	const { data: suitsResponse, isLoading: isLoadingProfiles } = useQuery({
		queryKey: ["configSuits", "dashboard"],
		queryFn: configSuitsApi.getAll,
		refetchInterval: 30000,
		retry: 1,
	});

	const suitsList = suitsResponse?.suits ?? [];
	const totalProfiles = suitsList.length;
	const activeProfiles = suitsList.filter((suit) => suit.is_active).length;

	const connectedServers =
		servers?.servers?.filter((server) => server.status === "connected")
			.length || 0;
	const totalClients = clientsData?.total ?? clientsData?.client?.length ?? 0;
	const managedClients =
		clientsData?.client?.filter((client) => client.managed).length ?? 0;

	return (
		<div className="space-y-4">
			<div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
				<Link to="/runtime" className="block h-full">
					<Card className="h-full min-h-[160px] hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 transition-all duration-200 cursor-pointer">
						<CardHeader className="flex flex-row items-center justify-between space-y-0">
							<CardTitle className="text-sm font-medium">
								{t("dashboard:cards.systemStatus", {
									defaultValue: "System Status",
								})}
							</CardTitle>
							<Activity className="h-4 w-4 text-slate-500" />
						</CardHeader>
						<CardContent>
							<div className="space-y-1.5">
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.status", { defaultValue: "Status" })}
									</CardDescription>
									{isLoadingSystem ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<StatusBadge status={systemStatus?.status || "unknown"} />
									)}
								</div>
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.uptime", { defaultValue: "Uptime" })}
									</CardDescription>
									{isLoadingSystem ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">
											{formatUptime(systemStatus?.uptime || 0)}
										</span>
									)}
								</div>
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.version", { defaultValue: "Version" })}
									</CardDescription>
									{isLoadingSystem ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">
											{systemStatus?.version || "alpha"}
										</span>
									)}
								</div>
							</div>
						</CardContent>
					</Card>
				</Link>

				<Link to="/profiles" className="block h-full">
					<Card className="h-full min-h-[160px] hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 transition-all duration-200 cursor-pointer">
						<CardHeader className="flex flex-row items-center justify-between space-y-0">
							<div>
								<CardTitle className="text-sm font-medium">
									{t("dashboard:cards.profiles", { defaultValue: "Profiles" })}
								</CardTitle>
							</div>
							<Sliders className="h-4 w-4 text-slate-500" />
						</CardHeader>
						<CardContent>
							<div className="space-y-1.5">
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.totalProfiles", {
											defaultValue: "Total Profiles",
										})}
									</CardDescription>
									{isLoadingProfiles ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">{totalProfiles}</span>
									)}
								</div>
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.activeProfiles", {
											defaultValue: "Active Profiles",
										})}
									</CardDescription>
									{isLoadingProfiles ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">
											{activeProfiles}
										</span>
									)}
								</div>
							</div>
						</CardContent>
					</Card>
				</Link>

				<Link to="/servers" className="block h-full">
					<Card className="h-full min-h-[160px] hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 transition-all duration-200 cursor-pointer">
						<CardHeader className="flex flex-row items-center justify-between space-y-0">
							<CardTitle className="text-sm font-medium">
								{t("dashboard:cards.servers", { defaultValue: "Servers" })}
							</CardTitle>
							<Server className="h-4 w-4 text-slate-500" />
						</CardHeader>
						<CardContent>
							<div className="space-y-1.5">
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.totalServers", {
											defaultValue: "Total Servers",
										})}
									</CardDescription>
									{isLoadingServers ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">
											{servers?.servers?.length || 0}
										</span>
									)}
								</div>
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.connected", {
											defaultValue: "Connected",
										})}
									</CardDescription>
									{isLoadingServers ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">
											{connectedServers}
										</span>
									)}
								</div>
							</div>
						</CardContent>
					</Card>
				</Link>

				<Link to="/clients" className="block h-full">
					<Card className="h-full min-h-[160px] hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 transition-all duration-200 cursor-pointer">
						<CardHeader className="flex flex-row items-center justify-between space-y-0">
							<CardTitle className="text-sm font-medium">
								{t("dashboard:cards.clients", { defaultValue: "Clients" })}
							</CardTitle>
							<Users className="h-4 w-4 text-slate-500" />
						</CardHeader>
						<CardContent>
							<div className="space-y-1.5">
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.totalClients", {
											defaultValue: "Total Clients",
										})}
									</CardDescription>
									{isLoadingClients ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">{totalClients}</span>
									)}
								</div>
								<div className="flex items-center justify-between">
									<CardDescription>
										{t("dashboard:labels.managed", { defaultValue: "Managed" })}
									</CardDescription>
									{isLoadingClients ? (
										<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										<span className="text-sm font-medium">
											{managedClients}
										</span>
									)}
								</div>
							</div>
						</CardContent>
					</Card>
				</Link>
			</div>

			<div className="grid gap-4 md:grid-cols-2">
				<div className="col-span-1 md:col-span-2">
					<Card className="h-full">
						<CardHeader>
							<CardTitle>
								{t("dashboard:metrics.title", { defaultValue: "Metrics" })}
							</CardTitle>
							<CardDescription>
								{t("dashboard:metrics.description", {
									defaultValue:
										"MCPMate Process vs host utilization sampled every 10 seconds",
								})}
							</CardDescription>
						</CardHeader>
						<CardContent>
							{isLoadingMetrics ? (
								<div className="space-y-4">
									<div className="h-4 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									<div className="h-[260px] animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
								</div>
							) : metricsHistory.length === 0 ? (
								<p className="text-sm text-slate-500">
									{t("dashboard:metrics.noData", {
										defaultValue: "No metrics have been reported yet.",
									})}
								</p>
							) : (
								<div className="h-[280px]">
									<ResponsiveContainer width="100%" height="100%">
										<LineChart
											data={metricsHistory}
											margin={{ top: 10, right: 24, left: 10, bottom: 8 }}
										>
											<CartesianGrid
												strokeDasharray="3 3"
												stroke={gridStroke}
											/>
											<XAxis
												dataKey="time"
												stroke="#9ca3af"
												fontSize={11}
												height={26}
												axisLine={false}
												tickLine={false}
											/>
											<YAxis
												domain={[0, 100]}
												stroke="#9ca3af"
												fontSize={11}
												tickFormatter={(value) =>
													Number.isFinite(value)
														? `${Math.round(Number(value))}%`
														: ""
												}
												width={52}
											/>
											<Tooltip content={renderTooltip} />
											<Legend
												content={(legendProps) => (
													<MetricsLegend {...legendProps} />
												)}
											/>
											<Line
												type="monotone"
												dataKey="mcpmateCpuPercent"
												name={t("dashboard:metrics.mcpmateCpu", {
													defaultValue: "MCPMate CPU (%)",
												})}
												stroke="#3b82f6"
												strokeWidth={2}
												dot={createDot("#3b82f6")}
												activeDot={{ r: 5, strokeWidth: 0 }}
												isAnimationActive={false}
											/>
											<Line
												type="monotone"
												dataKey="mcpmateMemoryPercent"
												name={t("dashboard:metrics.mcpmateMemory", {
													defaultValue: "MCPMate Memory (%)",
												})}
												stroke="#10b981"
												strokeWidth={2}
												strokeDasharray="6 4"
												dot={createDot("#10b981")}
												activeDot={{ r: 5, strokeWidth: 0 }}
												isAnimationActive={false}
											/>
											<Line
												type="monotone"
												dataKey="systemCpuPercent"
												name={t("dashboard:metrics.hostCpu", {
													defaultValue: "Host CPU (%)",
												})}
												stroke="#6366f1"
												strokeWidth={2}
												dot={createDot("#6366f1")}
												activeDot={{ r: 5, strokeWidth: 0 }}
												isAnimationActive={false}
											/>
											<Line
												type="monotone"
												dataKey="systemMemoryPercent"
												name={t("dashboard:metrics.hostMemory", {
													defaultValue: "Host Memory (%)",
												})}
												stroke="#f97316"
												strokeWidth={2}
												dot={createDot("#f97316")}
												activeDot={{ r: 5, strokeWidth: 0 }}
												isAnimationActive={false}
											/>
										</LineChart>
									</ResponsiveContainer>
								</div>
							)}
						</CardContent>
					</Card>
				</div>
			</div>
		</div>
	);
}
