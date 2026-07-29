import { useQuery } from "@tanstack/react-query";
import {
	Activity,
	AlertTriangle,
	AppWindow,
	ClipboardCheck,
	ChevronRight,
	Play,
	RefreshCw,
	Server,
	Sliders,
	Square,
} from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Button } from "../../components/ui/button";
import { useDesktopCoreState } from "../../lib/desktop-core-state";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { MetricsTrendChart } from "../../components/metrics-trend-chart";
import { TokenSavingsTrendCard } from "../../components/token-savings-trend-card";
import { APP_VERSION_LABEL } from "../../lib/app-version";
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
	surfaceReviewsApi,
	systemApi,
} from "../../lib/api";
import type { ClientCheckData } from "../../lib/types";
import { getSurfaceReviewDestination } from "../../lib/surface-reviews";
import { cn, formatUptime } from "../../lib/utils";

export function DashboardPage() {
	usePageTranslations("dashboard");
	const { t } = useTranslation();
	const {
		isTauriShell,
		coreView,
		busyAction,
		refreshCoreView,
		manageLocalCore,
	} = useDesktopCoreState();
	const { data: systemStatus, isLoading: isLoadingSystem } = useQuery({
		queryKey: ["systemStatus"],
		queryFn: systemApi.getStatus,
		refetchInterval: 30000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const { data: servers, isLoading: isLoadingServers } = useQuery({
		queryKey: ["servers"],
		queryFn: serversApi.getAll,
		refetchInterval: 30000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const { data: clientsData, isLoading: isLoadingClients } =
		useQuery<ClientCheckData | null>({
			queryKey: ["clients", "dashboard"],
			queryFn: () => clientsApi.list(false),
			refetchInterval: 30000,
			retry: false,
			refetchOnWindowFocus: false,
		});

	// Runtime card removed on Dashboard

	const { data: suitsResponse, isLoading: isLoadingProfiles } = useQuery({
		queryKey: ["configSuits", "dashboard"],
		queryFn: configSuitsApi.getAll,
		refetchInterval: 30000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const {
		data: pendingReviewItems = [],
		isLoading: isLoadingReviews,
		error: reviewItemsError,
	} = useQuery({
		queryKey: ["surfaceReviews", "pending"],
		queryFn: () => surfaceReviewsApi.list({ state: "pending" }),
		refetchInterval: 30_000,
		retry: 1,
		refetchOnWindowFocus: false,
	});
	const {
		data: reviewSummary,
		error: reviewSummaryError,
	} = useQuery({
		queryKey: ["surfaceReviews", "summary"],
		queryFn: surfaceReviewsApi.summary,
		refetchInterval: 30_000,
		retry: 1,
		refetchOnWindowFocus: false,
	});

	const suitsList = suitsResponse?.suits ?? [];
	const totalProfiles = suitsList.length;
	const activeProfiles = suitsList.filter((suit) => suit.is_active).length;

	const connectedServers =
		servers?.servers?.filter((server) => server.status === "connected")
			.length || 0;
	const totalClients = clientsData?.total ?? clientsData?.client?.length ?? 0;
	const approvedClients =
		clientsData?.client?.filter((client) => client.approval_status === "approved").length ?? 0;

	const effectiveSystemStatus = React.useMemo(() => {
		if (
			isTauriShell &&
			coreView?.selectedSource === "localhost" &&
			!coreView.localService.running
		) {
			return {
				...systemStatus,
				status: "stopped",
				uptime: 0,
			};
		}
		return systemStatus;
	}, [
		coreView?.localService.running,
		coreView?.selectedSource,
		isTauriShell,
		systemStatus,
	]);

	const showLocalCoreBanner =
		isTauriShell && coreView?.selectedSource === "localhost";
	const localCoreRuntimeMode = coreView?.localhostRuntimeMode;
	const localCoreServiceStatus = coreView?.localService.status;
	const localCoreStatusLabel = localCoreServiceStatus
		? t(`dashboard:core.localServiceStatus.${localCoreServiceStatus}`, {
				defaultValue: localCoreServiceStatus,
			})
		: "";
	const localCoreDetail =
		localCoreRuntimeMode && localCoreServiceStatus
			? t(
					`dashboard:core.localServiceDetail.${localCoreRuntimeMode}.${localCoreServiceStatus}`,
					{
						defaultValue: t("dashboard:core.localServiceDetailFallback", {
							defaultValue:
								"The configured local core status will appear here.",
						}),
					},
				)
			: "";

	/**
	 * Hover lift + shadow need top paint room inside the scrollport. Paired -mt/pt on this wrapper
	 * when Local Core sits above; without a sibling above, -mt would be clipped by overflow-y-auto.
	 * Tight band: pt-1 / -mt-1 (4px) after stepping down from 16px / 12px.
	 */
	const statsGridBleedClass = showLocalCoreBanner ? "-mt-1 pt-1" : "pt-1";

	return (
		<div className="flex min-h-full flex-col gap-4">
			{showLocalCoreBanner ? (
				<div className="flex w-full shrink-0 items-center justify-between gap-4 px-1 py-2">
					<div className="min-w-0 space-y-1">
						<div className="flex items-center gap-2">
							<span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
								{t("dashboard:core.title", { defaultValue: "Local Core" })}
							</span>
							<span className="text-xs text-slate-500">
								{coreView.localhostRuntimeMode === "service"
									? t("dashboard:core.modeService", {
										defaultValue: "Service",
									})
									: t("dashboard:core.modeDesktopManaged", {
										defaultValue: "Desktop",
									})}
							</span>
						</div>
						<p className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
							<span className="text-sm text-slate-700 dark:text-slate-300">
								{localCoreStatusLabel}
							</span>
							<span className="text-xs text-slate-500 dark:text-slate-400">
								{localCoreDetail}
							</span>
						</p>
					</div>
					<div className="flex shrink-0 items-center gap-2">
						<Button
							variant="ghost"
							size="icon"
							className="rounded-full"
							disabled={busyAction !== null}
							onClick={() => {
								void refreshCoreView();
							}}
						>
							<RefreshCw className="h-4 w-4" />
						</Button>
						<Button
							variant={
								coreView.localService.running ? "destructive" : "default"
							}
							size="icon"
							className={`rounded-full ${coreView.localService.running ? "" : "bg-emerald-600 hover:bg-emerald-700"}`}
							disabled={busyAction !== null}
							onClick={() =>
								void manageLocalCore(
									coreView.localService.running ? "stop" : "start",
								)
							}
						>
							{coreView.localService.running ? (
								<Square className="h-4 w-4" />
							) : (
								<Play className="h-4 w-4" />
							)}
						</Button>
					</div>
				</div>
			) : null}
			<div className={cn("shrink-0 px-0.5", statsGridBleedClass)}>
				<div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
					<Link to="/runtime" className="block h-full">
						<Card className="h-full min-h-[160px] cursor-pointer transition-all duration-200 hover:-translate-y-1 hover:border-primary/40 hover:shadow-xl">
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
											<StatusBadge
												status={effectiveSystemStatus?.status || "unknown"}
											/>
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
												{formatUptime(effectiveSystemStatus?.uptime || 0)}
											</span>
										)}
									</div>
									<div className="flex items-center justify-between">
										<CardDescription>
											{t("dashboard:labels.version", {
												defaultValue: "Version",
											})}
										</CardDescription>
										{isLoadingSystem ? (
											<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
										) : (
											<span className="text-sm font-medium">
												{APP_VERSION_LABEL || "—"}
											</span>
										)}
									</div>
								</div>
							</CardContent>
						</Card>
					</Link>

					<Link to="/profiles" className="block h-full">
						<Card className="h-full min-h-[160px] cursor-pointer transition-all duration-200 hover:-translate-y-1 hover:border-primary/40 hover:shadow-xl">
							<CardHeader className="flex flex-row items-center justify-between space-y-0">
								<div>
									<CardTitle className="text-sm font-medium">
										{t("dashboard:cards.profiles", {
											defaultValue: "Profiles",
										})}
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
											<span className="text-sm font-medium">
												{totalProfiles}
											</span>
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

					<Link to="/clients" className="block h-full">
						<Card className="h-full min-h-[160px] cursor-pointer transition-all duration-200 hover:-translate-y-1 hover:border-primary/40 hover:shadow-xl">
							<CardHeader className="flex flex-row items-center justify-between space-y-0">
								<CardTitle className="text-sm font-medium">
									{t("dashboard:cards.clients", { defaultValue: "Clients" })}
								</CardTitle>
								<AppWindow className="h-4 w-4 text-slate-500" />
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
											<span className="text-sm font-medium">
												{totalClients}
											</span>
										)}
									</div>
									<div className="flex items-center justify-between">
										<CardDescription>
											{t("dashboard:labels.approved", {
												defaultValue: "Approved",
											})}
										</CardDescription>
										{isLoadingClients ? (
											<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
										) : (
											<span className="text-sm font-medium">
												{approvedClients}
											</span>
										)}
									</div>
								</div>
							</CardContent>
						</Card>
					</Link>

					<Link to="/servers" className="block h-full">
						<Card className="h-full min-h-[160px] cursor-pointer transition-all duration-200 hover:-translate-y-1 hover:border-primary/40 hover:shadow-xl">
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
				</div>
			</div>

			<div className="grid shrink-0 items-stretch gap-4 md:grid-cols-2">
				<div className="h-full">
					<Card className="h-full">
						<CardHeader>
							<div className="flex items-center gap-2">
								<Activity className="h-5 w-5 text-sky-500" />
								<CardTitle className="text-base">
									{t("dashboard:metrics.title", { defaultValue: "Metrics" })}
								</CardTitle>
							</div>
							<CardDescription className="text-xs">
								{t("dashboard:metrics.description", {
									defaultValue:
										"MCPMate process CPU and memory utilization sampled every 30 seconds",
								})}
							</CardDescription>
						</CardHeader>
						<CardContent>
							<MetricsTrendChart />
						</CardContent>
					</Card>
				</div>

				<TokenSavingsTrendCard className="h-full" />
			</div>

			<Card className="flex flex-1 flex-col">
				<CardHeader className="pb-3">
					<div className="flex items-start justify-between gap-3">
						<div>
							<div className="flex items-center gap-2">
								<ClipboardCheck className="h-5 w-5 text-amber-600" />
								<CardTitle className="text-base">
									{t("surfaceReview:dashboard.title", {
										defaultValue: "Review todos",
									})}
								</CardTitle>
							</div>
							<CardDescription className="mt-1 text-xs">
								{t("surfaceReview:dashboard.description", {
									defaultValue:
										"Capability changes that need confirmation at their configuration source.",
								})}
							</CardDescription>
						</div>
						<span className="rounded-full border border-amber-300 bg-amber-100 px-2 py-1 text-xs font-medium text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-200">
							{t("surfaceReview:dashboard.pending", {
								count: pendingReviewItems.length,
								defaultValue: "{{count}} pending",
							})}
						</span>
					</div>
				</CardHeader>
				<CardContent>
					{reviewSummaryError ? (
						<div className="mb-3 flex items-center gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
							<AlertTriangle className="h-4 w-4" />
							{t("surfaceReview:errors.summary", {
								defaultValue: "Unable to load Surface reconciliation status.",
							})}
						</div>
					) : reviewSummary &&
						reviewSummary.failed_reconciliation_count > 0 ? (
						<div className="mb-3 flex items-center gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
							<AlertTriangle className="h-4 w-4" />
							{t("surfaceReview:dashboard.failedReconciliation", {
								count: reviewSummary.failed_reconciliation_count,
								defaultValue:
									"{{count}} Surface reconciliation jobs failed. Review the audit log before relying on the affected clients.",
							})}
						</div>
					) : null}
					{isLoadingReviews ? (
						<div className="space-y-2">
							{Array.from({ length: 2 }, (_, index) => (
								<div
									key={`review-todo-skeleton-${index}`}
									className="h-11 animate-pulse rounded bg-muted"
								/>
							))}
						</div>
					) : reviewItemsError ? (
						<div className="flex items-center gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
							<AlertTriangle className="h-4 w-4" />
							{t("surfaceReview:errors.load", {
								defaultValue: "Unable to load pending capability reviews.",
							})}
						</div>
					) : pendingReviewItems.length === 0 ? (
						<p className="text-sm text-muted-foreground">
							{t("surfaceReview:dashboard.empty", {
								defaultValue: "No capability reviews are pending.",
							})}
						</p>
					) : (
						<div className="divide-y rounded-md border">
							{pendingReviewItems.slice(0, 6).map((item) => {
								const owner = item.owners[0];
								const destination = owner
									? getSurfaceReviewDestination(
											item,
											owner,
											clientsData?.client ?? [],
										)
									: `/clients?filter=needs_review`;
								return (
									<Link
										key={item.review_item_id}
										to={destination}
										className="flex items-center justify-between gap-3 px-3 py-2.5 transition-colors hover:bg-muted/60"
									>
										<div className="min-w-0">
											<div className="truncate text-sm font-medium">
												{owner?.owner_id ?? item.consumer_id}
											</div>
											<div className="truncate font-mono text-xs text-muted-foreground">
												{item.change_class} · {item.ref_id}
											</div>
										</div>
										<ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
									</Link>
								);
							})}
						</div>
					)}
				</CardContent>
			</Card>
		</div>
	);
}
