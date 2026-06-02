import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import {
	Activity,
	AlertTriangle,
	AppWindow,
	ChevronDown,
	Maximize2,
	Pin,
	PinOff,
	ScrollText,
	Server,
	Sliders,
	X,
} from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Button } from "../../components/ui/button";
import { TooltipProvider } from "../../components/ui/tooltip";
import { onboardingApi } from "../../lib/onboarding-api";
import {
	closeOperatorPanel,
	openFullBoardFromOperator,
	setOperatorPanelPinned,
} from "../../lib/desktop-operator";
import { useDesktopCoreState } from "../../lib/desktop-core-state";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { auditApi, systemApi } from "../../lib/api";
import { isTauriEnvironmentSync } from "../../lib/platform";
import type { AuditEventRecord, ServerSummary } from "../../lib/types";
import { cn, formatUptime } from "../../lib/utils";
import { OperatorActivityRowDetail } from "./operator-activity-row-detail";
import { formatOperatorAuditAction } from "./operator-audit-format";
import { listOperatorClients, listOperatorProfiles, listOperatorServers } from "./operator-api";
import { OperatorChartCarousel } from "./operator-chart-carousel";
import { OperatorClientsRowDetail } from "./operator-clients-row-detail";
import { OperatorCoreRowDetail } from "./operator-core-row-detail";
import { OperatorProfilesRowDetail } from "./operator-profiles-row-detail";
import { OperatorServersRowDetail } from "./operator-servers-row-detail";
import { OperatorServerImportSheet } from "./operator-server-import-sheet";
import { useOperatorServerImport } from "./use-operator-server-import";
import { OperatorPanelHeader, operatorNoDragRegionStyle } from "./operator-panel-header";
import { OperatorPanelFrame, OperatorPanelShell } from "./operator-panel-shell";

type OperatorSection = "core" | "profiles" | "clients" | "servers" | "activity";
type OperatorStatus = "ready" | "warning" | "error" | "idle" | "loading";

interface OperatorRow {
	id: OperatorSection;
	icon: React.ComponentType<{ className?: string }>;
	title: string;
	summary?: string;
	meta?: string;
	status: OperatorStatus;
	highlight?: string;
	detailHint: string;
	showSummary: boolean;
	metaNode?: React.ReactNode;
}

const OPERATOR_SURFACE_ATTRIBUTE = "data-mcpmate-surface";
const OPERATOR_SURFACE_VALUE = "operator";

function isServerAttention(server: ServerSummary): boolean {
	const status = String(server.status || "").toLowerCase();
	return status === "error" || status === "disconnected" || server.enabled === false;
}

function statusIconTileClass(status: OperatorStatus): string {
	if (status === "ready") return "bg-emerald-500 text-white dark:bg-emerald-600";
	if (status === "warning") return "bg-amber-500 text-white dark:bg-amber-600";
	if (status === "error") return "bg-red-500 text-white dark:bg-red-600";
	if (status === "loading") return "bg-sky-500 text-white dark:bg-sky-600";
	return "bg-slate-400 text-white dark:bg-slate-600";
}

const OPERATOR_ACTIVITY_EVENT_LIMIT = 8;

function formatActivityMeta(
	t: TFunction,
	latestAuditEvent?: AuditEventRecord,
): string {
	if (latestAuditEvent?.action) {
		return t("operator:rows.activity.lastEvent", {
			action: formatOperatorAuditAction(latestAuditEvent.action, t),
			defaultValue: "Latest: {{action}}",
		});
	}

	return t("operator:rows.activity.noEvents", {
		defaultValue: "No recent activity",
	});
}

const ROW_EXPAND_CHEVRON_CLASS =
	"h-4 w-4 shrink-0 text-slate-400 transition-transform duration-200 dark:text-slate-500";

function OperatorFullBoardControl({
	className,
	isTauriShell,
	label,
	onDesktopOpen,
	path,
}: {
	className?: string;
	isTauriShell: boolean;
	label: string;
	onDesktopOpen: (path: string) => void;
	path: string;
}) {
	if (isTauriShell) {
		return (
			<Button
				type="button"
				variant="ghost"
				size="icon"
				className={className}
				aria-label={label}
				title={label}
				style={operatorNoDragRegionStyle}
				onClick={(event) => {
					event.stopPropagation();
					onDesktopOpen(path);
				}}
			>
				<Maximize2 className="h-3.5 w-3.5" aria-hidden />
			</Button>
		);
	}

	return (
		<Button
			asChild
			variant="ghost"
			size="icon"
			className={className}
			style={operatorNoDragRegionStyle}
		>
			<Link to={path} aria-label={label} title={label} style={operatorNoDragRegionStyle}>
				<Maximize2 className="h-3.5 w-3.5" aria-hidden />
			</Link>
		</Button>
	);
}

function OperatorPanelRow({
	detailContent,
	onToggleSelect,
	row,
	selected,
}: {
	detailContent?: React.ReactNode;
	onToggleSelect: (rowId: OperatorSection) => void;
	row: OperatorRow;
	selected: OperatorSection | null;
}) {
	const { t } = useTranslation();
	const Icon = row.icon;
	const isSelected = row.id === selected;
	const detailId = `operator-row-detail-${row.id}`;
	const rowToggleLabel = t(
		isSelected ? "operator:actions.collapseRowDetails" : "operator:actions.expandRowDetails",
		{
			title: row.title,
			defaultValue: isSelected ? "Collapse {{title}} details" : "Expand {{title}} details",
		},
	);

	return (
		<div>
			<div
				className={cn(
					"min-h-[52px] px-3 py-2.5 transition-colors",
					"hover:bg-slate-50 focus-within:bg-slate-50 dark:hover:bg-slate-900/70 dark:focus-within:bg-slate-900/70",
					isSelected && "bg-slate-50 dark:bg-slate-900/50",
				)}
			>
				<button
					type="button"
					className="grid w-full min-w-0 grid-cols-[36px_minmax(0,1fr)_auto] items-start gap-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-slate-950"
					onClick={() => onToggleSelect(row.id)}
					aria-expanded={isSelected}
					aria-controls={detailId}
					aria-label={rowToggleLabel}
					title={rowToggleLabel}
				>
					<span
						className={cn(
							"mt-0.5 flex h-9 w-9 items-center justify-center rounded-md",
							statusIconTileClass(row.status),
						)}
					>
						<Icon className="h-4 w-4" aria-hidden />
					</span>
					<span className="min-w-0 pt-0.5">
						<span className="flex min-w-0 items-center gap-2">
							<span className="truncate text-sm font-medium">{row.title}</span>
							{row.highlight ? (
								<span className="shrink-0 rounded-full bg-amber-50 px-1.5 py-0.5 text-[10px] font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
									{row.highlight}
								</span>
							) : null}
						</span>
						{row.showSummary && row.summary ? (
							<span className="mt-0.5 block truncate text-xs text-slate-600 dark:text-slate-300">
								{row.summary}
							</span>
						) : null}
						{row.metaNode ??
							(row.meta ? (
								<span className="mt-0.5 block truncate text-[11px] text-slate-500 dark:text-slate-400">
									{row.meta}
								</span>
							) : null)}
					</span>
					<span className="mt-0.5 flex h-9 items-center" aria-hidden>
						<ChevronDown
							className={cn(ROW_EXPAND_CHEVRON_CLASS, isSelected && "rotate-180")}
						/>
					</span>
				</button>
			</div>
			{isSelected ? (
				detailContent ?? (
					<div
						id={detailId}
						className="border-t border-slate-100 px-3 py-2.5 text-xs text-slate-500 dark:border-slate-800 dark:text-slate-400"
						data-testid="operator-inline-detail"
					>
						<p className="text-[10px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
							{t("operator:detail.currentFocus", {
								defaultValue: "Next action",
							})}
						</p>
						<p className="mt-1 text-slate-700 dark:text-slate-300">{row.detailHint}</p>
						<p className="mt-2 truncate text-[11px]">
							{t("operator:detail.fullBoardHint", {
								defaultValue:
									"Use Open Full Board in the header for deep editing, raw capability data, or Inspector workflows.",
							})}
						</p>
					</div>
				)
			) : null}
		</div>
	);
}

function OperatorOnboardingLoadingShell() {
	const { t } = useTranslation();

	return (
		<OperatorPanelFrame>
			<OperatorPanelShell>
			<OperatorPanelHeader />

			<section className="flex min-h-0 flex-1 items-center justify-center px-5 py-8 text-center">
				<p className="text-sm text-slate-500 dark:text-slate-400">
					{t("operator:onboarding.checking", { defaultValue: "Checking setup status" })}
				</p>
			</section>
			</OperatorPanelShell>
		</OperatorPanelFrame>
	);
}

function OperatorOnboardingGate({
	desktopError,
	isTauriShell,
	onOpenFullBoardSetup,
	state,
}: {
	desktopError: string | null;
	isTauriShell: boolean;
	onOpenFullBoardSetup: () => Promise<void>;
	state: "idle" | "opening" | "opened";
}) {
	const { t } = useTranslation();
	const busy = state === "opening";

	return (
		<OperatorPanelFrame>
			<OperatorPanelShell>
			<OperatorPanelHeader />

			<section className="flex min-h-0 flex-1 flex-col justify-center px-5 py-8 text-center">
				<div className="mx-auto flex h-10 w-10 items-center justify-center rounded-lg border border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-900/70 dark:bg-amber-950/30 dark:text-amber-300">
					<AlertTriangle className="h-5 w-5" aria-hidden />
				</div>
				<h2 className="mt-4 text-base font-semibold">
					{t("operator:onboarding.title", { defaultValue: "Setup required" })}
				</h2>
				<p className="mt-2 text-sm text-slate-600 dark:text-slate-300">
					{isTauriShell
						? t("operator:onboarding.desktopMessage", {
							defaultValue:
								"MCPMate setup continues in Full Board. This tray panel will stay compact instead of hosting onboarding.",
						})
						: t("operator:onboarding.webMessage", {
							defaultValue:
								"Web preview can open the setup route here, but desktop onboarding requires the MCPMate shell.",
						})}
				</p>
				<div className="mt-5">
					{isTauriShell ? (
						<Button
							type="button"
							disabled={busy}
							style={operatorNoDragRegionStyle}
							onClick={() => {
								void onOpenFullBoardSetup();
							}}
						>
							{state === "opened"
								? t("operator:onboarding.opened", {
									defaultValue: "Full Board setup opened",
								})
								: busy
									? t("operator:onboarding.opening", {
										defaultValue: "Opening Full Board setup",
									})
									: t("operator:onboarding.openSetup", {
										defaultValue: "Open Full Board setup",
									})}
						</Button>
					) : (
						<Button asChild style={operatorNoDragRegionStyle}>
							<Link to="/onboarding" style={operatorNoDragRegionStyle}>
								{t("operator:onboarding.openSetup", {
									defaultValue: "Open Full Board setup",
								})}
							</Link>
						</Button>
					)}
				</div>
				{desktopError && isTauriShell ? (
					<p className="mt-4 text-xs text-red-600 dark:text-red-400">
						{t("operator:detail.error", {
							message: desktopError,
							defaultValue: "Desktop action failed: {{message}}",
						})}
					</p>
				) : null}
			</section>
			</OperatorPanelShell>
		</OperatorPanelFrame>
	);
}

export function TrayOperatorPanelPage() {
	usePageTranslations("operator");
	usePageTranslations("audit");
	const { t, i18n } = useTranslation();
	const queryClient = useQueryClient();
	const [selected, setSelected] = React.useState<OperatorSection | null>(null);
	const [pinned, setPinned] = React.useState(false);
	const [pinPending, setPinPending] = React.useState(false);
	const [desktopError, setDesktopError] = React.useState<string | null>(null);
	const [coreRestartBusy, setCoreRestartBusy] = React.useState(false);
	const [onboardingForwardState, setOnboardingForwardState] = React.useState<
		"idle" | "opening" | "opened"
	>("idle");
	const isTauriShell = React.useMemo(() => isTauriEnvironmentSync(), []);
	const {
		busyAction: coreBusyAction,
		coreView,
		manageLocalCore,
	} = useDesktopCoreState();

	React.useEffect(() => {
		const html = document.documentElement;
		const body = document.body;
		const root = document.getElementById("root");

		if (!root) {
			throw new Error("MCPMate root element is missing.");
		}

		html.setAttribute(OPERATOR_SURFACE_ATTRIBUTE, OPERATOR_SURFACE_VALUE);
		body.setAttribute(OPERATOR_SURFACE_ATTRIBUTE, OPERATOR_SURFACE_VALUE);
		root.setAttribute(OPERATOR_SURFACE_ATTRIBUTE, OPERATOR_SURFACE_VALUE);

		return () => {
			for (const element of [html, body, root]) {
				if (element.getAttribute(OPERATOR_SURFACE_ATTRIBUTE) === OPERATOR_SURFACE_VALUE) {
					element.removeAttribute(OPERATOR_SURFACE_ATTRIBUTE);
				}
			}
		};
	}, []);

	const onboardingQuery = useQuery({
		queryKey: ["onboardingStatus"],
		queryFn: () => onboardingApi.getStatus(),
		staleTime: 60_000,
		refetchInterval: (query) =>
			query.state.data?.data?.completed === false ? 2_000 : false,
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
	const systemSettingsQuery = useQuery({
		queryKey: ["systemSettings"],
		queryFn: systemApi.getSettings,
		staleTime: 60_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const serversQuery = useQuery({
		queryKey: ["operator", "servers"],
		queryFn: listOperatorServers,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const clientsQuery = useQuery({
		queryKey: ["operator", "clients"],
		queryFn: listOperatorClients,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const profilesQuery = useQuery({
		queryKey: ["operator", "profiles"],
		queryFn: listOperatorProfiles,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});
	const auditQuery = useQuery({
		queryKey: ["audit", "operator-panel"],
		queryFn: () => auditApi.list({ limit: OPERATOR_ACTIVITY_EVENT_LIMIT }),
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const operatorServerImport = useOperatorServerImport({
		onImported: () => {
			void queryClient.invalidateQueries({ queryKey: ["operator", "servers"] });
			void queryClient.invalidateQueries({ queryKey: ["servers"] });
		},
	});

	const onboardingCompleted = onboardingQuery.data?.data?.completed;
	const { refetch: refetchOnboardingStatus } = onboardingQuery;

	React.useEffect(() => {
		const refetchOnActivation = () => {
			if (document.visibilityState === "hidden") {
				return;
			}
			void refetchOnboardingStatus();
		};

		window.addEventListener("focus", refetchOnActivation);
		document.addEventListener("visibilitychange", refetchOnActivation);
		return () => {
			window.removeEventListener("focus", refetchOnActivation);
			document.removeEventListener("visibilitychange", refetchOnActivation);
		};
	}, [refetchOnboardingStatus]);

	const servers = serversQuery.data?.servers ?? [];
	const connectedServers = servers.filter(
		(server) => String(server.status).toLowerCase() === "connected",
	).length;
	const activeServers = servers.filter((server) => server.enabled).length;
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
	const latestAuditEvent = auditQuery.data?.events?.[0];
	const auditEvents = auditQuery.data?.events ?? [];
	const systemStatus = systemQuery.data?.status ?? "unknown";
	const systemReady = systemStatus === "running" || systemStatus === "degraded";

	const rows = React.useMemo<OperatorRow[]>(
		() => {
			const rowT = (key: string, options: Record<string, unknown>) =>
				t(key, { lng: i18n.language, ...options });

			const coreDetailHint = systemReady
				? rowT("operator:detail.core.ready", {
					defaultValue: "Confirm local Core status before changing configuration.",
				})
				: rowT("operator:detail.core.notReady", {
					defaultValue: "Open Runtime for service control, ports, and health checks.",
				});

			const profilesDetailHint =
				activeProfiles > 0
					? rowT("operator:detail.profiles.active", {
						defaultValue: "Keep the active profile visible without capability bulk controls.",
					})
					: rowT("operator:detail.profiles.empty", {
						defaultValue: "Open Full Board to create or activate a profile.",
					});

			const clientsDetailHint =
				pendingClients > 0
					? rowT("operator:detail.clients.pending", {
						defaultValue: "Review pending clients before they receive profile access.",
					})
					: rowT("operator:detail.clients.clear", {
						defaultValue: "Detect local clients and keep config-file editing in Full Board.",
					});

			const serversDetailHint =
				serverAttentionCount > 0 || servers.length > 0
					? rowT("operator:detail.servers.attention", {
							defaultValue: "Watch connection health before opening Inspector details.",
						})
					: rowT("operator:detail.servers.empty", {
							defaultValue:
								"Drag an MCP server JSON snippet onto Drop-in to install.",
						});

			return [
				{
					id: "core",
					icon: Activity,
					title: rowT("operator:rows.core.title", { defaultValue: "Core" }),
					summary: systemQuery.isLoading
						? rowT("operator:rows.core.loading", { defaultValue: "Checking Core status" })
						: systemQuery.isError
							? rowT("operator:rows.core.error", {
								defaultValue: "Core status is unavailable",
							})
							: systemReady
								? undefined
								: rowT("operator:rows.core.notReady", {
									defaultValue: "Core needs attention",
								}),
					meta: rowT("operator:rows.core.meta", {
						status: systemStatus,
						uptime: formatUptime(systemQuery.data?.uptime ?? 0),
						defaultValue: "{{status}} · {{uptime}} uptime",
					}),
					status: systemQuery.isLoading ? "loading" : systemReady ? "ready" : "error",
					detailHint: coreDetailHint,
					showSummary: !systemReady || systemQuery.isLoading || systemQuery.isError,
				},
				{
					id: "profiles",
					icon: Sliders,
					title: rowT("operator:rows.profiles.title", { defaultValue: "Profiles" }),
					summary: profilesQuery.isLoading
						? rowT("operator:rows.profiles.loading", { defaultValue: "Loading profiles" })
						: profilesQuery.isError
							? rowT("operator:rows.profiles.error", {
								defaultValue: "Profiles are unavailable",
							})
							: rowT("operator:rows.profiles.summary", {
								count: activeProfiles,
								active: activeProfiles,
								total: profiles.length,
								defaultValue:
									activeProfiles === 1
										? "{{active}} active profile"
										: "{{active}} active of {{total}} profiles",
							}),
					status: profilesQuery.isLoading
						? "loading"
						: profilesQuery.isError
							? "error"
							: activeProfiles > 0
								? "ready"
								: "warning",
					detailHint: profilesDetailHint,
					showSummary: true,
				},
				{
					id: "clients",
					icon: AppWindow,
					title: rowT("operator:rows.clients.title", { defaultValue: "Clients" }),
					summary: clientsQuery.isLoading
						? rowT("operator:rows.clients.loading", { defaultValue: "Loading clients" })
						: clientsQuery.isError
							? rowT("operator:rows.clients.error", {
								defaultValue: "Clients are unavailable",
							})
							: rowT("operator:rows.clients.summary", {
								count: clients.length,
								defaultValue: "{{count}} clients detected",
							}),
					meta: rowT("operator:rows.clients.meta", {
						approved: approvedClients,
						pending: pendingClients,
						defaultValue: "{{approved}} approved · {{pending}} pending",
					}),
					metaNode:
						clientsQuery.isLoading || clientsQuery.isError ? undefined : (
							<span className="mt-0.5 block truncate text-[11px] text-slate-500 dark:text-slate-400">
								<span>
									{rowT("operator:rows.clients.metaApproved", {
										count: approvedClients,
										defaultValue: "{{count}} approved",
									})}
								</span>
								<span aria-hidden> · </span>
								<span
									className={cn(
										pendingClients > 0 &&
										"font-semibold text-amber-700 dark:text-amber-300",
									)}
								>
									{rowT("operator:rows.clients.metaPending", {
										count: pendingClients,
										defaultValue: "{{count}} pending",
									})}
								</span>
							</span>
						),
					status: clientsQuery.isLoading
						? "loading"
						: clientsQuery.isError
							? "error"
							: pendingClients > 0
								? "warning"
								: "ready",
					detailHint: clientsDetailHint,
					showSummary: clientsQuery.isLoading || clientsQuery.isError,
				},
				{
					id: "servers",
					icon: Server,
					title: rowT("operator:rows.servers.title", { defaultValue: "Servers" }),
					summary: serversQuery.isLoading
						? rowT("operator:rows.servers.loading", { defaultValue: "Loading servers" })
						: serversQuery.isError
							? rowT("operator:rows.servers.error", {
								defaultValue: "Servers are unavailable",
							})
							: rowT("operator:rows.servers.summary", {
								count: servers.length,
								defaultValue: "{{count}} servers installed",
							}),
					meta: rowT("operator:rows.servers.meta", {
						active: activeServers,
						attention: serverAttentionCount,
						defaultValue: "{{active}} active · {{attention}} need attention",
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
					highlight:
						serverAttentionCount > 0
							? rowT("operator:rows.servers.attention", {
								count: serverAttentionCount,
								defaultValue: "{{count}} need attention",
							})
							: undefined,
					detailHint: serversDetailHint,
					showSummary: serversQuery.isLoading || serversQuery.isError,
				},
				{
					id: "activity",
					icon: ScrollText,
					title: rowT("operator:rows.activity.title", { defaultValue: "Activity" }),
					summary: undefined,
					meta: formatActivityMeta(t, latestAuditEvent),
					status: auditQuery.isLoading
						? "loading"
						: auditQuery.isError
							? "error"
							: auditEvents.length > 0
								? "ready"
								: "idle",
					detailHint: "",
					showSummary: false,
				},
			];
		},
		[
			activeProfiles,
			approvedClients,
			clients.length,
			clientsQuery.isError,
			clientsQuery.isLoading,
			activeServers,
			connectedServers,
			i18n.language,
			auditEvents.length,
			auditQuery.isError,
			auditQuery.isLoading,
			latestAuditEvent,
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

	const coreRow = rows.find((row) => row.id === "core");
	const toggleRowSelection = React.useCallback((rowId: OperatorSection) => {
		setSelected((current) => (current === rowId ? null : rowId));
	}, []);

	const mcpEndpointUrl = React.useMemo(() => {
		const settingsUrl = systemSettingsQuery.data?.mcp_http_url?.trim();
		if (settingsUrl) {
			return settingsUrl;
		}
		if (typeof coreView?.localhostMcpPort === "number" && coreView.localhostMcpPort > 0) {
			return `http://127.0.0.1:${coreView.localhostMcpPort}/mcp`;
		}
		return null;
	}, [coreView?.localhostMcpPort, systemSettingsQuery.data?.mcp_http_url]);

	const coreRestartAvailable =
		!isTauriShell || coreView?.selectedSource === "localhost";
	const localCoreServiceControlsAvailable =
		isTauriShell && coreView?.selectedSource === "localhost";

	const coreServiceRunning = isTauriShell
		? localCoreServiceControlsAvailable && Boolean(coreView?.localService?.running)
		: systemQuery.data?.status === "running";

	const coreActionBusy = React.useMemo(() => {
		if (coreRestartBusy) {
			return "restart" as const;
		}
		if (coreBusyAction === "start" || coreBusyAction === "stop") {
			return coreBusyAction;
		}
		return null;
	}, [coreBusyAction, coreRestartBusy]);

	const handleCoreRestart = React.useCallback(async () => {
		if (!coreRestartAvailable) {
			return;
		}
		setDesktopError(null);
		setCoreRestartBusy(true);
		try {
			if (isTauriShell) {
				await manageLocalCore("restart");
			} else {
				await systemApi.restart();
				await Promise.all([
					queryClient.invalidateQueries({ queryKey: ["systemStatus"] }),
					queryClient.invalidateQueries({ queryKey: ["systemSettings"] }),
				]);
			}
		} catch (error) {
			setDesktopError(error instanceof Error ? error.message : String(error));
		} finally {
			setCoreRestartBusy(false);
		}
	}, [coreRestartAvailable, isTauriShell, manageLocalCore, queryClient]);

	const handleCoreToggleService = React.useCallback(async () => {
		if (!localCoreServiceControlsAvailable) {
			return;
		}
		setDesktopError(null);
		try {
			await manageLocalCore(coreServiceRunning ? "stop" : "start");
		} catch (error) {
			setDesktopError(error instanceof Error ? error.message : String(error));
		}
	}, [coreServiceRunning, localCoreServiceControlsAvailable, manageLocalCore]);

	const runDesktopAction = React.useCallback(async (action: () => Promise<void>) => {
		if (!isTauriEnvironmentSync()) {
			return;
		}
		setDesktopError(null);
		try {
			await action();
		} catch (error) {
			setDesktopError(error instanceof Error ? error.message : String(error));
		}
	}, []);

	const openFullBoardPath = React.useCallback(
		(path: string) => {
			void runDesktopAction(() => openFullBoardFromOperator(path));
		},
		[runDesktopAction],
	);

	const coreDetailContent =
		selected === "core" ? (
			<OperatorCoreRowDetail
				busyAction={coreActionBusy}
				detailId="operator-row-detail-core"
				isTauriShell={isTauriShell}
				mcpEndpointLoading={systemSettingsQuery.isLoading}
				mcpEndpointUrl={mcpEndpointUrl}
				onRestart={() => {
					void handleCoreRestart();
				}}
				onToggleService={() => {
					void handleCoreToggleService();
				}}
				restartAvailable={coreRestartAvailable}
				serviceControlsAvailable={localCoreServiceControlsAvailable}
				serviceRunning={coreServiceRunning}
			/>
		) : undefined;

	const profilesDetailContent =
		selected === "profiles" ? (
			<OperatorProfilesRowDetail
				detailId="operator-row-detail-profiles"
				isError={profilesQuery.isError}
				isLoading={profilesQuery.isLoading}
				isTauriShell={isTauriShell}
				onOpenProfilesBoard={() => openFullBoardPath("/profiles")}
				profiles={profiles}
			/>
		) : undefined;

	const clientsDetailContent =
		selected === "clients" ? (
			<OperatorClientsRowDetail
				clients={clients}
				detailId="operator-row-detail-clients"
				isError={clientsQuery.isError}
				isLoading={clientsQuery.isLoading}
				isTauriShell={isTauriShell}
				onOpenClient={(identifier) =>
					openFullBoardPath(`/clients/${encodeURIComponent(identifier)}`)
				}
				onOpenClientsBoard={() => openFullBoardPath("/clients")}
			/>
		) : undefined;

	const serversDetailContent =
		selected === "servers" ? (
			<OperatorServersRowDetail
				detailId="operator-row-detail-servers"
				isError={serversQuery.isError}
				isLoading={serversQuery.isLoading}
				isTauriShell={isTauriShell}
				onImportDrop={operatorServerImport.handleImportDrop}
				onOpenServer={(serverId) =>
					openFullBoardPath(`/servers/${encodeURIComponent(serverId)}`)
				}
				servers={servers}
			/>
		) : undefined;

	const activityDetailContent =
		selected === "activity" ? (
			<OperatorActivityRowDetail
				detailId="operator-row-detail-activity"
				events={auditEvents}
				isError={auditQuery.isError}
				isLoading={auditQuery.isLoading}
				isTauriShell={isTauriShell}
				onOpenLogsBoard={() => openFullBoardPath("/audit")}
			/>
		) : undefined;

	const detailContentByRow: Partial<Record<OperatorSection, React.ReactNode>> = {
		activity: activityDetailContent,
		clients: clientsDetailContent,
		profiles: profilesDetailContent,
		servers: serversDetailContent,
	};

	const rowGroups = [
		{
			id: "workspace",
			label: t("operator:groups.workspace", {
				defaultValue: "Workspace",
			}),
			rows: rows.filter(
				(row) => row.id === "profiles" || row.id === "clients" || row.id === "servers",
			),
		},
		{
			id: "activity",
			label: t("operator:groups.activity", { defaultValue: "Activity" }),
			rows: rows.filter((row) => row.id === "activity"),
		},
	];

	const openFullBoardSetup = React.useCallback(async () => {
		setDesktopError(null);
		setOnboardingForwardState("opening");
		try {
			await openFullBoardFromOperator("/onboarding");
			setOnboardingForwardState("opened");
		} catch (error) {
			setOnboardingForwardState("idle");
			setDesktopError(error instanceof Error ? error.message : String(error));
		}
	}, []);

	React.useEffect(() => {
		if (onboardingCompleted !== false || !isTauriShell) {
			return;
		}
		void openFullBoardSetup();
	}, [isTauriShell, onboardingCompleted, openFullBoardSetup]);

	React.useEffect(() => {
		if (!isTauriShell) {
			return;
		}

		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key !== "Escape" || pinned) {
				return;
			}
			event.preventDefault();
			void runDesktopAction(closeOperatorPanel);
		};

		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, [isTauriShell, pinned, runDesktopAction]);

	const togglePinned = React.useCallback(async () => {
		const nextPinned = !pinned;
		setPinPending(true);
		setDesktopError(null);
		try {
			await setOperatorPanelPinned(nextPinned);
			setPinned(nextPinned);
		} catch (error) {
			setDesktopError(error instanceof Error ? error.message : String(error));
		} finally {
			setPinPending(false);
		}
	}, [pinned]);

	if (onboardingQuery.isPending) {
		return <OperatorOnboardingLoadingShell />;
	}

	if (onboardingCompleted === false) {
		return (
			<OperatorOnboardingGate
				desktopError={desktopError}
				isTauriShell={isTauriShell}
				onOpenFullBoardSetup={openFullBoardSetup}
				state={onboardingForwardState}
			/>
		);
	}

	return (
		<OperatorPanelFrame>
			<OperatorPanelShell>
			<OperatorPanelHeader
				controls={
					<>
						{isTauriShell ? (
							<>
								<Button
									type="button"
									variant="ghost"
									size="icon"
									className="h-8 w-8"
									aria-pressed={pinned}
									aria-label={t(pinned ? "operator:unpin" : "operator:pin", {
										defaultValue: pinned ? "Unpin" : "Pin on top",
									})}
									style={operatorNoDragRegionStyle}
									title={t(pinned ? "operator:unpin" : "operator:pin", {
										defaultValue: pinned ? "Unpin" : "Pin on top",
									})}
									disabled={pinPending}
									onClick={() => {
										void togglePinned();
									}}
								>
									{pinned ? <PinOff className="h-4 w-4" /> : <Pin className="h-4 w-4" />}
								</Button>
								<Button
									type="button"
									variant="ghost"
									size="icon"
									className="h-8 w-8 text-slate-600 dark:text-slate-300"
									aria-label={t("operator:close", { defaultValue: "Close" })}
									style={operatorNoDragRegionStyle}
									title={t("operator:close", { defaultValue: "Close" })}
									onClick={() => {
										void runDesktopAction(closeOperatorPanel);
									}}
								>
									<X className="h-4 w-4" />
								</Button>
							</>
						) : null}
						<OperatorFullBoardControl
							className="h-8 w-8 text-slate-600 dark:text-slate-300"
							isTauriShell={isTauriShell}
							label={t("operator:openFullBoard", { defaultValue: "Open Full Board" })}
							onDesktopOpen={openFullBoardPath}
							path="/"
						/>
					</>
				}
			/>

			<OperatorChartCarousel />

			{coreRow ? (
				<section
					className="shrink-0 border-b border-slate-200 dark:border-slate-800"
					data-testid="operator-core-hero"
				>
					<TooltipProvider delayDuration={200}>
						<OperatorPanelRow
							detailContent={coreDetailContent}
							onToggleSelect={toggleRowSelection}
							row={coreRow}
							selected={selected}
						/>
					</TooltipProvider>
				</section>
			) : null}

			<section
				className={cn(
					"min-h-0 flex-1",
					selected === "activity" ? "overflow-hidden" : "overflow-y-auto",
				)}
			>
				<TooltipProvider delayDuration={200}>
					<div className="space-y-2 py-2">
						{rowGroups.map((group) => (
							<div key={group.id}>
								{group.label ? (
									<p className="px-3 pb-1 pt-1 text-[10px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
										{group.label}
									</p>
								) : null}
								<div className="divide-y divide-slate-100 dark:divide-slate-800">
									{group.rows.map((row) => (
										<OperatorPanelRow
											key={row.id}
											detailContent={detailContentByRow[row.id]}
											onToggleSelect={toggleRowSelection}
											row={row}
											selected={selected}
										/>
									))}
								</div>
							</div>
						))}
					</div>
				</TooltipProvider>
			</section>

			{desktopError ? (
				<footer className="shrink-0 border-t border-slate-200 p-3 dark:border-slate-800">
					<p className="text-xs text-red-600 dark:text-red-400">
						{t("operator:detail.error", {
							message: desktopError,
							defaultValue: "Desktop action failed: {{message}}",
						})}
					</p>
				</footer>
			) : null}

			<OperatorServerImportSheet
				canInstall={operatorServerImport.canInstall}
				drafts={operatorServerImport.drafts}
				dryRunError={operatorServerImport.dryRunError}
				dryRunStats={operatorServerImport.dryRunStats}
				dryRunWarning={operatorServerImport.dryRunWarning}
				isDryRunLoading={operatorServerImport.isDryRunLoading}
				onCancel={operatorServerImport.cancel}
				onConfirm={() => {
					void operatorServerImport.confirmInstall();
				}}
				open={operatorServerImport.open}
				parseError={operatorServerImport.parseError}
				phase={operatorServerImport.phase}
			/>
			</OperatorPanelShell>
		</OperatorPanelFrame>
	);
}
