import { useQuery } from "@tanstack/react-query";
import {
	Cpu,
	Database,
	Gauge,
	MemoryStick as Memory,
	Network,
	RefreshCw,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
	Area,
	AreaChart,
	CartesianGrid,
	Line,
	LineChart,
	ResponsiveContainer,
	Tooltip,
	XAxis,
	YAxis,
} from "recharts";
import { StatusBadge } from "../../components/status-badge";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { systemApi } from "../../lib/api";
import { APP_VERSION_LABEL } from "../../lib/app-version";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { formatBytes, formatUptime } from "../../lib/utils";

const mockCpuMemoryData = Array.from({ length: 50 }, (_, i) => ({
	time: i,
	cpu: Math.min(
		100,
		Math.max(0, 20 + Math.sin(i / 5) * 15 + Math.random() * 10),
	),
	memory: Math.min(
		100,
		Math.max(0, 40 + Math.cos(i / 10) * 20 + Math.random() * 5),
	),
}));

const mockNetworkData = Array.from({ length: 50 }, (_, i) => ({
	time: i,
	in: Math.max(0, 50 + Math.sin(i / 3) * 30 + Math.random() * 20),
	out: Math.max(0, 30 + Math.cos(i / 4) * 20 + Math.random() * 15),
}));

const mockRequestsData = Array.from({ length: 24 }, (_, i) => ({
	hour: i,
	requests: Math.floor(
		Math.max(0, 100 + Math.sin(i / 3) * 80 + Math.random() * 50),
	),
	errors: Math.floor(Math.max(0, Math.random() * 5 + (i === 12 ? 15 : 0))),
}));

export function SystemPage() {
	usePageTranslations("system");
	const { t } = useTranslation();
	const versionLabel = APP_VERSION_LABEL || "—";
	const {
		data: systemStatus,
		isLoading: isLoadingStatus,
		refetch,
		isRefetching,
	} = useQuery({
		queryKey: ["systemStatus"],
		queryFn: systemApi.getStatus,
		refetchInterval: 30000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const { data: metrics, isLoading: isLoadingMetrics } = useQuery({
		queryKey: ["systemMetrics"],
		queryFn: systemApi.getMetrics,
		refetchInterval: 10000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<h2 className="text-3xl font-bold tracking-tight">
					{t("system:title", { defaultValue: "System" })}
				</h2>
				<Button
					onClick={() => refetch()}
					disabled={isRefetching}
					variant="outline"
					size="sm"
				>
					<RefreshCw
						className={`mr-2 h-4 w-4 ${isRefetching ? "animate-spin" : ""}`}
					/>
					{t("system:actions.refresh", { defaultValue: "Refresh" })}
				</Button>
			</div>

			<div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
				<Card>
					<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
						<CardTitle className="text-sm font-medium">
							{t("system:status.title", { defaultValue: "Status" })}
						</CardTitle>
						<Gauge className="h-4 w-4 text-slate-500" />
					</CardHeader>
					<CardContent>
						{isLoadingStatus ? (
							<div className="h-6 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						) : (
							<div className="flex flex-col gap-1">
								<StatusBadge status={systemStatus?.status || "unknown"} />
								<p className="text-xs text-slate-500">{versionLabel}</p>
							</div>
						)}
					</CardContent>
				</Card>

				<Card>
					<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
						<CardTitle className="text-sm font-medium">
							{t("system:cpu.title", { defaultValue: "CPU" })}
						</CardTitle>
						<Cpu className="h-4 w-4 text-slate-500" />
					</CardHeader>
					<CardContent>
						{isLoadingMetrics ? (
							<div className="h-6 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						) : (
							<div className="flex flex-col gap-1">
								<span className="text-2xl font-bold">
									{metrics?.cpu_usage_percent !== undefined
										? metrics.cpu_usage_percent.toFixed(1)
										: "0.0"}
									%
								</span>
								<p className="text-xs text-slate-500">
									{t("system:cpu.description", {
										defaultValue: "Current CPU Usage",
									})}
								</p>
							</div>
						)}
					</CardContent>
				</Card>

				<Card>
					<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
						<CardTitle className="text-sm font-medium">
							{t("system:memory.title", { defaultValue: "Memory" })}
						</CardTitle>
						<Memory className="h-4 w-4 text-slate-500" />
					</CardHeader>
					<CardContent>
						{isLoadingMetrics ? (
							<div className="h-6 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						) : (
							<div className="flex flex-col gap-1">
								<span className="text-2xl font-bold">
									{formatBytes(metrics?.memory_usage_bytes || 0)}
								</span>
								<p className="text-xs text-slate-500">
									{t("system:memory.description", {
										defaultValue: "Current Memory Usage",
									})}
								</p>
							</div>
						)}
					</CardContent>
				</Card>

				<Card>
					<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
						<CardTitle className="text-sm font-medium">
							{t("system:uptime.title", { defaultValue: "Uptime" })}
						</CardTitle>
						<Network className="h-4 w-4 text-slate-500" />
					</CardHeader>
					<CardContent>
						{isLoadingStatus ? (
							<div className="h-6 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						) : (
							<div className="flex flex-col gap-1">
								<span className="text-2xl font-bold">
									{formatUptime(systemStatus?.uptime || 0)}
								</span>
								<p className="text-xs text-slate-500">
									{t("system:uptime.description", {
										defaultValue: "System Running Time",
									})}
								</p>
							</div>
						)}
					</CardContent>
				</Card>
			</div>

			<div className="grid gap-4 md:grid-cols-2">
				<Card className="col-span-1 md:col-span-2">
					<CardHeader>
						<CardTitle>
							{t("system:resourceChart.title", {
								defaultValue: "CPU & Memory Usage",
							})}
						</CardTitle>
						<CardDescription>
							{t("system:resourceChart.description", {
								defaultValue: "System resource utilization over time",
							})}
						</CardDescription>
					</CardHeader>
					<CardContent>
						<div className="h-[300px]">
							<ResponsiveContainer width="100%" height="100%">
								<LineChart
									data={mockCpuMemoryData}
									margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
								>
									<CartesianGrid strokeDasharray="3 3" stroke="#374151" />
									<XAxis
										dataKey="time"
										stroke="#9ca3af"
										fontSize={12}
										label={{
											value: t("system:resourceChart.timeAxis", {
												defaultValue: "Time (minutes)",
											}),
											position: "insideBottomRight",
											offset: -10,
										}}
									/>
									<YAxis
										stroke="#9ca3af"
										fontSize={12}
										domain={[0, 100]}
										label={{
											value: t("system:resourceChart.usageAxis", {
												defaultValue: "Usage %",
											}),
											angle: -90,
											position: "insideLeft",
										}}
									/>
									<Tooltip
										contentStyle={{
											backgroundColor: "#1f2937",
											border: "1px solid #4b5563",
											borderRadius: "6px",
											color: "#e5e7eb",
										}}
										formatter={(value) =>
											typeof value === "number"
												? `${value.toFixed(1)}%`
												: String(value)
										}
									/>
									<Line
										type="monotone"
										dataKey="cpu"
										stroke="#3b82f6"
										name={t("system:resourceChart.cpuSeries", {
											defaultValue: "CPU",
										})}
										strokeWidth={2}
										dot={false}
										activeDot={{ r: 6 }}
									/>
									<Line
										type="monotone"
										dataKey="memory"
										stroke="#10b981"
										name={t("system:resourceChart.memorySeries", {
											defaultValue: "Memory",
										})}
										strokeWidth={2}
										dot={false}
										activeDot={{ r: 6 }}
									/>
								</LineChart>
							</ResponsiveContainer>
						</div>
					</CardContent>
				</Card>

				<Card>
					<CardHeader>
						<CardTitle>
							{t("system:networkChart.title", {
								defaultValue: "Network Traffic",
							})}
						</CardTitle>
						<CardDescription>
							{t("system:networkChart.description", {
								defaultValue: "Inbound and outbound network traffic",
							})}
						</CardDescription>
					</CardHeader>
					<CardContent>
						<div className="h-[250px]">
							<ResponsiveContainer width="100%" height="100%">
								<AreaChart
									data={mockNetworkData}
									margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
								>
									<CartesianGrid strokeDasharray="3 3" stroke="#374151" />
									<XAxis
										dataKey="time"
										stroke="#9ca3af"
										fontSize={12}
										label={{
											value: t("system:networkChart.timeAxis", {
												defaultValue: "Time (minutes)",
											}),
											position: "insideBottomRight",
											offset: -10,
										}}
									/>
									<YAxis
										stroke="#9ca3af"
										fontSize={12}
										label={{
											value: t("system:networkChart.throughputAxis", {
												defaultValue: "KB/s",
											}),
											angle: -90,
											position: "insideLeft",
										}}
									/>
									<Tooltip
										contentStyle={{
											backgroundColor: "#1f2937",
											border: "1px solid #4b5563",
											borderRadius: "6px",
											color: "#e5e7eb",
										}}
										formatter={(value) =>
											typeof value === "number"
												? `${value.toFixed(1)} KB/s`
												: String(value)
										}
									/>
									<Area
										type="monotone"
										dataKey="in"
										stackId="1"
										stroke="#3b82f6"
										fill="#3b82f6"
										name={t("system:networkChart.inboundSeries", {
											defaultValue: "Inbound",
										})}
										fillOpacity={0.3}
									/>
									<Area
										type="monotone"
										dataKey="out"
										stackId="1"
										stroke="#f59e0b"
										fill="#f59e0b"
										name={t("system:networkChart.outboundSeries", {
											defaultValue: "Outbound",
										})}
										fillOpacity={0.3}
									/>
								</AreaChart>
							</ResponsiveContainer>
						</div>
					</CardContent>
				</Card>

				<Card>
					<CardHeader>
						<CardTitle>
							{t("system:apiChart.title", {
								defaultValue: "API Requests",
							})}
						</CardTitle>
						<CardDescription>
							{t("system:apiChart.description", {
								defaultValue: "Request volume and error rate",
							})}
						</CardDescription>
					</CardHeader>
					<CardContent>
						<div className="h-[250px]">
							<ResponsiveContainer width="100%" height="100%">
								<AreaChart
									data={mockRequestsData}
									margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
								>
									<CartesianGrid strokeDasharray="3 3" stroke="#374151" />
									<XAxis
										dataKey="hour"
										stroke="#9ca3af"
										fontSize={12}
										label={{
											value: t("system:apiChart.hourAxis", {
												defaultValue: "Hour",
											}),
											position: "insideBottomRight",
											offset: -10,
										}}
									/>
									<YAxis
										stroke="#9ca3af"
										fontSize={12}
										label={{
											value: t("system:apiChart.countAxis", {
												defaultValue: "Count",
											}),
											angle: -90,
											position: "insideLeft",
										}}
									/>
									<Tooltip
										contentStyle={{
											backgroundColor: "#1f2937",
											border: "1px solid #4b5563",
											borderRadius: "6px",
											color: "#e5e7eb",
										}}
									/>
									<Area
										type="monotone"
										dataKey="requests"
										stroke="#10b981"
										fill="#10b981"
										name={t("system:apiChart.requestsSeries", {
											defaultValue: "Requests",
										})}
										fillOpacity={0.3}
									/>
									<Area
										type="monotone"
										dataKey="errors"
										stroke="#ef4444"
										fill="#ef4444"
										name={t("system:apiChart.errorsSeries", {
											defaultValue: "Errors",
										})}
										fillOpacity={0.3}
									/>
								</AreaChart>
							</ResponsiveContainer>
						</div>
					</CardContent>
				</Card>
			</div>

			<Card>
				<CardHeader>
					<CardTitle>
						{t("system:connections.title", {
							defaultValue: "Active Connections",
						})}
					</CardTitle>
					<CardDescription>
						{t("system:connections.description", {
							defaultValue: "Current active connections to the MCPMate Proxy",
						})}
					</CardDescription>
				</CardHeader>
				<CardContent>
					<div className="flex items-center justify-center py-6">
						<div className="text-center">
							<div className="flex items-center justify-center">
								<Database className="mr-2 h-6 w-6 text-slate-600" />
								<span className="text-3xl font-bold">
									{isLoadingMetrics ? (
										<div className="h-10 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									) : (
										metrics?.active_connections || 0
									)}
								</span>
							</div>
							<p className="mt-2 text-sm text-slate-500">
								{t("system:connections.valueLabel", {
									defaultValue: "Active connections",
								})}
							</p>
						</div>
					</div>
				</CardContent>
			</Card>
		</div>
	);
}
