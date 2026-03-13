import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
    BadgeCheck,
    Bug,
    Check,
    Edit3,
    Play,
    RefreshCw,
    Square,
    Trash2,
    Eye,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { CachedAvatar } from "../../components/cached-avatar";
import CapabilityList from "../../components/capability-list";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import { ProfileFormDrawer } from "../../components/profile-form-drawer";
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
	CardDescription,
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
import { configSuitsApi, serversApi } from "../../lib/api";
import { notifyError, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type {
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitServer,
	ConfigSuitTool,
} from "../../lib/types";

const toTitleCase = (value?: string | null) =>
	(value ?? "")
		.trim()
		.split(/[\s_-]+/)
		.filter(Boolean)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
		.join(" ") ||
	value ||
	"";

const formatProfileTypeLabel = (value?: string | null) =>
	value
		?.split(/[\s_]+/)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ") ?? "";

export function ProfileDetailPage() {
	const { t } = useTranslation();
	usePageTranslations("profiles");
	const { profileId } = useParams<{ profileId: string }>();
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const [activeTab, setActiveTab] = useState("overview");

	// Developer toggles
	const enableServerDebug = useAppStore(
		(state) => state.dashboardSettings.enableServerDebug,
	);
	const openDebugInNewWindow = useAppStore(
		(state) => state.dashboardSettings.openDebugInNewWindow,
	);

    const openDebug = (
        targetServerId: string,
        channel: "proxy" | "native" = "proxy",
    ) => {
        const url = `/servers/${encodeURIComponent(targetServerId)}?view=debug&channel=${channel}`;
        if (openDebugInNewWindow) {
            if (typeof window !== "undefined") {
                window.open(url, "_blank", "noopener,noreferrer");
            }
            return;
        }
        navigate(url);
    };
    const openBrowse = (targetServerId: string) => {
        const url = `/servers/${encodeURIComponent(targetServerId)}?view=browse`;
        if (openDebugInNewWindow) {
            if (typeof window !== "undefined") {
                window.open(url, "_blank", "noopener,noreferrer");
            }
            return;
        }
        navigate(url);
    };
	const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
	const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
	// Filters: servers
	const [serverQuery, setServerQuery] = useState("");
	const [serverStatus, setServerStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	// Filters: tools
	const [toolQuery, setToolQuery] = useState("");
	const [toolStatus, setToolStatus] = useState<"all" | "enabled" | "disabled">(
		"all",
	);
	const [toolServer, setToolServer] = useState<string>("all");
	// Filters: resources
	const [resourceQuery, setResourceQuery] = useState("");
	const [resourceStatus, setResourceStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [resourceServer, setResourceServer] = useState<string>("all");
	// Filters: prompts
	const [promptQuery, setPromptQuery] = useState("");
	const [promptStatus, setPromptStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [promptServer, setPromptServer] = useState<string>("all");
	// Bulk selection states for lists
	const [selectedServerIds, setSelectedServerIds] = useState<string[]>([]);
	const [selectedToolIds, setSelectedToolIds] = useState<string[]>([]);
	const [selectedResourceIds, setSelectedResourceIds] = useState<string[]>([]);
	const [selectedPromptIds, setSelectedPromptIds] = useState<string[]>([]);
	const [selectedTemplateIds, setSelectedTemplateIds] = useState<string[]>([]);

	// Bulk mutations using server-side batch manage to improve reliability
	const bulkToolsM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkTools(
				profileId!,
				selectedToolIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedToolIds([]);
			refetchTools();
			notifySuccess(
				t("profiles:detail.messages.toolsUpdated", { defaultValue: "Tools updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.toolsUpdateFailed", { defaultValue: "Tools update failed" }),
			String(e)
		),
	});
	const bulkResourcesM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkResources(
				profileId!,
				selectedResourceIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedResourceIds([]);
			refetchResources();
			notifySuccess(
				t("profiles:detail.messages.resourcesUpdated", { defaultValue: "Resources updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.resourcesUpdateFailed", { defaultValue: "Resources update failed" }),
			String(e)
		),
	});
	const bulkPromptsM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkPrompts(
				profileId!,
				selectedPromptIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedPromptIds([]);
			refetchPrompts();
			notifySuccess(
				t("profiles:detail.messages.promptsUpdated", { defaultValue: "Prompts updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.promptsUpdateFailed", { defaultValue: "Prompts update failed" }),
			String(e)
		),
	});

	const bulkTemplatesM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkResourceTemplates(
				profileId!,
				selectedTemplateIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedTemplateIds([]);
			refetchTemplates();
			notifySuccess(
				t("profiles:detail.messages.templatesUpdated", { defaultValue: "Templates updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.templatesUpdateFailed", { defaultValue: "Templates update failed" }),
			String(e)
		),
	});

	const bulkServersM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkServers(
				profileId!,
				selectedServerIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedServerIds([]);
			refetchServers();
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

	const toggleDefaultMutation = useMutation({
		mutationFn: (nextDefault: boolean) =>
			configSuitsApi.updateSuit(profileId!, { is_default: nextDefault }),
		onSuccess: (_response, nextDefault) => {
			queryClient.invalidateQueries({ queryKey: ["configSuit", profileId] });
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.defaultBundleUpdated", { defaultValue: "Default bundle updated" }),
				nextDefault
					? t("profiles:detail.messages.defaultBundleUpdatedDescription", { defaultValue: "Profile is now part of the default bundle" })
					: t("profiles:detail.messages.defaultBundleRemovedDescription", { defaultValue: "Profile removed from the default bundle" }),
			);
		},
		onError: (error, nextDefault) => {
			notifyError(
				t("profiles:detail.messages.defaultUpdateFailed", { defaultValue: "Default update failed" }),
				`Failed to ${nextDefault ? "set" : "remove"} default: ${
					error instanceof Error ? error.message : String(error)
				}`,
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
		// Refetch all capability data to update counts in tabs
		refetchServers();
		refetchTools();
		refetchResources();
		refetchPrompts();
		refetchTemplates();

			// Invalidate profile statistics cache for config page
			queryClient.invalidateQueries({
				queryKey: ["configSuitStats", profileId],
			});

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
		refetchTools();
		refetchTemplates();
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
		refetchResources();
		refetchTemplates();
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
		refetchPrompts();
		refetchTemplates();
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
	const isDefaultAnchor = suitRole === "default_anchor";
	const defaultButtonDisabled =
		!suit || isDefaultAnchor || toggleDefaultMutation.isPending;
	const defaultButtonLabel = !suit
		? t("profiles:badges.defaultAnchor", { defaultValue: "Default" })
		: isDefaultAnchor
			? t("profiles:badges.defaultAnchor", { defaultValue: "Default Anchor" })
			: suit.is_default
				? t("profiles:detail.buttons.leaveDefault", { defaultValue: "Leave Default" })
				: t("profiles:detail.buttons.joinDefault", { defaultValue: "Join Default" });
	const defaultButtonIcon = suit?.is_default ? (
		<BadgeCheck
			className={`h-4 w-4 ${
				toggleDefaultMutation.isPending ? "animate-spin" : ""
			}`}
		/>
	) : (
		<Square className="h-4 w-4" />
	);

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

	const handleDefaultToggle = () => {
		if (!suit || isDefaultAnchor || toggleDefaultMutation.isPending) {
			return;
		}
		const nextDefault = !suit.is_default;
		toggleDefaultMutation.mutate(nextDefault);
	};

	const handleRefreshAll = () => {
		refetchSuit();
		refetchServers();
		refetchTools();
		refetchResources();
		refetchPrompts();
		refetchTemplates();
	};

	const handleEditDrawerClose = (open: boolean) => {
		setIsEditDialogOpen(open);
	};

	const servers = (serversResponse?.servers ?? []) as ConfigSuitServer[];
	const tools = (toolsResponse?.tools ?? []) as ConfigSuitTool[];
	const resources = (resourcesResponse?.resources ?? []) as ConfigSuitResource[];
	const prompts = (promptsResponse?.prompts ?? []) as ConfigSuitPrompt[];
	const templates = (templatesResponse?.templates ?? []) as any[];

	const enabledServers = servers.filter((s: ConfigSuitServer) => s.enabled);
	const enabledTools = tools.filter((t: ConfigSuitTool) => t.enabled);
	const enabledResources = resources.filter(
		(r: ConfigSuitResource) => r.enabled,
	);
	const enabledPrompts = prompts.filter((p: ConfigSuitPrompt) => p.enabled);
	const enabledTemplates = templates.filter((t: any) => t.enabled);

	// Global servers for availability(connected) calculation
	const { data: globalServersResp } = useQuery({
		queryKey: ["all-servers-for-profile-overview"],
		queryFn: serversApi.getAll,
		staleTime: 30_000,
	});
	const globalServers = globalServersResp?.servers ?? [];
	// For profile counts, available = total in this profile (not global state)
	const availableServersInProfile = servers;

	// Derived server name options for filters
	const serverNameOptions = Array.from(
		new Set(
			[
				...servers.map((s: ConfigSuitServer) => s.name),
				...tools.map((t: ConfigSuitTool) => t.server_name),
				...resources.map((r: ConfigSuitResource) => r.server_name),
				...prompts.map((p: ConfigSuitPrompt) => p.server_name),
				...templates.map((r: any) => r.server_name),
			].filter(Boolean),
		),
	).sort();

	// Filtered datasets
	const visibleServers = servers.filter((s: ConfigSuitServer) => {
		const queryPass =
			serverQuery.trim() === "" ||
			s.name.toLowerCase().includes(serverQuery.toLowerCase());
		const statusPass =
			serverStatus === "all" ||
			(serverStatus === "enabled" ? s.enabled : !s.enabled);
		return queryPass && statusPass;
	});

	const visibleTools = tools.filter((t: ConfigSuitTool) => {
		const text =
			`${t.tool_name ?? ""} ${t.unique_name ?? ""} ${t.server_name ?? ""}`.toLowerCase();
		const queryPass =
			toolQuery.trim() === "" || text.includes(toolQuery.toLowerCase());
		const statusPass =
			toolStatus === "all" ||
			(toolStatus === "enabled" ? t.enabled : !t.enabled);
		const serverPass = toolServer === "all" || t.server_name === toolServer;
		return queryPass && statusPass && serverPass;
	});

	const visibleResources = resources.filter((r: ConfigSuitResource) => {
		const text = `${r.resource_uri ?? ""} ${r.server_name ?? ""}`.toLowerCase();
		const queryPass =
			resourceQuery.trim() === "" || text.includes(resourceQuery.toLowerCase());
		const statusPass =
			resourceStatus === "all" ||
			(resourceStatus === "enabled" ? r.enabled : !r.enabled);
		const serverPass =
			resourceServer === "all" || r.server_name === resourceServer;
		return queryPass && statusPass && serverPass;
	});

	const visiblePrompts = prompts.filter((p: ConfigSuitPrompt) => {
		const text = `${p.prompt_name ?? ""} ${p.server_name ?? ""}`.toLowerCase();
		const queryPass =
			promptQuery.trim() === "" || text.includes(promptQuery.toLowerCase());
		const statusPass =
			promptStatus === "all" ||
			(promptStatus === "enabled" ? p.enabled : !p.enabled);
		const serverPass = promptServer === "all" || p.server_name === promptServer;
		return queryPass && statusPass && serverPass;
	});

	// Filters: templates
	const [templateQuery, setTemplateQuery] = useState("");
	const [templateStatus, setTemplateStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [templateServer, setTemplateServer] = useState<string>("all");

	const visibleTemplates = templates.filter((r: any) => {
		const text = `${r.uri_template ?? ""} ${r.server_name ?? ""}`.toLowerCase();
		const queryPass =
			templateQuery.trim() === "" || text.includes(templateQuery.toLowerCase());
		const statusPass =
			templateStatus === "all" ||
			(templateStatus === "enabled" ? r.enabled : !r.enabled);
		const serverPass = templateServer === "all" || r.server_name === templateServer;
		return queryPass && statusPass && serverPass;
	});

	// Template toggle mutations
	const templateToggleMutation = useMutation({
		mutationFn: ({ templateId, enable }: { templateId: string; enable: boolean }) =>
			enable
				? configSuitsApi.enableResourceTemplate(profileId!, templateId)
				: configSuitsApi.disableResourceTemplate(profileId!, templateId),
		onSuccess: () => {
			refetchTemplates();
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

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
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
									{suitRole === "default_anchor" ? (
										<Badge variant="outline">{t("profiles:badges.defaultAnchor", { defaultValue: "Default Anchor" })}</Badge>
									) : suit.is_default ? (
										<Badge variant="outline">{t("profiles:badges.inDefault", { defaultValue: "In Default" })}</Badge>
									) : null}
								</div>
								{suit.description && (
									<p className="text-sm text-muted-foreground mt-1">
										{suit.description}
									</p>
								)}
							</div>
						</div>
					)}
				</div>
				{/* page-level actions moved into Overview card */}
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
					className="space-y-4"
				>
					<div className="flex items-center justify-between">
						<TabsList className="flex items-center gap-2">
							<TabsTrigger value="overview">{t("profiles:detail.tabs.overview", { defaultValue: "Overview" })}</TabsTrigger>
							<TabsTrigger value="servers">
								{t("profiles:detail.tabs.servers", { defaultValue: "Servers" })} ({enabledServers.length}/{servers.length})
							</TabsTrigger>
							<TabsTrigger value="tools">
								{t("profiles:detail.tabs.tools", { defaultValue: "Tools" })} ({enabledTools.length}/{tools.length})
							</TabsTrigger>
							<TabsTrigger value="prompts">
								{t("profiles:detail.tabs.prompts", { defaultValue: "Prompts" })} ({enabledPrompts.length}/{prompts.length})
							</TabsTrigger>
							<TabsTrigger value="resources">
								{t("profiles:detail.tabs.resources", { defaultValue: "Resources" })} ({enabledResources.length}/{resources.length})
							</TabsTrigger>
							<TabsTrigger value="templates">
								{t("profiles:detail.tabs.templates", { defaultValue: "Templates" })} ({enabledTemplates.length}/{templates.length})
							</TabsTrigger>
						</TabsList>
					</div>

					<TabsContent value="overview">
						<div className="grid gap-4">
							<Card>
								<CardContent className="p-4">
									<div className="flex flex-col gap-4">
										<div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
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
														className={`justify-self-start border px-2.5 py-0.5 leading-none min-h-[1.5rem] ${
															suit.is_active
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
												<Button
													variant="outline"
													size="sm"
													onClick={handleRefreshAll}
													disabled={isRefetchingSuit}
													className="gap-2"
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
						className="gap-2"
					>
						<Edit3 className="h-4 w-4" />
						{t("profiles:detail.buttons.edit", { defaultValue: "Edit" })}
					</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={handleDefaultToggle}
													disabled={defaultButtonDisabled}
													className="gap-2"
													aria-pressed={suit?.is_default ?? false}
												>
													{defaultButtonIcon}
													{defaultButtonLabel}
												</Button>
												{suitRole === "user" && (
													<Button
														variant="outline"
														size="sm"
														onClick={handleSuitToggle}
														disabled={
															isDefaultAnchor ||
															activateSuitMutation.isPending ||
															deactivateSuitMutation.isPending
														}
														className="gap-2"
													>
														{suit?.is_active ? (
															<Square className="h-4 w-4" />
														) : (
															<Play className="h-4 w-4" />
														)}
														{suit?.is_active ? t("profiles:detail.buttons.disable", { defaultValue: "Disable" }) : t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
													</Button>
												)}
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
											</ButtonGroup>
										</div>
									</div>
								</CardContent>
							</Card>

							<div className="grid gap-4 md:grid-cols-4">
								<Card>
										<CardHeader
											className="pb-2 cursor-pointer"
											onClick={() => setActiveTab("servers")}
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
								<Card>
										<CardHeader
											className="pb-2 cursor-pointer"
											onClick={() => setActiveTab("tools")}
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
								<Card>
										<CardHeader
											className="pb-2 cursor-pointer"
											onClick={() => setActiveTab("resources")}
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
								<Card>
										<CardHeader
											className="pb-2 cursor-pointer"
											onClick={() => setActiveTab("prompts")}
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
						</div>
					</TabsContent>

					<TabsContent value="servers">
						<Card>
							<CardHeader>
								<div className="flex items-center justify-between gap-2">
									<div>
											<CardTitle>
												{t("profiles:detail.labels.servers", { defaultValue: "Servers" })}
											</CardTitle>
											<CardDescription>
												{t("profiles:detail.descriptions.servers", {
													defaultValue: "Manage servers included in this profile",
												})}
											</CardDescription>
									</div>
									{!isLoadingServers && (
										<div className="flex flex-wrap items-center gap-2">
												<Input
													placeholder={t("profiles:detail.placeholders.searchServers", {
														defaultValue: "Search servers...",
													})}
												value={serverQuery}
												onChange={(e) => setServerQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={serverStatus}
													onValueChange={(v) =>
														setServerStatus(v as "all" | "enabled" | "disabled")
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
														<SelectContent>
															<SelectItem value="all">
																{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
															</SelectItem>
															<SelectItem value="enabled">
																{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
															</SelectItem>
															<SelectItem value="disabled">
																{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
															</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedServerIds(
															visibleServers.map((s: any) => s.id),
														)
													}
													>
														{t("profiles:detail.buttons.selectAll", {
															defaultValue: "Select all",
														})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedServerIds([])}
													>
														{t("profiles:detail.buttons.clearSelection", {
															defaultValue: "Clear",
														})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkServersM.isPending ||
														selectedServerIds.length === 0
													}
													onClick={() => bulkServersM.mutate({ enable: true })}
													>
														{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkServersM.isPending ||
														selectedServerIds.length === 0
													}
													onClick={() => bulkServersM.mutate({ enable: false })}
													>
														{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent>
								{isLoadingServers ? (
									<div className="space-y-4">
										{["s1", "s2", "s3"].map((id) => (
											<div
												key={`servers-skel-${id}`}
												className="flex items-center justify-between rounded-lg border p-4"
											>
												<div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
												<div className="h-6 w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
											</div>
										))}
									</div>
								) : visibleServers.length > 0 ? (
									<CapsuleStripeList>
										{visibleServers.map((server) => {
											const global = (globalServers as any[]).find(
												(gs: any) => gs.name === server.name,
											);
											const globallyEnabled: boolean | undefined =
												global?.enabled;
											const globalIcon = global?.icons?.[0]?.src;
											const avatarFallback = (server.name || server.id || "S")
												.slice(0, 1)
												.toUpperCase();
											const iconAlt = global?.name || server.name || server.id;
											const globalDescription =
												global?.meta?.description?.trim();
											const selected = selectedServerIds.includes(server.id);
											return (
												<CapsuleStripeListItem
													key={server.id}
													interactive
													className={`group relative transition-colors ${
														selected
															? "bg-primary/10 ring-1 ring-primary/40"
															: ""
													}`}
													onClick={() =>
														setSelectedServerIds((prev) =>
															prev.includes(server.id)
																? prev.filter((x) => x !== server.id)
																: [...prev, server.id],
														)
													}
													onKeyDown={(e) => {
														if (e.key === "Enter" || e.key === " ") {
															e.preventDefault();
															setSelectedServerIds((prev) =>
																prev.includes(server.id)
																	? prev.filter((x) => x !== server.id)
																	: [...prev, server.id],
															);
														}
													}}
												>
													<div className="flex w-full items-center justify-between gap-4">
														<div className="flex flex-1 items-center gap-3">
															<div
																className={`flex h-6 w-6 items-center justify-center rounded-full border text-[0px] transition-all duration-200 ${
																	selected
																		? "border-primary bg-primary text-white shadow-sm"
																		: "border-slate-300 text-transparent group-hover:border-primary/50 group-hover:text-primary/60 dark:border-slate-700 dark:group-hover:border-primary/50"
																}`}
															>
																<Check className="h-3 w-3" />
															</div>
															<CachedAvatar
																src={globalIcon}
																alt={iconAlt ? `${iconAlt} icon` : undefined}
																fallback={avatarFallback}
																size="sm"
																shape="rounded"
																className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
															/>
															<div className="min-w-0">
																<h3 className="font-medium text-slate-900 dark:text-slate-100">
																	{server.name}
																</h3>
																<p className="text-sm text-slate-500">
																	ID: {server.id}
																</p>
																{globalDescription ? (
																	<p className="text-xs text-slate-500 line-clamp-2">
																		{globalDescription}
																	</p>
																) : null}
															</div>
														</div>
														<div className="ml-auto flex items-center gap-2">
                                            {/* Hover actions: Browse (left) + Inspect (right) */}
                                            {enableServerDebug && (
                                                <>
                                                    <div className="opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                                                        <button
                                                            type="button"
                                                            onClick={(ev) => {
                                                                ev.stopPropagation();
                                                                openBrowse(server.id);
                                                            }}
                                                            aria-label={t("profiles:detail.labels.browseServer", { defaultValue: "Browse server" })}
                                                            className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
                                                        >
                                                            <Eye size={20} />
                                                        </button>
                                                    </div>
                                                    <div className="opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                                                        <button
                                                            type="button"
                                                            onClick={(ev) => {
                                                                ev.stopPropagation();
                                                                openDebug(
                                                                    server.id,
                                                                    server.enabled ? "proxy" : "native",
                                                                );
                                                            }}
                                                            aria-label={t("profiles:detail.labels.debugServer", { defaultValue: "Inspect server" })}
                                                            className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
                                                        >
                                                            <Bug size={20} />
                                                        </button>
                                                    </div>
                                                </>
                                            )}

															{/* Global status badges and switch - positioned on the right */}
															{globallyEnabled !== undefined &&
																(globallyEnabled ? (
																	<Badge>
																		{t("profiles:detail.globalStatus.enabled", {
																			defaultValue: "Global Enabled",
																		})}
																	</Badge>
																) : (
																	<Badge variant="outline">
																		{t("profiles:detail.globalStatus.disabled", {
																			defaultValue: "Global Disabled",
																		})}
																	</Badge>
																))}

															{/* Always show switch */}
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
													</div>
												</CapsuleStripeListItem>
											);
										})}
									</CapsuleStripeList>
								) : (
									<p className="text-center text-slate-500 py-8">
										{t("profiles:detail.emptyStates.noServers", {
											defaultValue: "No servers found in this profile",
										})}
									</p>
								)}
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="tools">
						<Card>
							<CardHeader>
								<div className="flex items-center justify-between gap-2">
										<div>
											<CardTitle>
												{t("profiles:detail.labels.tools", { defaultValue: "Tools" })}
											</CardTitle>
											<CardDescription>
												{t("profiles:detail.descriptions.tools", {
													defaultValue: "Manage tools included in this profile",
												})}
											</CardDescription>
									</div>
									{!isLoadingTools && (
										<div className="flex flex-wrap items-center gap-2">
												<Input
													placeholder={t("profiles:detail.placeholders.searchTools", {
														defaultValue: "Search tools...",
													})}
												value={toolQuery}
												onChange={(e) => setToolQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={toolStatus}
													onValueChange={(v) =>
														setToolStatus(v as "all" | "enabled" | "disabled")
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
														<SelectContent>
															<SelectItem value="all">
																{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
															</SelectItem>
															<SelectItem value="enabled">
																{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
															</SelectItem>
															<SelectItem value="disabled">
																{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
															</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select
													value={toolServer}
													onValueChange={(v) => setToolServer(v)}
												>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.server.all", {
																defaultValue: "All Servers",
															})}
														</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`tool-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedToolIds(
															visibleTools.map((t: any) => t.id),
														)
													}
												>
													{t("profiles:detail.buttons.selectAll", {
														defaultValue: "Select all",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedToolIds([])}
												>
													{t("profiles:detail.buttons.clearSelection", {
														defaultValue: "Clear",
													})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkToolsM.isPending || selectedToolIds.length === 0
													}
													onClick={() => bulkToolsM.mutate({ enable: true })}
												>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkToolsM.isPending || selectedToolIds.length === 0
													}
													onClick={() => bulkToolsM.mutate({ enable: false })}
												>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent>
								<CapabilityList
									asCard={false}
									title={t("profiles:detail.labels.tools", { defaultValue: "Tools" })}
									kind="tools"
									context="profile"
									items={visibleTools as any}
									loading={isLoadingTools}
									enableToggle
									getId={(t: any) => t.id}
									getEnabled={(t: any) => !!t.enabled}
									onToggle={(id, next) =>
										toolToggleMutation.mutate({ toolId: id, enable: next })
									}
									emptyText={t("profiles:detail.emptyStates.noTools", { defaultValue: "No tools found in this profile" })}
									filterText={toolQuery}
									selectable
									selectedIds={selectedToolIds}
									onSelectToggle={(id) => {
										setSelectedToolIds((prev) =>
											prev.includes(id)
												? prev.filter((x) => x !== id)
												: [...prev, id],
										);
									}}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="prompts">
						<Card>
							<CardHeader>
								<div className="flex items-center justify-between gap-2">
										<div>
											<CardTitle>
												{t("profiles:detail.labels.prompts", { defaultValue: "Prompts" })}
											</CardTitle>
											<CardDescription>
												{t("profiles:detail.descriptions.prompts", {
													defaultValue: "Manage prompts included in this profile",
												})}
											</CardDescription>
									</div>
									{!isLoadingPrompts && (
										<div className="flex flex-wrap items-center gap-2">
												<Input
													placeholder={t("profiles:detail.placeholders.searchPrompts", {
														defaultValue: "Search prompts...",
													})}
												value={promptQuery}
												onChange={(e) => setPromptQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={promptStatus}
													onValueChange={(v) =>
														setPromptStatus(v as "all" | "enabled" | "disabled")
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
														<SelectContent>
															<SelectItem value="all">
																{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
															</SelectItem>
															<SelectItem value="enabled">
																{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
															</SelectItem>
															<SelectItem value="disabled">
																{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
															</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select
													value={promptServer}
													onValueChange={(v) => setPromptServer(v)}
												>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.server.all", {
																defaultValue: "All Servers",
															})}
														</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`prm-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedPromptIds(
															visiblePrompts.map((p: any) => p.id),
														)
													}
												>
													{t("profiles:detail.buttons.selectAll", {
														defaultValue: "Select all",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedPromptIds([])}
												>
													{t("profiles:detail.buttons.clearSelection", {
														defaultValue: "Clear",
													})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkPromptsM.isPending ||
														selectedPromptIds.length === 0
													}
													onClick={() => bulkPromptsM.mutate({ enable: true })}
												>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkPromptsM.isPending ||
														selectedPromptIds.length === 0
													}
													onClick={() => bulkPromptsM.mutate({ enable: false })}
												>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent>
								<CapabilityList
									asCard={false}
									title={t("profiles:detail.labels.prompts", { defaultValue: "Prompts" })}
									kind="prompts"
									context="profile"
									items={visiblePrompts as any}
									loading={isLoadingPrompts}
									enableToggle
									getId={(p: any) => p.id}
									getEnabled={(p: any) => !!p.enabled}
									onToggle={(id, next) =>
										promptToggleMutation.mutate({ promptId: id, enable: next })
									}
									emptyText={t("profiles:detail.emptyStates.noPrompts", { defaultValue: "No prompts found in this profile" })}
									filterText={promptQuery}
									selectable
									selectedIds={selectedPromptIds}
									onSelectToggle={(id) => {
										setSelectedPromptIds((prev) =>
											prev.includes(id)
												? prev.filter((x) => x !== id)
												: [...prev, id],
										);
									}}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="resources">
						<Card>
							<CardHeader>
								<div className="flex items-center justify-between gap-2">
					<div>
						<CardTitle>
							{t("profiles:detail.labels.resources", { defaultValue: "Resources" })}
						</CardTitle>
						<CardDescription>
							{t("profiles:detail.descriptions.resources", {
								defaultValue: "Manage resources included in this profile",
							})}
						</CardDescription>
					</div>
									{!isLoadingResources && (
										<div className="flex flex-wrap items-center gap-2">
							<Input
								placeholder={t("profiles:detail.placeholders.searchResources", {
									defaultValue: "Search resources...",
								})}
												value={resourceQuery}
												onChange={(e) => setResourceQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={resourceStatus}
													onValueChange={(v) =>
														setResourceStatus(
															v as "all" | "enabled" | "disabled",
														)
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
								<SelectContent>
									<SelectItem value="all">
										{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
									</SelectItem>
									<SelectItem value="enabled">
										{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
									</SelectItem>
									<SelectItem value="disabled">
										{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
									</SelectItem>
								</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select
													value={resourceServer}
													onValueChange={(v) => setResourceServer(v)}
												>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
								<SelectContent>
									<SelectItem value="all">
										{t("profiles:detail.filters.server.all", {
											defaultValue: "All Servers",
										})}
									</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`res-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
							<Button
								variant="outline"
								size="sm"
								onClick={() =>
									setSelectedResourceIds(
										visibleResources.map((r: any) => r.id),
									)
								}
							>
								{t("profiles:detail.buttons.selectAll", {
									defaultValue: "Select all",
								})}
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => setSelectedResourceIds([])}
							>
								{t("profiles:detail.buttons.clearSelection", {
									defaultValue: "Clear",
								})}
							</Button>
							<Button
								size="sm"
								disabled={
									bulkResourcesM.isPending ||
									selectedResourceIds.length === 0
								}
								onClick={() =>
									bulkResourcesM.mutate({ enable: true })
								}
							>
								{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
							</Button>
							<Button
								size="sm"
								variant="secondary"
								disabled={
									bulkResourcesM.isPending ||
									selectedResourceIds.length === 0
								}
								onClick={() =>
									bulkResourcesM.mutate({ enable: false })
								}
							>
								{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
							</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent>
								<CapabilityList
									asCard={false}
									title={t("profiles:detail.labels.resources", { defaultValue: "Resources" })}
									kind="resources"
									context="profile"
									items={visibleResources as any}
									loading={isLoadingResources}
									enableToggle
									getId={(r: any) => r.id}
									getEnabled={(r: any) => !!r.enabled}
									onToggle={(id, next) =>
										resourceToggleMutation.mutate({
											resourceId: id,
											enable: next,
										})
									}
									emptyText={t("profiles:detail.emptyStates.noResources", { defaultValue: "No resources found in this profile" })}
									filterText={resourceQuery}
									selectable
									selectedIds={selectedResourceIds}
									onSelectToggle={(id) => {
										setSelectedResourceIds((prev) =>
											prev.includes(id)
												? prev.filter((x) => x !== id)
												: [...prev, id],
										);
									}}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="templates">
						<Card>
							<CardHeader>
								<div className="flex items-center justify-between gap-2">
									<div>
										<CardTitle>
											{t("profiles:detail.labels.templates", { defaultValue: "Templates" })}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.descriptions.templates", {
												defaultValue: "Manage resource templates included in this profile",
											})}
										</CardDescription>
									</div>
									{!isLoadingTemplates && (
										<div className="flex flex-wrap items-center gap-2">
											<Input
												placeholder={t("profiles:detail.placeholders.searchTemplates", { defaultValue: "Search templates..." })}
												value={templateQuery}
												onChange={(e) => setTemplateQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={templateStatus}
													onValueChange={(v) => setTemplateStatus(v as "all" | "enabled" | "disabled")}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">{t("profiles:detail.filters.status.all", { defaultValue: "All" })}</SelectItem>
														<SelectItem value="enabled">{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}</SelectItem>
														<SelectItem value="disabled">{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select value={templateServer} onValueChange={(v) => setTemplateServer(v)}>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">{t("profiles:detail.filters.server.all", { defaultValue: "All Servers" })}</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`tpl-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button variant="outline" size="sm" onClick={() => setSelectedTemplateIds(visibleTemplates.map((p: any) => p.id))}>
													{t("profiles:detail.buttons.selectAll", { defaultValue: "Select all" })}
												</Button>
												<Button variant="outline" size="sm" onClick={() => setSelectedTemplateIds([])}>
													{t("profiles:detail.buttons.clearSelection", { defaultValue: "Clear" })}
												</Button>
												<Button size="sm" disabled={bulkTemplatesM.isPending || selectedTemplateIds.length === 0} onClick={() => bulkTemplatesM.mutate({ enable: true })}>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button size="sm" variant="secondary" disabled={bulkTemplatesM.isPending || selectedTemplateIds.length === 0} onClick={() => bulkTemplatesM.mutate({ enable: false })}>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent>
                        <CapabilityList
                            asCard={false}
                            title={t("profiles:detail.labels.templates", { defaultValue: "Templates" })}
                            kind="templates"
                            context="profile"
                            items={visibleTemplates as any}
                            loading={isLoadingTemplates}
                            enableToggle
                            getId={(p: any) => p.id}
                            getEnabled={(p: any) => !!p.enabled}
                            onToggle={(id, next) => templateToggleMutation.mutate({ templateId: id, enable: next })}
                            emptyText={t("profiles:detail.emptyStates.noTemplates", { defaultValue: "No templates found in this profile" })}
                            filterText={templateQuery}
                            selectable
                            selectedIds={selectedTemplateIds}
                            onSelectToggle={(id) => setSelectedTemplateIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))}
                            renderAction={undefined}
                        />
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
