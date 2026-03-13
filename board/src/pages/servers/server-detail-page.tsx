import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	AlertTriangle,
	Bug,
	Copy,
	Edit3,
	Eye,
	Loader2,
	Play,
	Power,
	PowerOff,
	RefreshCw,
	ShieldAlert,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import CapabilityList from "../../components/capability-list";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import InspectorDrawer, {
	type InspectorLogEntry,
} from "../../components/inspector-drawer";
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
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
} from "../../components/ui/avatar";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ButtonGroup } from "../../components/ui/button-group";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { configSuitsApi, inspectorApi, serversApi } from "../../lib/api";
import { writeClipboardText } from "../../lib/clipboard";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import { maskHeaderValue, sanitizeRecord } from "../../lib/security";
import { useAppStore } from "../../lib/store";
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

interface InspectorListResponse {
	success?: boolean;
	data?: {
		tools?: CapabilityRecord[];
		resources?: CapabilityRecord[];
		prompts?: CapabilityRecord[];
		items?: CapabilityRecord[];
		meta?: unknown;
		state?: string;
	} | null;
	error?: unknown;
}

interface InspectorListParams {
	server_id: string;
	server_name?: string;
	mode: InspectorChannel;
	refresh: boolean;
}

interface CapabilityListResponse {
	items: CapabilityRecord[];
	meta?: unknown;
	state?: string;
}

const VIEW_MODES = {
	browse: "browse" as const,
	debug: "debug" as const,
};

type InspectorChannel = "proxy" | "native";
type DebugKind = "tools" | "resources" | "prompts";

type InspectorTarget = {
	kind: "tool" | "resource" | "prompt" | "template";
	item: CapabilityRecord | null;
};

function getInitialInspectorChannel(): InspectorChannel {
	try {
		if (typeof window !== "undefined") {
			const saved = window.localStorage.getItem("mcp_inspector_channel");
			if (saved === "proxy" || saved === "native") {
				return saved;
			}
		}
	} catch {
		/* noop */
	}
	return "native";
}

interface DebugState {
	items: CapabilityRecord[];
	loading: boolean;
	error: string | null;
	fetched: boolean;
	lastFetched?: number;
}

const createDebugState = (): DebugState => ({
	items: [],
	loading: false,
	error: null,
	fetched: false,
});

function makeLogId() {
	if (
		typeof crypto !== "undefined" &&
		typeof crypto.randomUUID === "function"
	) {
		return crypto.randomUUID();
	}
	return `log_${Date.now()}_${Math.random().toString(16).slice(2, 8)}`;
}

export function ServerDetailPage() {
	usePageTranslations("servers");
	const { t } = useTranslation("servers");
	const { serverId } = useParams<{ serverId: string }>();
	const navigate = useNavigate();
	const location = useLocation();
	const queryClient = useQueryClient();
	const viewMode = useAppStore((state) => state.inspectorViewMode);
	const setViewMode = useAppStore((state) => state.setInspectorViewMode);
	const enableServerDebug = useAppStore(
		(state) => state.dashboardSettings.enableServerDebug,
	);

	const initialChannel = useMemo(() => getInitialInspectorChannel(), []);
	const [requestedChannel, setRequestedChannel] =
		useState<InspectorChannel>(initialChannel);
	const [channel, setChannel] = useState<InspectorChannel>(initialChannel);
	const ignoreLocationChange = useRef(false);
	const isUserAction = useRef(false);
	const debounceTimer = useRef<NodeJS.Timeout | null>(null);
	const [isEditOpen, setIsEditOpen] = useState(false);
	const [isDeleteOpen, setIsDeleteOpen] = useState(false);
	const [inspector, setInspector] = useState<InspectorTarget | null>(null);
	const [logs, setLogs] = useState<InspectorLogEntry[]>([]);
	const [debugData, setDebugData] = useState<
		Record<DebugKind | "templates", DebugState>
	>({
		tools: createDebugState(),
		resources: createDebugState(),
		prompts: createDebugState(),
		templates: createDebugState(),
	});

	const {
		data: server,
		isLoading,
		isRefetching,
	} = useQuery({
		queryKey: ["server", serverId],
		queryFn: () => serversApi.getServer(serverId || ""),
		enabled: !!serverId,
	});

	const toggleServerM = useMutation({
		mutationFn: async (enable: boolean) => {
			if (!serverId) throw new Error("Server ID is required");
			return enable
				? serversApi.enableServer(serverId)
				: serversApi.disableServer(serverId);
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

	const activeProfilesQ = useQuery({
		queryKey: ["inspector-proxy-profiles", serverId],
		enabled:
			!!serverId &&
			viewMode === VIEW_MODES.debug &&
			requestedChannel === "proxy",
		queryFn: async () => {
			const suitsResp = await configSuitsApi.getAll();
			const active = suitsResp.suits.filter((s) => s.is_active);
			const enabled: string[] = [];
			await Promise.all(
				active.map(async (suit) => {
					try {
						const res = await configSuitsApi.getServers(suit.id);
						const match = (res.servers || []).find(
							(srv) => srv.id === serverId && srv.enabled,
						);
						if (match) enabled.push(suit.name || suit.id);
					} catch (error) {
						console.error("Failed to load servers for suit", suit.id, error);
					}
				}),
			);
			return {
				activeNames: active.map((s) => s.name || s.id),
				enabledNames: enabled,
			};
		},
	});

	const rawProxyAvailable =
		(activeProfilesQ.data?.enabledNames?.length ?? 0) > 0;
	const isProxyChecking =
		viewMode === VIEW_MODES.debug &&
		activeProfilesQ.isFetching &&
		!activeProfilesQ.isFetched;
	const [proxyAvailable, setProxyAvailable] = useState(rawProxyAvailable);

	useEffect(() => {
		const timer = setTimeout(() => {
			setProxyAvailable(rawProxyAvailable);
		}, 180);
		return () => clearTimeout(timer);
	}, [rawProxyAvailable]);

	useEffect(() => {
		if (ignoreLocationChange.current) {
			ignoreLocationChange.current = false;
			return;
		}

		// Skip if this is a user action to prevent loops
		if (isUserAction.current) {
			isUserAction.current = false;
			return;
		}

		const params = new URLSearchParams(location.search);
		const viewParam = params.get("view");

		// Only update viewMode if URL parameter is different and valid
		if (viewParam === VIEW_MODES.debug || viewParam === VIEW_MODES.browse) {
			if (viewParam !== viewMode) {
				setViewMode(viewParam);
			}
		} else if (viewParam === null && viewMode !== VIEW_MODES.browse) {
			// Only set to browse if there's no view parameter and current mode is not browse
			setViewMode(VIEW_MODES.browse);
		}

		const channelParam = params.get("channel");
		if (channelParam === "proxy" || channelParam === "native") {
			if (channelParam !== requestedChannel) {
				setRequestedChannel(channelParam);
			}
		} else if (requestedChannel !== "native") {
			setRequestedChannel("native");
		}
	}, [location.search, setViewMode, viewMode, requestedChannel]);

	useEffect(() => {
		if (viewMode !== VIEW_MODES.debug) {
			if (channel !== "native") {
				setChannel("native");
			}
			return;
		}

		// Skip channel adjustment if this is a user action to prevent loops
		if (isUserAction.current) {
			return;
		}

		const desiredChannel: InspectorChannel =
			requestedChannel === "proxy"
				? proxyAvailable
					? "proxy"
					: "native"
				: "native";

		if (channel !== desiredChannel) {
			setChannel(desiredChannel);
		}
	}, [channel, proxyAvailable, requestedChannel, viewMode]);

	useEffect(() => {
		// Skip if we're currently ignoring location changes to prevent loops
		if (ignoreLocationChange.current) {
			return;
		}

		// Clear any existing debounce timer
		if (debounceTimer.current) {
			clearTimeout(debounceTimer.current);
		}

		// Debounce the URL update to prevent rapid changes
		debounceTimer.current = setTimeout(() => {
			const params = new URLSearchParams(location.search);
			let changed = false;

			// Only update URL if the current URL doesn't match the state
			const currentView = params.get("view");
			if (currentView !== viewMode) {
				params.set("view", viewMode);
				changed = true;
			}

			const currentChannel = params.get("channel");
			if (currentChannel !== requestedChannel) {
				params.set("channel", requestedChannel);
				changed = true;
			}

			if (changed) {
				ignoreLocationChange.current = true;
				navigate(
					{ pathname: location.pathname, search: params.toString() },
					{ replace: true },
				);
			}
		}, 50); // 50ms debounce

		// Cleanup function
		return () => {
			if (debounceTimer.current) {
				clearTimeout(debounceTimer.current);
			}
		};
	}, [
		viewMode,
		requestedChannel,
		location.pathname,
		location.search,
		navigate,
	]);

	useEffect(() => {
		try {
			if (typeof window !== "undefined") {
				window.localStorage.setItem("mcp_inspector_channel", requestedChannel);
			}
		} catch {
			/* noop */
		}
	}, [requestedChannel]);

	const pushLog = useCallback((entry: InspectorLogEntry) => {
		setLogs((prev) => {
			const next = [...prev, entry];
			if (next.length > 200) next.shift();
			return next;
		});
	}, []);

	const clearLogsByPrefix = useCallback((prefix: string) => {
		setLogs((prev) => prev.filter((entry) => !entry.method.startsWith(prefix)));
	}, []);

	const updateDebugState = useCallback(
		(kind: DebugKind | "templates", patch: Partial<DebugState>) => {
			setDebugData((prev) => ({
				...prev,
				[kind]: {
					...prev[kind],
					...patch,
				},
			}));
		},
		[],
	);

	const runList = useCallback(
		async (kind: DebugKind | "templates") => {
			if (!serverId) return;
			if (channel === "proxy" && !proxyAvailable) {
				updateDebugState(kind, {
					error: t("detail.debug.proxyUnavailable", {
						defaultValue:
							"Proxy mode unavailable: server not enabled in any active profile.",
					}),
				});
				return;
			}
			const method =
				kind === "tools"
					? "tools/list"
					: kind === "resources"
						? "resources/list"
						: kind === "prompts"
							? "prompts/list"
							: "templates/list";
			const requestPayload: InspectorListParams = {
				server_id: serverId,
				mode: channel,
				refresh: true,
			};
			if (server?.name) requestPayload.server_name = server.name;

			updateDebugState(kind, { loading: true, error: null });
			pushLog({
				id: makeLogId(),
				timestamp: Date.now(),
				channel: "inspector",
				event: "request",
				method,
				mode: channel,
				payload: requestPayload,
			});

			try {
				let resp: InspectorListResponse | undefined;
				if (kind === "tools") {
					resp = (await inspectorApi.toolsList(
						requestPayload,
					)) as InspectorListResponse;
				} else if (kind === "resources") {
					resp = (await inspectorApi.resourcesList(
						requestPayload,
					)) as InspectorListResponse;
				} else if (kind === "prompts") {
					resp = (await inspectorApi.promptsList(
						requestPayload,
					)) as InspectorListResponse;
				} else {
					resp = (await inspectorApi.templatesList(
						requestPayload,
					)) as InspectorListResponse;
				}
				const data = resp?.data ?? {};
				const list: CapabilityRecord[] = Array.isArray(data.items)
					? data.items
					: Array.isArray(data.tools)
						? data.tools
						: Array.isArray(data.resources)
							? data.resources
							: Array.isArray(data.prompts)
								? data.prompts
								: Array.isArray((data as any).templates)
									? (data as any).templates
									: [];
				updateDebugState(kind, {
					loading: false,
					error: null,
					fetched: true,
					items: list,
					lastFetched: Date.now(),
				});
				pushLog({
					id: makeLogId(),
					timestamp: Date.now(),
					channel: "inspector",
					event: "success",
					method,
					mode: channel,
					payload: { count: list.length },
				});
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				updateDebugState(kind, { loading: false, error: message });
				pushLog({
					id: makeLogId(),
					timestamp: Date.now(),
					channel: "inspector",
					event: "error",
					method,
					mode: channel,
					message,
					payload: error,
				});
			}
		},
		[
			channel,
			proxyAvailable,
			pushLog,
			server?.name,
			serverId,
			updateDebugState,
			t,
		],
	);

	const handleInspect = useCallback(
		(kind: InspectorTarget["kind"], item: CapabilityRecord | null) => {
			setInspector({ kind, item });
		},
		[],
	);

	const serverDisplayName = server?.name || serverId;
	const primaryIconSrc = server?.icons?.[0]?.src;
	const primaryIconAlt = primaryIconSrc
		? `${serverDisplayName} icon`
		: undefined;
	const serverDescription = server?.meta?.description?.trim();
	const serverCategory = (server?.meta as Record<string, unknown>)?.category as
		| string
		| undefined;
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
		server?.server_version ?? readLegacyString(server, "serverVersion");
	const defaultTab = viewMode === VIEW_MODES.debug ? "tools" : "overview";
	const serverEnabled = Boolean(server?.enabled ?? server?.globally_enabled);
	const runtimeStatus = server?.status ?? (serverEnabled ? "idle" : "disabled");
	const showDefaultHeaders = useAppStore(
		(state) => state.dashboardSettings.showDefaultHeaders,
	);
	const redactedHeaders = useMemo(() => {
		const src = sanitizeRecord(server?.headers);
		if (!src) return undefined;
		const out: Record<string, string> = {};
		for (const [k, v] of Object.entries(src)) out[k] = maskHeaderValue(k, v);
		return out;
	}, [server?.headers]);

	if (!serverId) {
		return (
			<div className="p-4">
				{t("detail.errors.noServerId", {
					defaultValue: "No server ID provided",
				})}
			</div>
		);
	}

	return (
		<div className="space-y-4">
			<div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
				<div className="flex items-center gap-3">
					<h2 className="text-3xl font-bold tracking-tight">
						{serverDisplayName}
					</h2>
					{server ? (
						<StatusBadge
							status={runtimeStatus}
							instances={server.instances || []}
							isServerEnabled={serverEnabled}
						/>
					) : null}
				</div>
				{enableServerDebug && (
					<div className="inline-flex rounded-md border bg-white dark:bg-slate-900 self-start">
						<Button
							type="button"
							size="sm"
							variant={viewMode === VIEW_MODES.browse ? "default" : "ghost"}
							onClick={() => {
								isUserAction.current = true;
								setViewMode(VIEW_MODES.browse);
							}}
							className="gap-1 rounded-l-md rounded-r-none"
						>
							<Eye className="h-4 w-4" />
							{t("detail.viewModes.browse", { defaultValue: "Browse" })}
						</Button>
						<Button
							type="button"
							size="sm"
							variant={viewMode === VIEW_MODES.debug ? "default" : "ghost"}
							onClick={() => {
								isUserAction.current = true;
								setViewMode(VIEW_MODES.debug);
							}}
							className="gap-1 rounded-r-md rounded-l-none"
						>
							<Bug className="h-4 w-4" />
							{t("detail.viewModes.debug", { defaultValue: "Inspect" })}
						</Button>
					</div>
				)}
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
				<Tabs key={viewMode} defaultValue={defaultTab} className="space-y-4">
					<div className="flex items-center justify-between gap-2 flex-wrap">
						<ServerCapabilityTabsHeader
							serverId={serverId}
							viewMode={viewMode}
						/>
						{viewMode === VIEW_MODES.debug ? (
							<InspectorChannelControls
								selected={requestedChannel}
								active={channel}
								proxyAvailable={proxyAvailable}
								isChecking={isProxyChecking}
								onSelect={setRequestedChannel}
								onOpenProfiles={() => navigate("/profiles")}
								onUserAction={() => {
									isUserAction.current = true;
								}}
							/>
						) : null}
					</div>

					{viewMode === VIEW_MODES.browse ? (
						<TabsContent value="overview">
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
														<Avatar className="text-sm">
															{primaryIconSrc ? (
																<AvatarImage
																	src={primaryIconSrc}
																	alt={primaryIconAlt}
																/>
															) : null}
															<AvatarFallback>
																{serverDisplayName?.slice(0, 1).toUpperCase() ??
																	"?"}
															</AvatarFallback>
														</Avatar>
														<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-sm">
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.service", {
																	defaultValue: "Service",
																})}
															</span>
															<Badge
																variant={
																	serverEnabled ? "secondary" : "outline"
																}
																className={`justify-self-start ${serverEnabled ? "bg-emerald-100 text-emerald-700 dark:bg-emerald-500/20 dark:text-emerald-200" : "text-slate-600 dark:text-slate-300"}`}
															>
																{serverEnabled
																	? t("detail.overview.status.enabled", {
																			defaultValue: "Enabled",
																		})
																	: t("detail.overview.status.disabled", {
																			defaultValue: "Disabled",
																		})}
															</Badge>
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.runtime", {
																	defaultValue: "Runtime",
																})}
															</span>
															<StatusBadge
																status={runtimeStatus}
																instances={server.instances || []}
																isServerEnabled={serverEnabled}
																className="justify-self-start"
															/>
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.type", {
																	defaultValue: "Type",
																})}
															</span>
															<span className="font-mono text-sm leading-tight">
																{server.server_type}
															</span>
															{protocolVersion ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.protocol", {
																			defaultValue: "Protocol",
																		})}
																	</span>
																	<span className="font-mono text-xs text-slate-600 dark:text-slate-300">
																		{protocolVersion}
																	</span>
																</>
															) : null}
															{serverVersion ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.version", {
																			defaultValue: "Version",
																		})}
																	</span>
																	<span className="font-mono text-xs text-slate-600 dark:text-slate-300">
																		{serverVersion}
																	</span>
																</>
															) : null}
															{capabilityOverviewText ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.capabilities", {
																			defaultValue: "Capabilities",
																		})}
																	</span>
																	<span className="text-sm text-slate-600 dark:text-slate-300">
																		{capabilityOverviewText}
																	</span>
																</>
															) : null}
															{serverDescription ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.description", {
																			defaultValue: "Description",
																		})}
																	</span>
																	<span className="text-sm text-slate-600 dark:text-slate-300">
																		{serverDescription}
																	</span>
																</>
															) : null}

															{/* Default HTTP Headers (redacted) */}
															{showDefaultHeaders &&
															redactedHeaders &&
															Object.keys(redactedHeaders).length ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t(
																			"detail.overview.labels.defaultHeaders",
																			{
																				defaultValue: "Default Headers",
																			},
																		)}
																	</span>
																	<div className="grid grid-cols-1 gap-1">
																		{Object.entries(redactedHeaders).map(
																			([k, v]) => (
																				<div key={k} className="text-sm">
																					<span className="font-mono text-slate-600 dark:text-slate-300">
																						{k}
																					</span>
																					<span className="mx-2 text-slate-400">
																						:
																					</span>
																					<span className="font-mono">{v}</span>
																				</div>
																			),
																		)}
																	</div>
																</>
															) : null}
															{serverCategory ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.category", {
																			defaultValue: "Category",
																		})}
																	</span>
																	<span className="text-sm text-slate-600 dark:text-slate-300">
																		{serverCategory}
																	</span>
																</>
															) : null}
															{serverScenario ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.scenario", {
																			defaultValue: "Scenario",
																		})}
																	</span>
																	<span className="text-sm text-slate-600 dark:text-slate-300">
																		{serverScenario}
																	</span>
																</>
															) : null}
															{server.command ? (
																<>
																	<span className="text-xs uppercase text-slate-500">
																		{t("detail.overview.labels.command", {
																			defaultValue: "Command",
																		})}
																	</span>
																	<span className="font-mono text-xs md:text-sm break-all">
																		{server.command}
																	</span>
																</>
															) : null}
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.repository", {
																	defaultValue: "Repository",
																})}
															</span>
															<span className="font-mono text-xs text-slate-500">
																—
															</span>
														</div>
													</div>
													{viewMode === VIEW_MODES.browse ? (
														<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
															<Button
																size="sm"
																variant="outline"
																onClick={() => {
																	refreshCapabilitiesMutation.mutate();
																}}
																disabled={isOverviewRefreshing}
																className="gap-2"
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
																variant="outline"
																onClick={() => setIsEditOpen(true)}
																className="gap-2"
															>
																<Edit3 className="h-4 w-4" />
																{t("detail.actions.edit", {
																	defaultValue: "Edit",
																})}
															</Button>
															<Button
																size="sm"
																variant="outline"
																onClick={() =>
																	toggleServerM.mutate(!serverEnabled)
																}
																disabled={toggleServerM.isPending}
																className="gap-2"
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
																className="gap-2"
															>
																<Trash2 className="h-4 w-4" />
																{t("detail.actions.delete", {
																	defaultValue: "Delete",
																})}
															</Button>
														</ButtonGroup>
													) : null}
												</div>
											</div>
										</CardContent>
									</Card>

									{viewMode === VIEW_MODES.browse ? (
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
									) : null}
								</div>
							)}
						</TabsContent>
					) : null}

					<TabsContent value="tools">
						{viewMode === VIEW_MODES.browse ? (
							<ServerCapabilityList kind="tools" serverId={serverId} />
						) : (
							<InspectorDebugSection
								kind="tools"
								state={debugData.tools}
								disabled={channel === "proxy" && !proxyAvailable}
								onFetch={() => runList("tools")}
								onInspect={(item) => handleInspect("tool", item)}
								logs={logs}
								onClearLogs={() => clearLogsByPrefix("tools/")}
							/>
						)}
					</TabsContent>

					<TabsContent value="prompts">
						{viewMode === VIEW_MODES.browse ? (
							<ServerCapabilityList kind="prompts" serverId={serverId} />
						) : (
							<InspectorDebugSection
								kind="prompts"
								state={debugData.prompts}
								disabled={channel === "proxy" && !proxyAvailable}
								onFetch={() => runList("prompts")}
								onInspect={(item) => handleInspect("prompt", item)}
								logs={logs}
								onClearLogs={() => clearLogsByPrefix("prompts/")}
							/>
						)}
					</TabsContent>

					<TabsContent value="resources">
						{viewMode === VIEW_MODES.browse ? (
							<ServerCapabilityList kind="resources" serverId={serverId} />
						) : (
							<InspectorDebugSection
								kind="resources"
								state={debugData.resources}
								disabled={channel === "proxy" && !proxyAvailable}
								onFetch={() => runList("resources")}
								onInspect={(item) => handleInspect("resource", item)}
								logs={logs}
								onClearLogs={() => clearLogsByPrefix("resources/")}
							/>
						)}
					</TabsContent>

					<TabsContent value="templates">
						{viewMode === VIEW_MODES.browse ? (
							<ServerCapabilityList kind="templates" serverId={serverId} />
						) : (
							<InspectorDebugSection
								kind="templates"
								state={debugData.templates}
								disabled={channel === "proxy" && !proxyAvailable}
								onFetch={() => runList("templates")}
								onInspect={(item) => handleInspect("template", item)}
								logs={logs}
								onClearLogs={() => clearLogsByPrefix("templates/")}
							/>
						)}
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
				mode={channel}
				onLog={pushLog}
			/>
		</div>
	);
}

type HeaderKinds = "tools" | "resources" | "prompts" | "templates";

function ServerCapabilityTabsHeader({
	serverId,
	viewMode,
}: {
	serverId: string;
	viewMode: keyof typeof VIEW_MODES;
}) {
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
	const disableEmpty = true;

	return (
		<TabsList className="flex flex-wrap gap-2">
			{viewMode === VIEW_MODES.browse ? (
				<TabsTrigger value="overview">
					{t("detail.tabs.overview", { defaultValue: "Overview" })}
				</TabsTrigger>
			) : null}
			<TabsTrigger value="tools" disabled={disableEmpty && toolsCount === 0}>
				{t("detail.tabs.tools", {
					count: toolsCount,
					defaultValue: "Tools ({{count}})",
				})}
			</TabsTrigger>
			<TabsTrigger
				value="prompts"
				disabled={disableEmpty && promptsCount === 0}
			>
				{t("detail.tabs.prompts", {
					count: promptsCount,
					defaultValue: "Prompts ({{count}})",
				})}
			</TabsTrigger>
			<TabsTrigger
				value="resources"
				disabled={disableEmpty && resourcesCount === 0}
			>
				{t("detail.tabs.resources", {
					count: resourcesCount,
					defaultValue: "Resources ({{count}})",
				})}
			</TabsTrigger>
			<TabsTrigger
				value="templates"
				disabled={disableEmpty && templatesCount === 0}
			>
				{t("detail.tabs.templates", {
					count: templatesCount,
					defaultValue: "Resource Templates ({{count}})",
				})}
			</TabsTrigger>
		</TabsList>
	);
}

function ServerCapabilityList({
	kind,
	serverId,
}: {
	kind: "tools" | "resources" | "prompts" | "templates";
	serverId: string;
}) {
	const [search, setSearch] = useState("");
	const { t } = useTranslation("servers");
	const capabilityQueryOptions = {
		staleTime: 0,
		refetchOnMount: "always" as const,
	};
	const queryMap = {
		tools: useQuery<CapabilityListResponse>({
			queryKey: ["server-cap", "tools", serverId],
			queryFn: async () => {
				const response = await serversApi.listTools(serverId);
				return {
					items: Array.isArray(response.items)
						? (response.items as CapabilityRecord[])
						: [],
					meta: response.meta,
					state: response.state,
				};
			},
			...capabilityQueryOptions,
		}),
		resources: useQuery<CapabilityListResponse>({
			queryKey: ["server-cap", "resources", serverId],
			queryFn: async () => {
				const response = await serversApi.listResources(serverId);
				return {
					items: Array.isArray(response.items)
						? (response.items as CapabilityRecord[])
						: [],
					meta: response.meta,
					state: response.state,
				};
			},
			...capabilityQueryOptions,
		}),
		prompts: useQuery<CapabilityListResponse>({
			queryKey: ["server-cap", "prompts", serverId],
			queryFn: async () => {
				const response = await serversApi.listPrompts(serverId);
				return {
					items: Array.isArray(response.items)
						? (response.items as CapabilityRecord[])
						: [],
					meta: response.meta,
					state: response.state,
				};
			},
			...capabilityQueryOptions,
		}),
		templates: useQuery<CapabilityListResponse>({
			queryKey: ["server-cap", "templates", serverId],
			queryFn: async () => {
				const response = await serversApi.listResourceTemplates(serverId);
				return {
					items: Array.isArray(response.items)
						? (response.items as CapabilityRecord[])
						: [],
					meta: response.meta,
					state: response.state,
				};
			},
			...capabilityQueryOptions,
		}),
	} as const;
	const q = queryMap[kind];
	const labelMap: Record<HeaderKinds, string> = {
		tools: t("detail.capabilityList.labels.tools", { defaultValue: "Tools" }),
		resources: t("detail.capabilityList.labels.resources", {
			defaultValue: "Resources",
		}),
		prompts: t("detail.capabilityList.labels.prompts", {
			defaultValue: "Prompts",
		}),
		templates: t("detail.capabilityList.labels.templates", {
			defaultValue: "Resource Templates",
		}),
	};
	const label = labelMap[kind];

	return (
		<CapabilityList
			title={t("detail.capabilityList.title", {
				label,
				count: q.data?.items?.length ?? 0,
				defaultValue: "{{label}} ({{count}})",
			})}
			kind={kind}
			context="server"
			items={q.data?.items ?? []}
			loading={q.isLoading}
			filterText={search}
			onFilterTextChange={setSearch}
			dense
			hoverActions
			clickToToggleDetails
			emptyText={t("detail.capabilityList.empty", {
				label: label.toLowerCase(),
				defaultValue: "No {{label}} from this server",
			})}
		/>
	);
}

interface InspectorDebugSectionProps {
	kind: DebugKind | "templates";
	state: DebugState;
	disabled?: boolean;
	onFetch: () => void;
	onInspect: (item: CapabilityRecord | null) => void;
	logs: InspectorLogEntry[];
	onClearLogs: () => void;
}

interface InspectorChannelControlsProps {
	selected: InspectorChannel;
	active: InspectorChannel;
	proxyAvailable: boolean;
	isChecking?: boolean;
	onSelect: (channel: InspectorChannel) => void;
	onOpenProfiles: () => void;
	onUserAction: () => void;
}

function InspectorChannelControls({
	selected,
	active,
	proxyAvailable,
	isChecking,
	onSelect,
	onOpenProfiles,
	onUserAction,
}: InspectorChannelControlsProps) {
	const [hintVisible, setHintVisible] = useState(false);
	const containerRef = useRef<HTMLDivElement | null>(null);
	const { t } = useTranslation("servers");

	useEffect(() => {
		if (proxyAvailable) {
			setHintVisible(false);
		}
	}, [proxyAvailable]);

	useEffect(() => {
		if (isChecking) {
			setHintVisible(false);
		}
	}, [isChecking]);

	useEffect(() => {
		if (selected !== "proxy") {
			setHintVisible(false);
		}
	}, [selected]);

	useEffect(() => {
		if (!hintVisible) return;
		const handler = (event: PointerEvent) => {
			if (containerRef.current?.contains(event.target as Node)) {
				return;
			}
			setHintVisible(false);
		};
		window.addEventListener("pointerdown", handler, { capture: true });
		return () =>
			window.removeEventListener("pointerdown", handler, { capture: true });
	}, [hintVisible]);

	const handleProxy = () => {
		onUserAction();
		onSelect("proxy");
		if (proxyAvailable && !isChecking) {
			setHintVisible(false);
		} else {
			setHintVisible(true);
		}
	};

	const handleNative = () => {
		onUserAction();
		onSelect("native");
		setHintVisible(false);
	};

	return (
		<div ref={containerRef} className="relative inline-flex items-center gap-2">
			{selected === "proxy" && active !== "proxy" && !isChecking ? (
				<span className="text-xs text-amber-600 dark:text-amber-400 mr-2">
					{t("detail.inspector.channel.fallback", {
						defaultValue: "Fallback to native until proxy is available",
					})}
				</span>
			) : null}
			<div className="inline-flex rounded-md border bg-white dark:bg-slate-900">
				<Button
					type="button"
					size="sm"
					variant={selected === "proxy" ? "default" : "ghost"}
					className="gap-1 h-8 px-3 text-xs rounded-l-md rounded-r-none"
					disabled={isChecking}
					onClick={handleProxy}
				>
					{isChecking ? (
						<Loader2 className="h-3.5 w-3.5 animate-spin" />
					) : (
						<ShieldAlert className="h-4 w-4" />
					)}
					{t("detail.inspector.channel.proxy", { defaultValue: "Proxy" })}
				</Button>
				<Button
					type="button"
					size="sm"
					variant={selected === "native" ? "default" : "ghost"}
					className="gap-1 h-8 px-3 text-xs rounded-r-md rounded-l-none"
					onClick={handleNative}
				>
					<AlertTriangle className="h-4 w-4" />
					{t("detail.inspector.channel.native", { defaultValue: "Native" })}
				</Button>
			</div>
			{!proxyAvailable && !isChecking && hintVisible ? (
				<div className="absolute right-0 top-full mt-1 w-64 rounded-md border bg-white dark:bg-slate-900 p-3 text-xs shadow-lg z-10">
					<p className="font-medium text-slate-700 dark:text-slate-200 mb-1">
						{t("detail.inspector.channel.hintTitle", {
							defaultValue: "Proxy unavailable",
						})}
					</p>
					<p className="text-slate-600 dark:text-slate-300 mb-2">
						{t("detail.inspector.channel.hintDescription", {
							defaultValue:
								"Enable this server in an active profile to exercise proxy mode.",
						})}
					</p>
					<Button
						size="sm"
						variant="outline"
						className="text-xs"
						onClick={onOpenProfiles}
					>
						{t("detail.inspector.channel.openProfiles", {
							defaultValue: "Open Profiles",
						})}
					</Button>
				</div>
			) : null}
		</div>
	);
}

function InspectorDebugSection({
	kind,
	state,
	disabled,
	onFetch,
	onInspect,
	logs,
	onClearLogs,
}: InspectorDebugSectionProps) {
	const [filter, setFilter] = useState("");
	const [logFilter, setLogFilter] = useState("");
	const [tab, setTab] = useState<"results" | "logs">("results");
	const { t } = useTranslation("servers");

	const title = useMemo(() => {
		if (kind === "tools") {
			return t("detail.inspector.labels.tools", { defaultValue: "Tools" });
		}
		if (kind === "resources") {
			return t("detail.inspector.labels.resources", {
				defaultValue: "Resources",
			});
		}
		if (kind === "prompts") {
			return t("detail.inspector.labels.prompts", {
				defaultValue: "Prompts",
			});
		}
		return t("detail.inspector.labels.templates", {
			defaultValue: "Resource Templates",
		});
	}, [kind, t]);

	const sectionLogs = useMemo(() => {
		const prefix =
			kind === "tools"
				? "tools/"
				: kind === "resources"
					? "resources/"
					: kind === "prompts"
						? "prompts/"
						: "templates/";
		let filteredLogs = logs.filter((entry) => entry.method.startsWith(prefix));

		// Apply log filter if provided
		if (logFilter.trim()) {
			const searchTerm = logFilter.toLowerCase();
			filteredLogs = filteredLogs.filter((entry) => {
				return (
					entry.method.toLowerCase().includes(searchTerm) ||
					entry.event.toLowerCase().includes(searchTerm) ||
					entry.mode.toLowerCase().includes(searchTerm) ||
					entry.message?.toLowerCase().includes(searchTerm) ||
					(entry.payload &&
						safeLog(entry.payload).toLowerCase().includes(searchTerm))
				);
			});
		}

		return filteredLogs;
	}, [logs, kind, logFilter]);

	return (
		<Tabs
			value={tab}
			onValueChange={(v) => setTab(v as "results" | "logs")}
			className="w-full space-y-4"
		>
			<div className="flex items-center justify-between gap-2 flex-wrap">
				<TabsList className="flex flex-wrap gap-2">
					<TabsTrigger value="results">
						{t("detail.inspector.tabs.results", {
							count: state.items.length,
							defaultValue: "Results ({{count}})",
						})}
					</TabsTrigger>
					<TabsTrigger value="logs">
						{t("detail.inspector.tabs.logs", {
							count: sectionLogs.length,
							defaultValue: "Logs ({{count}})",
						})}
					</TabsTrigger>
				</TabsList>
				{tab === "results" ? (
					<div className="flex items-center gap-2 flex-wrap">
						<Input
							placeholder={t("detail.inspector.filterPlaceholder", {
								label: title,
								defaultValue: "Filter {{label}}...",
							})}
							value={filter}
							onChange={(e) => setFilter(e.target.value)}
							className="h-8 w-48"
						/>
						<Button
							size="sm"
							onClick={onFetch}
							disabled={disabled || state.loading}
							className="h-8 gap-2"
						>
							{state.loading ? (
								<Loader2 className="h-4 w-4 animate-spin" />
							) : (
								<RefreshCw className="h-4 w-4" />
							)}
							{state.fetched
								? t("detail.inspector.actions.refresh", {
										defaultValue: "Refresh",
									})
								: t("detail.inspector.actions.list", {
										defaultValue: "List",
									})}
						</Button>
					</div>
				) : (
					<div className="flex items-center gap-2 flex-wrap">
						<Input
							placeholder={t("detail.inspector.logs.search", {
								defaultValue: "Search logs...",
							})}
							value={logFilter}
							onChange={(e) => setLogFilter(e.target.value)}
							className="h-8 w-48"
						/>
						<Button
							size="sm"
							variant="outline"
							className="h-8"
							onClick={onClearLogs}
							disabled={!sectionLogs.length}
						>
							{t("detail.inspector.logs.clear", {
								defaultValue: "Clear Logs",
							})}
						</Button>
					</div>
				)}
			</div>

			<TabsContent value="results" className="space-y-4">
				<Card className="min-h-[220px]">
					{state.error ? (
						<CardHeader className="pl-4 pt-4 pr-0 pb-0">
							<p className="text-xs text-red-500 mt-1">{state.error}</p>
						</CardHeader>
					) : null}
					<CardContent className="p-4">
						<CapabilityList
							asCard={false}
							title={undefined}
							kind={kind}
							context="server"
							items={state.fetched ? state.items : []}
							loading={state.loading}
							filterText={filter}
							hoverActions
							clickToToggleDetails
							emptyText={
								state.fetched
									? t("detail.inspector.results.emptyFetched", {
											label: title,
											defaultValue: "No {{label}} returned.",
										})
									: t("detail.inspector.results.emptyPrompt", {
											label: title,
											defaultValue: "Run {{label}} list to fetch live data.",
										})
							}
							renderAction={(_, item) => (
								<Button
									type="button"
									size="sm"
									variant="outline"
									className="gap-1"
									onClick={() => onInspect(item)}
								>
									<Play className="w-3.5 h-3.5" />
									{t("detail.inspector.actions.inspect", {
										defaultValue: "Inspect",
									})}
								</Button>
							)}
						/>
					</CardContent>
				</Card>
			</TabsContent>

			<TabsContent value="logs" className="space-y-4">
				<Card className="min-h-[220px]">
					<CardContent className="space-y-2 p-4 max-h-[60vh] overflow-auto">
						{sectionLogs.length === 0 ? (
							<p className="text-sm text-slate-500">
								{t("detail.inspector.logs.empty", {
									defaultValue: "No inspector events yet.",
								})}
							</p>
						) : (
							<CapsuleStripeList>
								{sectionLogs.map((entry) => (
									<CapsuleStripeListItem
										key={entry.id}
										className="group items-start text-xs"
									>
										{/* 错误消息 */}
										{entry.message ? (
											<p className="text-red-500 mb-1">{entry.message}</p>
										) : null}

										{/* 日志内容区域：悬浮信息移至文本区右下角 */}
										{entry.payload !== undefined ? (
											<div className="relative group w-full">
												<pre className="bg-transparent border-0 rounded-none p-0 max-h-48 overflow-auto pr-8">
													{safeLog(entry.payload)}
												</pre>
												{/* 复制按钮（右上角，悬停显示） */}
												<Button
													size="sm"
													variant="outline"
													className="absolute top-0 right-0 h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
													onClick={() => {
														void writeClipboardText(safeLog(entry.payload));
													}}
												>
													<Copy className="h-3 w-3" />
												</Button>
												{/* 右下角：时间戳、Action、Mode、Event 徽标（悬停显示） */}
												<div className="absolute bottom-0 right-0 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
													<Badge
														variant="secondary"
														className="text-[10px] font-mono pointer-events-none"
													>
														{new Date(entry.timestamp).toLocaleTimeString()}
													</Badge>
													<Badge
														variant="outline"
														className="text-[10px] font-mono pointer-events-none"
													>
														{entry.method}
													</Badge>
													<Badge
														variant="outline"
														className="text-[10px] uppercase pointer-events-none"
													>
														{t(
															`detail.inspector.logs.status.mode.${entry.mode}`,
															{
																defaultValue: entry.mode.toUpperCase(),
															},
														)}
													</Badge>
													<Badge
														variant={
															entry.event === "error"
																? "destructive"
																: entry.event === "success"
																	? "success"
																	: "secondary"
														}
														className="text-[10px] uppercase pointer-events-none"
													>
														{t(
															`detail.inspector.logs.status.event.${entry.event}`,
															{
																defaultValue: entry.event.toUpperCase(),
															},
														)}
													</Badge>
												</div>
											</div>
										) : null}
									</CapsuleStripeListItem>
								))}
							</CapsuleStripeList>
						)}
					</CardContent>
				</Card>
			</TabsContent>
		</Tabs>
	);
}

function safeLog(value: unknown) {
	return smartFormat(value);
}

export default ServerDetailPage;

import { smartFormat } from "../../lib/format";
