import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	Check,
	Edit3,
	Eye,
	GripVertical,
	Play,
	RefreshCw,
	Square,
	Trash2,
} from "lucide-react";
import {
	useCallback,
	useEffect,
	useMemo,
	useState,
	type PointerEvent as ReactPointerEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { capabilityRecordMatchesSearch } from "../../lib/capability-search";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { CachedAvatar } from "../../components/cached-avatar";
import { AuditLogsPanel } from "../../components/audit-logs-panel";
import {
	BulkSelectionCheckbox,
	BulkSelectionHeader,
	useBulkSelection,
	useBulkSelectionLabels,
	useEnableDisableBulkActions,
} from "../../components/bulk-selection";
import CapabilityList, {
	type CapabilityKind,
} from "../../components/capability-list";
import {
	CapabilityPreviewList,
	type CapabilityPreviewFlatItem,
} from "../../components/capability-preview-list";
import { CapabilityToolbar } from "../../components/capability-toolbar";
import { CardListScrollBody } from "../../components/card-list-scroll-body";
import { CAPABILITY_SCROLL_CARD_CLASS } from "../../components/capability-scroll-card-layout";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import { CapsuleStripeRowBody } from "../../components/capsule-stripe-row";
import { ProfileFormDrawer } from "../../components/profile-form-drawer";
import { DETAIL_TAB_CONTENT_CLASS } from "../../components/detail-tab-content-class";
import { ProfileTokenUsageChart } from "./components/profile-token-usage-chart";
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
import { Avatar, AvatarFallback } from "../../components/ui/avatar";
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
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { auditApi, configSuitsApi, serversApi, useProfileTokenChartSource } from "../../lib/api";
import { DEFAULT_ANCHOR_ROLE } from "../../lib/default-profile";
import { notifyError, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import { toTitleCase } from "../../lib/utils";
import type {
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitResourceTemplate,
	ConfigSuitServer,
	ConfigSuitTool,
} from "../../lib/types";
import type { CapabilityRecord } from "../../types/capabilities";

const PROFILE_DETAIL_TABS = [
	"overview",
	"capabilities",
];

const ALL_CAPABILITY_SERVERS_ID = "__all_servers__";

type ProfileFlatCapabilityItem = CapabilityRecord & {
	__profileCapabilityKind: CapabilityKind;
};

function capabilityDetailsCacheToken(items: ProfileFlatCapabilityItem[]) {
	return items
		.map((item) =>
			[
				item.__profileCapabilityKind,
				item.id,
				item.server_id ?? "",
				item.enabled ? "1" : "0",
				item.description ?? "",
			].join(":"),
		)
		.join("|");
}

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

function profileCapabilityDetailKey(
	item: ProfileFlatCapabilityItem,
	kind: CapabilityKind,
) {
	if (kind === "tools") {
		return firstCapabilityString(item, ["tool_name", "name", "unique_name"]);
	}
	if (kind === "resources") {
		return firstCapabilityString(item, ["resource_uri", "uri", "name", "unique_uri"]);
	}
	if (kind === "prompts") {
		return firstCapabilityString(item, ["prompt_name", "name", "unique_name"]);
	}
	return firstCapabilityString(item, [
		"uri_template",
		"uriTemplate",
		"uri",
		"name",
		"unique_uri_template",
	]);
}

type ProfileGlobalServerSummary = {
	name?: string;
	icons?: Array<{ src?: string }>;
};

const compactSelectTriggerClass =
	"relative h-9 w-full min-w-9 px-2 pr-8 [&>span]:min-w-0 [&>span]:truncate [&>svg]:pointer-events-none [&>svg]:absolute [&>svg]:right-2.5 [&>svg]:top-1/2 [&>svg]:-translate-y-1/2";

const formatProfileTypeLabel = (value?: string | null) =>
	value
		?.split(/[\s_]+/)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ") ?? "";

const capabilityKey = (type: string, id: string) => `${type}:${id}`;

const splitCapabilityKey = (
	key: string,
): { capability_type: CapabilityKind; capability_id: string } => {
	const separator = key.indexOf(":");
	return {
		capability_type: key.slice(0, separator) as CapabilityKind,
		capability_id: key.slice(separator + 1),
	};
};

export function ProfileDetailPage() {
	const { t, i18n } = useTranslation();
	usePageTranslations("profiles");
	const { profileId } = useParams<{ profileId: string }>();
	const [searchParams] = useSearchParams();
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	const openServerDetail = useCallback(
		(targetServerId: string) => {
			navigate(`/servers/${encodeURIComponent(targetServerId)}`);
		},
		[navigate],
	);

	/** Refetch capability JSON payloads when server membership or live MCP definitions may have changed. */
	const invalidateProfileCapabilityLedger = useCallback(() => {
		if (profileId) {
			void queryClient.invalidateQueries({ queryKey: ["capabilityTokenLedger", profileId] });
			void queryClient.invalidateQueries({ queryKey: ["profileChartTokenEstimate", profileId] });
		}
	}, [profileId, queryClient]);

	const showProfileLiveLogs = useAppStore(
		(state) => state.dashboardSettings.showProfileLiveLogs,
	);
	const profileTokenEstimateMethod = useAppStore(
		(state) => state.dashboardSettings.profileTokenEstimateMethod,
	);
	const { activeTab, setActiveTab } = useUrlTab({
		paramName: "tab",
		defaultTab: "overview",
		validTabs: PROFILE_DETAIL_TABS,
	});

	const mode = searchParams.get("mode");

	const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
	const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
	// Filters: servers
	const [serverQuery, setServerQuery] = useState("");
	const [serverStatus, setServerStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [selectedCapabilityServerId, setSelectedCapabilityServerId] = useState<
		string
	>(ALL_CAPABILITY_SERVERS_ID);
	const [serverColumnWidth, setServerColumnWidth] = useState(300);
	const [capabilityQuery, setCapabilityQuery] = useState("");
	const [capabilityServerFilters, setCapabilityServerFilters] = useState<
		string[]
	>([]);
	const [capabilityKindFilters, setCapabilityKindFilters] = useState<
		CapabilityKind[]
	>([]);
	const [capabilityStatus, setCapabilityStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const { bulkModeDescription } = useBulkSelectionLabels();
	const serverBulk = useBulkSelection<string>();
	const capabilityBulk = useBulkSelection<string>();
	const [logFilter, setLogFilter] = useState("");
	const [logPageSize, setLogPageSize] = useState<number>(10);
	const [logPageCursors, setLogPageCursors] = useState<string[]>([]);
	const [logCurrentPageIndex, setLogCurrentPageIndex] = useState(0);
	const [isLogPaginationActionLoading, setIsLogPaginationActionLoading] =
		useState(false);
	const logCurrentCursor = logPageCursors[logCurrentPageIndex];

	const profileLogsQuery = useQuery({
		queryKey: [
			"profile-audit-logs",
			profileId,
			logCurrentCursor,
			logPageSize,
			showProfileLiveLogs,
		],
		queryFn: () =>
			auditApi.list({
				limit: logPageSize,
				cursor: logCurrentCursor,
				profile_id: profileId,
			}),
		enabled: Boolean(profileId && showProfileLiveLogs),
		refetchOnWindowFocus: false,
		retry: false,
	});

	useEffect(() => {
		setLogPageCursors([]);
		setLogCurrentPageIndex(0);
	}, [profileId, logPageSize]);

	const filteredProfileLogs = useMemo(() => {
		const logs = profileLogsQuery.data?.events ?? [];
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
	}, [profileLogsQuery.data?.events, logFilter]);

	const handleProfileLogsNextPage = () => {
		if (!profileLogsQuery.data?.next_cursor) return;
		const nextCursor = profileLogsQuery.data.next_cursor;
		setLogPageCursors((prev) => {
			const next = [...prev];
			next[logCurrentPageIndex + 1] = nextCursor;
			return next;
		});
		setLogCurrentPageIndex((prev) => prev + 1);
	};

	const handleProfileLogsPrevPage = () => {
		if (logCurrentPageIndex > 0) {
			setLogCurrentPageIndex((prev) => prev - 1);
		}
	};

	const handleProfileLogsFirstPage = () => {
		setLogCurrentPageIndex(0);
	};

	const handleProfileLogsLastPage = async () => {
		if (!profileLogsQuery.data?.next_cursor || !profileId) return;
		setIsLogPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = profileLogsQuery.data.next_cursor;
			let targetPageIndex = logCurrentPageIndex;
			const nextPageCursors = [...logPageCursors];
			while (nextCursor) {
				targetPageIndex += 1;
				nextPageCursors[targetPageIndex] = nextCursor;
				const page = await auditApi.list({
					limit: logPageSize,
					cursor: nextCursor,
					profile_id: profileId,
				});
				nextCursor = page.next_cursor ?? undefined;
			}
			setLogPageCursors(nextPageCursors);
			setLogCurrentPageIndex(targetPageIndex);
		} finally {
			setIsLogPaginationActionLoading(false);
		}
	};

	const bulkCapabilitiesM = useMutation({
		mutationFn: async ({
			enable,
			ids,
		}: {
			enable: boolean;
			ids: string[];
		}) => {
			const grouped = ids.reduce(
				(acc, key) => {
					const capability = splitCapabilityKey(key);
					if (capability.capability_type === "tools") {
						acc.tools.push(capability.capability_id);
					} else if (capability.capability_type === "resources") {
						acc.resources.push(capability.capability_id);
					} else if (capability.capability_type === "prompts") {
						acc.prompts.push(capability.capability_id);
					} else if (capability.capability_type === "templates") {
						acc.templates.push(capability.capability_id);
					}
					return acc;
				},
				{
					tools: [] as string[],
					resources: [] as string[],
					prompts: [] as string[],
					templates: [] as string[],
				},
			);
			const action = enable ? "enable" : "disable";
			await Promise.all([
				grouped.tools.length
					? configSuitsApi.bulkTools(profileId!, grouped.tools, action)
					: Promise.resolve(),
				grouped.resources.length
					? configSuitsApi.bulkResources(profileId!, grouped.resources, action)
					: Promise.resolve(),
				grouped.prompts.length
					? configSuitsApi.bulkPrompts(profileId!, grouped.prompts, action)
					: Promise.resolve(),
				grouped.templates.length
					? configSuitsApi.bulkResourceTemplates(
							profileId!,
							grouped.templates,
							action,
						)
					: Promise.resolve(),
			]);
		},
		onSuccess: () => {
			capabilityBulk.clearSelection();
			capabilityBulk.exitBulkMode();
			refreshProfileCapabilitySurface();
			notifySuccess(
				t("profiles:detail.messages.capabilitiesUpdated", {
					defaultValue: "Capabilities updated",
				}),
				t("profiles:detail.messages.bulkOperationCompleted", {
					defaultValue: "Bulk operation completed",
				}),
			);
		},
		onError: (e) =>
			notifyError(
				t("profiles:detail.messages.capabilitiesUpdateFailed", {
					defaultValue: "Capabilities update failed",
				}),
				String(e),
			),
	});

	const bulkServersM = useMutation({
		mutationFn: ({ enable, ids }: { enable: boolean; ids: string[] }) =>
			configSuitsApi.bulkServers(
				profileId!,
				ids,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			serverBulk.clearSelection();
			serverBulk.exitBulkMode();
			refreshProfileCapabilitySurface();
			notifySuccess(
				t("profiles:detail.messages.serversUpdated", { defaultValue: "Servers updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.serversUpdateFailed", { defaultValue: "Servers update failed" }),
			String(e)
		),
	});

	// Force cleanup when drawer closes to prevent overlay issues
	useEffect(() => {
		if (!isEditDialogOpen) {
			// 使用 requestAnimationFrame 确保在正确时机清理
			requestAnimationFrame(() => {
				setTimeout(() => {
					// 清理所有可能的遮罩层和覆盖元素
					const overlays = document.querySelectorAll(
						"[data-radix-popper-content-wrapper], [data-radix-dialog-overlay], [data-vaul-overlay], [data-vaul-drawer-wrapper], .fixed.inset-0, [data-vaul-drawer]",
					);
					overlays.forEach((overlay) => {
						const element = overlay as HTMLElement;
						if (
							element.getAttribute("data-state") === "closed" ||
							!element.closest('[data-state="open"]') ||
							element.style.pointerEvents === "none"
						) {
							element.remove();
						}
					});

					// 确保 body 样式被正确重置
					document.body.style.removeProperty("pointer-events");
					document.body.style.removeProperty("overflow");
					document.body.removeAttribute("data-scroll-locked");
					document.body.removeAttribute("aria-hidden");
					document.body.removeAttribute("data-vaul-drawer-wrapper");
				}, 50);
			});
		}
	}, [isEditDialogOpen]);

	// Do not early-return before hooks; guard queries with `enabled`

	// Fetch config suit details
	const {
		data: suit,
		isLoading: isLoadingSuit,
		refetch: refetchSuit,
		isRefetching: isRefetchingSuit,
	} = useQuery({
		queryKey: ["configSuit", profileId],
		queryFn: async () => {
			if (!profileId) return undefined;
			console.log("Fetching profile details for:", profileId);
			const result = await configSuitsApi.getSuit(profileId);
			console.log("Profile details response:", result);
			return result;
		},
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch servers in suit
	const {
		data: serversResponse,
		isLoading: isLoadingServers,
		refetch: refetchServers,
	} = useQuery({
		queryKey: ["configSuitServers", profileId],
		queryFn: async () => {
			if (!profileId) return undefined;
			console.log("Fetching servers for profile:", profileId);
			const result = await configSuitsApi.getServers(profileId);
			console.log("Profile servers response:", result);
			return result;
		},
		enabled: !!profileId,
		retry: 1,
	});
	// Fetch tools in suit
	const {
		data: toolsResponse,
		isLoading: isLoadingTools,
		refetch: refetchTools,
	} = useQuery({
		queryKey: ["configSuitTools", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getTools(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch resources in suit
	const {
		data: resourcesResponse,
		isLoading: isLoadingResources,
		refetch: refetchResources,
	} = useQuery({
		queryKey: ["configSuitResources", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getResources(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch prompts in suit
	const {
		data: promptsResponse,
		isLoading: isLoadingPrompts,
		refetch: refetchPrompts,
	} = useQuery({
		queryKey: ["configSuitPrompts", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getPrompts(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch resource templates in suit
	const {
		data: templatesResponse,
		isLoading: isLoadingTemplates,
		refetch: refetchTemplates,
	} = useQuery({
		queryKey: ["configSuitResourceTemplates", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getResourceTemplates(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	const refreshProfileCapabilitySurface = useCallback((): void => {
		invalidateProfileCapabilityLedger();
		void refetchServers();
		void refetchTools();
		void refetchResources();
		void refetchPrompts();
		void refetchTemplates();
		void queryClient.invalidateQueries({
			queryKey: ["configSuitStats", profileId],
		});
	}, [
		invalidateProfileCapabilityLedger,
		profileId,
		queryClient,
		refetchPrompts,
		refetchResources,
		refetchServers,
		refetchTemplates,
		refetchTools,
	]);

	// Activation/deactivation mutations
	const activateSuitMutation = useMutation({
		mutationFn: () => configSuitsApi.activateSuit(profileId!),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuit", profileId] });
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.profileActivated", { defaultValue: "Profile activated" }),
				t("profiles:detail.messages.profileActivatedDescription", { defaultValue: "Profile has been successfully activated" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.activationFailed", { defaultValue: "Activation failed" }),
				`${t("profiles:detail.messages.activationFailedDescription", { defaultValue: "Failed to activate profile" })}: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	const deactivateSuitMutation = useMutation({
		mutationFn: () => configSuitsApi.deactivateSuit(profileId!),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuit", profileId] });
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.profileDeactivated", { defaultValue: "Profile deactivated" }),
				t("profiles:detail.messages.profileDeactivatedDescription", { defaultValue: "Profile has been successfully deactivated" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.deactivationFailed", { defaultValue: "Deactivation failed" }),
				`${t("profiles:detail.messages.deactivationFailedDescription", { defaultValue: "Failed to deactivate profile" })}: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Delete profile mutation
	const deleteSuitMutation = useMutation({
		mutationFn: () => {
			if (!profileId) return Promise.reject(t("profiles:detail.errors.noSuitId", { defaultValue: "No suit ID" }));
			return configSuitsApi.deleteSuit(profileId);
		},
		onSuccess: () => {
			// Invalidate queries to refresh the profiles list
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.profileDeleted", { defaultValue: "Profile deleted" }),
				t("profiles:detail.messages.profileDeletedDescription", { defaultValue: "Profile has been successfully deleted" })
			);
			navigate("/profiles");
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.deleteFailed", { defaultValue: "Delete failed" }),
				`Failed to delete profile: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Server toggle mutations
	const serverToggleMutation = useMutation({
		mutationFn: ({
			serverId,
			enable,
		}: {
			serverId: string;
			enable: boolean;
		}) => {
			return enable
				? configSuitsApi.enableServer(profileId!, serverId)
				: configSuitsApi.disableServer(profileId!, serverId);
		},
		onSuccess: () => {
			refreshProfileCapabilitySurface();

			notifySuccess(
				t("profiles:detail.messages.serverUpdated", { defaultValue: "Server updated" }),
				"Server status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.serverUpdateFailed", { defaultValue: "Server update failed" }),
				`Failed to update server: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Tool toggle mutations
	const toolToggleMutation = useMutation({
		mutationFn: ({ toolId, enable }: { toolId: string; enable: boolean }) => {
			return enable
				? configSuitsApi.enableTool(profileId!, toolId)
				: configSuitsApi.disableTool(profileId!, toolId);
		},
		onSuccess: () => {
			refreshProfileCapabilitySurface();
			notifySuccess(
				t("profiles:detail.messages.toolUpdated", { defaultValue: "Tool updated" }),
				"Tool status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.toolUpdateFailed", { defaultValue: "Tool update failed" }),
				`Failed to update tool: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Resource toggle mutations
	const resourceToggleMutation = useMutation({
		mutationFn: ({
			resourceId,
			enable,
		}: {
			resourceId: string;
			enable: boolean;
		}) => {
			return enable
				? configSuitsApi.enableResource(profileId!, resourceId)
				: configSuitsApi.disableResource(profileId!, resourceId);
		},
		onSuccess: () => {
			refreshProfileCapabilitySurface();
			notifySuccess(
				t("profiles:detail.messages.resourceUpdated", { defaultValue: "Resource updated" }),
				"Resource status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.resourceUpdateFailed", { defaultValue: "Resource update failed" }),
				`Failed to update resource: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Prompt toggle mutations
	const promptToggleMutation = useMutation({
		mutationFn: ({
			promptId,
			enable,
		}: {
			promptId: string;
			enable: boolean;
		}) => {
			return enable
				? configSuitsApi.enablePrompt(profileId!, promptId)
				: configSuitsApi.disablePrompt(profileId!, promptId);
		},
		onSuccess: () => {
			refreshProfileCapabilitySurface();
			notifySuccess(
				t("profiles:detail.messages.promptUpdated", { defaultValue: "Prompt updated" }),
				"Prompt status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.promptUpdateFailed", { defaultValue: "Prompt update failed" }),
				`Failed to update prompt: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	const suitRole = suit?.role ?? "user";
	const isDefaultAnchor = suitRole === DEFAULT_ANCHOR_ROLE;
	const isHostApp = suit?.suit_type === "host_app";
	const isCustomMode = mode === "custom";

	const handleSuitToggle = () => {
		if (isDefaultAnchor) {
			return;
		}
		if (suit?.is_active) {
			deactivateSuitMutation.mutate();
		} else {
			activateSuitMutation.mutate();
		}
	};

	const handleRefreshAll = () => {
		void refetchSuit();
		refreshProfileCapabilitySurface();
	};
	const overviewActionButtonClass =
		"gap-2 rounded-none first:rounded-l-md last:rounded-r-md";

	const handleEditDrawerClose = (open: boolean) => {
		setIsEditDialogOpen(open);
	};

	const servers = useMemo(
		() => (serversResponse?.servers ?? []) as ConfigSuitServer[],
		[serversResponse],
	);
	const tools = useMemo(
		() => (toolsResponse?.tools ?? []) as ConfigSuitTool[],
		[toolsResponse],
	);
	const resources = useMemo(
		() => (resourcesResponse?.resources ?? []) as ConfigSuitResource[],
		[resourcesResponse],
	);
	const prompts = useMemo(
		() => (promptsResponse?.prompts ?? []) as ConfigSuitPrompt[],
		[promptsResponse],
	);
	const templates = useMemo(
		() => (templatesResponse?.templates ?? []) as ConfigSuitResourceTemplate[],
		[templatesResponse],
	);

	const enabledServers = servers.filter((s: ConfigSuitServer) => s.enabled);
	const enabledTools = tools.filter((t: ConfigSuitTool) => t.enabled);
	const enabledResources = resources.filter(
		(r: ConfigSuitResource) => r.enabled,
	);
	const enabledPrompts = prompts.filter((p: ConfigSuitPrompt) => p.enabled);

	const enabledByComponentId = useMemo(() => {
		const m = new Map<string, boolean>();
		for (const s of servers) {
			m.set(s.id, s.enabled);
		}
		for (const item of tools) {
			m.set(item.id, item.enabled);
		}
		for (const r of resources) {
			m.set(r.id, r.enabled);
		}
		for (const p of prompts) {
			m.set(p.id, p.enabled);
		}
		for (const tmpl of templates as ReadonlyArray<{ id: string; enabled: boolean }>) {
			m.set(tmpl.id, tmpl.enabled);
		}
		return m;
	}, [servers, tools, resources, prompts, templates]);

	const tokenChartSource = useProfileTokenChartSource(profileId, enabledByComponentId);

	// Global servers for availability(connected) calculation
	const { data: globalServersResp } = useQuery({
		queryKey: ["all-servers-for-profile-overview"],
		queryFn: serversApi.getAll,
		staleTime: 30_000,
	});
	const globalServers = globalServersResp?.servers ?? [];
	// For profile counts, available = total in this profile (not global state)
	const availableServersInProfile = servers;

	// Filtered datasets
	const visibleServers = useMemo(
		() =>
			servers.filter((s: ConfigSuitServer) => {
				const queryPass =
					serverQuery.trim() === "" ||
					s.name.toLowerCase().includes(serverQuery.toLowerCase());
				const statusPass =
					serverStatus === "all" ||
					(serverStatus === "enabled" ? s.enabled : !s.enabled);
				return queryPass && statusPass;
			}),
		[serverQuery, serverStatus, servers],
	);

	const capabilityCountsByServerId = useMemo(() => {
		const createCounts = () => ({
			tools: 0,
			resources: 0,
			prompts: 0,
			templates: 0,
			enabled: 0,
			total: 0,
		});
		const counts = new Map<string, ReturnType<typeof createCounts>>();
		const ensure = (serverId: string) => {
			const current = counts.get(serverId);
			if (current) return current;
			const next = createCounts();
			counts.set(serverId, next);
			return next;
		};
		for (const tool of tools) {
			const entry = ensure(tool.server_id);
			entry.tools += 1;
			entry.total += 1;
			if (tool.enabled) entry.enabled += 1;
		}
		for (const resource of resources) {
			const entry = ensure(resource.server_id);
			entry.resources += 1;
			entry.total += 1;
			if (resource.enabled) entry.enabled += 1;
		}
		for (const prompt of prompts) {
			const entry = ensure(prompt.server_id);
			entry.prompts += 1;
			entry.total += 1;
			if (prompt.enabled) entry.enabled += 1;
		}
		for (const template of templates) {
			const entry = ensure(template.server_id);
			entry.templates += 1;
			entry.total += 1;
			if (template.enabled) entry.enabled += 1;
		}
		return counts;
	}, [prompts, resources, templates, tools]);

	const isAllCapabilityServersSelected =
		selectedCapabilityServerId === ALL_CAPABILITY_SERVERS_ID;

	const selectedCapabilityServer = useMemo(() => {
		if (isAllCapabilityServersSelected) return null;
		const candidates = visibleServers.length ? visibleServers : servers;
		return (
			candidates.find((server) => server.id === selectedCapabilityServerId) ??
			null
		);
	}, [isAllCapabilityServersSelected, selectedCapabilityServerId, servers, visibleServers]);

	useEffect(() => {
		if (isAllCapabilityServersSelected) {
			return;
		}
		const stillVisible = visibleServers.some(
			(server) => server.id === selectedCapabilityServerId,
		);
		if (!stillVisible) {
			setSelectedCapabilityServerId(ALL_CAPABILITY_SERVERS_ID);
		}
	}, [isAllCapabilityServersSelected, selectedCapabilityServerId, visibleServers]);

	useEffect(() => {
		if (!isAllCapabilityServersSelected) {
			if (capabilityServerFilters.length > 0) {
				setCapabilityServerFilters([]);
			}
			return;
		}
		const visibleServerIds = new Set(visibleServers.map((server) => server.id));
		const nextFilters = capabilityServerFilters.filter((serverId) =>
			visibleServerIds.has(serverId),
		);
		if (
			nextFilters.length !== capabilityServerFilters.length ||
			nextFilters.length === visibleServers.length
		) {
			if (
				nextFilters.length === 0 &&
				capabilityServerFilters.length === 0
			) {
				return;
			}
			setCapabilityServerFilters(
				nextFilters.length === visibleServers.length ? [] : nextFilters,
			);
		}
	}, [
		capabilityServerFilters,
		isAllCapabilityServersSelected,
		visibleServers,
	]);

	const selectedCapabilityServerIds = useMemo(() => {
		if (isAllCapabilityServersSelected) {
			if (capabilityServerFilters.length > 0) {
				return new Set(capabilityServerFilters);
			}
			return new Set(visibleServers.map((server) => server.id));
		}
		return new Set(selectedCapabilityServer ? [selectedCapabilityServer.id] : []);
	}, [
		capabilityServerFilters,
		isAllCapabilityServersSelected,
		selectedCapabilityServer,
		visibleServers,
	]);
	const visibleServerCapabilityCounts = useMemo(
		() =>
			visibleServers.reduce(
				(acc, server) => {
					const counts = capabilityCountsByServerId.get(server.id);
					if (!counts) {
						return acc;
					}
					acc.enabled += counts.enabled;
					acc.total += counts.total;
					return acc;
				},
				{ enabled: 0, total: 0 },
			),
		[capabilityCountsByServerId, visibleServers],
	);

	const capabilityStatusFilter = useCallback(
		(item: { enabled: boolean }) =>
			capabilityStatus === "all" ||
			(capabilityStatus === "enabled" ? item.enabled : !item.enabled),
		[capabilityStatus],
	);

	const capabilityKindMatches = useCallback(
		(kind: CapabilityKind) =>
			capabilityKindFilters.length === 0 ||
			capabilityKindFilters.includes(kind),
		[capabilityKindFilters],
	);

	const hasCapabilitySelection =
		isAllCapabilityServersSelected || selectedCapabilityServer !== null;

	const selectedServerTools = useMemo(
		() =>
			hasCapabilitySelection
				? tools.filter(
						(tool) =>
							selectedCapabilityServerIds.has(tool.server_id) &&
							capabilityStatusFilter(tool),
					)
				: [],
		[
			capabilityStatusFilter,
			hasCapabilitySelection,
			selectedCapabilityServerIds,
			tools,
		],
	);
	const selectedServerResources = useMemo(
		() =>
			hasCapabilitySelection
				? resources.filter(
						(resource) =>
							selectedCapabilityServerIds.has(resource.server_id) &&
							capabilityStatusFilter(resource),
					)
				: [],
		[
			capabilityStatusFilter,
			hasCapabilitySelection,
			resources,
			selectedCapabilityServerIds,
		],
	);
	const selectedServerPrompts = useMemo(
		() =>
			hasCapabilitySelection
				? prompts.filter(
						(prompt) =>
							selectedCapabilityServerIds.has(prompt.server_id) &&
							capabilityStatusFilter(prompt),
					)
				: [],
		[
			capabilityStatusFilter,
			hasCapabilitySelection,
			prompts,
			selectedCapabilityServerIds,
		],
	);
	const selectedServerTemplates = useMemo(
		() =>
			hasCapabilitySelection
				? templates.filter(
						(template) =>
							selectedCapabilityServerIds.has(template.server_id) &&
							capabilityStatusFilter(template),
					)
				: [],
		[
			capabilityStatusFilter,
			hasCapabilitySelection,
			selectedCapabilityServerIds,
			templates,
		],
	);

	const showToolsSection =
		capabilityKindMatches("tools") &&
		(isLoadingTools || selectedServerTools.length > 0);
	const showResourcesSection =
		capabilityKindMatches("resources") &&
		(isLoadingResources || selectedServerResources.length > 0);
	const showPromptsSection =
		capabilityKindMatches("prompts") &&
		(isLoadingPrompts || selectedServerPrompts.length > 0);
	const showTemplatesSection =
		capabilityKindMatches("templates") &&
		(isLoadingTemplates || selectedServerTemplates.length > 0);

	const visibleCapabilityKeys = useMemo(
		() => [
			...(capabilityKindMatches("tools")
				? selectedServerTools
						.filter((tool) =>
							capabilityRecordMatchesSearch(
								tool as CapabilityRecord,
								capabilityQuery,
							),
						)
						.map((tool) => capabilityKey("tools", tool.id))
				: []),
			...(capabilityKindMatches("resources")
				? selectedServerResources
						.filter((resource) =>
							capabilityRecordMatchesSearch(
								resource as CapabilityRecord,
								capabilityQuery,
							),
						)
						.map((resource) => capabilityKey("resources", resource.id))
				: []),
			...(capabilityKindMatches("prompts")
				? selectedServerPrompts
						.filter((prompt) =>
							capabilityRecordMatchesSearch(
								prompt as CapabilityRecord,
								capabilityQuery,
							),
						)
						.map((prompt) => capabilityKey("prompts", prompt.id))
				: []),
			...(capabilityKindMatches("templates")
				? selectedServerTemplates
						.filter((template) =>
							capabilityRecordMatchesSearch(
								template as CapabilityRecord,
								capabilityQuery,
							),
						)
						.map((template) => capabilityKey("templates", template.id))
				: []),
		],
		[
			capabilityQuery,
			capabilityKindMatches,
			selectedServerPrompts,
			selectedServerResources,
			selectedServerTemplates,
			selectedServerTools,
		],
	);

	const capabilityServerFilterLabel = useMemo(() => {
		if (capabilityServerFilters.length === 0) {
			return t("profiles:detail.filters.server.all", {
				defaultValue: "All Servers",
			});
		}
		if (capabilityServerFilters.length === 1) {
			return (
				visibleServers.find((server) => server.id === capabilityServerFilters[0])
					?.name ??
				t("profiles:detail.placeholders.server", { defaultValue: "Server" })
			);
		}
		return t("profiles:detail.filters.server.selected", {
			count: capabilityServerFilters.length,
			defaultValue: "{{count}} Servers",
		});
	}, [capabilityServerFilters, i18n.language, t, visibleServers]);

	const capabilityKindFilterLabel = useMemo(() => {
		if (capabilityKindFilters.length === 0) {
			return t("profiles:detail.filters.kind.all", {
				defaultValue: "All Types",
			});
		}
		if (capabilityKindFilters.length === 1) {
			const [kind] = capabilityKindFilters;
			if (kind === "tools") {
				return t("profiles:detail.labels.tools", { defaultValue: "Tools" });
			}
			if (kind === "resources") {
				return t("profiles:detail.labels.resources", {
					defaultValue: "Resources",
				});
			}
			if (kind === "prompts") {
				return t("profiles:detail.labels.prompts", { defaultValue: "Prompts" });
			}
			return t("profiles:detail.labels.templates", {
				defaultValue: "Resource Templates",
			});
		}
		return t("profiles:detail.filters.kind.selected", {
			count: capabilityKindFilters.length,
			defaultValue: "{{count}} Types",
		});
	}, [capabilityKindFilters, i18n.language, t]);

	const capabilityStatusLabel = useMemo(() => {
		if (capabilityStatus === "enabled") {
			return t("profiles:detail.filters.status.enabled", {
				defaultValue: "Enabled",
			});
		}
		if (capabilityStatus === "disabled") {
			return t("profiles:detail.filters.status.disabled", {
				defaultValue: "Disabled",
			});
		}
		return t("profiles:detail.filters.status.all", {
			defaultValue: "All",
		});
	}, [capabilityStatus, i18n.language, t]);

	const serverStatusLabel = useMemo(() => {
		if (serverStatus === "enabled") {
			return t("profiles:detail.filters.status.enabled", {
				defaultValue: "Enabled",
			});
		}
		if (serverStatus === "disabled") {
			return t("profiles:detail.filters.status.disabled", {
				defaultValue: "Disabled",
			});
		}
		return t("profiles:detail.filters.status.all", {
			defaultValue: "All",
		});
	}, [i18n.language, serverStatus, t]);

	const toggleCapabilityServerFilter = useCallback(
		(serverId: string, checked: boolean) => {
			setCapabilityServerFilters((current) => {
				if (checked) {
					return current.includes(serverId) ? current : [...current, serverId];
				}
				return current.filter((value) => value !== serverId);
			});
		},
		[],
	);

	const toggleCapabilityKindFilter = useCallback(
		(kind: CapabilityKind, checked: boolean) => {
			setCapabilityKindFilters((current) => {
				if (checked) {
					return current.includes(kind) ? current : [...current, kind];
				}
				return current.filter((value) => value !== kind);
			});
		},
		[],
	);

	const serverBulkActions = useEnableDisableBulkActions(
		serverBulk,
		visibleServers.map((server) => server.id),
		bulkServersM,
	);
	const capabilityBulkActions = useEnableDisableBulkActions(
		capabilityBulk,
		visibleCapabilityKeys,
		bulkCapabilitiesM,
	);

	// Template toggle mutations
	const templateToggleMutation = useMutation({
		mutationFn: ({ templateId, enable }: { templateId: string; enable: boolean }) =>
			enable
				? configSuitsApi.enableResourceTemplate(profileId!, templateId)
				: configSuitsApi.disableResourceTemplate(profileId!, templateId),
		onSuccess: () => {
			refreshProfileCapabilitySurface();
			notifySuccess(
				t("profiles:detail.messages.templateUpdated", { defaultValue: "Template updated" }),
				"Template status has been updated",
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.templateUpdateFailed", { defaultValue: "Template update failed" }),
				`Failed to update template: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	const handleCapabilityDividerPointerDown = useCallback(
		(event: ReactPointerEvent<HTMLButtonElement>) => {
			event.preventDefault();
			const startX = event.clientX;
			const startWidth = serverColumnWidth;
			const handlePointerMove = (moveEvent: PointerEvent) => {
				const nextWidth = startWidth + moveEvent.clientX - startX;
				setServerColumnWidth(Math.min(460, Math.max(240, nextWidth)));
			};
			const handlePointerUp = () => {
				window.removeEventListener("pointermove", handlePointerMove);
				window.removeEventListener("pointerup", handlePointerUp);
			};
			window.addEventListener("pointermove", handlePointerMove);
			window.addEventListener("pointerup", handlePointerUp);
		},
		[serverColumnWidth],
	);

	const loadProfileCapabilityDetails = useCallback(
		async (
			item: ProfileFlatCapabilityItem,
			kind: CapabilityKind,
		): Promise<CapabilityRecord | null> => {
			const serverId =
				typeof item.server_id === "string" ? item.server_id : undefined;
			if (!serverId) {
				return null;
			}

			const key = profileCapabilityDetailKey(item, kind);
			if (!key) {
				return null;
			}

			const detail = await serversApi.getCapabilityDetail(
				serverId,
				kind,
				key,
			);
			return (detail.item ?? null) as CapabilityRecord | null;
		},
		[],
	);

	const renderProfileFlatCapabilityList = useCallback(
		(items: CapabilityPreviewFlatItem[]) => {
			const flatItems: ProfileFlatCapabilityItem[] = items.map(
				({ kind, item }) => ({
					...item,
					__profileCapabilityKind: kind,
				}),
			);

			return (
				<CapabilityList<ProfileFlatCapabilityItem>
					asCard={false}
					kind="tools"
					getKind={(item) => item.__profileCapabilityKind}
					context="profile"
					leadingIcon="kind"
					items={flatItems}
					scrollContainedBody
					enableToggle
					getId={(item) => capabilityKey(item.__profileCapabilityKind, item.id)}
					getEnabled={(item) => !!item.enabled}
					onToggle={(_, next, item) => {
						if (item.__profileCapabilityKind === "tools") {
							toolToggleMutation.mutate({
								toolId: item.id,
								enable: next,
							});
							return;
						}
						if (item.__profileCapabilityKind === "resources") {
							resourceToggleMutation.mutate({
								resourceId: item.id,
								enable: next,
							});
							return;
						}
						if (item.__profileCapabilityKind === "prompts") {
							promptToggleMutation.mutate({
								promptId: item.id,
								enable: next,
							});
							return;
						}
						templateToggleMutation.mutate({
							templateId: item.id,
							enable: next,
						});
					}}
					emptyText={t("profiles:detail.emptyStates.noCapabilitiesForSelection", {
						defaultValue:
							"No capabilities match the current server and status selection.",
					})}
					selectable={capabilityBulk.isBulkMode}
					selectedIds={capabilityBulk.selectedIds}
					onSelectToggle={(id) => capabilityBulk.toggleItem(id)}
					loadDetails={loadProfileCapabilityDetails}
					detailsCacheScope={capabilityDetailsCacheToken(flatItems)}
				/>
			);
		},
		[
			capabilityBulk,
			loadProfileCapabilityDetails,
			promptToggleMutation,
			resourceToggleMutation,
			t,
			templateToggleMutation,
			toolToggleMutation,
		],
	);

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<div className="flex shrink-0 items-center justify-between">
				<div className="flex items-center">
					{suit && (
						<div className="flex items-center gap-3">
							<div className="flex flex-col">
								<div className="flex items-center gap-3">
									<h2 className="text-3xl font-bold tracking-tight">
										{toTitleCase(suit.name)}
									</h2>
									<Badge variant={suit.is_active ? "default" : "secondary"}>
										{suit.suit_type}
									</Badge>
									{suit.is_active && (
										<span className="flex items-center rounded-full bg-emerald-50 px-2 py-1 text-xs font-medium text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400">
											<Check className="mr-1 h-3 w-3" />
											{t("profiles:detail.status.active", { defaultValue: "Active" })}
										</span>
									)}
									{suitRole === DEFAULT_ANCHOR_ROLE ? (
										<Badge variant="outline">{t("profiles:badges.defaultAnchor", { defaultValue: "Default Anchor" })}</Badge>
									) : suit.is_default ? (
										<Badge variant="outline">{t("profiles:badges.inDefault", { defaultValue: "In Default" })}</Badge>
									) : null}
								</div>
							</div>
						</div>
					)}
				</div>
				<div className="flex flex-shrink-0 items-center gap-3">
					{profileId ? (
						<ProfileTokenUsageChart
							ledgerItems={tokenChartSource.ledgerItems}
							fallbackEstimate={tokenChartSource.fallbackEstimate}
							isLoading={tokenChartSource.isLoading}
							isError={tokenChartSource.isError}
							enabledByComponentId={enabledByComponentId}
							estimateMethod={profileTokenEstimateMethod}
							profileServerCount={
								isLoadingServers ? undefined : servers.length
							}
						/>
					) : null}
				</div>
			</div>

			{!profileId ? (
				<Card>
					<CardContent className="p-4">
						<p className="text-center text-slate-500">
							{t("profiles:detail.labels.profileId", { defaultValue: "Profile ID not provided" })}
						</p>
					</CardContent>
				</Card>
			) : isLoadingSuit ? (
				<Card>
					<CardContent className="p-4">
						<div className="h-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
					</CardContent>
				</Card>
			) : suit ? (
				<Tabs
					value={activeTab}
					onValueChange={setActiveTab}
					className="flex min-h-0 flex-1 flex-col gap-4"
				>
					<div className="flex shrink-0 items-center justify-between">
						<TabsList className="flex items-center gap-2">
							<TabsTrigger value="overview">{t("profiles:detail.tabs.overview", { defaultValue: "Overview" })}</TabsTrigger>
							<TabsTrigger value="capabilities">
								{t("profiles:detail.tabs.capabilities", { defaultValue: "Capabilities" })}
							</TabsTrigger>
						</TabsList>
						<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
							<Button
								variant="outline"
								size="sm"
								onClick={handleRefreshAll}
								disabled={isRefetchingSuit}
								className={overviewActionButtonClass}
							>
								<RefreshCw
									className={`h-4 w-4 ${isRefetchingSuit ? "animate-spin" : ""}`}
								/>
								{t("profiles:detail.buttons.refresh", { defaultValue: "Refresh" })}
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => setIsEditDialogOpen(true)}
								className={overviewActionButtonClass}
							>
								<Edit3 className="h-4 w-4" />
								{t("profiles:detail.buttons.edit", { defaultValue: "Edit" })}
							</Button>
						</ButtonGroup>
					</div>

					<TabsContent
						value="overview"
						className="mt-0 flex min-h-0 flex-1 flex-col overflow-y-auto data-[state=inactive]:hidden"
					>
						<div className="grid gap-4">
							<Card>
								<CardContent className="relative p-4">
									{!isHostApp && !isCustomMode && (
										<div className="absolute right-4 top-4">
											<Button
												variant="destructive"
												size="sm"
												onClick={() => setIsDeleteDialogOpen(true)}
												disabled={!!suit?.is_default}
												className="gap-2"
											>
												<Trash2 className="h-4 w-4" />
												{t("profiles:detail.buttons.delete", { defaultValue: "Delete" })}
											</Button>
										</div>
									)}
									<div className="flex flex-col gap-4">
										<div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-start">
											<div className="flex flex-wrap items-start gap-4">
												<Avatar className="text-sm">
													<AvatarFallback>
														{suit.name.slice(0, 1).toUpperCase()}
													</AvatarFallback>
												</Avatar>
												<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-sm">
													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.status", { defaultValue: "Status" })}
													</span>
													<Badge
														variant="secondary"
														className={`justify-self-start border px-2.5 py-0.5 leading-none min-h-[1.5rem] ${suit.is_active
															? "border-emerald-200 bg-emerald-100 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-400/50 dark:bg-emerald-500/20 dark:text-emerald-200"
															: "border-slate-200 bg-slate-50 text-slate-600 hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-800/80 dark:text-slate-300"
															}`}
													>
														{suit.is_active ? t("profiles:detail.status.active", { defaultValue: "Active" }) : t("profiles:detail.status.inactive", { defaultValue: "Inactive" })}
													</Badge>

													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.type", { defaultValue: "Type" })}
													</span>
													<span className="font-mono text-sm leading-tight">
														{t(`profiles:suitTypes.${suit.suit_type}`, {
															defaultValue: formatProfileTypeLabel(suit.suit_type),
														})}
													</span>

													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.multiSelect", { defaultValue: "Multi-select" })}
													</span>
													<span className="text-sm leading-tight">
														{suit.multi_select ? t("profiles:detail.status.yes", { defaultValue: "Yes" }) : t("profiles:detail.status.no", { defaultValue: "No" })}
													</span>

													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.priority", { defaultValue: "Priority" })}
													</span>
													<span className="font-mono text-sm leading-tight">
														{suit.priority}
													</span>
												</div>
											</div>
											<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
												{suitRole === "user" && !isHostApp && !isCustomMode && (
													<Button
														variant="outline"
														size="sm"
														onClick={handleSuitToggle}
														disabled={
															isDefaultAnchor ||
															activateSuitMutation.isPending ||
															deactivateSuitMutation.isPending
														}
														className={overviewActionButtonClass}
													>
														{suit?.is_active ? (
															<Square className="h-4 w-4" />
														) : (
															<Play className="h-4 w-4" />
														)}
														{suit?.is_active
															? t("profiles:detail.buttons.disable", { defaultValue: "Disable" })
															: t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
													</Button>
												)}
											</ButtonGroup>
										</div>
									</div>
								</CardContent>
							</Card>

							<div className="grid grid-cols-2 md:grid-cols-4 gap-4">
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("capabilities")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.servers", { defaultValue: "Servers" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledServers.length}/{availableServersInProfile.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("capabilities")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.tools", { defaultValue: "Tools" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledTools.length}/{tools.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("capabilities")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.resources", { defaultValue: "Resources" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledResources.length}/{resources.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("capabilities")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.prompts", { defaultValue: "Prompts" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledPrompts.length}/{prompts.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
							</div>
							{showProfileLiveLogs ? (
								<AuditLogsPanel
									title={t("profiles:detail.logs.title", { defaultValue: "Logs" })}
									description={t("profiles:detail.logs.description", {
										defaultValue: "Runtime and activity logs related to this profile.",
									})}
									searchPlaceholder={t("profiles:detail.logs.searchPlaceholder", {
										defaultValue: "Search logs...",
									})}
									refreshLabel={t("profiles:detail.logs.refresh", {
										defaultValue: "Refresh Logs",
									})}
									loadingLabel={t("profiles:detail.logs.loading", {
										defaultValue: "Loading logs...",
									})}
									emptyLabel={t("profiles:detail.logs.empty", {
										defaultValue:
											"No log entries recorded for this profile yet.",
									})}
									headers={{
										timestamp: t("profiles:detail.logs.headers.timestamp", {
											defaultValue: "Timestamp",
										}),
										action: t("profiles:detail.logs.headers.action", {
											defaultValue: "Action",
										}),
										category: t("profiles:detail.logs.headers.category", {
											defaultValue: "Category",
										}),
										status: t("profiles:detail.logs.headers.status", {
											defaultValue: "Status",
										}),
										target: t("profiles:detail.logs.headers.target", {
											defaultValue: "Target",
										}),
									}}
									searchValue={logFilter}
									onSearchChange={setLogFilter}
									onRefresh={() => void profileLogsQuery.refetch()}
									rows={filteredProfileLogs}
									isLoading={profileLogsQuery.isLoading}
									isFetching={profileLogsQuery.isFetching}
									isPaginationActionLoading={isLogPaginationActionLoading}
									currentPage={logCurrentPageIndex + 1}
									hasPreviousPage={logCurrentPageIndex > 0}
									hasNextPage={Boolean(profileLogsQuery.data?.next_cursor)}
									itemsPerPage={logPageSize}
									onItemsPerPageChange={setLogPageSize}
									onPreviousPage={handleProfileLogsPrevPage}
									onFirstPage={handleProfileLogsFirstPage}
									onNextPage={handleProfileLogsNextPage}
									onLastPage={() => void handleProfileLogsLastPage()}
									expandLabel={t("profiles:detail.logs.expand", {
										defaultValue: "Expand Logs",
									})}
									collapseLabel={t("profiles:detail.logs.collapse", {
										defaultValue: "Collapse Logs",
									})}
								/>
							) : null}
						</div>
					</TabsContent>

					<TabsContent value="capabilities" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className={CAPABILITY_SCROLL_CARD_CLASS}>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
								<div
									className="grid min-h-0 flex-1 overflow-hidden"
									style={{
										gridTemplateColumns: `${serverColumnWidth}px 8px minmax(0, 1fr)`,
									}}
								>
									<div className="flex min-h-0 flex-col">
										<div className="shrink-0 p-3">
											<BulkSelectionHeader
												className="mb-3"
												title={t("profiles:detail.labels.servers", {
													defaultValue: "Servers",
												})}
												description={
													serverBulk.isBulkMode
														? bulkModeDescription(serverBulk.selectedCount)
														: t("profiles:detail.descriptions.capabilityServers", {
															defaultValue:
																"Select a server to manage its profile capabilities.",
														})
												}
												isBulkMode={serverBulk.isBulkMode}
												onToggleBulkMode={serverBulk.toggleMode}
												actions={serverBulkActions}
											/>
												<div className="grid min-w-0 grid-cols-[minmax(0,3fr)_minmax(2.25rem,1fr)] gap-2">
													<Input
														placeholder={t(
															"profiles:detail.placeholders.searchServers",
															{ defaultValue: "Search servers..." },
														)}
														value={serverQuery}
														onChange={(e) => setServerQuery(e.target.value)}
														className="h-9 min-w-0"
													/>
												<Select
													value={serverStatus}
													onValueChange={(v) =>
														setServerStatus(v as "all" | "enabled" | "disabled")
													}
												>
														<SelectTrigger
															title={serverStatusLabel}
															className={compactSelectTriggerClass}
														>
														<SelectValue
															placeholder={t(
																"profiles:detail.placeholders.status",
																{ defaultValue: "Status" },
															)}
														/>
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.status.all", {
																defaultValue: "All",
															})}
														</SelectItem>
														<SelectItem value="enabled">
															{t("profiles:detail.filters.status.enabled", {
																defaultValue: "Enabled",
															})}
														</SelectItem>
														<SelectItem value="disabled">
															{t("profiles:detail.filters.status.disabled", {
																defaultValue: "Disabled",
															})}
														</SelectItem>
													</SelectContent>
												</Select>
											</div>
										</div>
										<CardListScrollBody className="mx-3 mb-3 mt-0">
											{isLoadingServers ? (
												<div className="space-y-3">
													{["s1", "s2", "s3"].map((id) => (
														<div
															key={`capabilities-server-skel-${id}`}
															className="h-16 animate-pulse rounded-md bg-slate-200 dark:bg-slate-800"
														/>
													))}
												</div>
											) : visibleServers.length > 0 ? (
													<CapsuleStripeList className="rounded-none border-0 overflow-visible">
														<CapsuleStripeListItem
															key={ALL_CAPABILITY_SERVERS_ID}
															interactive
															className={`group relative px-3 transition-colors ${
																isAllCapabilityServersSelected
																	? "bg-primary/10"
																	: ""
															}`}
															onClick={() =>
																setSelectedCapabilityServerId(
																	ALL_CAPABILITY_SERVERS_ID,
																)
															}
															onKeyDown={(event) => {
																if (event.key === "Enter" || event.key === " ") {
																	event.preventDefault();
																	setSelectedCapabilityServerId(
																		ALL_CAPABILITY_SERVERS_ID,
																	);
																}
															}}
														>
															<CapsuleStripeRowBody
																lead={
																	<div className="flex h-9 w-9 items-center justify-center rounded-md border border-slate-200 bg-white text-[10px] font-semibold uppercase text-slate-600 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
																		{t("profiles:detail.labels.allServersShort", {
																			defaultValue: "All",
																		})}
																	</div>
																}
																trailing={
																	<Badge variant="outline">
																		{visibleServers.length}
																	</Badge>
																}
															>
																<div className="min-w-0">
																	<div
																		className="truncate font-medium text-slate-900 dark:text-slate-100"
																		title={t("profiles:detail.labels.allServers", {
																			defaultValue: "All servers",
																		})}
																	>
																		{t("profiles:detail.labels.allServers", {
																			defaultValue: "All servers",
																		})}
																	</div>
																	<div
																		className="mt-1 truncate text-xs text-slate-500"
																		title={`${visibleServerCapabilityCounts.enabled}/${visibleServerCapabilityCounts.total} ${t(
																			"profiles:detail.labels.enabledCapabilities",
																			{
																				defaultValue: "enabled capabilities",
																			},
																		)}`}
																	>
																		{visibleServerCapabilityCounts.enabled}/
																		{visibleServerCapabilityCounts.total}{" "}
																		{t(
																			"profiles:detail.labels.enabledCapabilities",
																			{
																				defaultValue: "enabled capabilities",
																			},
																		)}
																	</div>
																</div>
															</CapsuleStripeRowBody>
														</CapsuleStripeListItem>
														{visibleServers.map((server) => {
															const global = (
																globalServers as ProfileGlobalServerSummary[]
															).find(
																(gs) => gs.name === server.name,
															);
															const globalIcon = global?.icons?.[0]?.src;
															const iconAlt =
																global?.name || server.name || server.id;
															const avatarFallback = (server.name || server.id || "S")
																.slice(0, 1)
																.toUpperCase();
															const counts =
																capabilityCountsByServerId.get(server.id) ?? {
																	enabled: 0,
																	prompts: 0,
																	resources: 0,
																	templates: 0,
																	tools: 0,
																	total: 0,
																};
															const isSelected =
																selectedCapabilityServer?.id === server.id;
															const bulkSelected =
																serverBulk.isBulkMode &&
																serverBulk.selectedIdSet.has(server.id);
															let serverItemStateClass = "";
															if (isSelected) {
																serverItemStateClass = "bg-primary/10";
															} else if (bulkSelected) {
																serverItemStateClass = "bg-accent/40";
															}
															const serverLeadClassName = serverBulk.isBulkMode
																? "flex items-center gap-3"
																: "flex items-center gap-0";

															return (
																<CapsuleStripeListItem
																	key={server.id}
																	interactive
																	className={`group relative px-3 transition-colors ${serverItemStateClass}`}
																	onClick={() => setSelectedCapabilityServerId(server.id)}
																	onKeyDown={(event) => {
																		if (event.key === "Enter" || event.key === " ") {
																			event.preventDefault();
																			setSelectedCapabilityServerId(server.id);
																		}
																	}}
																>
																	<CapsuleStripeRowBody
																		lead={
																			<div className={serverLeadClassName}>
																				<BulkSelectionCheckbox
																					visible={serverBulk.isBulkMode}
																					checked={bulkSelected}
																					onToggle={() =>
																						serverBulk.toggleItem(server.id)
																					}
																					ariaLabel={t(
																						"profiles:detail.bulk.selectItem",
																						{
																							name: server.name,
																							defaultValue: "Select {{name}}",
																						},
																					)}
																				/>
																				<CachedAvatar
																					src={globalIcon}
																					alt={
																						iconAlt ? `${iconAlt} icon` : undefined
																					}
																					fallback={avatarFallback}
																					size="sm"
																					shape="rounded"
																					className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
																				/>
																			</div>
																		}
																		trailing={
																			<div className="flex w-[4.25rem] shrink-0 items-center justify-end gap-1">
																				{!serverBulk.isBulkMode ? (
																					<button
																						type="button"
																						className="flex h-7 w-7 shrink-0 items-center justify-center border-0 bg-transparent p-0 text-muted-foreground opacity-0 shadow-none transition-[color,opacity] hover:bg-transparent hover:text-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent/60 group-hover:opacity-100"
																						onClick={(event) => {
																							event.stopPropagation();
																							openServerDetail(server.id);
																						}}
																						aria-label={t(
																							"profiles:detail.labels.browseServer",
																							{ defaultValue: "Browse server" },
																						)}
																					>
																						<Eye className="h-4 w-4" />
																					</button>
																				) : null}
																				<Switch
																					checked={server.enabled}
																					onClick={(e) => e.stopPropagation()}
																					onCheckedChange={(enabled) =>
																						serverToggleMutation.mutate({
																							serverId: server.id,
																							enable: enabled,
																						})
																					}
																					disabled={serverToggleMutation.isPending}
																				/>
																			</div>
																		}
																>
																	<div className="min-w-0">
																		<div
																			className="truncate font-medium text-slate-900 dark:text-slate-100"
																			title={server.name}
																		>
																			{server.name}
																		</div>
																		<div
																			className="mt-1 truncate text-xs text-slate-500"
																			title={`${counts.enabled}/${counts.total} ${t(
																				"profiles:detail.labels.enabledCapabilities",
																				{
																					defaultValue: "enabled capabilities",
																				},
																			)}`}
																		>
																			{counts.enabled}/{counts.total}{" "}
																			{t("profiles:detail.labels.enabledCapabilities", {
																				defaultValue: "enabled capabilities",
																			})}
																		</div>
																	</div>
																</CapsuleStripeRowBody>
															</CapsuleStripeListItem>
														);
													})}
												</CapsuleStripeList>
											) : (
												<div className="flex min-h-full items-center justify-center px-4 py-8 text-center text-sm text-muted-foreground">
													{t("profiles:detail.emptyStates.noServers", {
														defaultValue: "No servers found in this profile",
													})}
												</div>
											)}
										</CardListScrollBody>
									</div>

									<button
										type="button"
										aria-label={t("profiles:detail.labels.resizeCapabilityColumns", {
											defaultValue: "Resize capability columns",
										})}
										className="group flex cursor-col-resize items-center justify-center border-x border-border bg-muted/20 text-muted-foreground transition-colors hover:bg-muted/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
										onPointerDown={handleCapabilityDividerPointerDown}
									>
										<GripVertical className="h-4 w-4 opacity-50 group-hover:opacity-80" />
									</button>

									<div className="flex min-h-0 flex-col">
										<div className="shrink-0 p-3">
											<BulkSelectionHeader
												className="mb-3"
												title={
													isAllCapabilityServersSelected
														? t("profiles:detail.labels.allServers", {
																defaultValue: "All servers",
															})
														: selectedCapabilityServer
															? selectedCapabilityServer.name
															: t("profiles:detail.labels.capabilities", {
																	defaultValue: "Capabilities",
																})
												}
												description={
													isAllCapabilityServersSelected
														? t("profiles:detail.descriptions.allCapabilityGroups", {
																defaultValue:
																	"Manage tools, resources, prompts, and resource templates across the visible servers.",
															})
														: selectedCapabilityServer
															? t("profiles:detail.descriptions.capabilityGroups", {
																	defaultValue:
																		"Manage tools, resources, prompts, and resource templates for the selected server.",
																})
															: t("profiles:detail.emptyStates.selectServer", {
																	defaultValue:
																		"Select a server to inspect its capabilities.",
																})
												}
												isBulkMode={capabilityBulk.isBulkMode}
												onToggleBulkMode={capabilityBulk.toggleMode}
												actions={capabilityBulkActions}
											/>
											<CapabilityToolbar
												searchValue={capabilityQuery}
												onSearchChange={setCapabilityQuery}
												searchPlaceholder={t(
													"profiles:detail.placeholders.searchCapabilities",
													{ defaultValue: "Search capabilities..." },
												)}
												serverFilter={
													isAllCapabilityServersSelected
														? {
																label: capabilityServerFilterLabel,
																allLabel: t(
																	"profiles:detail.filters.server.all",
																	{ defaultValue: "All Servers" },
																),
																options: visibleServers.map((server) => ({
																	value: server.id,
																	label: server.name,
																	title: server.name,
																})),
																selectedValues: capabilityServerFilters,
																onClear: () => setCapabilityServerFilters([]),
																onToggle: toggleCapabilityServerFilter,
															}
														: undefined
												}
												kindFilter={{
													label: capabilityKindFilterLabel,
													allLabel: t("profiles:detail.filters.kind.all", {
														defaultValue: "All Types",
													}),
													options: [
														{
															value: "tools",
															label: t("profiles:detail.labels.tools", {
																defaultValue: "Tools",
															}),
														},
														{
															value: "resources",
															label: t("profiles:detail.labels.resources", {
																defaultValue: "Resources",
															}),
														},
														{
															value: "prompts",
															label: t("profiles:detail.labels.prompts", {
																defaultValue: "Prompts",
															}),
														},
														{
															value: "templates",
															label: t("profiles:detail.labels.templates", {
																defaultValue: "Resource Templates",
															}),
														},
													],
													selectedValues: capabilityKindFilters,
													onClear: () => setCapabilityKindFilters([]),
													onToggle: (value, checked) =>
														toggleCapabilityKindFilter(
															value as CapabilityKind,
															checked,
														),
												}}
												statusFilter={{
													label: capabilityStatusLabel,
													value: capabilityStatus,
													placeholder: t(
														"profiles:detail.placeholders.status",
														{ defaultValue: "Status" },
													),
													options: [
														{
															value: "all",
															label: t("profiles:detail.filters.status.all", {
																defaultValue: "All",
															}),
														},
														{
															value: "enabled",
															label: t(
																"profiles:detail.filters.status.enabled",
																{ defaultValue: "Enabled" },
															),
														},
														{
															value: "disabled",
															label: t(
																"profiles:detail.filters.status.disabled",
																{ defaultValue: "Disabled" },
															),
														},
													],
													onValueChange: (value) =>
														setCapabilityStatus(
															value as "all" | "enabled" | "disabled",
														),
												}}
											/>
										</div>
										<CapabilityPreviewList
											className="mx-3 mb-3 mt-0"
											contentClassName="flex min-h-0 flex-1 flex-col p-0"
											framed={false}
											showHeader={false}
											hasSource={hasCapabilitySelection}
											isLoading={
												isLoadingTools ||
												isLoadingResources ||
												isLoadingPrompts ||
												isLoadingTemplates
											}
											searchValue={capabilityQuery}
											tools={
												showToolsSection
													? (selectedServerTools as CapabilityRecord[])
													: []
											}
											resources={
												showResourcesSection
													? (selectedServerResources as CapabilityRecord[])
													: []
											}
											prompts={
												showPromptsSection
													? (selectedServerPrompts as CapabilityRecord[])
													: []
											}
											templates={
												showTemplatesSection
													? (selectedServerTemplates as CapabilityRecord[])
													: []
											}
											selectHintText={t("profiles:detail.emptyStates.selectServer", {
												defaultValue: "Select a server to inspect its capabilities.",
											})}
											emptyText={t(
												"profiles:detail.emptyStates.noCapabilitiesForSelection",
												{
													defaultValue:
														"No capabilities match the current server and status selection.",
												},
											)}
											emptySearchText={t(
												"profiles:detail.emptyStates.noCapabilitiesForSelection",
												{
													defaultValue:
														"No capabilities match the current server and status selection.",
												},
											)}
											renderFlatList={renderProfileFlatCapabilityList}
										/>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>


				</Tabs>
			) : (
				<Card>
					<CardContent className="p-4">
						<p className="text-center text-slate-500">
							{t("profiles:detail.emptyStates.profileNotFound", {
								defaultValue: "Profile not found",
							})}
						</p>
					</CardContent>
				</Card>
			)}

			{/* Edit Suit Drawer */}
			<ProfileFormDrawer
				open={isEditDialogOpen}
				onOpenChange={handleEditDrawerClose}
				mode="edit"
				suit={suit}
				restrictProfileType={isHostApp ? "host_app" : undefined}
				onSuccess={() => {
					handleEditDrawerClose(false);
					handleRefreshAll();
				}}
			/>

			{/* Delete Confirmation Dialog */}
			<AlertDialog
				open={isDeleteDialogOpen}
				onOpenChange={setIsDeleteDialogOpen}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>
							{t("profiles:detail.dialogs.deleteTitle", {
								defaultValue: "Delete Configuration Profile",
							})}
						</AlertDialogTitle>
						<AlertDialogDescription>
							{t("profiles:detail.dialogs.deleteDescription", {
								defaultValue:
									'Are you sure you want to delete "{{name}}"? This action cannot be undone. All associated configurations will be permanently removed.',
								name: suit?.name ?? "",
							})}
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>
							{t("profiles:form.buttons.cancel", { defaultValue: "Cancel" })}
						</AlertDialogCancel>
						<AlertDialogAction
							onClick={() => {
								deleteSuitMutation.mutate();
								setIsDeleteDialogOpen(false);
							}}
							className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
							disabled={deleteSuitMutation.isPending}
						>
							{deleteSuitMutation.isPending ? t("profiles:detail.buttons.deleting", { defaultValue: "Deleting..." }) : t("profiles:detail.buttons.delete", { defaultValue: "Delete" })}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</div>
	);
}
