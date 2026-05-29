import { useQuery } from "@tanstack/react-query";
import {
	Activity,
	AlertTriangle,
	AppWindow,
	ArrowRight,
	BarChart3,
	ExternalLink,
	PanelTopClose,
	Pin,
	PinOff,
	Server,
	Sliders,
	X,
} from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { Navigate } from "react-router-dom";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { onboardingApi } from "../../lib/onboarding-api";
import {
	closeOperatorPanel,
	openFullBoardFromOperator,
	setOperatorPanelPinned,
} from "../../lib/desktop-operator";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import {
	auditApi,
	clientsApi,
	configSuitsApi,
	serversApi,
	systemApi,
} from "../../lib/api";
import type { ClientCheckData, ServerSummary } from "../../lib/types";
import { cn, formatUptime } from "../../lib/utils";

type OperatorSection = "core" | "profiles" | "clients" | "servers" | "traffic" | "attention";
type OperatorStatus = "ready" | "warning" | "error" | "idle" | "loading";

interface OperatorRow {
	id: OperatorSection;
	icon: React.ComponentType<{ className?: string }>;
	title: string;
	summary: string;
	meta: string;
	status: OperatorStatus;
	action: string;
	fullBoardPath: string;
	highlight?: string;
}

function isServerAttention(server: ServerSummary): boolean {
	const status = String(server.status || "").toLowerCase();
	return status === "error" || status === "disconnected" || server.enabled === false;
}

function memoryLabel(bytes?: number): string {
	if (typeof bytes !== "number" || !Number.isFinite(bytes)) {
		return "—";
	}
	const megabytes = bytes / (1024 * 1024);
	if (megabytes >= 1024) {
		return `${(megabytes / 1024).toFixed(1)} GB`;
	}
	return `${megabytes.toFixed(0)} MB`;
}

function statusBadgeVariant(
	status: OperatorStatus,
): "success" | "warning" | "destructive" | "secondary" {
	if (status === "ready") return "success";
	if (status === "warning") return "warning";
	if (status === "error") return "destructive";
	return "secondary";
}

function statusDotClass(status: OperatorStatus): string {
	if (status === "ready") return "bg-emerald-500";
	if (status === "warning") return "bg-amber-500";
	if (status === "error") return "bg-red-500";
	if (status === "loading") return "bg-sky-500";
	return "bg-slate-400";
}

function readinessKey(status: string | undefined, isLoading: boolean, isError: boolean): string {
	if (isLoading) return "starting";
	if (isError) return "unreachable";
	if (status === "running") return "ready";
	if (status === "degraded") return "degraded";
	if (status === "stopped") return "stopped";
	if (status === "initializing" || status === "starting") return "starting";
	return "unknown";
}

export function TrayOperatorPanelPage() {
	usePageTranslations("operator");
	const { t, i18n } = useTranslation();
	const [selected, setSelected] = React.useState<OperatorSection>("core");
	const [pinned, setPinned] = React.useState(false);
	const [desktopError, setDesktopError] = React.useState<string | null>(null);

	const onboardingQuery = useQuery({
		queryKey: ["onboardingStatus"],
		queryFn: () => onboardingApi.getStatus(),
		staleTime: 60_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const systemQuery = useQuery({
		queryKey: ["systemStatus"],
		queryFn: systemApi.getStatus,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const metricsQuery = useQuery({
		queryKey: ["systemMetrics"],
		queryFn: systemApi.getMetrics,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const serversQuery = useQuery({
		queryKey: ["servers"],
		queryFn: serversApi.getAll,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const clientsQuery = useQuery<ClientCheckData | null>({
		queryKey: ["clients", "operator"],
		queryFn: () => clientsApi.list(false),
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const profilesQuery = useQuery({
		queryKey: ["configSuits", "operator"],
		queryFn: configSuitsApi.getAll,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const auditQuery = useQuery({
		queryKey: ["audit", "operator-panel"],
		queryFn: () => auditApi.list({ limit: 8 }),
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const onboardingCompleted = onboardingQuery.data?.data?.completed;
	const servers = serversQuery.data?.servers ?? [];
	const connectedServers = servers.filter(
		(server) => String(server.status).toLowerCase() === "connected",
	).length;
	const serverAttentionCount = servers.filter(isServerAttention).length;
	const clients = clientsQuery.data?.client ?? [];
	const approvedClients = clients.filter(
		(client) => client.approval_status === "approved",
	).length;
	const pendingClients = clients.filter(
		(client) => client.approval_status === "pending",
	).length;
	const profiles = profilesQuery.data?.suits ?? [];
	const activeProfiles = profiles.filter((profile) => profile.is_active).length;
	const defaultProfile = profiles.find((profile) => profile.is_default);
	const attentionCount = serverAttentionCount + pendingClients;
	const latestAuditEvent = auditQuery.data?.events?.[0];
	const systemStatus = systemQuery.data?.status ?? "unknown";
	const systemReady = systemStatus === "running" || systemStatus === "degraded";
	const readiness = readinessKey(systemQuery.data?.status, systemQuery.isLoading, systemQuery.isError);

	const rows = React.useMemo<OperatorRow[]>(
		() => [
			{
				id: "core",
				icon: Activity,
				title: t("operator:rows.core.title", { defaultValue: "Core" }),
				summary: systemQuery.isLoading
					? t("operator:rows.core.loading", { defaultValue: "Checking Core status" })
					: systemQuery.isError
						? t("operator:rows.core.error", {
								defaultValue: "Core status is unavailable",
							})
						: systemReady
							? t("operator:rows.core.ready", {
									defaultValue: "MCPMate Core is ready",
								})
							: t("operator:rows.core.notReady", {
									defaultValue: "Core needs attention",
								}),
				meta: t("operator:rows.core.meta", {
					status: systemStatus,
					uptime: formatUptime(systemQuery.data?.uptime ?? 0),
					defaultValue: "{{status}} · {{uptime}} uptime",
				}),
				status: systemQuery.isLoading ? "loading" : systemReady ? "ready" : "error",
				action: t("operator:actions.openRuntime", { defaultValue: "Open Runtime" }),
				fullBoardPath: "/runtime",
			},
			{
				id: "profiles",
				icon: Sliders,
				title: t("operator:rows.profiles.title", { defaultValue: "Profiles" }),
				summary: profilesQuery.isLoading
					? t("operator:rows.profiles.loading", { defaultValue: "Loading profiles" })
					: profilesQuery.isError
						? t("operator:rows.profiles.error", {
								defaultValue: "Profiles are unavailable",
							})
						: t("operator:rows.profiles.summary", {
								active: activeProfiles,
								total: profiles.length,
								defaultValue: "{{active}} active of {{total}} profiles",
							}),
				meta:
					defaultProfile?.name ??
					t("operator:rows.profiles.noDefault", {
						defaultValue: "No default profile selected",
					}),
				status: profilesQuery.isLoading
					? "loading"
					: profilesQuery.isError
						? "error"
						: activeProfiles > 0
							? "ready"
							: "warning",
				action: t("operator:actions.manage", { defaultValue: "Manage Profiles" }),
				fullBoardPath: "/profiles",
			},
			{
				id: "clients",
				icon: AppWindow,
				title: t("operator:rows.clients.title", { defaultValue: "Clients" }),
				summary: clientsQuery.isLoading
					? t("operator:rows.clients.loading", { defaultValue: "Loading clients" })
					: clientsQuery.isError
						? t("operator:rows.clients.error", {
								defaultValue: "Clients are unavailable",
							})
						: t("operator:rows.clients.summary", {
								total: clients.length,
								defaultValue: "{{total}} clients connected",
							}),
				meta: t("operator:rows.clients.meta", {
					approved: approvedClients,
					pending: pendingClients,
					defaultValue: "{{approved}} approved · {{pending}} pending",
				}),
				status: clientsQuery.isLoading
					? "loading"
					: clientsQuery.isError
						? "error"
						: pendingClients > 0
							? "warning"
							: "ready",
				action: t("operator:actions.review", { defaultValue: "Review Clients" }),
				fullBoardPath: "/clients",
				highlight:
					pendingClients > 0
						? t("operator:rows.clients.pending", {
								count: pendingClients,
								defaultValue: "{{count}} pending",
							})
						: undefined,
			},
			{
				id: "servers",
				icon: Server,
				title: t("operator:rows.servers.title", { defaultValue: "Servers" }),
				summary: serversQuery.isLoading
					? t("operator:rows.servers.loading", { defaultValue: "Loading servers" })
					: serversQuery.isError
						? t("operator:rows.servers.error", {
								defaultValue: "Servers are unavailable",
							})
						: t("operator:rows.servers.summary", {
								total: servers.length,
								defaultValue: "{{total}} servers installed",
							}),
				meta: t("operator:rows.servers.meta", {
					connected: connectedServers,
					attention: serverAttentionCount,
					defaultValue: "{{connected}} connected · {{attention}} needs attention",
				}),
				status: serversQuery.isLoading
					? "loading"
					: serversQuery.isError
						? "error"
						: serverAttentionCount > 0
							? "warning"
							: connectedServers > 0
								? "ready"
								: "idle",
				action: t("operator:actions.install", { defaultValue: "Open Servers" }),
				fullBoardPath: "/servers",
				highlight:
					serverAttentionCount > 0
						? t("operator:rows.servers.attention", {
								count: serverAttentionCount,
								defaultValue: "{{count}} needs attention",
							})
						: undefined,
			},
			{
				id: "traffic",
				icon: BarChart3,
				title: t("operator:rows.traffic.title", { defaultValue: "Traffic" }),
				summary: metricsQuery.isLoading
					? t("operator:rows.traffic.loading", { defaultValue: "Loading traffic" })
					: metricsQuery.isError
						? t("operator:rows.traffic.error", {
								defaultValue: "Traffic metrics are unavailable",
							})
						: t("operator:rows.traffic.summary", {
								requests: metricsQuery.data?.total_requests_mcp ?? 0,
								defaultValue: "{{requests}} MCP requests",
							}),
				meta: t("operator:rows.traffic.meta", {
					cpu: metricsQuery.data?.cpu_usage_percent ?? metricsQuery.data?.cpu_usage ?? 0,
					memory: memoryLabel(
						metricsQuery.data?.memory_usage_bytes ?? metricsQuery.data?.memory_usage,
					),
					defaultValue: "{{cpu}}% CPU · {{memory}} memory",
				}),
				status: metricsQuery.isLoading ? "loading" : metricsQuery.isError ? "error" : "ready",
				action: t("operator:actions.openLogs", { defaultValue: "Open Logs" }),
				fullBoardPath: "/audit",
			},
			{
				id: "attention",
				icon: AlertTriangle,
				title: t("operator:rows.attention.title", { defaultValue: "Attention" }),
				summary:
					attentionCount > 0
						? t("operator:rows.attention.summary", {
								count: attentionCount,
								defaultValue: "{{count}} items need review",
							})
						: t("operator:rows.attention.clear", {
								defaultValue: "No urgent operator actions",
							}),
				meta:
					latestAuditEvent?.action ??
					t("operator:rows.attention.noEvents", {
						defaultValue: "No recent activity",
					}),
				status: attentionCount > 0 ? "warning" : "ready",
				action: t("operator:actions.inspect", { defaultValue: "Inspect" }),
				fullBoardPath: attentionCount > 0 ? "/clients" : "/audit",
			},
		],
		[
			activeProfiles,
			approvedClients,
			attentionCount,
			clients.length,
			clientsQuery.isError,
			clientsQuery.isLoading,
			connectedServers,
			defaultProfile?.name,
			i18n.language,
			latestAuditEvent?.action,
			metricsQuery.data,
			metricsQuery.isError,
			metricsQuery.isLoading,
			pendingClients,
			profiles.length,
			profilesQuery.isError,
			profilesQuery.isLoading,
			serverAttentionCount,
			servers.length,
			serversQuery.isError,
			serversQuery.isLoading,
			systemQuery.data?.uptime,
			systemQuery.isError,
			systemQuery.isLoading,
			systemReady,
			systemStatus,
			t,
		],
	);

	const selectedRow = rows.find((row) => row.id === selected) ?? rows[0];
	const SelectedIcon = selectedRow.icon;

	const runDesktopAction = React.useCallback(
		async (action: () => Promise<void>) => {
			setDesktopError(null);
			try {
				await action();
			} catch (error) {
				setDesktopError(error instanceof Error ? error.message : String(error));
			}
		},
		[],
	);

	if (onboardingCompleted === false) {
		return <Navigate to="/onboarding" replace />;
	}

	return (
		<main className="flex h-screen min-h-[420px] w-screen max-w-[420px] flex-col overflow-hidden bg-white text-slate-950 dark:bg-slate-950 dark:text-slate-50">
			<header
				className="flex h-11 shrink-0 items-center justify-between border-b border-slate-200 px-3 dark:border-slate-800"
				data-tauri-drag-region
			>
				<div className="flex min-w-0 items-center gap-2" data-tauri-drag-region>
					<span
						className={cn("h-2.5 w-2.5 shrink-0 rounded-full", statusDotClass(selectedRow.status))}
						aria-hidden
					/>
					<div className="min-w-0" data-tauri-drag-region>
						<h1 className="truncate text-sm font-semibold">
							{t("operator:title", { defaultValue: "Operator Panel" })}
						</h1>
					</div>
				</div>
				<div className="flex shrink-0 items-center gap-1">
					<Button
						type="button"
						variant="ghost"
						size="icon"
						className="h-8 w-8"
						aria-pressed={pinned}
						aria-label={t(pinned ? "operator:unpin" : "operator:pin", {
							defaultValue: pinned ? "Unpin" : "Pin on top",
						})}
						title={t(pinned ? "operator:unpin" : "operator:pin", {
							defaultValue: pinned ? "Unpin" : "Pin on top",
						})}
						onClick={() => {
							const next = !pinned;
							setPinned(next);
							void runDesktopAction(() => setOperatorPanelPinned(next));
						}}
					>
						{pinned ? <PinOff className="h-4 w-4" /> : <Pin className="h-4 w-4" />}
					</Button>
					<Button
						type="button"
						variant="ghost"
						size="icon"
						className="h-8 w-8"
						aria-label={t("operator:openFullBoard", { defaultValue: "Open Full Board" })}
						title={t("operator:openFullBoard", { defaultValue: "Open Full Board" })}
						onClick={() => {
							void runDesktopAction(() => openFullBoardFromOperator("/"));
						}}
					>
						<ExternalLink className="h-4 w-4" />
					</Button>
					<Button
						type="button"
						variant="ghost"
						size="icon"
						className="h-8 w-8"
						aria-label={t("operator:close", { defaultValue: "Close" })}
						title={t("operator:close", { defaultValue: "Close" })}
						onClick={() => {
							void runDesktopAction(closeOperatorPanel);
						}}
					>
						<X className="h-4 w-4" />
					</Button>
				</div>
			</header>

			<section className="shrink-0 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
				<div className="flex items-center justify-between gap-3">
					<div className="min-w-0">
						<p className="truncate text-xs font-medium">
							{t(`operator:readiness.${readiness}`, {
								defaultValue: t("operator:readiness.unknown", {
									defaultValue: "Core status unknown",
								}),
							})}
						</p>
						<p className="mt-0.5 truncate text-[11px] text-slate-500 dark:text-slate-400">
							{t("operator:readiness.activeProfile", {
								count: activeProfiles,
								defaultValue: "{{count}} active profiles",
							})}
						</p>
					</div>
					<Badge variant={attentionCount > 0 ? "warning" : "secondary"} className="shrink-0">
						{t("operator:readiness.attention", {
							count: attentionCount,
							defaultValue: "{{count}} attention items",
						})}
					</Badge>
				</div>
			</section>

			<section className="min-h-0 flex-1 overflow-y-auto">
				<div className="divide-y divide-slate-100 dark:divide-slate-800">
					{rows.map((row) => {
						const Icon = row.icon;
						const isSelected = row.id === selected;
						return (
							<div key={row.id}>
								<button
									type="button"
									className={cn(
										"grid min-h-[60px] w-full grid-cols-[36px_minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5 text-left transition-colors",
										"hover:bg-slate-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-inset dark:hover:bg-slate-900/70",
										isSelected && "bg-emerald-50/70 dark:bg-emerald-950/20",
									)}
									onClick={() => setSelected(row.id)}
									aria-pressed={isSelected}
									aria-label={row.title}
								>
									<span className="relative flex h-9 w-9 items-center justify-center rounded-md bg-slate-100 text-slate-700 dark:bg-slate-900 dark:text-slate-200">
										<span
											className={cn(
												"absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full ring-2 ring-white dark:ring-slate-950",
												statusDotClass(row.status),
											)}
											aria-hidden
										/>
										<Icon className="h-4 w-4" aria-hidden />
									</span>
									<span className="min-w-0">
										<span className="flex min-w-0 items-center gap-2">
											<span className="truncate text-sm font-medium">{row.title}</span>
											{row.highlight ? (
												<Badge variant="warning" className="shrink-0 text-[10px]">
													{row.highlight}
												</Badge>
											) : null}
										</span>
										<span className="mt-0.5 block truncate text-xs text-slate-600 dark:text-slate-300">
											{row.summary}
										</span>
										<span className="mt-0.5 block truncate text-[11px] text-slate-500 dark:text-slate-400">
											{row.meta}
										</span>
									</span>
									<Badge variant={statusBadgeVariant(row.status)} className="text-[10px]">
										{t(`operator:status.${row.status}`, {
											defaultValue: row.status,
										})}
									</Badge>
								</button>
								{isSelected ? (
									<div
										className="border-t border-emerald-100 bg-emerald-50/40 px-3 py-3 dark:border-emerald-950/60 dark:bg-emerald-950/10"
										data-testid="operator-inline-detail"
									>
										<div className="flex min-w-0 items-start gap-2">
											<div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-white text-slate-800 shadow-sm dark:bg-slate-900 dark:text-slate-100">
												<SelectedIcon className="h-4 w-4" aria-hidden />
											</div>
											<div className="min-w-0 flex-1">
												<p className="text-[11px] font-medium uppercase text-slate-500">
													{t("operator:detail.currentFocus", {
														defaultValue: "Current focus",
													})}
												</p>
												<p className="mt-1 text-sm font-medium text-slate-900 dark:text-slate-100">
													{selectedRow.summary}
												</p>
												<p className="mt-1 text-xs text-slate-600 dark:text-slate-300">
													{selectedRow.meta}
												</p>
											</div>
										</div>
										<div className="mt-3 flex items-center justify-between gap-2">
											<p className="min-w-0 truncate text-[11px] text-slate-500 dark:text-slate-400">
												{t("operator:detail.fullBoardHint", {
													defaultValue:
														"Open Full Board for deep editing, raw capability data, or Inspector workflows.",
												})}
											</p>
											<Button
												type="button"
												variant="outline"
												size="sm"
												className="h-8 shrink-0"
												onClick={() => {
													void runDesktopAction(() =>
														openFullBoardFromOperator(selectedRow.fullBoardPath),
													);
												}}
											>
												<span className="sr-only">{selectedRow.action}</span>
												<ArrowRight className="h-4 w-4" aria-hidden />
											</Button>
										</div>
									</div>
								) : null}
							</div>
						);
					})}
				</div>
			</section>

			<footer className="shrink-0 border-t border-slate-200 p-3 dark:border-slate-800">
				<Button
					type="button"
					className="h-9 w-full justify-between"
					onClick={() => {
						void runDesktopAction(() => openFullBoardFromOperator("/"));
					}}
				>
					<span>{t("operator:openFullBoard", { defaultValue: "Open Full Board" })}</span>
					<PanelTopClose className="h-4 w-4" aria-hidden />
				</Button>
				{desktopError ? (
					<p className="mt-2 text-xs text-red-600 dark:text-red-400">
						{t("operator:detail.error", {
							message: desktopError,
							defaultValue: "Desktop action failed: {{message}}",
						})}
					</p>
				) : null}
			</footer>
		</main>
	);
}
