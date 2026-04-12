import {
	useMutation,
	useQueries,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import {
	Check,
	Download,
	HardDrive,
	Info,
	MoreVertical,
	Pencil,
	RefreshCw,
	RotateCcw,
	Square,
	Trash2,
} from "lucide-react";
import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "react-router-dom";
import { AuditLogsPanel } from "../../components/audit-logs-panel";
import { CachedAvatar } from "../../components/cached-avatar";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import {
	CapsuleStripeLeadCircle,
	CapsuleStripeRowBody,
} from "../../components/capsule-stripe-row";
import { ClientFormDrawer } from "../../components/client-form-drawer";
import { ConfirmDialog } from "../../components/confirm-dialog";
import { DETAIL_TAB_CONTENT_CLASS } from "../../components/detail-tab-content-class";
import { useUrlTab } from "../../lib/hooks/use-url-state";

import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ButtonGroup, ButtonGroupText } from "../../components/ui/button-group";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "../../components/ui/drawer";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Segment } from "../../components/ui/segment";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { auditApi, clientsApi, configSuitsApi, serversApi } from "../../lib/api";
import { mapDashboardSettingsToClientBackupPolicy } from "../../lib/client-backup-policy";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type {
	ClientBackupEntry,
	ClientBackupPolicySetReq,
	ClientCapabilityConfigData,
	ClientCapabilityConfigReq,
	ClientCapabilitySourceSelection,
	ClientConfigImportData,
	ClientConfigMode,
	ClientConfigSelected,
	ClientConfigUpdateData,
	ClientInfo,
	ConfigSuit,
	ServerSummary,
} from "../../lib/types";
import { formatBackupTime } from "../../lib/utils";
import { ConfigurationProfileTokenChart } from "./components/configuration-profile-token-chart";

const governanceClientsApi = clientsApi as typeof clientsApi & {
	suspendRecord: (payload: { identifier: string }) => Promise<unknown>;
};

const arrangeProfilesWithDefaultFirst = (items: ConfigSuit[] = []) => {
	if (!items.length) {
		return [] as ConfigSuit[];
	}
	const byName = (a: ConfigSuit, b: ConfigSuit) =>
		a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
	const defaults = items.filter((profile) => profile.is_default).sort(byName);
	const others = items.filter((profile) => !profile.is_default).sort(byName);
	return [...defaults, ...others];
};

const SUPPORTED_TRANSPORT_OPTIONS = ["streamable_http", "sse", "stdio"];

function getUnifyServerSurfaces<T extends { server_id: string }>(
	selectedSurfaces: T[],
	serverId: string,
): T[] {
	return selectedSurfaces.filter((entry) => entry.server_id === serverId);
}

function resolveNextCapabilityLevelSurfaces<T extends { server_id: string }>(
	currentSurfaces: T[],
	serverId: string,
	hasSelectedForServer: boolean,
	resolvedServerSurfaces: T[],
): T[] {
	const otherServerSurfaces = currentSurfaces.filter(
		(entry) => entry.server_id !== serverId,
	);

	if (hasSelectedForServer) {
		return otherServerSurfaces;
	}

	return [...otherServerSurfaces, ...resolvedServerSurfaces];
}

function isUnifyServerMixedRouting(
	routeMode: "broker_only" | "server_live" | "capability_level",
	toolSurfaceCount: number,
	toolsCount: number | undefined,
): boolean {
	if (routeMode !== "capability_level" || toolSurfaceCount === 0) {
		return false;
	}

	if (!toolsCount) {
		return true;
	}

	return toolSurfaceCount < toolsCount;
}

function getClientDirectCapabilitiesPath(
	identifier: string | undefined,
	serverId: string,
): string | null {
	if (!identifier) {
		return null;
	}

	return `/clients/${identifier}/direct/${serverId}`;
}

function toggleSelectedServerIds(currentIds: string[], serverId: string): string[] {
	if (currentIds.includes(serverId)) {
		return currentIds.filter((id) => id !== serverId);
	}

	return [...currentIds, serverId];
}

function getTransportOptionLabel(
	transport: string,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	switch (transport) {
		case "streamable_http":
			return t("detail.configuration.transportOptions.streamableHttpLegacy", {
				defaultValue: "Streamable HTTP",
			});
		case "sse":
			return t("detail.configuration.transportOptions.sseLegacy", {
				defaultValue: "SSE (Legacy)",
			});
		case "stdio":
			return t("detail.configuration.transportOptions.stdio", {
				defaultValue: "STDIO",
			});
		default:
			return transport.toUpperCase();
	}
}

function resolveConfigModeForWritableState(
	requestedMode: ClientConfigMode,
	isWritableConfig: boolean,
): ClientConfigMode {
	if (requestedMode === "transparent" && !isWritableConfig) {
		return "hosted";
	}

	return requestedMode;
}

function isDeniedApprovalStatus(status?: string | null): boolean {
	return status === "rejected" || status === "suspended";
}

function extractServers(obj: unknown): string[] {
	if (!obj || typeof obj !== "object") return [];
	const objRecord = obj as Record<string, unknown>;
	const collected = new Set<string>();
	const addFromValue = (value: unknown) => {
		if (!value) return;
		if (Array.isArray(value)) {
			for (const entry of value) {
				if (typeof entry === "string") {
					collected.add(entry);
				} else if (
					entry &&
					typeof entry === "object" &&
					"name" in entry &&
					typeof (entry as { name?: unknown }).name === "string"
				) {
					collected.add((entry as { name: string }).name);
				}
			}
			return;
		}
		if (typeof value === "object") {
			for (const key of Object.keys(value as Record<string, unknown>)) {
				collected.add(key);
			}
		}
	};

	addFromValue(objRecord.mcpServers);
	addFromValue(objRecord.mcp_servers);
	addFromValue(objRecord.servers);
	addFromValue(objRecord.context_servers);
	addFromValue(objRecord.contextServers);
	addFromValue(objRecord.agent_servers);

	if (
		objRecord.mcp &&
		typeof objRecord.mcp === "object" &&
		(objRecord.mcp as Record<string, unknown>).servers
	) {
		addFromValue((objRecord.mcp as Record<string, unknown>).servers);
	}

	return Array.from(collected);
}

export function ClientDetailPage() {
	const { identifier } = useParams<{ identifier: string }>();
	const qc = useQueryClient();
	const navigate = useNavigate();
	usePageTranslations("clients");
	const { t, i18n } = useTranslation("clients");
	const showClientLiveLogs = useAppStore(
		(state) => state.dashboardSettings.showClientLiveLogs,
	);
	const dashboardSettings = useAppStore((state) => state.dashboardSettings);
	const clientDefaultMode = useAppStore(
		(state) => state.dashboardSettings.clientDefaultMode,
	);
	const [displayName, setDisplayName] = useState("");
	const [selectedBackups, setSelectedBackups] = useState<string[]>([]);
	const [detected, setDetected] = useState<boolean>(false);
	const [isClientFormOpen, setIsClientFormOpen] = useState(false);
	const [bulkConfirmOpen, setBulkConfirmOpen] = useState(false);

	const { data: clientsData } = useQuery({
		queryKey: ["clients"],
		queryFn: () => clientsApi.list(false),
		retry: 1,
	});

	const currentClient = useMemo(
		() => clientsData?.client?.find((client) => client.identifier === identifier),
		[clientsData?.client, identifier],
	);
	const supportsBackupOperations =
		currentClient?.writable_config !== false &&
		currentClient?.template?.managed_source !== "runtime_active_client";
	const detailTabs = useMemo(
		() =>
			!supportsBackupOperations
				? ["overview", "configuration"]
				: ["overview", "configuration", "backups"],
		[supportsBackupOperations],
	);
	const { activeTab: tabValue, setActiveTab: setTabValue } = useUrlTab({
		paramName: "tab",
		defaultTab: "overview",
		validTabs: detailTabs,
	});
	const [selectedProfiles, setSelectedProfiles] = useState<string[]>([]);

	const limitId = useId();
	const [mode, setMode] = useState<ClientConfigMode>("hosted");
	const [transport, setTransport] = useState<string>("auto");
	const [unifyRouteMode, setUnifyRouteMode] = useState<"broker_only" | "server_live" | "capability_level">("broker_only");
	const [unifySelectedServers, setUnifySelectedServers] = useState<string[]>([]);
	const [unifySelectedToolSurfaces, setUnifySelectedToolSurfaces] = useState<{ server_id: string; tool_name: string }[]>([]);
	const [hasUnifyDraftChanges, setHasUnifyDraftChanges] = useState(false);

	useEffect(() => {
		if (!currentClient) {
			return;
		}
		const isWritableConfig = currentClient.writable_config !== false;
		setDisplayName(currentClient.display_name || "");
		setDetected(!!currentClient.detected);
		if (typeof currentClient.config_mode === "string") {
			const configMode = currentClient.config_mode;
			if (configMode === "unify") {
				setMode("unify");
			} else if (configMode === "transparent") {
				setMode(resolveConfigModeForWritableState("transparent", isWritableConfig));
			} else {
				setMode("hosted");
			}
		} else {
			setMode(resolveConfigModeForWritableState(clientDefaultMode, isWritableConfig));
		}
		const currentTransport =
			typeof currentClient.transport === "string" ? currentClient.transport : "auto";
		setTransport(
			currentTransport !== "auto" &&
				!currentClient.supported_transports.includes(currentTransport)
				? "auto"
				: currentTransport,
		);
	}, [clientDefaultMode, currentClient]);

	useEffect(() => {
		if (currentClient?.writable_config === false && tabValue === "backups") {
			setTabValue("configuration");
		}
	}, [currentClient?.writable_config, setTabValue, tabValue]);

	const [selectedConfig, setSelectedConfig] =
		useState<ClientCapabilitySourceSelection>("default");
	const [policyOpen, setPolicyOpen] = useState(false);
	const [importPreviewOpen, setImportPreviewOpen] = useState(false);
	const [importPreviewData, setImportPreviewData] =
		useState<ClientConfigImportData | null>(null);

	const {
		data: configDetails,
		isLoading: loadingConfig,
		refetch: refetchDetails,
	} = useQuery({
		queryKey: ["client-config", identifier],
		queryFn: () => clientsApi.configDetails(identifier || "", false),
		enabled: !!identifier,
	});

	const { data: capabilityConfig, refetch: refetchCapabilityConfig } = useQuery({
		queryKey: ["client-capability-config", identifier],
		queryFn: () => clientsApi.getCapabilityConfig(identifier || ""),
		enabled: !!identifier,
	});

	const {
		data: backupsData,
		isLoading: loadingBackups,
		refetch: refetchBackups,
	} = useQuery({
		queryKey: ["client-backups", identifier],
		queryFn: () => clientsApi.listBackups(identifier || undefined),
		enabled: Boolean(identifier && supportsBackupOperations),
	});

	// Fetch eligible servers for Unify direct exposure
	const { data: serversListData } = useQuery({
		queryKey: ["servers", "unify-candidates"],
		queryFn: () => serversApi.getAll(),
		staleTime: 0,
		gcTime: 0,
		refetchOnMount: "always",
		refetchOnWindowFocus: "always",
		refetchOnReconnect: "always",
		retry: 1,
	});
	const eligibleServers = useMemo(() => {
		return serversListData?.servers?.filter((s) => s.unify_direct_exposure_eligible) || [];
	}, [serversListData]);

	const { data: profilesData, isLoading: loadingProfiles } = useQuery({
		queryKey: ["profiles"],
		queryFn: () => configSuitsApi.getAll(),
		retry: 1,
	});

	const visibleBackups = useMemo(() => {
		const backups: ClientBackupEntry[] = backupsData?.backups || [];
		return backups.filter((backup) => backup.identifier === identifier);
	}, [backupsData?.backups, identifier]);

	// Process profiles data
	const activeProfiles = useMemo(() => {
		const profiles: ConfigSuit[] = profilesData?.suits || [];
		return arrangeProfilesWithDefaultFirst(
			profiles.filter((profile) => profile.is_active),
		);
	}, [profilesData?.suits]);
	const sharedProfiles = useMemo(() => {
		const profiles: ConfigSuit[] = profilesData?.suits || [];
		return arrangeProfilesWithDefaultFirst(
			profiles.filter((profile) => profile.suit_type === "shared"),
		);
	}, [profilesData?.suits]);

	const effectiveCapabilityConfig = useMemo<ClientCapabilityConfigData | null>(() => {
		if (capabilityConfig) {
			return capabilityConfig;
		}
		if (!identifier) {
			return null;
		}
		return {
			identifier,
			capability_source: configDetails?.capability_source || "activated",
			selected_profile_ids: configDetails?.selected_profile_ids || [],
			custom_profile_id: configDetails?.custom_profile_id ?? null,
		};
	}, [capabilityConfig, configDetails?.capability_source, configDetails?.custom_profile_id, configDetails?.selected_profile_ids, identifier]);

	const customProfileId =
		effectiveCapabilityConfig?.custom_profile_id ?? configDetails?.custom_profile_id ?? null;

	useEffect(() => {
		setHasUnifyDraftChanges(false);
	}, [identifier]);

	useEffect(() => {
		if (!effectiveCapabilityConfig) {
			return;
		}
		switch (effectiveCapabilityConfig.capability_source) {
			case "profiles":
				setSelectedConfig("profile");
				setSelectedProfiles(effectiveCapabilityConfig.selected_profile_ids || []);
				break;
			case "custom":
				setSelectedConfig("custom");
				setSelectedProfiles([]);
				break;
			case "activated":
			default:
				setSelectedConfig("default");
				setSelectedProfiles([]);
				break;
		}
		if (!hasUnifyDraftChanges) {
			if (effectiveCapabilityConfig.unify_direct_exposure) {
				setUnifyRouteMode(effectiveCapabilityConfig.unify_direct_exposure.route_mode || "broker_only");
				setUnifySelectedServers(effectiveCapabilityConfig.unify_direct_exposure.selected_server_ids || []);
				setUnifySelectedToolSurfaces(effectiveCapabilityConfig.unify_direct_exposure.selected_tool_surfaces || []);
			} else {
				setUnifyRouteMode("broker_only");
				setUnifySelectedServers([]);
				setUnifySelectedToolSurfaces([]);
			}
		}
	}, [effectiveCapabilityConfig, hasUnifyDraftChanges]);

	const capabilityProfileIds = useMemo(() => {
		const ids = new Set<string>();
		for (const profile of [...activeProfiles, ...sharedProfiles]) {
			ids.add(profile.id);
		}
		if (customProfileId) {
			ids.add(customProfileId);
		}
		return Array.from(ids);
	}, [activeProfiles, customProfileId, sharedProfiles]);

	// Fetch capabilities for profiles
	const profileCapabilitiesQueries = useQueries({
		queries: capabilityProfileIds.map((profileId) => ({
			queryKey: ["profile-capabilities", profileId],
			queryFn: async () => {
				const [serversRes, toolsRes, resourcesRes, promptsRes, templatesRes] =
					await Promise.all([
						configSuitsApi.getServers(profileId),
						configSuitsApi.getTools(profileId),
						configSuitsApi.getResources(profileId),
						configSuitsApi.getPrompts(profileId),
						configSuitsApi.getResourceTemplates(profileId),
					]);

				const enabledByComponentId = new Map<string, boolean>();
				for (const s of serversRes?.servers ?? []) {
					enabledByComponentId.set(s.id, Boolean(s.enabled));
				}
				for (const tool of toolsRes?.tools ?? []) {
					enabledByComponentId.set(tool.id, Boolean(tool.enabled));
				}
				for (const r of resourcesRes?.resources ?? []) {
					enabledByComponentId.set(r.id, Boolean(r.enabled));
				}
				for (const p of promptsRes?.prompts ?? []) {
					enabledByComponentId.set(p.id, Boolean(p.enabled));
				}
				for (const tmpl of templatesRes?.templates ?? []) {
					enabledByComponentId.set(tmpl.id, Boolean(tmpl.enabled));
				}

				return {
					profileId,
					enabledByComponentId,
					servers: {
						total: serversRes?.servers?.length || 0,
						enabled:
							serversRes?.servers?.filter(
								(s: { enabled?: boolean }) => s.enabled,
							).length || 0,
					},
					tools: {
						total: toolsRes?.tools?.length || 0,
						enabled:
							toolsRes?.tools?.filter((tool: { enabled?: boolean }) => tool.enabled)
								.length || 0,
					},
					resources: {
						total: resourcesRes?.resources?.length || 0,
						enabled:
							resourcesRes?.resources?.filter(
								(r: { enabled?: boolean }) => r.enabled,
							).length || 0,
					},
					prompts: {
						total: promptsRes?.prompts?.length || 0,
						enabled:
							promptsRes?.prompts?.filter(
								(p: { enabled?: boolean }) => p.enabled,
							).length || 0,
					},
				};
			},
			enabled: capabilityProfileIds.length > 0,
			retry: 1,
		})),
	});

	// Create capabilities map
	const profileCapabilities = useMemo(() => {
		const map = new Map();
		profileCapabilitiesQueries.forEach((query) => {
			const data = query.data ?? undefined;
			if (data) {
				map.set(data.profileId, data);
			}
		});
		return map;
	}, [profileCapabilitiesQueries]);

	const customProfileCapabilities = customProfileId
		? profileCapabilities.get(customProfileId)
		: undefined;
	const managedTransportSupported = (configDetails?.supported_transports?.length ?? 0) > 0;
	const canWriteClientConfig = configDetails?.writable_config !== false;
	const isPendingApproval = configDetails?.approval_status === "pending";
	const isRejectedClient = isDeniedApprovalStatus(configDetails?.approval_status);
	const canSaveManagementSettings = !isPendingApproval;
	const canApplyTransparentConfig =
		canSaveManagementSettings &&
		canWriteClientConfig &&
		!isDeniedApprovalStatus(configDetails?.approval_status);
	const canSyncManagedConfig =
		canWriteClientConfig &&
		canSaveManagementSettings &&
		!isDeniedApprovalStatus(configDetails?.approval_status) &&
		managedTransportSupported;
	const shouldRequireLocalConfigWrite = mode === "transparent";

	const supportedTransportOptions = useMemo(() => {
		const supported = (configDetails?.supported_transports || []) as string[];
		if (supported.length < 2) {
			return [] as string[];
		}

		const allowed = new Set(supported);
		return SUPPORTED_TRANSPORT_OPTIONS.filter((transportOption) =>
			allowed.has(transportOption),
		);
	}, [configDetails?.supported_transports]);

	const unifyRouteModeSegmentOptions = useMemo(
		() => {
			return [
				{
					value: "broker_only",
					label: t("detail.configuration.sections.source.unifyRouteModes.broker_only", {
						defaultValue: "Broker Only",
					}),
				},
				{
					value: "server_live",
					label: t("detail.configuration.sections.source.unifyRouteModes.server_live", {
						defaultValue: "Server Level",
					}),
				},
				{
					value: "capability_level",
					label: t("detail.configuration.sections.source.unifyRouteModes.capability_level", {
						defaultValue: "Capability Level",
					}),
				},
			];
		},
		[t, i18n.language],
	);

	const configurationSourceSegmentOptions = useMemo(
		() => {
			const statusOrUndefined = (raw: string) => {
				const trimmed = raw.trim();
				return trimmed.length > 0 ? trimmed : undefined;
			};
			return [
				{
					value: "default",
					label: t("detail.configuration.sections.source.options.default", {
						defaultValue: "Active",
					}),
					status: statusOrUndefined(
						t("detail.configuration.sections.source.statusLabel.default", {
							defaultValue: "",
						}),
					),
				},
				{
					value: "profile",
					label: t("detail.configuration.sections.source.options.profile", {
						defaultValue: "Profiles",
					}),
					status: statusOrUndefined(
						t("detail.configuration.sections.source.statusLabel.profile", {
							defaultValue: "",
						}),
					),
				},
				{
					value: "custom",
					label: t("detail.configuration.sections.source.options.custom", {
						defaultValue: "Customize",
					}),
					status: statusOrUndefined(
						t("detail.configuration.sections.source.statusLabel.custom", {
							defaultValue: "",
						}),
					),
				},
			];
		},
		[t, i18n.language],
	);

	const detailDescription = configDetails?.description ?? "";
	const detailHomepageUrl = configDetails?.homepage_url ?? "";
	const detailDocsUrl = configDetails?.docs_url ?? "";
	const detailSupportUrl = configDetails?.support_url ?? "";
	const managementModeSegmentOptions = useMemo(
		() => {
			return [
				{
					value: "unify",
					label: t("detail.configuration.sections.mode.options.unify", {
						defaultValue: "Unify",
					}),
				},
				{
					value: "hosted",
					label: t("detail.configuration.sections.mode.options.hosted", {
						defaultValue: "Hosted",
					}),
				},
				{
					value: "transparent",
					label: t("detail.configuration.sections.mode.options.transparent", {
						defaultValue: "Transparent",
					}),
					disabled: configDetails?.writable_config === false,
					tooltip:
						configDetails?.writable_config === false
							? t("detail.configuration.sections.mode.transparentDisabledReason", {
								defaultValue: "Transparent requires a writable local config path.",
							})
							: undefined,
				},
			];
		},
		[configDetails?.writable_config, t, i18n.language],
	);
	const [logFilter, setLogFilter] = useState("");
	const [logPageSize, setLogPageSize] = useState<number>(10);
	const [logPageCursors, setLogPageCursors] = useState<string[]>([]);
	const [logCurrentPageIndex, setLogCurrentPageIndex] = useState(0);
	const [isLogPaginationActionLoading, setIsLogPaginationActionLoading] =
		useState(false);
	const logCurrentCursor = logPageCursors[logCurrentPageIndex];

	const logsQuery = useQuery({
		queryKey: [
			"client-audit-logs",
			identifier,
			logCurrentCursor,
			logPageSize,
			showClientLiveLogs,
		],
		queryFn: () =>
			auditApi.list({
				limit: logPageSize,
				cursor: logCurrentCursor,
				client_id: identifier,
			}),
		enabled: Boolean(identifier && showClientLiveLogs),
		refetchOnWindowFocus: false,
		retry: false,
	});

	useEffect(() => {
		setLogPageCursors([]);
		setLogCurrentPageIndex(0);
	}, [identifier, logPageSize]);

	const filteredLogs = useMemo(() => {
		const logs = logsQuery.data?.events ?? [];
		const term = logFilter.trim().toLowerCase();
		if (!term) {
			return logs;
		}
		return logs.filter((event) => {
			const haystacks = [
				event.action,
				event.category,
				event.status,
				event.target,
				event.route,
				event.error_message,
				event.detail,
				event.mcp_method,
				event.request_id,
			]
				.filter(Boolean)
				.map((value) => String(value).toLowerCase());
			return haystacks.some((value) => value.includes(term));
		});
	}, [logsQuery.data?.events, logFilter]);

	const handleLogsNextPage = () => {
		if (!logsQuery.data?.next_cursor) {
			return;
		}
		const nextCursor = logsQuery.data.next_cursor;
		setLogPageCursors((prev) => {
			const next = [...prev];
			next[logCurrentPageIndex + 1] = nextCursor;
			return next;
		});
		setLogCurrentPageIndex((prev) => prev + 1);
	};

	const handleLogsPrevPage = () => {
		if (logCurrentPageIndex > 0) {
			setLogCurrentPageIndex((prev) => prev - 1);
		}
	};

	const handleLogsFirstPage = () => {
		setLogCurrentPageIndex(0);
	};

	const handleLogsLastPage = async () => {
		if (!logsQuery.data?.next_cursor || !identifier) {
			return;
		}
		setIsLogPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = logsQuery.data.next_cursor;
			let targetPageIndex = logCurrentPageIndex;
			const nextPageCursors = [...logPageCursors];
			while (nextCursor) {
				targetPageIndex += 1;
				nextPageCursors[targetPageIndex] = nextCursor;
				const page = await auditApi.list({
					limit: logPageSize,
					cursor: nextCursor,
					client_id: identifier,
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

	const buildCapabilityConfigPayload = (): ClientCapabilityConfigReq => {
		if (!identifier) {
			throw new Error("No identifier provided");
		}

		const payload: ClientCapabilityConfigReq = {
			identifier,
			capability_source: "activated",
			selected_profile_ids: [],
		};

		if (mode === "unify") {
			const currentUnifyExposure = effectiveCapabilityConfig?.unify_direct_exposure;
			payload.capability_source = "activated";
			payload.selected_profile_ids = [];
			payload.unify_direct_exposure = {
				route_mode: unifyRouteMode,
				selected_server_ids:
					unifyRouteMode === "server_live" ? unifySelectedServers : [],
				selected_tool_surfaces: unifySelectedToolSurfaces,
				selected_prompt_surfaces: currentUnifyExposure?.selected_prompt_surfaces ?? [],
				selected_resource_surfaces: currentUnifyExposure?.selected_resource_surfaces ?? [],
				selected_template_surfaces: currentUnifyExposure?.selected_template_surfaces ?? [],
			};
		} else {
			payload.unify_direct_exposure = effectiveCapabilityConfig?.unify_direct_exposure || null;
			if (selectedConfig === "profile") {
				const profileIds = selectedProfiles.length > 0
					? selectedProfiles
					: [sharedProfiles.find((p) => p.is_default)?.id].filter(Boolean) as string[];

				if (profileIds.length === 0) {
					throw new Error(
						t("detail.configuration.errors.profileRequired", {
							defaultValue:
								"Select at least one shared profile before applying this capability source.",
						}),
					);
				}
				payload.capability_source = "profiles";
				payload.selected_profile_ids = profileIds;
			} else if (selectedConfig === "custom") {
				payload.capability_source = "custom";
				payload.selected_profile_ids = [];
			} else {
				payload.capability_source = "activated";
				payload.selected_profile_ids = [];
			}
		}

		return payload;
	};

	const bulkCapabilityLevelServerMutation = useMutation<
		ClientCapabilityConfigData | null,
		unknown,
		{ server: ServerSummary }
	>({
		mutationFn: async ({ server }) => {
			if (!identifier) {
				throw new Error("No identifier provided");
			}

			const currentConfig =
				(await clientsApi.getCapabilityConfig(identifier)) ?? effectiveCapabilityConfig;
			if (!currentConfig) {
				throw new Error(
					t("detail.configuration.errors.capabilityConfigMissing", {
						defaultValue:
							"Capability configuration update returned no data. Please try again.",
					}),
				);
			}

			const [toolsResponse, promptsResponse, resourcesResponse, templatesResponse] =
				await Promise.all([
					serversApi.listTools(server.id).catch(() => ({ items: [] })),
					serversApi.listPrompts(server.id).catch(() => ({ items: [] })),
					serversApi.listResources(server.id).catch(() => ({ items: [] })),
					serversApi.listResourceTemplates(server.id).catch(() => ({ items: [] })),
				]);

			const currentUnifyExposure = currentConfig.unify_direct_exposure ?? {
				route_mode: "capability_level" as const,
				selected_server_ids: [],
				selected_tool_surfaces: [],
				selected_prompt_surfaces: [],
				selected_resource_surfaces: [],
				selected_template_surfaces: [],
			};

			const currentToolSurfaces: NonNullable<
				ClientCapabilityConfigData["unify_direct_exposure"]
			>["selected_tool_surfaces"] =
				currentUnifyExposure.selected_tool_surfaces ?? [];
			const currentPromptSurfaces: NonNullable<
				ClientCapabilityConfigData["unify_direct_exposure"]
			>["selected_prompt_surfaces"] =
				currentUnifyExposure.selected_prompt_surfaces ?? [];
			const currentResourceSurfaces: NonNullable<
				ClientCapabilityConfigData["unify_direct_exposure"]
			>["selected_resource_surfaces"] =
				currentUnifyExposure.selected_resource_surfaces ?? [];
			const currentTemplateSurfaces: NonNullable<
				ClientCapabilityConfigData["unify_direct_exposure"]
			>["selected_template_surfaces"] =
				currentUnifyExposure.selected_template_surfaces ?? [];

			const hasAnySelectedForServer =
				getUnifyServerSurfaces(currentToolSurfaces, server.id).length > 0 ||
				getUnifyServerSurfaces(currentPromptSurfaces, server.id).length > 0 ||
				getUnifyServerSurfaces(currentResourceSurfaces, server.id).length > 0 ||
				getUnifyServerSurfaces(currentTemplateSurfaces, server.id).length > 0;

			const rawTools = Array.isArray(toolsResponse.items)
				? (toolsResponse.items as Array<Record<string, unknown>>)
				: [];
			const rawPrompts = Array.isArray(promptsResponse.items)
				? (promptsResponse.items as Array<Record<string, unknown>>)
				: [];
			const rawResources = Array.isArray(resourcesResponse.items)
				? (resourcesResponse.items as Array<Record<string, unknown>>)
				: [];
			const rawTemplates = Array.isArray(templatesResponse.items)
				? (templatesResponse.items as Array<Record<string, unknown>>)
				: [];

			const nextToolSurfaces = resolveNextCapabilityLevelSurfaces(
				currentToolSurfaces,
				server.id,
				hasAnySelectedForServer,
				rawTools
					.map((tool) => String(tool.tool_name ?? tool.name ?? ""))
					.filter(Boolean)
					.map((tool_name) => ({ server_id: server.id, tool_name })),
			);
			const nextPromptSurfaces = resolveNextCapabilityLevelSurfaces(
				currentPromptSurfaces,
				server.id,
				hasAnySelectedForServer,
				rawPrompts
					.map((prompt) => String(prompt.prompt_name ?? prompt.name ?? ""))
					.filter(Boolean)
					.map((prompt_name) => ({ server_id: server.id, prompt_name })),
			);
			const nextResourceSurfaces = resolveNextCapabilityLevelSurfaces(
				currentResourceSurfaces,
				server.id,
				hasAnySelectedForServer,
				rawResources
					.map((resource) => String(resource.resource_uri ?? resource.uri ?? ""))
					.filter(Boolean)
					.map((resource_uri) => ({ server_id: server.id, resource_uri })),
			);
			const nextTemplateSurfaces = resolveNextCapabilityLevelSurfaces(
				currentTemplateSurfaces,
				server.id,
				hasAnySelectedForServer,
				rawTemplates
					.map((template) =>
						String(template.uri_template ?? template.template ?? ""),
					)
					.filter(Boolean)
					.map((uri_template) => ({ server_id: server.id, uri_template })),
			);

			return clientsApi.updateCapabilityConfig({
				identifier,
				capability_source: currentConfig.capability_source,
				selected_profile_ids: currentConfig.selected_profile_ids,
				unify_direct_exposure: {
					...currentUnifyExposure,
					route_mode: "capability_level",
					selected_server_ids: [],
					selected_tool_surfaces: nextToolSurfaces,
					selected_prompt_surfaces: nextPromptSurfaces,
					selected_resource_surfaces: nextResourceSurfaces,
					selected_template_surfaces: nextTemplateSurfaces,
				},
			});
		},
		onSuccess: async () => {
			await Promise.allSettled([
				qc.invalidateQueries({ queryKey: ["client-capability-config", identifier] }),
				qc.invalidateQueries({ queryKey: ["client-config", identifier] }),
				qc.invalidateQueries({ queryKey: ["clients"] }),
				refetchCapabilityConfig(),
				refetchDetails(),
			]);
		},
		onError: (error) =>
			notifyError(
				t("detail.directExposure.notifications.saveFailedTitle", {
					defaultValue: "Unable to update direct capabilities",
				}),
				String(error),
			),
	});

	const buildApplySelectedConfig = (
		capabilityData: ClientCapabilityConfigData,
	): ClientConfigSelected => {
		if (capabilityData.capability_source === "custom") {
			if (!capabilityData.custom_profile_id) {
				throw new Error(
					t("detail.configuration.errors.customProfileMissing", {
						defaultValue:
							"The custom profile has not been provisioned yet. Save again and reopen the editor.",
					}),
				);
			}
			return { profile: { profile_id: capabilityData.custom_profile_id } };
		}

		// For "profiles" and "activated" modes, return "default" to let backend resolve
		// from capability_config.selected_profile_ids (supports multi-profile merge)
		return "default";
	};

	const renderProfileCapabilitySummary = (
		capabilities: NonNullable<ReturnType<typeof profileCapabilities.get>>,
	) => {
		const capabilityItems = [
			{
				key: "servers",
				label: t("detail.configuration.labels.servers", {
					defaultValue: "Servers",
				}),
				counts: capabilities.servers,
			},
			{
				key: "tools",
				label: t("detail.configuration.labels.tools", {
					defaultValue: "Tools",
				}),
				counts: capabilities.tools,
			},
			{
				key: "resources",
				label: t("detail.configuration.labels.resources", {
					defaultValue: "Resources",
				}),
				counts: capabilities.resources,
			},
			{
				key: "prompts",
				label: t("detail.configuration.labels.prompts", {
					defaultValue: "Prompts",
				}),
				counts: capabilities.prompts,
			},
		];

		return (
			<div className="mt-1 flex gap-4 text-xs text-slate-500">
				{capabilityItems.map((item) => (
					<span key={item.key}>
						{item.label}: {item.counts.enabled}/{item.counts.total}
					</span>
				))}
			</div>
		);
	};

	const renderUnifyEligibleServerCapabilitySummary = (
		server: ServerSummary,
		routeMode: "server_live" | "capability_level",
		exposedToolCount: number,
	) => {
		const cap = server.capabilities ?? server.capability;
		const toolsTotal = cap?.tools_count ?? 0;
		const toolsValue =
			routeMode === "capability_level"
				? `${exposedToolCount}/${toolsTotal}`
				: String(toolsTotal);
		const items = [
			{
				key: "tools",
				label: t("detail.configuration.labels.tools", {
					defaultValue: "Tools",
				}),
				value: toolsValue,
			},
			{
				key: "resources",
				label: t("detail.configuration.labels.resources", {
					defaultValue: "Resources",
				}),
				value: String(cap?.resources_count ?? 0),
			},
			{
				key: "prompts",
				label: t("detail.configuration.labels.prompts", {
					defaultValue: "Prompts",
				}),
				value: String(cap?.prompts_count ?? 0),
			},
			{
				key: "resourceTemplates",
				label: t("detail.configuration.labels.resourceTemplates", {
					defaultValue: "Resource templates",
				}),
				value: String(cap?.resource_templates_count ?? 0),
			},
		];
		return (
			<div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-xs text-slate-500">
				{items.map((item) => (
					<span key={item.key}>
						{item.label}: {item.value}
					</span>
				))}
			</div>
		);
	};

	const getConfigurationProfileTokenSlot = (
		profile: ConfigSuit,
		stopPropagationOnNavigate: boolean,
	) => {
		const capData = profileCapabilities.get(profile.id);
		const capIdx = capabilityProfileIds.indexOf(profile.id);
		const capQuery =
			capIdx >= 0 ? profileCapabilitiesQueries[capIdx] : undefined;

		if (capQuery?.isError) {
			return (
				<Link
					to={`/profiles/${profile.id}`}
					onClick={
						stopPropagationOnNavigate ? (e) => e.stopPropagation() : undefined
					}
					className="text-xs text-destructive underline underline-offset-2"
				>
					{t("detail.configuration.labels.openProfileDetail", {
						defaultValue: "Open profile details",
					})}
				</Link>
			);
		}

		if (!capData) {
			return (
				<div
					className="h-14 w-14 shrink-0 animate-pulse rounded-full bg-muted/50"
					aria-hidden
				/>
			);
		}

		return (
			<ConfigurationProfileTokenChart
				profileId={profile.id}
				enabledByComponentId={capData.enabledByComponentId}
				profileServerCount={capData.servers.total}
				stopPropagationOnNavigate={stopPropagationOnNavigate}
			/>
		);
	};

	const { data: policyData, refetch: refetchPolicy } = useQuery({
		queryKey: ["client-policy", identifier],
		queryFn: () => clientsApi.getBackupPolicy(identifier || ""),
		enabled: Boolean(identifier && supportsBackupOperations),
	});

	const reviewMutation = useMutation({
		mutationFn: async (action: "approve" | "reject") => {
			if (!identifier) throw new Error("No identifier provided");
			return action === "approve"
				? clientsApi.approveRecord({ identifier })
				: clientsApi.rejectRecord({ identifier });
		},
		onSuccess: (_, action) => {
			notifySuccess(
				t("detail.notifications.reviewSuccess.title", { defaultValue: "Success" }),
				t(
					action === "approve"
						? "detail.notifications.reviewSuccess.messageApproved"
						: "detail.notifications.reviewSuccess.messageRejected",
					{
						defaultValue:
							action === "approve"
								? "Record approved successfully."
								: "Record rejected successfully.",
					},
				),
			);
			qc.invalidateQueries({ queryKey: ["clients"] });
			qc.invalidateQueries({ queryKey: ["client-config", identifier] });
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.reviewFailed.title", { defaultValue: "Review failed" }),
				String(e),
			),
	});

	const applyMutation = useMutation<
		{
			data: ClientConfigUpdateData | null;
			preview: boolean;
			clientConfigApplied: boolean;
		},
		unknown,
		{ preview: boolean }
	>({
		mutationFn: async ({ preview }) => {
			if (!identifier) throw new Error("No identifier provided");
			if (!canSaveManagementSettings) {
				throw new Error(
					t("detail.configuration.managementSettingsPendingReason", {
						defaultValue:
							"Save management settings after this client leaves pending approval.",
					}),
				);
			}

			// Validate transparent-mode requirements before persisting any management
			// settings, so a failed precondition never leaves partial state in MCPMate.
			if (shouldRequireLocalConfigWrite) {
				if (!canWriteClientConfig) {
					throw new Error(
						t("detail.configuration.writeTargetRequiredReason", {
							defaultValue:
								"Applying governance to the client configuration requires a verified writable local MCP config file.",
						}),
					);
				}
				if (!canApplyTransparentConfig) {
					throw new Error(
						t("detail.configuration.applyRequiresApprovedReason", {
							defaultValue:
								"Applying client configuration requires an approved governance state and a verified local config target.",
						}),
					);
				}
			}

			await clientsApi.update({
				identifier,
				config_mode: mode,
			});

			const capabilityData = await clientsApi.updateCapabilityConfig(buildCapabilityConfigPayload());
			if (!capabilityData) {
				throw new Error(
					t("detail.configuration.errors.capabilityConfigMissing", {
						defaultValue:
							"Capability configuration update returned no data. Please try again.",
					}),
				);
			}

			const selectedConfigForManagedApply =
				mode === "unify" ? "default" : buildApplySelectedConfig(capabilityData);

			if (shouldRequireLocalConfigWrite) {
				const data = await clientsApi.applyConfig({
					identifier,
					mode,
					selected_config: buildApplySelectedConfig(capabilityData),
					preview,
					backup_policy: preview
						? undefined
						: mapDashboardSettingsToClientBackupPolicy(dashboardSettings),
				});
				return { data: data ?? null, preview, clientConfigApplied: true };
			}

			if (!canSyncManagedConfig) {
				return { data: null, preview, clientConfigApplied: false };
			}

			const data = await clientsApi.applyConfig({
				identifier,
				mode,
				selected_config: selectedConfigForManagedApply,
				preview,
				backup_policy: preview
					? undefined
					: mapDashboardSettingsToClientBackupPolicy(dashboardSettings),
			});
			return { data: data ?? null, preview, clientConfigApplied: true };
		},
		onSuccess: ({ data, preview, clientConfigApplied }) => {
			if (preview) {
				if (data) {
					notifyInfo(
						t("detail.notifications.previewReady.title", {
							defaultValue: "Preview ready",
						}),
						t("detail.notifications.previewReady.message", {
							defaultValue: "Review the diff before applying.",
						}),
					);
				} else {
					notifyInfo(
						t("detail.notifications.previewReady.title", {
							defaultValue: "Preview ready",
						}),
						t("detail.notifications.previewReady.noChanges", {
							defaultValue: "No changes detected in this configuration.",
						}),
					);
				}
			} else if (clientConfigApplied) {
				notifySuccess(
					t("detail.notifications.applied.title", {
						defaultValue: "Applied",
					}),
					t("detail.notifications.applied.message", {
						defaultValue: "Configuration applied",
					}),
				);
				refetchBackups();
			} else {
				notifySuccess(
					t("detail.notifications.managementSaved.title", {
						defaultValue: "Saved",
					}),
					t("detail.notifications.managementSaved.message", {
						defaultValue:
							"Management settings were saved in MCPMate. Local client configuration was not updated.",
					}),
				);
			}
			qc.invalidateQueries({ queryKey: ["clients"] });
			qc.invalidateQueries({ queryKey: ["client-config", identifier] });
			qc.invalidateQueries({ queryKey: ["client-capability-config", identifier] });
			setHasUnifyDraftChanges(false);
			refetchCapabilityConfig();
			refetchDetails();
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.applyFailed.title", {
					defaultValue: "Apply failed",
				}),
				String(e),
			),
	});

	const importMutation = useMutation<ClientConfigImportData | null>({
		mutationFn: async () => {
			// If no preview yet, generate one first
			if (!importPreviewData) {
				if (!identifier) throw new Error("No identifier provided");
				const res = await clientsApi.importFromClient(identifier, {
					preview: true,
				});
				setImportPreviewData(res);
				return null; // indicate preview stage; caller handles UI
			}
			if (!identifier) throw new Error("No identifier provided");
			return clientsApi.importFromClient(identifier, { preview: false });
		},
		onSuccess: (res) => {
			// If onSuccess received null means we just did a preview; do not close
			if (!res) return;
			const imported =
				res.imported_servers?.length ?? res.summary?.imported_count ?? 0;
			if (imported > 0) {
				notifySuccess(
					t("detail.notifications.imported.title", {
						defaultValue: "Imported",
					}),
					t("detail.notifications.imported.message", {
						defaultValue: "{{count}} server(s) imported successfully",
						count: imported,
					}),
				);
				setImportPreviewOpen(false);
			} else {
				notifyInfo(
					t("detail.notifications.nothingToImport.title", {
						defaultValue: "Nothing to import",
					}),
					t("detail.notifications.nothingToImport.message", {
						defaultValue:
							"All entries were skipped or no importable servers found.",
					}),
				);
				setImportPreviewOpen(false);
			}
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.importFailed.title", {
					defaultValue: "Import failed",
				}),
				String(e),
			),
	});

	// Header actions: refresh detection and toggle managed
	const refreshDetectMutation = useMutation({
		mutationFn: async () => {
			const data = await clientsApi.list(true);
			if (data?.client) {
				const f = data.client.find((c) => c.identifier === identifier);
				if (f) {
					if (typeof f.display_name === "string")
						setDisplayName(f.display_name);
					setDetected(!!f.detected);
				}
			}
			await refetchDetails();
		},
		onSuccess: () =>
			notifySuccess(
				t("detail.notifications.refreshed.title", {
					defaultValue: "Refreshed",
				}),
				t("detail.notifications.refreshed.message", {
					defaultValue: "Detection refreshed",
				}),
			),
		onError: (e) =>
			notifyError(
				t("detail.notifications.refreshFailed.title", {
					defaultValue: "Refresh failed",
				}),
				String(e),
			),
	});

	const governanceMutation = useMutation<void, unknown, { nextStatus: "approved" | "suspended" }>({
		mutationFn: async ({ nextStatus }) => {
			if (!identifier) throw new Error("No identifier provided");
			if (nextStatus === "approved") {
				await clientsApi.approveRecord({ identifier });
				return;
			}

			await governanceClientsApi.suspendRecord({ identifier });
		},
		onSuccess: async (_, { nextStatus }) => {
			await Promise.allSettled([
				refetchDetails(),
				qc.invalidateQueries({ queryKey: ["clients"] }),
				qc.invalidateQueries({ queryKey: ["client-config", identifier] }),
			]);

			notifySuccess(
				t(
					nextStatus === "approved"
						? "detail.notifications.governanceAllowed.title"
						: "detail.notifications.governanceDenied.title",
					{
						defaultValue: "Updated",
					},
				),
				t(
					nextStatus === "approved"
						? "detail.notifications.governanceAllowed.message"
						: "detail.notifications.governanceDenied.message",
					{
						defaultValue:
							nextStatus === "approved"
								? "Client governance is now allowed."
								: "Client governance is now denied.",
					},
				),
			);
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.governanceFailed.title", {
					defaultValue: "Update failed",
				}),
				String(e),
			),
	});
	const importPreviewMutation = useMutation<ClientConfigImportData>({
		mutationFn: async () => {
			if (!identifier) throw new Error("No identifier provided");
			return clientsApi.importFromClient(identifier, { preview: true });
		},
		onSuccess: (res) => {
			setImportPreviewData(res);
			setImportPreviewOpen(true);
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.previewFailed.title", {
					defaultValue: "Preview failed",
				}),
				String(e),
			),
	});

	const restoreMutation = useMutation({
		mutationFn: ({ backup }: { backup: string }) => {
			if (!identifier) throw new Error("No identifier provided");
			return clientsApi.restoreConfig({ identifier, backup });
		},
		onSuccess: () => {
			notifySuccess(
				t("detail.notifications.restored.title", {
					defaultValue: "Restored",
				}),
				t("detail.notifications.restored.message", {
					defaultValue: "Local client configuration restored from backup",
				}),
			);
			refetchDetails();
			refetchBackups();
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.restoreFailed.title", {
					defaultValue: "Restore failed",
				}),
				String(e),
			),
	});

	const deleteBackupMutation = useMutation({
		mutationFn: ({ backup }: { backup: string }) => {
			if (!identifier) throw new Error("No identifier provided");
			return clientsApi.deleteBackup(identifier, backup);
		},
		onSuccess: () => {
			notifySuccess(
				t("detail.notifications.deleted.title", {
					defaultValue: "Deleted",
				}),
				t("detail.notifications.deleted.message", {
					defaultValue: "Backup deleted",
				}),
			);
			refetchBackups();
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.deleteFailed.title", {
					defaultValue: "Delete failed",
				}),
				String(e),
			),
	});

	const bulkDeleteMutation = useMutation({
		mutationFn: async () => {
			if (!identifier) throw new Error("No identifier provided");
			const items = [...selectedBackups];
			const results = await Promise.allSettled(
				items.map((b) => clientsApi.deleteBackup(identifier, b)),
			);
			const failed = results.filter((r) => r.status === "rejected").length;
			if (failed > 0) throw new Error(`${failed} deletions failed`);
		},
		onSuccess: async () => {
			notifySuccess(
				t("detail.notifications.bulkDeleted.title", {
					defaultValue: "Deleted",
				}),
				t("detail.notifications.bulkDeleted.message", {
					defaultValue: "Selected backups have been deleted",
				}),
			);
			setSelectedBackups([]);
			setBulkConfirmOpen(false);
			await refetchBackups();
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.bulkDeleteFailed.title", {
					defaultValue: "Bulk delete failed",
				}),
				String(e),
			),
	});

	const [confirm, setConfirm] = useState<null | {
		kind: "delete" | "restore";
		backup: string;
	}>(null);

	const setPolicyMutation = useMutation({
		mutationFn: (payload: ClientBackupPolicySetReq) =>
			clientsApi.setBackupPolicy(payload),
		onSuccess: () => {
			notifySuccess(
				t("detail.notifications.saved.title", {
					defaultValue: "Saved",
				}),
				t("detail.notifications.saved.message", {
					defaultValue: "Backup policy updated",
				}),
			);
			refetchPolicy();
		},
		onError: (e) =>
			notifyError(
				t("detail.notifications.saveFailed.title", {
					defaultValue: "Save failed",
				}),
				String(e),
			),
	});

	const [policyLabel, setPolicyLabel] = useState<string>("keep_n");
	const [policyLimit, setPolicyLimit] = useState<number | undefined>(30);
	useEffect(() => {
		if (policyData) {
			setPolicyLabel(policyData.policy || "keep_n");
			setPolicyLimit(policyData.limit ?? undefined);
		}
	}, [policyData]);
	const backupsDisabledByPolicy =
		visibleBackups.length === 0 && (policyData?.policy === "off" || policyData?.policy === "none");

	// Heuristic extract current servers from config content for preview
	const currentServers = useMemo(() => {
		const c = (configDetails as { content?: unknown })?.content;
		try {
			if (!c) return [] as string[];
			if (typeof c === "string") {
				const parsed = JSON.parse(c);
				return extractServers(parsed);
			}
			return extractServers(c);
		} catch {
			return [] as string[];
		}
	}, [configDetails]);

	if (!identifier)
		return (
			<div className="p-4">
				{t("detail.noIdentifier", {
					defaultValue: "No client identifier provided.",
				})}
			</div>
		);

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<div className="flex shrink-0 items-center justify-between">
				<div className="flex flex-col gap-1">
					<div className="flex items-center gap-3 flex-wrap">
						<h2 className="text-3xl font-bold tracking-tight">
							{displayName || identifier}
						</h2>

						{configDetails?.record_kind === "observed_unknown" && (
							<Badge variant="outline" className="bg-yellow-50 text-yellow-800 border-yellow-200 dark:bg-yellow-950 dark:text-yellow-200 dark:border-yellow-800">
								{t("detail.badges.observedUnknown", { defaultValue: "Observed (Unknown)" })}
							</Badge>
						)}
						{configDetails?.approval_status === "pending" && (
							<Badge variant="outline" className="bg-blue-50 text-blue-800 border-blue-200 dark:bg-blue-950 dark:text-blue-200 dark:border-blue-800">
								{t("detail.badges.pendingReview", { defaultValue: "Pending Review" })}
							</Badge>
						)}
						{isRejectedClient && (
							<Badge variant="destructive">
								{t("detail.badges.rejected", { defaultValue: "Rejected" })}
							</Badge>
						)}
						{/* Managed / Detected badges */}
						{typeof configDetails?.managed === "boolean" ? (
							<Badge variant={configDetails.managed ? "secondary" : "outline"}>
								{configDetails.managed
									? t("detail.badges.managed", { defaultValue: "Managed" })
									: t("detail.badges.unmanaged", {
										defaultValue: "Unmanaged",
									})}
							</Badge>
						) : null}
						<Badge variant={detected ? "default" : "secondary"}>
							{detected
								? t("detail.badges.detected", { defaultValue: "Detected" })
								: t("detail.badges.notDetected", {
									defaultValue: "Not Detected",
								})}
						</Badge>
					</div>
					{detailDescription ? (
						<p className="text-sm text-muted-foreground leading-snug w-full truncate">
							{detailDescription}
						</p>
					) : null}
				</div>
				{/* 操作按钮移至 Overview 卡片右上角 */}
			</div>

			<Tabs
				value={tabValue}
				onValueChange={setTabValue}
				className="flex min-h-0 flex-1 flex-col gap-4"
			>
				<div className="flex shrink-0 items-center justify-between">
					<TabsList>
						<TabsTrigger value="overview">
							{t("detail.tabs.overview", { defaultValue: "Overview" })}
						</TabsTrigger>
						<TabsTrigger value="configuration">
							{t("detail.tabs.configuration", {
								defaultValue: "Configuration",
							})}
						</TabsTrigger>
						{configDetails?.writable_config !== false && (
							<TabsTrigger value="backups">
								{t("detail.tabs.backups", { defaultValue: "Backups" })}
							</TabsTrigger>
						)}
					</TabsList>
				</div>

				<TabsContent
					value="overview"
					className="mt-0 flex min-h-0 flex-1 flex-col overflow-y-auto data-[state=inactive]:hidden"
				>
					<div className="grid gap-4">
						<Card>
							{loadingConfig ? (
								<CardContent className="text-sm">
									<div className="animate-pulse h-16 bg-slate-200 dark:bg-slate-800 rounded" />
								</CardContent>
							) : configDetails ? (
								<CardContent className="p-4">
									<div className="flex flex-col gap-4">
										<div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
											<div className="flex flex-wrap items-start gap-4">
												<CachedAvatar
													src={configDetails.logo_url ?? undefined}
													alt={displayName || identifier}
													fallback={displayName || identifier || "C"}
													className="text-sm"
												/>
												<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-sm">
													<span className="text-xs uppercase text-slate-500">
														{t("detail.overview.labels.configPath", {
															defaultValue: "Config Path",
														})}
													</span>
													<span className="font-mono text-xs truncate max-w-[520px]">
														{configDetails.config_path}
													</span>

													<span className="text-xs uppercase text-slate-500">
														{t("detail.overview.labels.lastModified", {
															defaultValue: "Last Modified",
														})}
													</span>
													<span className="text-xs">
														{formatBackupTime(configDetails.last_modified)}
													</span>

													<span className="text-xs uppercase text-slate-500">
														{t("detail.overview.labels.supportedTransports", {
															defaultValue: "Transports",
														})}
													</span>
													<span className="text-xs flex gap-2">
														{(configDetails.supported_transports || []).map(
															(t) => (
																<span
																	key={t}
																	className="inline-flex items-center rounded-full bg-slate-100 dark:bg-slate-800 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide"
																	title={t}
																>
																	{t}
																</span>
															),
														)}
													</span>

													{detailHomepageUrl ? (
														<>
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.homepage", {
																	defaultValue: "Homepage",
																})}
															</span>
															<a
																href={detailHomepageUrl}
																target="_blank"
																rel="noreferrer"
																className="text-xs underline underline-offset-2 truncate"
															>
																{detailHomepageUrl}
															</a>
														</>
													) : null}
													{detailDocsUrl ? (
														<>
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.docs", {
																	defaultValue: "Docs",
																})}
															</span>
															<a
																href={detailDocsUrl}
																target="_blank"
																rel="noreferrer"
																className="text-xs underline underline-offset-2 truncate"
															>
																{detailDocsUrl}
															</a>
														</>
													) : null}
													{detailSupportUrl ? (
														<>
															<span className="text-xs uppercase text-slate-500">
																{t("detail.overview.labels.support", {
																	defaultValue: "Support",
																})}
															</span>
															<a
																href={detailSupportUrl}
																target="_blank"
																rel="noreferrer"
																className="text-xs underline underline-offset-2 truncate"
															>
																{detailSupportUrl}
															</a>
														</>
													) : null}
												</div>
											</div>
											<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
												<Button
													variant="outline"
													size="sm"
													onClick={() => setIsClientFormOpen(true)}
													className="gap-2"
												>
													<Pencil className="h-4 w-4" />
													{t("detail.overview.buttons.edit", {
														defaultValue: "Edit",
													})}
												</Button>
												{configDetails?.approval_status === "pending" && (
													<>
														<Button
															variant="outline"
															size="sm"
															className="bg-green-50 text-green-700 hover:bg-green-100 hover:text-green-800 border-green-200 dark:bg-green-950 dark:text-green-300 dark:hover:bg-green-900 dark:border-green-800 gap-2"
															onClick={() => reviewMutation.mutate("approve")}
															disabled={reviewMutation.isPending}
														>
															<Check className="h-4 w-4" />
															{t("detail.overview.buttons.approve", { defaultValue: "Approve" })}
														</Button>
														<Button
															variant="outline"
															size="sm"
															className="bg-red-50 text-red-700 hover:bg-red-100 hover:text-red-800 border-red-200 dark:bg-red-950 dark:text-red-300 dark:hover:bg-red-900 dark:border-red-800 gap-2"
															onClick={() => reviewMutation.mutate("reject")}
															disabled={reviewMutation.isPending}
														>
															<Trash2 className="h-4 w-4" />
															{t("detail.overview.buttons.reject", { defaultValue: "Reject" })}
														</Button>
													</>
												)}
												<Button
													variant="outline"
													size="sm"
													onClick={() => refreshDetectMutation.mutate()}
													disabled={refreshDetectMutation.isPending}
													className="gap-2"
												>
													<RefreshCw
														className={`h-4 w-4 ${refreshDetectMutation.isPending ? "animate-spin" : ""}`}
													/>
													{t("detail.overview.buttons.refresh", {
														defaultValue: "Refresh",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														governanceMutation.mutate({
															nextStatus: isDeniedApprovalStatus(configDetails?.approval_status)
																? "approved"
																: "suspended",
														})
													}
													disabled={
														governanceMutation.isPending ||
														!configDetails ||
														configDetails.approval_status === "pending"
													}
													className="gap-2"
												>
													{isDeniedApprovalStatus(configDetails?.approval_status) ? (
														<Check className="h-4 w-4" />
													) : (
														<Square className="h-4 w-4" />
													)}
													{isDeniedApprovalStatus(configDetails?.approval_status)
														? t("detail.overview.buttons.allow", {
															defaultValue: "Allow",
														})
														: t("detail.overview.buttons.deny", {
															defaultValue: "Deny",
														})}
												</Button>
												{supportedTransportOptions.length > 0 ? (
													<ButtonGroupText className="h-9 p-0 select-none shadow-none">
														<Select
															value={transport}
															onValueChange={async (v) => {
																setTransport(v);
																try {
																	await clientsApi.update({
																		identifier: identifier!,
																		transport: v,
																	});
																	notifySuccess(
																		t("detail.overview.transport.updated", {
																			defaultValue: "Transport updated",
																		}),
																		"",
																	);
																	// refresh cache so list shows new transport immediately
																	qc.invalidateQueries({
																		queryKey: ["clients"],
																	});
																} catch (err) {
																	notifyError(
																		t(
																			"detail.overview.transport.updateFailed",
																			{ defaultValue: "Update failed" },
																		),
																		String(err),
																	);
																}
															}}
														>
															<SelectTrigger
																className="h-8 border-0 shadow-none focus:ring-0 focus:ring-offset-0 focus-visible:ring-0 focus:outline-none select-none bg-transparent px-3 min-w-[9rem]"
																aria-label={t("detail.overview.transport.selectorAria", {
																	defaultValue: "Transport selector",
																})}
															>
																<SelectValue />
															</SelectTrigger>
															<SelectContent align="end">
																<SelectItem value="auto">
																	{t("detail.configuration.transportOptions.auto", {
																		defaultValue: "Auto",
																	})}
																</SelectItem>
																{supportedTransportOptions.map((v) => (
																	<SelectItem key={v} value={v}>
																		{getTransportOptionLabel(v, t)}
																	</SelectItem>
																))}
															</SelectContent>
														</Select>
													</ButtonGroupText>
												) : null}
											</ButtonGroup>
										</div>
									</div>
								</CardContent>
							) : (
								<CardContent className="text-sm text-slate-500">
									{t("detail.overview.noDetails", {
										defaultValue: "No details available",
									})}
								</CardContent>
							)}
						</Card>
						<Card>
							<CardHeader>
								<div className="flex items-center justify-between">
									<CardTitle>
										{t("detail.overview.currentServers.title", {
											defaultValue: "Current Servers",
										})}
									</CardTitle>
									<div className="flex items-center gap-2">
										<Button
											size="sm"
											variant="outline"
											onClick={() => importPreviewMutation.mutate()}
											disabled={configDetails?.writable_config === false}
										>
											<Download className="mr-2 h-4 w-4" />{" "}
											{t("detail.overview.currentServers.import", {
												defaultValue: "Import from Config",
											})}
										</Button>
									</div>
								</div>
							</CardHeader>
							<CardContent>
								{loadingConfig ? (
									<div className="space-y-2">
										{[1, 2, 3].map((i) => (
											<div
												key={i}
												className="h-8 bg-slate-200 dark:bg-slate-800 animate-pulse rounded-[10px]"
											/>
										))}
									</div>
								) : currentServers.length ? (
									<CapsuleStripeList>
										{currentServers.map((n) => (
											<CapsuleStripeListItem key={n}>
												<div className="font-mono">{n}</div>
												<div className="text-xs text-slate-500">
													{t("detail.overview.currentServers.configuredLabel", {
														defaultValue: "configured",
													})}
												</div>
											</CapsuleStripeListItem>
										))}
									</CapsuleStripeList>
								) : (
									<div className="text-sm text-slate-500">
										{t("detail.overview.currentServers.empty", {
											defaultValue: "No servers extracted from current config.",
										})}
									</div>
								)}
							</CardContent>
						</Card>
						{showClientLiveLogs ? (
							<AuditLogsPanel
								title={t("detail.logs.title", { defaultValue: "Logs" })}
								description={t("detail.logs.description", {
									defaultValue: "Runtime warnings and backend notes for this client.",
								})}
								searchPlaceholder={t("detail.logs.searchPlaceholder", {
									defaultValue: "Search logs...",
								})}
								refreshLabel={t("detail.logs.refresh", { defaultValue: "Refresh Logs" })}
								loadingLabel={t("detail.logs.loading", {
									defaultValue: "Loading logs...",
								})}
								emptyLabel={t("detail.logs.empty", {
									defaultValue: "No log entries recorded for this client yet.",
								})}
								headers={{
									timestamp: t("detail.logs.headers.timestamp", {
										defaultValue: "Timestamp",
									}),
									action: t("detail.logs.headers.action", { defaultValue: "Action" }),
									category: t("detail.logs.headers.category", {
										defaultValue: "Category",
									}),
									status: t("detail.logs.headers.status", { defaultValue: "Status" }),
									target: t("detail.logs.headers.target", { defaultValue: "Target" }),
								}}
								searchValue={logFilter}
								onSearchChange={setLogFilter}
								onRefresh={() => void logsQuery.refetch()}
								rows={filteredLogs}
								isLoading={logsQuery.isLoading}
								isFetching={logsQuery.isFetching}
								isPaginationActionLoading={isLogPaginationActionLoading}
								currentPage={logCurrentPageIndex + 1}
								hasPreviousPage={logCurrentPageIndex > 0}
								hasNextPage={Boolean(logsQuery.data?.next_cursor)}
								itemsPerPage={logPageSize}
								onItemsPerPageChange={setLogPageSize}
								onPreviousPage={handleLogsPrevPage}
								onFirstPage={handleLogsFirstPage}
								onNextPage={handleLogsNextPage}
								onLastPage={() => void handleLogsLastPage()}
								expandLabel={t("detail.logs.expand", { defaultValue: "Expand Logs" })}
								collapseLabel={t("detail.logs.collapse", { defaultValue: "Collapse Logs" })}
							/>
						) : null}
					</div>
				</TabsContent>

				<TabsContent
					value="configuration"
					className={DETAIL_TAB_CONTENT_CLASS}
				>
					<div className="min-h-0 flex-1 overflow-y-auto">
						<div className="grid gap-4">
							<Card>
								<CardHeader className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
									<div>
										<CardTitle>
											{t("detail.configuration.title", {
												defaultValue: "Configuration Mode",
											})}
										</CardTitle>
										<CardDescription>
											{t("detail.configuration.description", {
												defaultValue:
													"If you don't understand what this means, please don't make any changes and keep the current settings.",
											})}
										</CardDescription>
									</div>
									<Button
										size="sm"
										variant="default"
										onClick={() => applyMutation.mutate({ preview: false })}
										disabled={
											loadingConfig ||
											applyMutation.isPending ||
											!canSaveManagementSettings ||
											(shouldRequireLocalConfigWrite && !canApplyTransparentConfig)
										}
										className="gap-2"
									>
										<HardDrive
											className={`h-4 w-4 ${applyMutation.isPending ? "animate-pulse" : ""}`}
										/>
										{t(
											configDetails?.managed
												? "detail.configuration.reapply"
												: "detail.configuration.apply",
											{
												defaultValue: configDetails?.managed ? "Re-Apply" : "Apply",
											},
										)}
									</Button>
								</CardHeader>
								<CardContent className="pt-0">
									<div className="grid grid-cols-10 gap-8">
										{/* Left side - Mode and Source (4/10) */}
										<div className="col-span-4 space-y-6">
											{/* Mode Selection */}
											<div className="space-y-3">
												<div className="space-y-1">
													<h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">
														{t("detail.configuration.sections.mode.title", {
															defaultValue: "1. Management Mode",
														})}
													</h4>
													<p className="text-xs text-slate-500 leading-relaxed">
														{mode === "hosted" &&
															t(
																"detail.configuration.sections.mode.descriptions.hosted",
																{
																	defaultValue:
																		"Hosted keeps a durable managed configuration for this client and remembers the selected working state.",
																},
															)}
														{mode === "unify" &&
															t(
																"detail.configuration.sections.mode.descriptions.unify",
																{
																	defaultValue:
																		"Unify starts with builtin control-plane tools only and keeps its workspace inside the current MCP session.",
																},
															)}
														{mode === "transparent" &&
															t(
																"detail.configuration.sections.mode.descriptions.transparent",
																{
																	defaultValue:
																		"MCPMate writes the selected profile servers directly into this client's MCP configuration and does not preserve capability-level controls.",
																},
															)}
													</p>
												</div>
												<Segment
													value={mode}
													onValueChange={(v) => setMode(v as ClientConfigMode)}
													options={managementModeSegmentOptions}
													showDots={true}
													className="w-full"
												/>
											</div>

											{/* Source Selection */}
											<div className="space-y-3">
												<div className="space-y-1">
													<h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">
														{t("detail.configuration.sections.source.title", {
															defaultValue: "2. Configuration",
														})}
													</h4>
													<p className="text-xs text-slate-500 leading-relaxed">
														{mode === "unify" && unifyRouteMode === "broker_only" &&
															t(
																"detail.configuration.sections.source.descriptions.unify_broker_only",
																{
																	defaultValue:
																		"All capabilities are kept behind the UCAN broker catalog. Builtin MCP tools will browse and call capabilities from globally enabled servers.",
																},
															)}
														{mode === "unify" && unifyRouteMode === "server_live" &&
															t(
																"detail.configuration.sections.source.descriptions.unify_server_live",
																{
																	defaultValue:
																		"Directly expose all capabilities from selected eligible servers to this client. Exposed capabilities are excluded from the UCAN catalog.",
																},
															)}
														{mode === "unify" && unifyRouteMode === "capability_level" &&
															t(
																"detail.configuration.sections.source.descriptions.unify_capability_level",
																{
																	defaultValue:
																		"Directly expose only selected capabilities (tools, prompts, resources, templates) from eligible servers to this client. (Advanced)",
																},
															)}
														{mode === "transparent" && selectedConfig === "default" &&
															t(
																"detail.configuration.sections.source.descriptions.transparentDefault",
																{
																	defaultValue:
																		"Write the servers from all currently activated profiles directly into this client's MCP configuration.",
																},
															)}
														{mode === "transparent" && selectedConfig === "profile" &&
															t(
																"detail.configuration.sections.source.descriptions.transparentProfile",
																{
																	defaultValue:
																		"Write the servers from the selected shared profiles directly into this client's MCP configuration.",
																},
															)}
														{mode === "transparent" && selectedConfig === "custom" &&
															t(
																"detail.configuration.sections.source.descriptions.transparentCustom",
																{
																	defaultValue:
																		"Write the servers from the client-specific custom profile directly into this client's MCP configuration.",
																},
															)}
														{mode === "hosted" && selectedConfig === "default" &&
															t(
																"detail.configuration.sections.source.descriptions.default",
																{
																	defaultValue:
																		"Review the profiles that are currently active for this client runtime.",
																},
															)}
														{mode === "hosted" && selectedConfig === "profile" &&
															t(
																"detail.configuration.sections.source.descriptions.profile",
																{
																	defaultValue:
																		"Browse the shared scene library and choose the exact working set for this client.",
																},
															)}
														{mode === "hosted" && selectedConfig === "custom" &&
															t(
																"detail.configuration.sections.source.descriptions.custom",
																{
																	defaultValue:
																		"Create client-specific adjustments on top of the current unify-mode working state.",
																},
															)}
													</p>
												</div>
												{mode === "unify" ? (
													<Segment
														value={unifyRouteMode}
														onValueChange={(v) => {
															setHasUnifyDraftChanges(true);
															setUnifyRouteMode(v as "broker_only" | "server_live" | "capability_level");
														}}
														options={unifyRouteModeSegmentOptions}
														showDots={true}
														className="w-full"
													/>
												) : (
													<Segment
														value={selectedConfig}
														onValueChange={(v) =>
															setSelectedConfig(v as ClientCapabilitySourceSelection)
														}
														options={configurationSourceSegmentOptions}
														showDots={true}
														className="w-full"
													/>
												)}
											</div>
										</div>

										{/* Right side - Profiles List (6/10) */}
										{(mode === "unify" || mode === "hosted" || mode === "transparent") && (
											<div className="col-span-6">
												<div className="mb-3">
													<h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">
														{mode === "unify" ? t("detail.configuration.sections.exposure.title", {
															defaultValue: "3. Direct Exposure",
														}) : t("detail.configuration.sections.profiles.title", {
															defaultValue: "3. Profiles",
														})}
													</h4>
													{mode === "unify" && unifyRouteMode === "broker_only" && (
														<p className="text-xs text-slate-500 mt-1 leading-relaxed">
															{t("detail.configuration.sections.exposure.descriptions.broker_only", {
																defaultValue:
																	"All enabled MCP servers, including servers marked for direct exposure, remain reachable through the builtin UCAN tools in Broker Only mode.",
															})}
														</p>
													)}
													{mode === "unify" && unifyRouteMode === "server_live" && (
														<p className="text-xs text-slate-500 mt-1 leading-relaxed">
															{t("detail.configuration.sections.exposure.descriptions.server_live", {
																defaultValue:
																	"Select the eligible servers whose tools, prompts, resources, and resource templates should be exposed directly to the client.",
															})}
														</p>
													)}
													{mode === "unify" && unifyRouteMode === "capability_level" && (
														<p className="text-xs text-slate-500 mt-1 leading-relaxed">
															{t("detail.configuration.sections.exposure.descriptions.capability_level", {
																defaultValue:
																	"Toggle each eligible server on the left, then use the more button to configure direct exposure at capability level across tools, prompts, resources, and templates. (Advanced)",
															})}
														</p>
													)}
													{/* Dynamic description based on source */}
													{mode !== "unify" && selectedConfig === "default" && (
														<p className="text-xs text-slate-500 mt-1 leading-relaxed">
															{mode === "transparent"
																? t(
																	"detail.configuration.sections.profiles.descriptions.transparentDefault",
																	{
																		defaultValue:
																			"Transparent mode will write the enabled servers from all currently activated profiles directly into this client's MCP configuration.",
																	},
																)
																: t(
																	"detail.configuration.sections.profiles.descriptions.default",
																	{
																		defaultValue:
																			"Review the profiles that are already active for this client runtime. This view is read-only to keep the active scene set consistent.",
																	},
																)}
														</p>
													)}
													{mode !== "unify" && selectedConfig === "profile" && (
														<p className="text-xs text-slate-500 mt-1 leading-relaxed">
															{mode === "transparent"
																? t(
																	"detail.configuration.sections.profiles.descriptions.transparentProfile",
																	{
																		defaultValue:
																			"Select which shared profiles contribute enabled servers to this client's MCP configuration in transparent mode.",
																	},
																)
																: t(
																	"detail.configuration.sections.profiles.descriptions.profile",
																	{
																		defaultValue:
																			"Choose the reusable shared profiles that define this client's working set.",
																	},
																)}
														</p>
													)}
													{mode !== "unify" && selectedConfig === "custom" && (
														<p className="text-xs text-slate-500 mt-1 leading-relaxed">
															{mode === "transparent"
																? t(
																	"detail.configuration.sections.profiles.descriptions.transparentCustom",
																	{
																		defaultValue:
																			"Transparent mode uses only the enabled servers from this client-specific custom profile when writing the MCP configuration.",
																	},
																)
																: t(
																	"detail.configuration.sections.profiles.descriptions.custom",
																	{
																		defaultValue:
																			"Create and maintain client-specific overrides for the current working state.",
																	},
																)}
														</p>
													)}
												</div>

												{loadingProfiles ? (
													<div className="space-y-2">
														{[1, 2, 3].map((i) => (
															<div
																key={i}
																className="h-12 bg-slate-200 dark:bg-slate-800 animate-pulse rounded"
															/>
														))}
													</div>
												) : (
													<CapsuleStripeList>
														{mode === "unify" ? (
															unifyRouteMode === "broker_only" ? (
																<CapsuleStripeListItem className="items-start">
																	<div className="w-full text-xs leading-relaxed text-slate-500 dark:text-slate-400">
																		{t("detail.configuration.sections.exposure.labels.ucanRoutingDescription", {
																			defaultValue:
																				"In Broker Only mode, all enabled MCP servers — including servers marked for direct exposure — are still accessed through the UCAN catalog and call tools.",
																		})}
																	</div>
																</CapsuleStripeListItem>
															) : eligibleServers.length > 0 ? (
																eligibleServers.map((server) => {
																	const isSelected = unifySelectedServers.includes(server.id);
																	const toolSurfaces = getUnifyServerSurfaces(
																		unifySelectedToolSurfaces,
																		server.id,
																	);
																	const promptSurfaces = getUnifyServerSurfaces(
																		effectiveCapabilityConfig?.unify_direct_exposure
																			?.selected_prompt_surfaces ?? [],
																		server.id,
																	);
																	const resourceSurfaces = getUnifyServerSurfaces(
																		effectiveCapabilityConfig?.unify_direct_exposure
																			?.selected_resource_surfaces ?? [],
																		server.id,
																	);
																	const templateSurfaces = getUnifyServerSurfaces(
																		effectiveCapabilityConfig?.unify_direct_exposure
																			?.selected_template_surfaces ?? [],
																		server.id,
																	);
																	const selectedCapabilityCount =
																		toolSurfaces.length +
																		promptSurfaces.length +
																		resourceSurfaces.length +
																		templateSurfaces.length;
																	const directPath = getClientDirectCapabilitiesPath(identifier, server.id);
																	const isMixed = isUnifyServerMixedRouting(
																		unifyRouteMode,
																		toolSurfaces.length,
																		server.capabilities?.tools_count,
																	);
																	const showDirectSelection =
																		unifyRouteMode === "capability_level"
																			? selectedCapabilityCount > 0
																			: isSelected;
																	const serverDescription =
																		server.meta?.description ||
																		t("detail.configuration.labels.noDescription", {
																			defaultValue: "No description",
																		});

																	return (
																		<CapsuleStripeListItem
																			key={server.id}
																			interactive={true}
																			className={`group relative transition-colors ${showDirectSelection ? "bg-primary/10 ring-1 ring-primary/40" : ""}`}
																			onClick={() => {
																				if (unifyRouteMode === "capability_level") {
																					bulkCapabilityLevelServerMutation.mutate({ server });
																				} else if (unifyRouteMode === "server_live") {
																					setHasUnifyDraftChanges(true);
																					setUnifySelectedServers((prev) =>
																						toggleSelectedServerIds(prev, server.id),
																					);
																				}
																			}}
																		>
																			<CapsuleStripeRowBody
																				lead={
																					<CapsuleStripeLeadCircle
																						variant="toggle"
																						selected={showDirectSelection}
																					/>
																				}
																				trailing={
																					unifyRouteMode === "capability_level" ? (
																						<Button
																							variant="ghost"
																							size="icon"
																							className="h-8 w-8 text-slate-400 hover:text-slate-900 dark:hover:text-slate-100"
																							onClick={(e) => {
																								e.stopPropagation();
																								if (directPath) {
																									navigate(directPath);
																								}
																							}}
																						>
																							<MoreVertical className="h-4 w-4" />
																						</Button>
																					) : undefined
																				}
																			>
																				<div className="font-medium text-sm truncate">
																					{server.name}
																					{unifyRouteMode === "capability_level" && selectedCapabilityCount > 0 && (
																						<span className="ml-2 text-xs font-normal text-slate-500 bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5 rounded-md">
																							{selectedCapabilityCount}{" "}
																							{t("detail.configuration.labels.capabilitiesExposed", {
																								defaultValue: "capabilities exposed",
																							})}
																						</span>
																					)}
																				</div>
																				<div className="text-xs text-slate-500 truncate">{serverDescription}</div>
																				{renderUnifyEligibleServerCapabilitySummary(
																					server,
																					unifyRouteMode,
																					toolSurfaces.length,
																				)}
																				{isMixed && (
																					<div className="mt-1 flex items-center gap-1 text-[11px] font-medium text-amber-600 dark:text-amber-500">
																						<Info className="h-3 w-3" />
																						{t("detail.configuration.warnings.mixedRouting", {
																							defaultValue:
																								"Mixed routing: splitting stateful workflows may cause issues.",
																						})}
																					</div>
																				)}
																			</CapsuleStripeRowBody>
																		</CapsuleStripeListItem>
																	);
																})
															) : (

																<CapsuleStripeListItem>
																	<div className="text-sm text-slate-500 py-4 text-center w-full">
																		{t("detail.configuration.sections.exposure.empty.no_eligible", {
																			defaultValue: "No eligible servers found. Enable Unify Direct Exposure on a server first.",
																		})}
																	</div>
																</CapsuleStripeListItem>
															)
														) : selectedConfig === "default" ? (
															// Show active profiles for default source
															activeProfiles.length > 0 ? (
																activeProfiles.map((profile) => {
																	const capabilities = profileCapabilities.get(
																		profile.id,
																	);
																	return (
																		<CapsuleStripeListItem
																			key={profile.id}
																			className="cursor-default"
																		>
																			<CapsuleStripeRowBody
																				lead={<CapsuleStripeLeadCircle variant="readOnlyActive" />}
																				trailing={getConfigurationProfileTokenSlot(profile, false)}
																			>
																				<div className="font-medium text-sm truncate">{profile.name}</div>
																				<div className="text-xs text-slate-500 truncate">
																					{profile.description ||
																						t("detail.configuration.labels.noDescription", {
																							defaultValue: "No description",
																						})}
																				</div>
																				{capabilities && renderProfileCapabilitySummary(capabilities)}
																			</CapsuleStripeRowBody>
																		</CapsuleStripeListItem>
																	);
																})
															) : (
																<CapsuleStripeListItem>
																	<div className="text-sm text-slate-500 py-4 text-center w-full">
																		{t(
																			"detail.configuration.sections.profiles.empty.active",
																			{
																				defaultValue: "No active profiles found",
																			},
																		)}
																	</div>
																</CapsuleStripeListItem>
															)
														) : selectedConfig === "profile" ? (
															// Show shared profiles for profile source
															sharedProfiles.length > 0 ? (
																sharedProfiles.map((profile) => {
																	const capabilities = profileCapabilities.get(
																		profile.id,
																	);
																	const isSelected = selectedProfiles.includes(
																		profile.id,
																	);
																	return (
																		<CapsuleStripeListItem
																			key={profile.id}
																			interactive={unifyRouteMode !== "broker_only"}
																			className={`group relative transition-colors ${isSelected
																				? "bg-primary/10 ring-1 ring-primary/40"
																				: ""
																				}`}
																			onClick={() => {
																				setSelectedProfiles((prev) =>
																					prev.includes(profile.id)
																						? prev.filter(
																							(id) => id !== profile.id,
																						)
																						: [...prev, profile.id],
																				);
																			}}
																		>
																			<CapsuleStripeRowBody
																				lead={
																					<CapsuleStripeLeadCircle variant="toggle" selected={isSelected} />
																				}
																				trailing={getConfigurationProfileTokenSlot(profile, true)}
																			>
																				<div className="font-medium text-sm truncate">{profile.name}</div>
																				<div className="text-xs text-slate-500 truncate">
																					{profile.description ||
																						t("detail.configuration.labels.noDescription", {
																							defaultValue: "No description",
																						})}
																				</div>
																				{capabilities && renderProfileCapabilitySummary(capabilities)}
																			</CapsuleStripeRowBody>
																		</CapsuleStripeListItem>
																	);
																})
															) : (
																<CapsuleStripeListItem>
																	<div className="text-sm text-slate-500 py-4 text-center w-full">
																		{t(
																			"detail.configuration.sections.profiles.empty.shared",
																			{
																				defaultValue: "No shared profiles found",
																			},
																		)}
																	</div>
																</CapsuleStripeListItem>
															)
														) : null}

														{mode !== "unify" && (
															<CapsuleStripeListItem
																interactive
																className={
																	selectedConfig === "custom" && customProfileId
																		? "border-slate-300 dark:border-slate-600 hover:border-slate-400 dark:hover:border-slate-500"
																		: "border-dashed border-slate-300 dark:border-slate-600 hover:border-slate-400 dark:hover:border-slate-500"
																}
																onClick={() => {
																	if (selectedConfig === "custom") {
																		if (customProfileId) {
																			navigate(`/profiles/${customProfileId}?mode=custom`);
																		} else {
																			applyMutation.mutate({ preview: false });
																		}
																	} else {
																		navigate("/profiles");
																	}
																}}
															>
																<CapsuleStripeRowBody
																	lead={
																		<CapsuleStripeLeadCircle
																			variant="ghost"
																			hasProfile={selectedConfig === "custom" && !!customProfileId}
																		/>
																	}
																>
																	<div className="font-medium text-sm truncate text-slate-700 dark:text-slate-300">
																		{selectedConfig === "custom"
																			? t("detail.configuration.sections.profiles.ghost.titleCustom", {
																				defaultValue: customProfileId
																					? "Customize current state"
																					: "Create custom workspace",
																			})
																			: t("detail.configuration.sections.profiles.ghost.titleDefault", {
																				defaultValue: "Open profiles library",
																			})}
																	</div>
																	<div className="text-xs text-slate-400 dark:text-slate-600 truncate">
																		{selectedConfig === "custom"
																			? t(
																				mode === "transparent"
																					? "detail.configuration.sections.profiles.ghost.subtitleCustomTransparent"
																					: "detail.configuration.sections.profiles.ghost.subtitleCustom",
																				{
																					defaultValue: customProfileId
																						? "Adjust client-specific capabilities on top of the current workspace"
																						: "Create client-specific overrides for this workspace",
																				},
																			)
																			: t("detail.configuration.sections.profiles.ghost.subtitleDefault", {
																				defaultValue:
																					"Browse reusable shared scenes and edit them from the profiles page",
																			})}
																	</div>
																	{customProfileCapabilities
																		? renderProfileCapabilitySummary(customProfileCapabilities)
																		: null}
																</CapsuleStripeRowBody>
															</CapsuleStripeListItem>
														)}
													</CapsuleStripeList>
												)}
											</div>
										)}
									</div>
								</CardContent>
							</Card>
						</div>
					</div>
				</TabsContent>

				<TabsContent
					value="backups"
					className="mt-0 flex min-h-0 flex-1 flex-col overflow-y-auto data-[state=inactive]:hidden"
				>
					<Card>
						<CardHeader className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
							<div>
								<CardTitle>
									{t("detail.backups.title", { defaultValue: "Backups" })}
								</CardTitle>
								<CardDescription>
									{t("detail.backups.description", {
										defaultValue: "Restore or delete configuration snapshots.",
									})}
								</CardDescription>
							</div>
							<div className="flex flex-wrap items-center gap-2">
								<Button
									size="sm"
									variant="outline"
									onClick={() => refetchBackups()}
									disabled={loadingBackups}
								>
									<RefreshCw
										className={`mr-2 h-4 w-4 ${loadingBackups ? "animate-spin" : ""}`}
									/>
									{t("detail.backups.buttons.refresh", {
										defaultValue: "Refresh",
									})}
								</Button>
								{!loadingBackups && visibleBackups.length > 0 && (
									<ButtonGroup>
										<Button
											variant="outline"
											size="sm"
											onClick={() =>
												setSelectedBackups(visibleBackups.map((b) => b.backup))
											}
										>
											{t("detail.backups.buttons.selectAll", {
												defaultValue: "Select all",
											})}
										</Button>
										<Button
											variant="outline"
											size="sm"
											onClick={() => setSelectedBackups([])}
										>
											{t("detail.backups.buttons.clear", {
												defaultValue: "Clear",
											})}
										</Button>
										<Button
											size="sm"
											variant="destructive"
											disabled={
												bulkDeleteMutation.isPending ||
												selectedBackups.length === 0
											}
											onClick={() => setBulkConfirmOpen(true)}
										>
											<Trash2 className="mr-2 h-4 w-4" />
											{t("detail.backups.buttons.deleteSelected", {
												defaultValue: "Delete selected ({{count}})",
												count: selectedBackups.length,
											})}
										</Button>
									</ButtonGroup>
								)}
							</div>
						</CardHeader>
						<CardContent className="space-y-4 pt-0">
							{loadingBackups ? (
								<div className="space-y-2">
									{[1, 2, 3].map((i) => (
										<div
											key={i}
											className="h-10 bg-slate-200 dark:bg-slate-800 animate-pulse rounded"
										/>
									))}
								</div>
							) : visibleBackups.length ? (
								<CapsuleStripeList>
									{visibleBackups.map((b) => {
										const selected = selectedBackups.includes(b.backup);
										return (
											<CapsuleStripeListItem
												key={b.path}
												interactive
												className={`group relative transition-colors ${selected ? "bg-primary/10 ring-1 ring-primary/40" : ""}`}
												onClick={() =>
													setSelectedBackups((prev) =>
														prev.includes(b.backup)
															? prev.filter((x) => x !== b.backup)
															: [...prev, b.backup],
													)
												}
											>
												<div className="flex items-center justify-between flex-1">
													<div className="flex items-center gap-3">
														<div
															className={`flex h-6 w-6 items-center justify-center rounded-full border text-[0px] transition-all duration-200 ${selected
																? "border-primary bg-primary text-white shadow-sm"
																: "border-slate-300 text-transparent group-hover:border-primary/50 group-hover:text-primary/60 dark:border-slate-700 dark:group-hover:border-primary/50"
																}`}
														>
															<Check className="h-3 w-3" />
														</div>
														<div
															className={`font-mono transition-colors duration-200 ${selected
																? "text-primary"
																: "text-slate-700 dark:text-slate-200"
																}`}
														>
															{b.backup}
														</div>
													</div>
													<div className="flex items-center justify-end gap-4">
														<div
															className={`flex items-center gap-4 text-slate-500 transition-all duration-200 ${selected ? "text-primary" : ""
																}`}
														>
															<div>{formatBackupTime(b.created_at)}</div>
															<div>{(b.size / 1024).toFixed(1)} KB</div>
														</div>
														<div className="flex items-center gap-2 overflow-hidden transition-all duration-200 opacity-0 max-w-0 pointer-events-none group-hover:max-w-[12rem] group-hover:opacity-100 group-hover:pointer-events-auto group-focus-within:max-w-[12rem] group-focus-within:opacity-100 group-focus-within:pointer-events-auto">
															<Button
																size="sm"
																variant="outline"
																onClick={(e) => {
																	e.stopPropagation();
																	setConfirm({
																		kind: "restore",
																		backup: b.backup,
																	});
																}}
															>
																<RotateCcw className="mr-2 h-4 w-4" />
																{t("detail.backups.buttons.restore", {
																	defaultValue: "Restore",
																})}
															</Button>
															<Button
																size="icon"
																variant="outline"
																onClick={(e) => {
																	e.stopPropagation();
																	setConfirm({
																		kind: "delete",
																		backup: b.backup,
																	});
																}}
															>
																<Trash2 className="h-4 w-4" />
															</Button>
														</div>
													</div>
												</div>
											</CapsuleStripeListItem>
										);
									})}
								</CapsuleStripeList>
							) : (
								<div className="text-slate-500 text-sm">
									{backupsDisabledByPolicy
										? t("detail.backups.emptyDisabledByPolicy", {
											defaultValue: "Backups are currently disabled by system policy.",
										})
										: t("detail.backups.empty", {
											defaultValue: "No backups.",
										})}
								</div>
							)}
						</CardContent>
					</Card>
				</TabsContent>

			</Tabs>

			<ConfirmDialog
				isOpen={!!confirm}
				onClose={() => setConfirm(null)}
				title={
					confirm?.kind === "delete"
						? t("detail.confirm.deleteTitle", { defaultValue: "Delete Backup" })
						: t("detail.confirm.restoreTitle", {
							defaultValue: "Restore Backup",
						})
				}
				description={
					confirm?.kind === "delete"
						? t("detail.confirm.deleteDescription", {
							defaultValue:
								"Are you sure you want to delete this backup? This action cannot be undone.",
						})
						: t("detail.confirm.restoreDescription", {
							defaultValue:
								"Restore the local client configuration file from the selected backup? MCPMate management mode and capability settings stay unchanged.",
						})
				}
				confirmLabel={
					confirm?.kind === "delete"
						? t("detail.confirm.deleteLabel", { defaultValue: "Delete" })
						: t("detail.confirm.restoreLabel", { defaultValue: "Restore" })
				}
				cancelLabel={t("detail.confirm.cancelLabel", {
					defaultValue: "Cancel",
				})}
				variant={confirm?.kind === "delete" ? "destructive" : "default"}
				isLoading={deleteBackupMutation.isPending || restoreMutation.isPending}
				onConfirm={async () => {
					if (!confirm) return;
					if (confirm.kind === "delete") {
						await deleteBackupMutation.mutateAsync({ backup: confirm.backup });
					} else {
						await restoreMutation.mutateAsync({ backup: confirm.backup });
					}
					setConfirm(null);
				}}
			/>

			{/* Bulk delete confirmation */}
			<ConfirmDialog
				isOpen={bulkConfirmOpen}
				onClose={() => setBulkConfirmOpen(false)}
				title={t("detail.backups.bulk.title", {
					defaultValue: "Delete Selected Backups",
				})}
				description={t("detail.backups.bulk.description", {
					defaultValue:
						"Are you sure you want to delete {{count}} backup(s)? This action cannot be undone.",
					count: selectedBackups.length,
				})}
				confirmLabel={t("detail.confirm.deleteLabel", {
					defaultValue: "Delete",
				})}
				cancelLabel={t("detail.confirm.cancelLabel", {
					defaultValue: "Cancel",
				})}
				variant="destructive"
				isLoading={bulkDeleteMutation.isPending}
				onConfirm={() => bulkDeleteMutation.mutate()}
			/>



			{/* Backup Policy Drawer */}
			<Drawer open={policyOpen} onOpenChange={setPolicyOpen}>
				<DrawerContent>
					<DrawerHeader>
						<DrawerTitle>
							{t("detail.policy.title", { defaultValue: "Backup Policy" })}
						</DrawerTitle>
					</DrawerHeader>
					<div className="p-4 space-y-4">
						<div className="space-y-1">
							<Label>
								{t("detail.policy.fields.policy", { defaultValue: "Policy" })}
							</Label>
							<p className="text-xs text-slate-500">
								{t("detail.policy.fields.policyDescription", {
									defaultValue:
										'Backup retention strategy. For now, only "keep_n" is supported, which keeps at most N recent backups and prunes older ones.',
								})}
							</p>
							<Select value={policyLabel} onValueChange={setPolicyLabel}>
								<SelectTrigger>
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="keep_n">
										{t("detail.policy.fields.options.keepN", {
											defaultValue: "keep_n",
										})}
									</SelectItem>
								</SelectContent>
							</Select>
						</div>
						<div className="space-y-1">
							<Label htmlFor={limitId}>
								{t("detail.policy.fields.limit", { defaultValue: "Limit" })}
							</Label>
							<p className="text-xs text-slate-500">
								{t("detail.policy.fields.limitDescription", {
									defaultValue:
										"Maximum number of backups to keep for this client. Set to 0 for no limit.",
								})}
							</p>
							<Input
								id={limitId}
								type="number"
								min={0}
								value={policyLimit ?? 0}
								onChange={(e) => setPolicyLimit(Number(e.target.value))}
							/>
						</div>
						<div>
							<Button
								onClick={() => {
									if (!identifier) return;
									setPolicyMutation.mutate({
										identifier,
										policy: { policy: policyLabel, limit: policyLimit },
									});
								}}
								disabled={setPolicyMutation.isPending}
							>
								{t("detail.policy.buttons.save", {
									defaultValue: "Save Policy",
								})}
							</Button>
						</div>
					</div>
					<DrawerFooter />
				</DrawerContent>
			</Drawer>

			{/* Import Preview Drawer */}
			<Drawer open={importPreviewOpen} onOpenChange={setImportPreviewOpen}>
				<DrawerContent>
					<DrawerHeader>
						<DrawerTitle>
							{t("detail.importPreview.title", {
								defaultValue: "Import Preview",
							})}
						</DrawerTitle>
						<DrawerDescription>
							{t("detail.importPreview.description", {
								defaultValue:
									"Summary of servers detected from current client config.",
							})}
						</DrawerDescription>
					</DrawerHeader>
					<div className="p-4 text-sm flex flex-col gap-4 max-h-[70vh]">
						{importPreviewMutation.isPending ? (
							<div className="h-16 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
						) : importPreviewData ? (
							<div className="flex-1 min-h-0 flex flex-col gap-4">
								<div className="grid grid-cols-[120px_1fr] gap-y-2 gap-x-4 text-sm leading-6">
									<div className="text-slate-500">
										{t("detail.importPreview.fields.attempted", {
											defaultValue: "Attempted",
										})}
									</div>
									<div>
										{typeof importPreviewData.summary?.attempted === "boolean"
											? importPreviewData.summary.attempted
												? t("states.yes", { defaultValue: "Yes" })
												: t("states.no", { defaultValue: "No" })
											: "-"}
									</div>
									<div className="text-slate-500">
										{t("detail.importPreview.fields.imported", {
											defaultValue: "Imported",
										})}
									</div>
									<div>{importPreviewData.summary?.imported_count ?? 0}</div>
									<div className="text-slate-500">
										{t("detail.importPreview.fields.skipped", {
											defaultValue: "Skipped",
										})}
									</div>
									<div>{importPreviewData.summary?.skipped_count ?? 0}</div>
									<div className="text-slate-500">
										{t("detail.importPreview.fields.failed", {
											defaultValue: "Failed",
										})}
									</div>
									<div>{importPreviewData.summary?.failed_count ?? 0}</div>
								</div>
								{Array.isArray(importPreviewData.items) &&
									importPreviewData.items.length > 0 ? (
									<div className="rounded border">
										<div className="px-3 py-2 text-xs text-slate-500 border-b">
											{t("detail.importPreview.sections.servers", {
												defaultValue: "Servers to import",
											})}
										</div>
										<ul className="divide-y max-h-[30vh] overflow-auto">
											{importPreviewData.items.map((it, idx: number) => (
												<li
													key={`import-item-${idx}-${it.name || it.server_name || "unnamed"}`}
													className="p-3 text-xs"
												>
													<div className="font-medium">
														{it.name || it.server_name || `#${idx + 1}`}
													</div>
													{it.error ? (
														<div className="text-red-500">
															{String(it.error)}
														</div>
													) : null}
													<div className="mt-1 text-slate-500">
														{t("detail.importPreview.sections.stats", {
															defaultValue:
																"tools: {{tools}} • resources: {{resources}} • templates: {{templates}} • prompts: {{prompts}}",
															tools: it.tools?.items?.length ?? 0,
															resources: it.resources?.items?.length ?? 0,
															templates:
																it.resource_templates?.items?.length ?? 0,
															prompts: it.prompts?.items?.length ?? 0,
														})}
													</div>
												</li>
											))}
										</ul>
									</div>
								) : null}
								{importPreviewData.summary?.errors ? (
									<details>
										<summary className="text-xs text-slate-500 cursor-pointer">
											{t("detail.importPreview.sections.errors", {
												defaultValue: "Errors",
											})}
										</summary>
										<pre className="text-xs bg-slate-50 dark:bg-slate-900 p-2 rounded overflow-auto max-h-[26vh]">
											{JSON.stringify(
												importPreviewData.summary.errors,
												null,
												2,
											)}
										</pre>
									</details>
								) : null}
								<details className="mt-2 flex-1 min-h-0">
									<summary className="text-xs text-slate-500 cursor-pointer">
										{t("detail.importPreview.sections.raw", {
											defaultValue: "Raw preview JSON",
										})}
									</summary>
									<pre className="text-xs bg-slate-50 dark:bg-slate-900 p-2 rounded overflow-auto flex-1 min-h-0 max-h-[40vh]">
										{JSON.stringify(importPreviewData, null, 2)}
									</pre>
								</details>
							</div>
						) : (
							<div className="text-slate-500">
								{t("detail.importPreview.noPreview", {
									defaultValue: "No preview data.",
								})}
							</div>
						)}
					</div>
					<DrawerFooter>
						<div className="flex w-full items-center justify-between">
							<Button
								variant="outline"
								onClick={() => setImportPreviewOpen(false)}
							>
								{t("detail.importPreview.buttons.close", {
									defaultValue: "Close",
								})}
							</Button>
							{importPreviewData ? (
								(importPreviewData?.summary?.imported_count ?? 0) > 0 ? (
									<Button
										onClick={() => importMutation.mutate()}
										disabled={importMutation.isPending}
									>
										{t("detail.importPreview.buttons.apply", {
											defaultValue: "Apply Import",
										})}
									</Button>
								) : (
									<div className="text-xs text-slate-500">
										{t("detail.importPreview.states.noImportNeeded", {
											defaultValue: "No import needed",
										})}
									</div>
								)
							) : (
								<Button
									onClick={() => importPreviewMutation.mutate()}
									disabled={importPreviewMutation.isPending}
								>
									{t("detail.importPreview.buttons.preview", {
										defaultValue: "Preview",
									})}
								</Button>
							)}
						</div>
					</DrawerFooter>
				</DrawerContent>
			</Drawer>
			<ClientFormDrawer
				open={isClientFormOpen}
				onOpenChange={setIsClientFormOpen}
				mode="edit"
				client={(currentClient as ClientInfo | undefined) ?? null}
				onSuccess={() => {
					void refetchDetails();
					qc.invalidateQueries({ queryKey: ["clients"] });
				}}
				onDeleteSuccess={() => {
					navigate("/clients");
				}}
			/>



		</div>
	);
}

export default ClientDetailPage;
