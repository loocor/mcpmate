import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	Edit3,
	AlertTriangle,
	Loader2,
	Play,
	Power,
	PowerOff,
	RefreshCw,
	Trash2,
	Wrench,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import CapabilityList from "../../components/capability-list";
import { CapabilityToolbar } from "../../components/capability-toolbar";
import {
	CapabilityPreviewList,
	type CapabilityPreviewFlatItem,
	type CapabilityPreviewKind,
} from "../../components/capability-preview-list";
import { DETAIL_TAB_CONTENT_CLASS } from "../../components/detail-tab-content-class";
import { AuditLogsPanel } from "../../components/audit-logs-panel";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import InspectorDrawer from "../../components/inspector-drawer";
import { ServerAuthBadge } from "../../components/server-auth-badge";
import {
	getOAuthReadinessActionTarget,
	resolveOAuthReadiness,
	resolveServerOAuthReadiness,
} from "../../lib/oauth-readiness";
import { ServerEditDrawer } from "../../components/server-edit-drawer";
import { StatusBadge } from "../../components/status-badge";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../../components/ui/alert-dialog";
import { CachedAvatar } from "../../components/cached-avatar";
import { Button } from "../../components/ui/button";
import { ButtonGroup } from "../../components/ui/button-group";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { auditApi, serversApi } from "../../lib/api";
import { useSecretStoreStatusQuery } from "../../lib/hooks/use-secret-store-status";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import { mergeCapabilityInspectorItem } from "../../lib/capability-detail";
import { collectLoadedInspectorOptions } from "../../lib/inspector-operation";
import { getServerDisplayName } from "../../lib/server-display";
import { useAppStore } from "../../lib/store";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import type { ServerCapabilitySummary, ServerDetail } from "../../lib/types";
import type { CapabilityRecord } from "../../types/capabilities";

const readLegacyCapability = (
	server: ServerDetail | undefined,
): ServerCapabilitySummary | undefined => {
	if (!server) return undefined;
	return server.capabilities ?? undefined;
};

const readLegacyString = (
	server: ServerDetail | undefined,
	key: "protocolVersion" | "serverVersion",
): string | undefined => {
	if (!server || typeof server !== "object") return undefined;
	const value = (server as unknown as Record<string, unknown>)[key];
	return typeof value === "string" ? value : undefined;
};

const TRANSITIONAL_SERVER_STATUSES = new Set([
	"initializing",
	"starting",
	"connecting",
	"busy",
	"stopping",
]);

function isTransitionalServerStatus(status: string | undefined): boolean {
	return TRANSITIONAL_SERVER_STATUSES.has(String(status || "").toLowerCase());
}

interface CapabilityListResponse {
	items: CapabilityRecord[];
	meta?: unknown;
	state?: string;
}

type InspectorTarget = {
	kind: "tool" | "resource" | "prompt" | "template";
	item: CapabilityRecord | null;
	capabilityOptionsByKind?: Partial<
		Record<"tool" | "resource" | "prompt" | "template", CapabilityRecord[]>
	>;
};

type ServerFlatCapabilityItem = CapabilityRecord & {
	__serverCapabilityKind: CapabilityPreviewKind;
};

const SERVER_CAPABILITY_KINDS: CapabilityPreviewKind[] = [
	"tools",
	"resources",
	"templates",
	"prompts",
];

const readCapabilityIdentifier = (value: unknown): string | undefined => {
	if (typeof value === "string" && value.trim()) return value;
  if (typeof value === "number" || typeof value === "boolean")
    return String(value);
	return undefined;
};

const serverCapabilityItemId = (item: ServerFlatCapabilityItem): string => {
	const key =
		readCapabilityIdentifier(item.id) ??
		readCapabilityIdentifier(item.unique_name) ??
    readCapabilityIdentifier(item.unique_uri) ??
    readCapabilityIdentifier(item.unique_uri_template) ??
		"unknown";
	return `${item.__serverCapabilityKind}:${key}`;
};

function firstCapabilityString(
	item: CapabilityRecord,
	keys: Array<keyof CapabilityRecord | string>,
) {
	for (const key of keys) {
		const value = item[key];
		if (typeof value === "string" && value.trim()) {
			return value;
		}
	}
	return undefined;
}

function serverCapabilityDetailKey(
	item: CapabilityRecord,
	kind: CapabilityPreviewKind,
) {
	if (kind === "tools") {
    return firstCapabilityString(item, ["unique_name"]);
	}
	if (kind === "resources") {
    return firstCapabilityString(item, ["unique_uri"]);
	}
	if (kind === "prompts") {
    return firstCapabilityString(item, ["unique_name"]);
	}
  return firstCapabilityString(item, ["unique_uri_template"]);
}

function toCapabilityPreviewKind(
	kind: InspectorTarget["kind"],
): CapabilityPreviewKind {
	if (kind === "tool") return "tools";
	if (kind === "resource") return "resources";
	if (kind === "prompt") return "prompts";
	return "templates";
}

function toInspectorKind(kind: CapabilityPreviewKind): InspectorTarget["kind"] {
	if (kind === "tools") return "tool";
	if (kind === "resources") return "resource";
	if (kind === "prompts") return "prompt";
	return "template";
}

function normalizeCapabilityListResponse(response: {
	items?: unknown;
	meta?: unknown;
	state?: unknown;
}): CapabilityListResponse {
	return {
		items: Array.isArray(response.items)
			? (response.items as CapabilityRecord[])
			: [],
		meta: response.meta,
		state: typeof response.state === "string" ? response.state : undefined,
	};
}

function capabilityKindLabel(
	kind: CapabilityPreviewKind,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	if (kind === "templates") {
		return t("detail.capabilityList.labels.templates", {
			defaultValue: "Resource Templates",
		});
	}

	return t(`detail.capabilityList.labels.${kind}`, {
		defaultValue: kind.charAt(0).toUpperCase() + kind.slice(1),
	});
}

const overviewMetadataGridClass =
	"grid grid-cols-[auto_minmax(0,1fr)] gap-x-5 gap-y-2 text-sm leading-5";
const overviewMetadataLabelClass = "text-xs uppercase leading-5 text-slate-500";
const overviewMetadataValueClass =
	"min-w-0 text-left text-sm leading-5 text-slate-600 dark:text-slate-300";
function OverviewMetadataRow({
	label,
	children,
	multiline = false,
}: {
	label: ReactNode;
	children: ReactNode;
	multiline?: boolean;
}) {
	const rowAlignClass = multiline ? "self-start" : "self-center min-h-6";

	return (
		<div className="contents">
			<span className={`${overviewMetadataLabelClass} ${rowAlignClass}`}>
				{label}
			</span>
			<div className={`${overviewMetadataValueClass} ${rowAlignClass}`}>
				{children}
			</div>
		</div>
	);
}

export function ServerDetailPage() {
	usePageTranslations("servers");
	const { t } = useTranslation("servers");
	const { serverId } = useParams<{ serverId: string }>();
	const navigate = useNavigate();
	const location = useLocation();
	const queryClient = useQueryClient();
	const enableServerDebug = useAppStore(
		(state) => state.dashboardSettings.enableServerDebug,
	);
	const syncServerStateToClients = useAppStore(
		(state) => state.dashboardSettings.syncServerStateToClients,
	);
	const showServerLevelLogs = useAppStore(
		(state) => state.dashboardSettings.showServerLevelLogs,
	);

	const [isEditOpen, setIsEditOpen] = useState(false);
	const [isDeleteOpen, setIsDeleteOpen] = useState(false);
	const [inspector, setInspector] = useState<InspectorTarget | null>(null);

	useEffect(() => {
		const params = new URLSearchParams(location.search);
		let changed = false;
		if (params.has("view")) {
			params.delete("view");
			changed = true;
		}
		if (params.has("channel")) {
			params.delete("channel");
			changed = true;
		}
		if (!changed) {
			return;
		}
		navigate(
			{ pathname: location.pathname, search: params.toString() },
			{ replace: true },
		);
	}, [location.pathname, location.search, navigate]);

	const {
		data: server,
		isLoading,
		isRefetching,
		isFetched,
	} = useQuery({
		queryKey: ["server", serverId],
		queryFn: () => serversApi.getServer(serverId || ""),
		enabled: !!serverId,
		refetchOnMount: "always",
		refetchInterval: (query) => {
			const currentServer = query.state.data;
			if (!currentServer || query.state.error) return false;
			return isTransitionalServerStatus(currentServer.status) ? 5000 : false;
		},
	});
	const isOAuthServer = (server?.auth_mode ?? "").toLowerCase() === "oauth";
	const oauthStatusQuery = useQuery({
		queryKey: ["server-oauth", serverId],
		queryFn: () => serversApi.getOAuthStatus(serverId!),
		enabled: Boolean(serverId && isOAuthServer),
		staleTime: 0,
		refetchOnMount: "always",
		retry: false,
	});
	const secretStoreStatusQuery = useSecretStoreStatusQuery({
		enabled: Boolean(serverId && isOAuthServer),
		staleTime: 0,
		refetchOnMount: "always",
		retry: false,
	});

	const toggleServerM = useMutation({
		mutationFn: async (enable: boolean) => {
			if (!serverId) throw new Error("Server ID is required");
			return enable
				? serversApi.enableServer(serverId, syncServerStateToClients)
				: serversApi.disableServer(serverId, syncServerStateToClients);
		},
		onSuccess: (_, enable) => {
			const titleKey = enable
				? "notifications.toggle.enabledTitle"
				: "notifications.toggle.disabledTitle";
			notifySuccess(
				t(titleKey, {
					defaultValue: enable ? "Server enabled" : "Server disabled",
				}),
			);
			queryClient.invalidateQueries({ queryKey: ["server", serverId] });
			queryClient.invalidateQueries({ queryKey: ["servers"] });
		},
		onError: (e, enable) => {
			const message = e instanceof Error ? e.message : String(e);
			const actionLabel = enable
				? t("notifications.toggle.enableAction", { defaultValue: "enable" })
				: t("notifications.toggle.disableAction", { defaultValue: "disable" });
			notifyError(
				t("notifications.genericError.title", {
					defaultValue: "Operation failed",
				}),
				t("notifications.toggle.error", {
					action: actionLabel,
					message,
					defaultValue: "Unable to {{action}} server: {{message}}",
				}),
			);
		},
	});

	const refreshCapabilitiesMutation = useMutation({
		mutationFn: async () => {
			if (!serverId) throw new Error("Server ID is required");
			const [tools, resources, prompts, templates] = await Promise.all([
				serversApi.listTools(serverId, "force"),
				serversApi.listResources(serverId, "force"),
				serversApi.listPrompts(serverId, "force"),
				serversApi.listResourceTemplates(serverId, "force"),
			]);
			return { tools, resources, prompts, templates };
		},
		onSuccess: async ({ tools, resources, prompts, templates }) => {
			const normalize = (response: {
				items: CapabilityRecord[] | undefined;
				meta?: unknown;
				state?: string;
			}) => ({
				items: Array.isArray(response.items) ? response.items : [],
				meta: response.meta,
				state: response.state,
			});

			queryClient.setQueryData(
				["server-cap", "tools", serverId],
				normalize({
					items: tools.items as CapabilityRecord[] | undefined,
					meta: tools.meta,
					state: tools.state,
				}),
			);
			queryClient.setQueryData(
				["server-cap", "resources", serverId],
				normalize({
					items: resources.items as CapabilityRecord[] | undefined,
					meta: resources.meta,
					state: resources.state,
				}),
			);
			queryClient.setQueryData(
				["server-cap", "prompts", serverId],
				normalize({
					items: prompts.items as CapabilityRecord[] | undefined,
					meta: prompts.meta,
					state: prompts.state,
				}),
			);
			queryClient.setQueryData(
				["server-cap", "templates", serverId],
				normalize({
					items: templates.items as CapabilityRecord[] | undefined,
					meta: templates.meta,
					state: templates.state,
				}),
			);

			await queryClient.invalidateQueries({ queryKey: ["server", serverId] });
		},
		onError: (error) => {
			const message =
				error instanceof Error
					? error.message
					: t("detail.notifications.refreshFailed.defaultMessage", {
						defaultValue: "Unknown error",
					});
			notifyError(
				t("detail.notifications.refreshFailed.title", {
					defaultValue: "Refresh failed",
				}),
				t("detail.notifications.refreshFailed.message", {
					message,
					defaultValue: "Unable to refresh server capabilities: {{message}}",
				}),
			);
		},
	});

	const isOverviewRefreshing =
		isRefetching || refreshCapabilitiesMutation.isPending;

	const deleteServerM = useMutation({
		mutationFn: async () => {
			if (!serverId) throw new Error("Server ID is required");
			return serversApi.deleteServer(serverId);
		},
		onSuccess: () => {
			notifySuccess(
				t("notifications.delete.title", { defaultValue: "Server deleted" }),
				t("notifications.delete.cleanupReview", {
					defaultValue:
						"Review Secure Store cleanup if this server used stored secrets.",
				}),
				"/secrets?lifecycle=unused",
			);
			queryClient.invalidateQueries({ queryKey: ["servers"] });
			queryClient.removeQueries({ queryKey: ["server", serverId] });
			navigate("/servers");
		},
		onError: (e) =>
			notifyError(
				t("notifications.delete.errorFallback", {
					defaultValue: "Error deleting server",
				}),
				String(e),
			),
	});

	const handleInspect = useCallback(
		async (
			kind: InspectorTarget["kind"],
			item: CapabilityRecord | null,
			capabilityOptionsByKind?: InspectorTarget["capabilityOptionsByKind"],
		) => {
			let detailItem = item;
			if (item && serverId) {
				const detailKind = toCapabilityPreviewKind(kind);
				const key = serverCapabilityDetailKey(item, detailKind);
				if (key) {
					try {
						const detail = await serversApi.getCapabilityDetail(
							serverId,
							detailKind,
							key,
						);
						detailItem = mergeCapabilityInspectorItem(
							item,
							(detail.item ?? null) as CapabilityRecord | null,
						);
					} catch (error) {
						notifyError(
							t("detail.inspector.messages.detailLoadFailed", {
								defaultValue: "Capability details failed to load",
							}),
							error instanceof Error ? error.message : String(error),
						);
						return;
					}
				}
			}
			setInspector({ kind, item: detailItem, capabilityOptionsByKind });
		},
		[serverId, t],
	);

	const handleServerLogsNextPage = () => {
		if (!serverLogsQuery.data?.next_cursor) return;
		const nextCursor = serverLogsQuery.data.next_cursor;
		setLogPageCursors((prev) => {
			const next = [...prev];
			next[logCurrentPageIndex + 1] = nextCursor;
			return next;
		});
		setLogCurrentPageIndex((prev) => prev + 1);
	};
	const handleServerLogsPrevPage = () => {
		if (logCurrentPageIndex > 0) {
			setLogCurrentPageIndex((prev) => prev - 1);
		}
	};
	const handleServerLogsFirstPage = () => {
		setLogCurrentPageIndex(0);
	};
	const handleServerLogsLastPage = async () => {
		if (!serverLogsQuery.data?.next_cursor || !serverId) return;
		setIsLogPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = serverLogsQuery.data.next_cursor;
			let targetPageIndex = logCurrentPageIndex;
			const nextPageCursors = [...logPageCursors];
			while (nextCursor) {
				targetPageIndex += 1;
				nextPageCursors[targetPageIndex] = nextCursor;
				const page = await auditApi.list({
					limit: logPageSize,
					cursor: nextCursor,
					server_id: serverId,
				});
				if (!page) {
					break;
				}
				nextCursor = page.next_cursor ?? undefined;
			}
			setLogPageCursors(nextPageCursors);
			setLogCurrentPageIndex(targetPageIndex);
		} finally {
			setIsLogPaginationActionLoading(false);
		}
	};

	const serverDisplayName = server ? getServerDisplayName(server) : serverId;
	const namespaceIssueStatusLabel = server?.namespace_issue
		? t(
				server.namespace_issue.code === "capability_collision" ||
					server.namespace_issue.conflicts?.length
					? "detail.namespaceIssue.statusConflict"
					: "detail.namespaceIssue.statusInvalid",
			)
		: undefined;
	const primaryIconSrc = server?.icons?.[0]?.src;
	const primaryIconAlt = primaryIconSrc
		? `${serverDisplayName} icon`
		: undefined;
	const serverCategory = (server?.meta as Record<string, unknown>)?.category as
    string | undefined;
	const serverScenario = (server?.meta as Record<string, unknown>)
		?.recommendedScenario as string | undefined;
	const capabilitySummary = server
		? (server.capability ?? readLegacyCapability(server))
		: undefined;
	const capabilityOverviewText = capabilitySummary
		? `Tools ${capabilitySummary.tools_count} | Prompts ${capabilitySummary.prompts_count} | Resources ${capabilitySummary.resources_count} | Templates ${capabilitySummary.resource_templates_count}`
		: undefined;
	const protocolVersion =
		server?.protocol_version ?? readLegacyString(server, "protocolVersion");
	const serverVersion =
		server?.server_info?.version ??
		server?.server_version ??
		readLegacyString(server, "serverVersion");
	const defaultTab = "overview";
	const validTabs = ["overview", "capabilities"];
  const { activeTab: capabilityTab, setActiveTab: setCapabilityTab } =
    useUrlTab({
		paramName: "tab",
		defaultTab,
		validTabs,
	});
	const [logFilter, setLogFilter] = useState("");
	const [logPageSize, setLogPageSize] = useState<number>(10);
	const [logPageCursors, setLogPageCursors] = useState<string[]>([]);
	const [logCurrentPageIndex, setLogCurrentPageIndex] = useState(0);
	const [isLogPaginationActionLoading, setIsLogPaginationActionLoading] =
		useState(false);
	const logCurrentCursor = logPageCursors[logCurrentPageIndex];
	const serverLogsQuery = useQuery({
		queryKey: [
			"server-audit-logs",
			serverId,
			logCurrentCursor,
			logPageSize,
			showServerLevelLogs,
		],
		queryFn: () =>
			auditApi.list({
				limit: logPageSize,
				cursor: logCurrentCursor,
				server_id: serverId,
			}),
		enabled: Boolean(serverId && showServerLevelLogs),
		refetchOnWindowFocus: false,
		retry: false,
	});
	useEffect(() => {
		setLogPageCursors([]);
		setLogCurrentPageIndex(0);
	}, [serverId, logPageSize]);
	const filteredServerLogs = useMemo(() => {
		const logs = serverLogsQuery.data?.events ?? [];
		const term = logFilter.trim().toLowerCase();
		if (!term) return logs;
		return logs.filter((event) => {
			const haystacks = [
				event.action,
				event.category,
				event.status,
				event.target,
				event.route,
				event.error_message,
				event.detail,
				event.request_id,
				event.mcp_method,
			]
				.filter(Boolean)
				.map((value) => String(value).toLowerCase());
			return haystacks.some((value) => value.includes(term));
		});
	}, [serverLogsQuery.data?.events, logFilter]);
	const serverEnabled = Boolean(server?.enabled ?? server?.globally_enabled);
	const runtimeStatus = server?.status ?? (serverEnabled ? "idle" : "disabled");
	const liveOAuthStatus = oauthStatusQuery.data ?? null;
	const authReadiness = (() => {
		if (!server?.auth_mode) return null;
		if (isOAuthServer && liveOAuthStatus) {
			return resolveOAuthReadiness({
				secretStoreStatus: secretStoreStatusQuery.data,
				oauthStatus: liveOAuthStatus,
			});
		}
		return resolveServerOAuthReadiness(server);
	})();
	const authBadgeOAuthStatus =
		isOAuthServer && liveOAuthStatus
			? liveOAuthStatus.state
			: server?.oauth_status;
	const handleAuthAction = useCallback(() => {
		if (getOAuthReadinessActionTarget(authReadiness) === "security-settings") {
			navigate("/settings?tab=security");
			return;
		}
		setIsEditOpen(true);
	}, [authReadiness, navigate]);
	const overviewActionButtonClass =
		"gap-2 rounded-none first:rounded-l-md last:rounded-r-md";
	const isServerPending = !server && (isLoading || !isFetched || isRefetching);

	if (!serverId) {
		return (
			<div className="p-4">
				{t("detail.errors.noServerId", {
					defaultValue: "No server ID provided",
				})}
			</div>
		);
	}

	if (isServerPending) {
		return (
			<div className="space-y-4">
				<Card>
					<CardContent className="flex min-h-[240px] flex-col items-center justify-center gap-3 p-6 text-center">
						<Loader2 className="h-8 w-8 animate-spin text-slate-400" />
						<div className="space-y-1">
							<p className="text-sm font-medium text-slate-900 dark:text-slate-100">
								{t("detail.loading.title", {
									defaultValue: "Loading server details",
								})}
							</p>
							<p className="text-sm text-slate-500 dark:text-slate-400">
								{t("detail.loading.description", {
									defaultValue:
										"The service is responding, but its detail snapshot is still warming up.",
								})}
							</p>
						</div>
					</CardContent>
				</Card>
			</div>
		);
	}

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<div className="flex shrink-0 flex-col gap-2 md:flex-row md:items-center md:justify-between">
				<div className="flex items-center gap-3">
					<h2 className="text-3xl font-bold tracking-tight">
						{serverDisplayName}
					</h2>
					{server?.namespace_issue ? (
						<StatusBadge
							status="pending"
							statusLabel={namespaceIssueStatusLabel}
							isServerEnabled={false}
						/>
					) : server ? (
						<StatusBadge
							status={runtimeStatus}
							instances={server.instances || []}
							isServerEnabled={serverEnabled}
						/>
					) : null}
				</div>
			</div>

			{server && (
				<>
					<ServerEditDrawer
						server={server}
						isOpen={isEditOpen}
						onClose={() => setIsEditOpen(false)}
						onSubmit={async (data) => {
							await serversApi.updateServer(serverId, data);
							queryClient.invalidateQueries({ queryKey: ["server", serverId] });
							queryClient.invalidateQueries({ queryKey: ["servers"] });
						}}
					/>

					<AlertDialog open={isDeleteOpen} onOpenChange={setIsDeleteOpen}>
						<AlertDialogContent>
							<AlertDialogHeader>
								<AlertDialogTitle>
									{t("detail.deleteDialog.title", {
										defaultValue: "Delete Server",
									})}
								</AlertDialogTitle>
								<AlertDialogDescription>
									{t("detail.deleteDialog.description", {
										defaultValue: "This action cannot be undone.",
									})}
								</AlertDialogDescription>
							</AlertDialogHeader>
							<AlertDialogFooter>
								<AlertDialogCancel>
									{t("detail.deleteDialog.cancel", { defaultValue: "Cancel" })}
								</AlertDialogCancel>
								<AlertDialogAction
									onClick={() => deleteServerM.mutate()}
									disabled={deleteServerM.isPending}
									className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
								>
									{deleteServerM.isPending
										? t("detail.deleteDialog.pending", {
											defaultValue: "Deleting...",
										})
										: t("detail.deleteDialog.confirm", {
											defaultValue: "Delete",
										})}
								</AlertDialogAction>
							</AlertDialogFooter>
						</AlertDialogContent>
					</AlertDialog>
				</>
			)}

			{server && (
				<Tabs
					value={capabilityTab}
					onValueChange={setCapabilityTab}
					className="flex min-h-0 flex-1 flex-col gap-4"
				>
					<div className="flex shrink-0 flex-wrap items-center justify-between gap-2">
						<ServerCapabilityTabsHeader serverId={serverId} />
						<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
							<Button
								size="sm"
								variant="outline"
								onClick={() => {
									refreshCapabilitiesMutation.mutate();
								}}
								disabled={isOverviewRefreshing}
								className={overviewActionButtonClass}
							>
								<RefreshCw
									className={`h-4 w-4 ${isOverviewRefreshing ? "animate-spin" : ""}`}
								/>
								{t("detail.actions.refresh", {
									defaultValue: "Refresh",
								})}
							</Button>
							<Button
								size="sm"
								variant={server.namespace_issue ? "warning" : "outline"}
								onClick={() => setIsEditOpen(true)}
								className={overviewActionButtonClass}
							>
								{server.namespace_issue ? (
									<Wrench className="h-4 w-4" />
								) : (
									<Edit3 className="h-4 w-4" />
								)}
								{server.namespace_issue
									? t("detail.namespaceIssue.action")
									: t("detail.actions.edit", {
											defaultValue: "Edit",
										})}
							</Button>
						</ButtonGroup>
					</div>

					<TabsContent
						value="overview"
						className="mt-0 flex min-h-0 flex-1 flex-col overflow-y-auto data-[state=inactive]:hidden"
					>
							{isLoading ? (
								<Card>
									<CardContent className="p-4">
										<div className="h-24 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
									</CardContent>
								</Card>
							) : (
								<div className="grid gap-4">
									<Card>
										<CardContent className="p-4">
											<div className="flex flex-col gap-4">
												<div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
													<div className="flex flex-wrap items-start gap-4">
														<CachedAvatar
															src={primaryIconSrc}
															alt={primaryIconAlt}
															fallback={serverDisplayName || "?"}
															className="text-sm"
														/>
                          <div
                            className={`min-w-0 flex-1 ${overviewMetadataGridClass}`}
															>
												<OverviewMetadataRow
													label={t("detail.overview.labels.upstreamName")}
															>
													{server.server_info?.name?.trim() || "—"}
												</OverviewMetadataRow>
										<OverviewMetadataRow
											label={t("detail.overview.labels.namespace")}
													>
									{server.namespace_issue ? (
										<Button
											type="button"
											variant="ghost"
											onClick={() => setIsEditOpen(true)}
											className="h-auto p-0 font-normal text-inherit hover:bg-transparent hover:text-inherit"
										>
											{server.name}
											<AlertTriangle
												className="ml-2 h-4 w-4 text-destructive"
												aria-label={t("detail.namespaceIssue.iconLabel")}
											/>
										</Button>
									) : (
										server.name
									)}
										</OverviewMetadataRow>
														<OverviewMetadataRow
																label={t("detail.overview.labels.type", {
																	defaultValue: "Type",
																})}
															>
																{server.server_type}
															</OverviewMetadataRow>
															{server.auth_mode ? (
																<OverviewMetadataRow
																	label={t("detail.overview.labels.auth", {
																		defaultValue: "Auth",
																	})}
																>
																	<ServerAuthBadge
																		authMode={server.auth_mode}
																		oauthStatus={authBadgeOAuthStatus}
																		readiness={authReadiness}
																		onAction={handleAuthAction}
																	/>
																</OverviewMetadataRow>
															) : null}
															{protocolVersion ? (
																<OverviewMetadataRow
																	label={t("detail.overview.labels.protocol", {
																		defaultValue: "Protocol",
																	})}
																>
													{protocolVersion}
																</OverviewMetadataRow>
															) : null}
															{serverVersion ? (
																<OverviewMetadataRow
																	label={t("detail.overview.labels.version", {
																		defaultValue: "Version",
																	})}
																>
													{serverVersion}
																</OverviewMetadataRow>
															) : null}
															{capabilityOverviewText ? (
																<OverviewMetadataRow
                                label={t(
                                  "detail.overview.labels.capabilities",
                                  {
																		defaultValue: "Capabilities",
                                  },
                                )}
																	multiline
																>
																	<span className="text-slate-600 dark:text-slate-300">
																		{capabilityOverviewText}
																	</span>
																</OverviewMetadataRow>
															) : null}
															{serverCategory ? (
																<OverviewMetadataRow
																	label={t("detail.overview.labels.category", {
																		defaultValue: "Category",
																	})}
																>
																	<span className="text-slate-600 dark:text-slate-300">
																		{serverCategory}
																	</span>
																</OverviewMetadataRow>
															) : null}
															{serverScenario ? (
																<OverviewMetadataRow
																	label={t("detail.overview.labels.scenario", {
																		defaultValue: "Scenario",
																	})}
																>
																	<span className="text-slate-600 dark:text-slate-300">
																		{serverScenario}
																	</span>
																</OverviewMetadataRow>
															) : null}
															{server.command ? (
																<OverviewMetadataRow
																	label={t("detail.overview.labels.command", {
																		defaultValue: "Command",
																	})}
																	multiline
																>
															<span className="break-all">
																		{server.command}
																	</span>
																</OverviewMetadataRow>
															) : null}
															<OverviewMetadataRow
																label={t("detail.overview.labels.repository", {
																	defaultValue: "Repository",
																})}
															>
																	—
															</OverviewMetadataRow>
														</div>
													</div>
													<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
														<Button
															size="sm"
															variant="outline"
															onClick={() => toggleServerM.mutate(!serverEnabled)}
															disabled={toggleServerM.isPending}
															className={overviewActionButtonClass}
														>
															{serverEnabled ? (
																<>
																	<PowerOff className="h-4 w-4" />
																	{t("detail.actions.disable", {
																		defaultValue: "Disable",
																	})}
																</>
															) : (
																<>
																	<Power className="h-4 w-4" />
																	{t("detail.actions.enable", {
																		defaultValue: "Enable",
																	})}
																</>
															)}
														</Button>
														<Button
															size="sm"
															variant="destructive"
															onClick={() => setIsDeleteOpen(true)}
															disabled={deleteServerM.isPending}
															className={overviewActionButtonClass}
														>
															<Trash2 className="h-4 w-4" />
															{t("detail.actions.delete", {
																defaultValue: "Delete",
															})}
														</Button>
													</ButtonGroup>
												</div>
											</div>
										</CardContent>
									</Card>

									<Card>
										<CardHeader>
											<CardTitle>
												{t("detail.instances.title", {
													count: server.instances?.length || 0,
													defaultValue: "Instances ({{count}})",
												})}
											</CardTitle>
										</CardHeader>
										<CardContent>
											{server.instances?.length ? (
												<CapsuleStripeList>
													{server.instances.map((i) => (
														<CapsuleStripeListItem
															key={i.id}
															interactive
															onClick={() =>
																navigate(
																	`/servers/${encodeURIComponent(serverId)}/instances/${encodeURIComponent(i.id)}`,
																)
															}
														>
															<div className="font-mono truncate">{i.id}</div>
															<StatusBadge
																status={i.status}
																className="text-xs"
															/>
														</CapsuleStripeListItem>
													))}
												</CapsuleStripeList>
											) : (
												<div className="text-slate-500">
													{t("detail.instances.empty", {
														defaultValue: "No instances.",
													})}
												</div>
											)}
										</CardContent>
									</Card>
									{showServerLevelLogs ? (
										<AuditLogsPanel
											title={t("detail.logs.title", { defaultValue: "Logs" })}
											description={t("detail.logs.description", {
												defaultValue:
													"Runtime and activity logs related to this server.",
											})}
											searchPlaceholder={t("detail.logs.searchPlaceholder", {
												defaultValue: "Search logs...",
											})}
											refreshLabel={t("detail.logs.refresh", {
												defaultValue: "Refresh Logs",
											})}
											loadingLabel={t("detail.logs.loading", {
												defaultValue: "Loading logs...",
											})}
											emptyLabel={t("detail.logs.empty", {
												defaultValue:
													"No log entries recorded for this server yet.",
											})}
											headers={{
												timestamp: t("detail.logs.headers.timestamp", {
													defaultValue: "Timestamp",
												}),
												action: t("detail.logs.headers.action", {
													defaultValue: "Action",
												}),
												category: t("detail.logs.headers.category", {
													defaultValue: "Category",
												}),
												status: t("detail.logs.headers.status", {
													defaultValue: "Status",
												}),
												target: t("detail.logs.headers.target", {
													defaultValue: "Target",
												}),
											}}
											searchValue={logFilter}
											onSearchChange={setLogFilter}
											onRefresh={() => void serverLogsQuery.refetch()}
											rows={filteredServerLogs}
											isLoading={serverLogsQuery.isLoading}
											isFetching={serverLogsQuery.isFetching}
											isPaginationActionLoading={isLogPaginationActionLoading}
											currentPage={logCurrentPageIndex + 1}
											hasPreviousPage={logCurrentPageIndex > 0}
											hasNextPage={Boolean(serverLogsQuery.data?.next_cursor)}
											itemsPerPage={logPageSize}
											onItemsPerPageChange={setLogPageSize}
											onPreviousPage={handleServerLogsPrevPage}
											onFirstPage={handleServerLogsFirstPage}
											onNextPage={handleServerLogsNextPage}
											onLastPage={() => void handleServerLogsLastPage()}
                    expandLabel={t("detail.logs.expand", {
                      defaultValue: "Expand Logs",
                    })}
                    collapseLabel={t("detail.logs.collapse", {
                      defaultValue: "Collapse Logs",
                    })}
										/>
									) : null}
								</div>
							)}
					</TabsContent>

          <TabsContent
            value="capabilities"
            className={DETAIL_TAB_CONTENT_CLASS}
          >
						<ServerCapabilitiesPanel
							serverId={serverId}
							enableInspect={enableServerDebug}
							onInspect={(kind, item, capabilityOptions) =>
								handleInspect(kind, item, capabilityOptions)
							}
						/>
					</TabsContent>
				</Tabs>
			)}
			<InspectorDrawer
				open={!!inspector}
				onOpenChange={(open) => {
					if (!open) setInspector(null);
				}}
				serverId={serverId}
				serverName={server?.name}
				kind={inspector?.kind ?? "tool"}
				item={inspector?.item ?? null}
				capabilityOptionsByKind={inspector?.capabilityOptionsByKind}
			/>
		</div>
	);
}

function ServerCapabilityTabsHeader({ serverId }: { serverId: string }) {
	const { t } = useTranslation("servers");
	const toolsQ = useQuery({
		queryKey: ["server-cap", "tools", serverId],
		queryFn: () => serversApi.listTools(serverId),
	});
	const resQ = useQuery({
		queryKey: ["server-cap", "resources", serverId],
		queryFn: () => serversApi.listResources(serverId),
	});
	const prmQ = useQuery({
		queryKey: ["server-cap", "prompts", serverId],
		queryFn: () => serversApi.listPrompts(serverId),
	});
	const tmpQ = useQuery({
		queryKey: ["server-cap", "templates", serverId],
		queryFn: () => serversApi.listResourceTemplates(serverId),
	});

	const toolsCount = toolsQ.data?.items?.length ?? 0;
	const resourcesCount = resQ.data?.items?.length ?? 0;
	const promptsCount = prmQ.data?.items?.length ?? 0;
	const templatesCount = tmpQ.data?.items?.length ?? 0;
	const totalCount =
		toolsCount + resourcesCount + promptsCount + templatesCount;
	return (
		<TabsList className="flex flex-wrap gap-2">
			<TabsTrigger value="overview">
				{t("detail.tabs.overview", { defaultValue: "Overview" })}
			</TabsTrigger>
			<TabsTrigger value="capabilities">
				{t("detail.tabs.capabilities", {
					count: totalCount,
					defaultValue: "Capabilities ({{count}})",
				})}
			</TabsTrigger>
		</TabsList>
	);
}

function ServerCapabilitiesPanel({
	serverId,
	enableInspect = false,
	onInspect,
	}: {
		serverId: string;
		enableInspect?: boolean;
		onInspect: (
			kind: InspectorTarget["kind"],
			item: CapabilityRecord | null,
			capabilityOptionsByKind?: InspectorTarget["capabilityOptionsByKind"],
		) => void | Promise<void>;
	}) {
	const [search, setSearch] = useState("");
	const [kindFilters, setKindFilters] = useState<CapabilityPreviewKind[]>([]);
	const { t } = useTranslation("servers");
	const capabilityQueryOptions = {
		staleTime: 0,
		refetchOnMount: "always" as const,
	};
	const toolsQ = useQuery<CapabilityListResponse>({
		queryKey: ["server-cap", "tools", serverId],
		queryFn: async () => {
			const response = await serversApi.listTools(serverId);
			return normalizeCapabilityListResponse(response);
		},
		...capabilityQueryOptions,
	});
	const resourcesQ = useQuery<CapabilityListResponse>({
		queryKey: ["server-cap", "resources", serverId],
		queryFn: async () => {
			const response = await serversApi.listResources(serverId);
			return normalizeCapabilityListResponse(response);
		},
		...capabilityQueryOptions,
	});
	const promptsQ = useQuery<CapabilityListResponse>({
		queryKey: ["server-cap", "prompts", serverId],
		queryFn: async () => {
			const response = await serversApi.listPrompts(serverId);
			return normalizeCapabilityListResponse(response);
		},
		...capabilityQueryOptions,
	});
	const templatesQ = useQuery<CapabilityListResponse>({
		queryKey: ["server-cap", "templates", serverId],
		queryFn: async () => {
			const response = await serversApi.listResourceTemplates(serverId);
			return normalizeCapabilityListResponse(response);
		},
		...capabilityQueryOptions,
	});
	const kindFilterOptions = useMemo(
		() =>
			SERVER_CAPABILITY_KINDS.map((kind) => ({
				value: kind,
				label: capabilityKindLabel(kind, t),
			})),
		[t],
	);
	const kindFilterLabel = useMemo(() => {
		if (kindFilters.length === 0) {
			return t("detail.filters.kind.all", { defaultValue: "All Types" });
		}
		if (kindFilters.length === 1) {
			const [kind] = kindFilters;
			return capabilityKindLabel(kind, t);
		}
		return t("detail.filters.kind.selected", {
			count: kindFilters.length,
			defaultValue: "{{count}} Types",
		});
	}, [kindFilters, t]);
	const kindMatches = (kind: CapabilityPreviewKind) =>
		kindFilters.length === 0 || kindFilters.includes(kind);
	const toggleKindFilter = (kind: CapabilityPreviewKind, checked: boolean) => {
		setKindFilters((current) => {
			if (checked) {
				return current.includes(kind) ? current : [...current, kind];
			}
			return current.filter((value) => value !== kind);
		});
	};
	const resolveVisibleItems = (
		kind: CapabilityPreviewKind,
		cachedItems: CapabilityRecord[] | undefined,
	): CapabilityRecord[] => {
		if (!kindMatches(kind)) return [];
		return cachedItems ?? [];
	};
	const renderInspectAction = (
		_mapped: unknown,
		item: ServerFlatCapabilityItem,
	): ReactNode => {
		const inspectorKind = toInspectorKind(item.__serverCapabilityKind);
		const capabilityOptionsByKind: InspectorTarget["capabilityOptionsByKind"] =
			collectLoadedInspectorOptions({
				tool: toolsQ.data?.items,
				resource: resourcesQ.data?.items,
				prompt: promptsQ.data?.items,
				template: templatesQ.data?.items,
			});

		return (
			<Button
				type="button"
				size="sm"
				variant="outline"
				className="gap-1"
				onClick={() =>
					void onInspect(inspectorKind, item, capabilityOptionsByKind)
				}
			>
				<Play className="h-3.5 w-3.5" />
				{t("detail.inspector.actions.inspect", {
					defaultValue: "Inspect",
				})}
			</Button>
		);
	};
	const showInspectActions = enableInspect;
	const renderAction = showInspectActions ? renderInspectAction : undefined;
	const anyCapabilitiesLoading =
		toolsQ.isLoading ||
		resourcesQ.isLoading ||
		promptsQ.isLoading ||
		templatesQ.isLoading;
	const hasLoadedCapabilityItems = Boolean(
		toolsQ.data?.items.length ||
			resourcesQ.data?.items.length ||
			promptsQ.data?.items.length ||
			templatesQ.data?.items.length,
	);
	const initialCapabilitiesLoading =
		anyCapabilitiesLoading && !hasLoadedCapabilityItems;
	const tools = resolveVisibleItems("tools", toolsQ.data?.items);
	const resources = resolveVisibleItems("resources", resourcesQ.data?.items);
	const prompts = resolveVisibleItems("prompts", promptsQ.data?.items);
	const templates = resolveVisibleItems("templates", templatesQ.data?.items);
	const emptyText = t("detail.capabilityList.emptyAll", {
		defaultValue: "No capabilities from this server",
	});
	const loadServerCapabilityDetails = useCallback(
		async (
			item: ServerFlatCapabilityItem,
			kind: CapabilityPreviewKind,
		): Promise<CapabilityRecord | null> => {
			const key = serverCapabilityDetailKey(item, kind);
			if (!key) {
				return null;
			}
			const detail = await serversApi.getCapabilityDetail(serverId, kind, key);
			return (detail.item ?? null) as CapabilityRecord | null;
		},
		[serverId],
	);
	const renderServerFlatCapabilityList = (
		items: CapabilityPreviewFlatItem[],
	): ReactNode => {
    const flatItems: ServerFlatCapabilityItem[] = items.map(
      ({ kind, item }) => ({
			...item,
			__serverCapabilityKind: kind,
      }),
    );

		return (
			<CapabilityList<ServerFlatCapabilityItem>
				asCard={false}
				kind="tools"
				getKind={(item) => item.__serverCapabilityKind}
				context="server"
				leadingIcon="kind"
				items={flatItems}
				hoverActions={showInspectActions}
				clickToToggleDetails
				scrollContainedBody
				loadDetails={loadServerCapabilityDetails}
				detailsCacheScope={serverId}
				getId={serverCapabilityItemId}
				renderAction={renderAction}
				emptyText={t("detail.capabilityList.emptyAll", {
					defaultValue: "No capabilities from this server",
				})}
			/>
		);
	};
	const toolbar = (
		<CapabilityToolbar
			searchValue={search}
			onSearchChange={setSearch}
			searchPlaceholder={t("wizard.preview.filterCapabilities", {
				defaultValue: "Filter capabilities...",
			})}
			kindFilter={{
				label: kindFilterLabel,
				allLabel: t("detail.filters.kind.all", { defaultValue: "All Types" }),
				options: kindFilterOptions,
				selectedValues: kindFilters,
				onClear: () => setKindFilters([]),
				onToggle: (value, checked) =>
					toggleKindFilter(value as CapabilityPreviewKind, checked),
			}}
		/>
	);

	return (
		<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
			<CardContent className="flex min-h-0 flex-1 flex-col p-4">
				<div className="shrink-0 pb-3">{toolbar}</div>
				<CapabilityPreviewList
					className="min-h-0 flex-1"
					contentClassName="flex min-h-0 flex-1 flex-col p-0"
					framed={false}
					showHeader={false}
					tools={tools}
					resources={resources}
					prompts={prompts}
					templates={templates}
					isLoading={initialCapabilitiesLoading}
					searchValue={search}
					emptyText={emptyText}
					renderFlatList={renderServerFlatCapabilityList}
				/>
			</CardContent>
		</Card>
	);
}

export default ServerDetailPage;
