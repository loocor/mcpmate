import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";
import CapabilityList from "../../components/capability-list";
import { CachedAvatar } from "../../components/cached-avatar";
import { DETAIL_TAB_CONTENT_CLASS } from "../../components/detail-tab-content-class";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ButtonGroup } from "../../components/ui/button-group";
import { Input } from "../../components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { clientsApi, serversApi } from "../../lib/api";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import type {
	ClientCapabilityConfigData,
	ClientCapabilityConfigReq,
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitResourceTemplate,
	ConfigSuitTool,
	ServerDetail,
} from "../../lib/types";

type ToolStatusFilter = "all" | "enabled" | "disabled";
type DirectCapabilityTab = "tools" | "prompts" | "resources" | "templates";
const DIRECT_CAPABILITY_TABS: DirectCapabilityTab[] = [
	"tools",
	"prompts",
	"resources",
	"templates",
];

function getServerIconSrc(server?: ServerDetail): string | undefined {
	const icon = server?.icons?.find(
		(entry: { src?: string | null }) =>
			typeof entry?.src === "string" && entry.src.length > 0,
	);
	return icon?.src;
}

function createEmptyCapabilityConfig(identifier: string): ClientCapabilityConfigData {
	return {
		identifier,
		capability_source: "activated",
		selected_profile_ids: [],
		unify_direct_exposure: {
			route_mode: "capability_level",
			selected_server_ids: [],
			selected_tool_surfaces: [],
			selected_prompt_surfaces: [],
			selected_resource_surfaces: [],
			selected_template_surfaces: [],
		},
	};
}

function getSelectedSurfaces(
	capabilityConfig: ClientCapabilityConfigData,
): {
	tools: Array<{ server_id: string; tool_name: string }>;
	prompts: Array<{ server_id: string; prompt_name: string }>;
	resources: Array<{ server_id: string; resource_uri: string }>;
	templates: Array<{ server_id: string; uri_template: string }>;
} {
	return {
		tools: capabilityConfig.unify_direct_exposure?.selected_tool_surfaces ?? [],
		prompts: capabilityConfig.unify_direct_exposure?.selected_prompt_surfaces ?? [],
		resources:
			capabilityConfig.unify_direct_exposure?.selected_resource_surfaces ?? [],
		templates:
			capabilityConfig.unify_direct_exposure?.selected_template_surfaces ?? [],
	};
}

function createCapabilityConfigPayload(
	identifier: string,
	existingConfig: ClientCapabilityConfigData,
	nextSurfaces: {
		tools: Array<{ server_id: string; tool_name: string }>;
		prompts: Array<{ server_id: string; prompt_name: string }>;
		resources: Array<{ server_id: string; resource_uri: string }>;
		templates: Array<{ server_id: string; uri_template: string }>;
	},
): ClientCapabilityConfigReq {
	return {
		identifier,
		capability_source: existingConfig.capability_source,
		selected_profile_ids: existingConfig.selected_profile_ids,
		unify_direct_exposure: {
			...existingConfig.unify_direct_exposure,
			route_mode: "capability_level",
			selected_server_ids: [],
			selected_tool_surfaces: nextSurfaces.tools,
			selected_prompt_surfaces: nextSurfaces.prompts,
			selected_resource_surfaces: nextSurfaces.resources,
			selected_template_surfaces: nextSurfaces.templates,
		},
	};
}

function filterVisibleTools<T extends { enabled: boolean }>(
	tools: T[],
	toolQuery: string,
	toolStatus: ToolStatusFilter,
	getSearchFields: (item: T) => string[],
): T[] {
	const keyword = toolQuery.trim().toLowerCase();

	return tools.filter((tool) => {
		if (toolStatus === "enabled" && !tool.enabled) return false;
		if (toolStatus === "disabled" && tool.enabled) return false;
		if (!keyword) return true;
		return getSearchFields(tool)
			.filter(Boolean)
			.some((value) => value.toLowerCase().includes(keyword));
	});
}

export function ClientDirectCapabilitiesPage() {
	usePageTranslations("clients");
	const { t } = useTranslation("clients");
	const { identifier, serverId } = useParams<{
		identifier: string;
		serverId: string;
	}>();
	const { activeTab, setActiveTab } = useUrlTab({
		paramName: "tab",
		defaultTab: "tools",
		validTabs: DIRECT_CAPABILITY_TABS,
	});
	const queryClient = useQueryClient();
	const [toolQuery, setToolQuery] = useState("");
	const [toolStatus, setToolStatus] = useState<ToolStatusFilter>("all");
	const [selectedToolIds, setSelectedToolIds] = useState<string[]>([]);
	const [promptQuery, setPromptQuery] = useState("");
	const [promptStatus, setPromptStatus] = useState<ToolStatusFilter>("all");
	const [selectedPromptIds, setSelectedPromptIds] = useState<string[]>([]);
	const [resourceQuery, setResourceQuery] = useState("");
	const [resourceStatus, setResourceStatus] = useState<ToolStatusFilter>("all");
	const [selectedResourceIds, setSelectedResourceIds] = useState<string[]>([]);
	const [templateQuery, setTemplateQuery] = useState("");
	const [templateStatus, setTemplateStatus] = useState<ToolStatusFilter>("all");
	const [selectedTemplateIds, setSelectedTemplateIds] = useState<string[]>([]);

	async function loadCapabilityConfig(): Promise<ClientCapabilityConfigData | null> {
		if (!identifier) {
			return null;
		}

		return clientsApi.getCapabilityConfig(identifier);
	}

	function invalidateDirectQueries(): void {
		if (!identifier || !serverId) {
			return;
		}

		void queryClient.invalidateQueries({
			queryKey: ["client-capability-config", identifier],
		});
		void queryClient.invalidateQueries({
			queryKey: ["client-direct-tools", identifier, serverId],
		});
	}

	const { data: serverDetails, isLoading: isLoadingServer } = useQuery<
		ServerDetail | undefined
	>({
		queryKey: ["direct-server-details", serverId],
		queryFn: () =>
			serverId ? serversApi.getServer(serverId) : Promise.resolve(undefined),
		enabled: Boolean(serverId),
		retry: 1,
	});

	const {
		data: capabilityResponse,
		isLoading: isLoadingCapabilities,
		refetch: refetchCapabilities,
	} = useQuery({
		queryKey: ["client-direct-tools", identifier, serverId],
		queryFn: async () => {
			if (!identifier || !serverId) {
				return {
					tools: [] as ConfigSuitTool[],
					prompts: [] as ConfigSuitPrompt[],
					resources: [] as ConfigSuitResource[],
					templates: [] as ConfigSuitResourceTemplate[],
				};
			}
			const [serverToolsResponse, serverPromptsResponse, serverResourcesResponse, serverTemplatesResponse, clientCapabilityConfig] = await Promise.all([
				serversApi.listTools(serverId).catch(() => ({ items: [] })),
				serversApi.listPrompts(serverId).catch(() => ({ items: [] })),
				serversApi.listResources(serverId).catch(() => ({ items: [] })),
				serversApi.listResourceTemplates(serverId).catch(() => ({ items: [] })),
				clientsApi.getCapabilityConfig(identifier).catch(() => null),
			]);
			const selectedSurfaces = getSelectedSurfaces(
				clientCapabilityConfig ?? createEmptyCapabilityConfig(identifier),
			);
			const selectedToolSet = new Set(
				selectedSurfaces.tools
					.filter((entry) => entry.server_id === serverId)
					.map((entry) => entry.tool_name),
			);
			const selectedPromptSet = new Set(
				selectedSurfaces.prompts
					.filter((entry) => entry.server_id === serverId)
					.map((entry) => entry.prompt_name),
			);
			const selectedResourceSet = new Set(
				selectedSurfaces.resources
					.filter((entry) => entry.server_id === serverId)
					.map((entry) => entry.resource_uri),
			);
			const selectedTemplateSet = new Set(
				selectedSurfaces.templates
					.filter((entry) => entry.server_id === serverId)
					.map((entry) => entry.uri_template),
			);
			const rawTools = Array.isArray(serverToolsResponse.items)
				? (serverToolsResponse.items as Array<Record<string, unknown>>)
				: [];
			const rawPrompts = Array.isArray(serverPromptsResponse.items)
				? (serverPromptsResponse.items as Array<Record<string, unknown>>)
				: [];
			const rawResources = Array.isArray(serverResourcesResponse.items)
				? (serverResourcesResponse.items as Array<Record<string, unknown>>)
				: [];
			const rawTemplates = Array.isArray(serverTemplatesResponse.items)
				? (serverTemplatesResponse.items as Array<Record<string, unknown>>)
				: [];

			const tools: ConfigSuitTool[] = rawTools.map((tool) => {
				const toolName = String(tool["tool_name"] ?? tool["name"] ?? "");
				return {
					id: toolName,
					server_id: serverId,
					server_name: serverDetails?.name ?? serverId,
					tool_name: toolName,
					enabled: selectedToolSet.has(toolName),
					allowed_operations: [],
				};
			});
			const prompts: ConfigSuitPrompt[] = rawPrompts.map((prompt) => {
				const promptName = String(prompt["prompt_name"] ?? prompt["name"] ?? "");
				return {
					id: promptName,
					server_id: serverId,
					server_name: serverDetails?.name ?? serverId,
					prompt_name: promptName,
					enabled: selectedPromptSet.has(promptName),
					allowed_operations: [],
				};
			});
			const resources: ConfigSuitResource[] = rawResources.map((resource) => {
				const resourceUri = String(resource["resource_uri"] ?? resource["uri"] ?? "");
				return {
					id: resourceUri,
					server_id: serverId,
					server_name: serverDetails?.name ?? serverId,
					resource_uri: resourceUri,
					enabled: selectedResourceSet.has(resourceUri),
					allowed_operations: [],
				};
			});
			const templates: ConfigSuitResourceTemplate[] = rawTemplates.map((template) => {
				const uriTemplate = String(
					template["uri_template"] ?? template["template"] ?? "",
				);
				return {
					id: uriTemplate,
					server_id: serverId,
					server_name: serverDetails?.name ?? serverId,
					uri_template: uriTemplate,
					enabled: selectedTemplateSet.has(uriTemplate),
					allowed_operations: [],
				};
			});
			return { tools, prompts, resources, templates };
		},
		enabled: Boolean(identifier && serverId),
		retry: 1,
	});

	const tools = useMemo(
		() => (capabilityResponse?.tools ?? []) as ConfigSuitTool[],
		[capabilityResponse?.tools],
	);
	const prompts = useMemo(
		() => (capabilityResponse?.prompts ?? []) as ConfigSuitPrompt[],
		[capabilityResponse?.prompts],
	);
	const resources = useMemo(
		() => (capabilityResponse?.resources ?? []) as ConfigSuitResource[],
		[capabilityResponse?.resources],
	);
	const templates = useMemo(
		() => (capabilityResponse?.templates ?? []) as ConfigSuitResourceTemplate[],
		[capabilityResponse?.templates],
	);
	const enabledToolsCount = useMemo(
		() => tools.filter((tool) => tool.enabled).length,
		[tools],
	);
	const enabledPromptsCount = useMemo(
		() => prompts.filter((entry) => entry.enabled).length,
		[prompts],
	);
	const enabledResourcesCount = useMemo(
		() => resources.filter((entry) => entry.enabled).length,
		[resources],
	);
	const enabledTemplatesCount = useMemo(
		() => templates.filter((entry) => entry.enabled).length,
		[templates],
	);
	const visibleTools = useMemo(
		() => filterVisibleTools(tools, toolQuery, toolStatus, (entry) => [entry.tool_name, entry.server_name]),
		[toolQuery, toolStatus, tools],
	);
	const visiblePrompts = useMemo(
		() =>
			filterVisibleTools(prompts, promptQuery, promptStatus, (entry) => [
				entry.prompt_name,
				entry.server_name,
			]),
		[prompts, promptQuery, promptStatus],
	);
	const visibleResources = useMemo(
		() =>
			filterVisibleTools(resources, resourceQuery, resourceStatus, (entry) => [
				entry.resource_uri,
				entry.server_name,
			]),
		[resources, resourceQuery, resourceStatus],
	);
	const visibleTemplates = useMemo(
		() =>
			filterVisibleTools(templates, templateQuery, templateStatus, (entry) => [
				entry.uri_template,
				entry.server_name,
			]),
		[templates, templateQuery, templateStatus],
	);

	const toolToggleMutation = useMutation<unknown, unknown, { toolId: string; enable: boolean }>({
		mutationFn: async ({ toolId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig =
				(await loadCapabilityConfig()) ?? createEmptyCapabilityConfig(identifier);
			const currentSurfaces = getSelectedSurfaces(existingConfig);
			const remainingToolSurfaces = currentSurfaces.tools.filter(
				(entry) => !(entry.server_id === serverId && entry.tool_name === toolId),
			);
			const nextToolSurfaces = enable
				? [...remainingToolSurfaces, { server_id: serverId, tool_name: toolId }]
				: remainingToolSurfaces;

			await clientsApi.updateCapabilityConfig(
				createCapabilityConfigPayload(identifier, existingConfig, {
					...currentSurfaces,
					tools: nextToolSurfaces,
				}),
			);
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
			notifySuccess(
				t("detail.directExposure.notifications.savedTitle", {
					defaultValue: "Direct capabilities updated",
				}),
				t("detail.directExposure.notifications.savedMessage", {
					defaultValue: "The direct capability exposure list has been updated.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("detail.directExposure.notifications.saveFailedTitle", {
					defaultValue: "Unable to update direct capabilities",
				}),
				String(error),
			);
		},
	});

	const promptToggleMutation = useMutation<unknown, unknown, { promptId: string; enable: boolean }>({
		mutationFn: async ({ promptId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig =
				(await loadCapabilityConfig()) ?? createEmptyCapabilityConfig(identifier);
			const currentSurfaces = getSelectedSurfaces(existingConfig);
			const remainingPromptSurfaces = currentSurfaces.prompts.filter(
				(entry) => !(entry.server_id === serverId && entry.prompt_name === promptId),
			);
			const nextPromptSurfaces = enable
				? [...remainingPromptSurfaces, { server_id: serverId, prompt_name: promptId }]
				: remainingPromptSurfaces;
			await clientsApi.updateCapabilityConfig(
				createCapabilityConfigPayload(identifier, existingConfig, {
					...currentSurfaces,
					prompts: nextPromptSurfaces,
				}),
			);
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
		},
	});

	const resourceToggleMutation = useMutation<unknown, unknown, { resourceId: string; enable: boolean }>({
		mutationFn: async ({ resourceId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig =
				(await loadCapabilityConfig()) ?? createEmptyCapabilityConfig(identifier);
			const currentSurfaces = getSelectedSurfaces(existingConfig);
			const remainingResourceSurfaces = currentSurfaces.resources.filter(
				(entry) => !(entry.server_id === serverId && entry.resource_uri === resourceId),
			);
			const nextResourceSurfaces = enable
				? [...remainingResourceSurfaces, { server_id: serverId, resource_uri: resourceId }]
				: remainingResourceSurfaces;
			await clientsApi.updateCapabilityConfig(
				createCapabilityConfigPayload(identifier, existingConfig, {
					...currentSurfaces,
					resources: nextResourceSurfaces,
				}),
			);
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
		},
	});

	const templateToggleMutation = useMutation<unknown, unknown, { templateId: string; enable: boolean }>({
		mutationFn: async ({ templateId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig =
				(await loadCapabilityConfig()) ?? createEmptyCapabilityConfig(identifier);
			const currentSurfaces = getSelectedSurfaces(existingConfig);
			const remainingTemplateSurfaces = currentSurfaces.templates.filter(
				(entry) => !(entry.server_id === serverId && entry.uri_template === templateId),
			);
			const nextTemplateSurfaces = enable
				? [...remainingTemplateSurfaces, { server_id: serverId, uri_template: templateId }]
				: remainingTemplateSurfaces;
			await clientsApi.updateCapabilityConfig(
				createCapabilityConfigPayload(identifier, existingConfig, {
					...currentSurfaces,
					templates: nextTemplateSurfaces,
				}),
			);
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
		},
	});

	const bulkToolsMutation = useMutation<unknown, unknown, { enable: boolean }>({
		mutationFn: async ({ enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig =
				(await loadCapabilityConfig()) ?? createEmptyCapabilityConfig(identifier);
			const currentSurfaces = getSelectedSurfaces(existingConfig);
			const selectedToolIdSet = new Set(selectedToolIds);
			const selectedPromptIdSet = new Set(selectedPromptIds);
			const selectedResourceIdSet = new Set(selectedResourceIds);
			const selectedTemplateIdSet = new Set(selectedTemplateIds);

			const toolsNext = (() => {
				const currentServer = currentSurfaces.tools.filter(
					(entry) => entry.server_id === serverId && !selectedToolIdSet.has(entry.tool_name),
				);
				if (!enable) return [...currentSurfaces.tools.filter((entry) => entry.server_id !== serverId), ...currentServer];
				return [
					...currentSurfaces.tools.filter((entry) => entry.server_id !== serverId),
					...currentServer,
					...tools
						.filter((entry) => selectedToolIdSet.has(entry.id))
						.map((entry) => ({ server_id: serverId, tool_name: entry.id })),
				];
			})();
			const promptsNext = (() => {
				const currentServer = currentSurfaces.prompts.filter(
					(entry) => entry.server_id === serverId && !selectedPromptIdSet.has(entry.prompt_name),
				);
				if (!enable) return [...currentSurfaces.prompts.filter((entry) => entry.server_id !== serverId), ...currentServer];
				return [
					...currentSurfaces.prompts.filter((entry) => entry.server_id !== serverId),
					...currentServer,
					...prompts
						.filter((entry) => selectedPromptIdSet.has(entry.id))
						.map((entry) => ({ server_id: serverId, prompt_name: entry.id })),
				];
			})();
			const resourcesNext = (() => {
				const currentServer = currentSurfaces.resources.filter(
					(entry) => entry.server_id === serverId && !selectedResourceIdSet.has(entry.resource_uri),
				);
				if (!enable) return [...currentSurfaces.resources.filter((entry) => entry.server_id !== serverId), ...currentServer];
				return [
					...currentSurfaces.resources.filter((entry) => entry.server_id !== serverId),
					...currentServer,
					...resources
						.filter((entry) => selectedResourceIdSet.has(entry.id))
						.map((entry) => ({ server_id: serverId, resource_uri: entry.id })),
				];
			})();
			const templatesNext = (() => {
				const currentServer = currentSurfaces.templates.filter(
					(entry) => entry.server_id === serverId && !selectedTemplateIdSet.has(entry.uri_template),
				);
				if (!enable) return [...currentSurfaces.templates.filter((entry) => entry.server_id !== serverId), ...currentServer];
				return [
					...currentSurfaces.templates.filter((entry) => entry.server_id !== serverId),
					...currentServer,
					...templates
						.filter((entry) => selectedTemplateIdSet.has(entry.id))
						.map((entry) => ({ server_id: serverId, uri_template: entry.id })),
				];
			})();

			await clientsApi.updateCapabilityConfig(
				createCapabilityConfigPayload(identifier, existingConfig, {
					tools: toolsNext,
					prompts: promptsNext,
					resources: resourcesNext,
					templates: templatesNext,
				}),
			);
			return null;
		},
		onSuccess: () => {
			setSelectedToolIds([]);
			setSelectedPromptIds([]);
			setSelectedResourceIds([]);
			setSelectedTemplateIds([]);
			invalidateDirectQueries();
			void refetchCapabilities();
			notifySuccess(
				t("detail.directExposure.notifications.savedTitle", {
					defaultValue: "Direct capabilities updated",
				}),
				t("detail.directExposure.notifications.savedMessage", {
					defaultValue: "The direct capability exposure list has been updated.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("detail.directExposure.notifications.saveFailedTitle", {
					defaultValue: "Unable to update direct capabilities",
				}),
				String(error),
			);
		},
	});

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<Card>
				<CardHeader>
					<div className="flex items-start gap-3">
						{serverDetails ? (
							<CachedAvatar
								src={getServerIconSrc(serverDetails)}
								alt={serverDetails.name}
								fallback={serverDetails.name}
								size="sm"
								shape="rounded"
							/>
						) : null}
						<div className="flex-1 min-w-0">
							<div className="flex items-center gap-2">
								<CardTitle>
									{serverDetails?.name ??
										serverId ??
										t("detail.directExposure.title", {
											defaultValue: "Capability Level",
										})}
								</CardTitle>
								<Badge variant="outline">
									{t("detail.directExposure.badge", {
										defaultValue: "Direct Exposure",
									})}
								</Badge>
							</div>
							<CardDescription>
								{serverDetails?.meta?.description ||
									t("detail.directExposure.serverDescriptionFallback", {
										defaultValue:
											"Choose which capabilities from this server should be exposed directly to the client.",
									})}
							</CardDescription>
						</div>
					</div>
				</CardHeader>
			</Card>

			<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
				<Tabs
					value={activeTab}
					onValueChange={(value) => setActiveTab(value as DirectCapabilityTab)}
					className="flex min-h-0 flex-1 flex-col"
				>
					<CardHeader className="shrink-0">
						<div className="flex shrink-0 items-center justify-between">
							<TabsList className="inline-flex w-auto justify-start items-center gap-2">
								<TabsTrigger value="tools">
									{t("detail.directExposure.tabs.toolsWithCounts", {
										enabled: enabledToolsCount,
										total: tools.length,
										defaultValue: "Tools ({{enabled}}/{{total}})",
									})}
								</TabsTrigger>
								<TabsTrigger value="prompts">
									{t("detail.directExposure.tabs.promptsWithCounts", {
										enabled: enabledPromptsCount,
										total: prompts.length,
										defaultValue: "Prompts ({{enabled}}/{{total}})",
									})}
								</TabsTrigger>
								<TabsTrigger value="resources">
									{t("detail.directExposure.tabs.resourcesWithCounts", {
										enabled: enabledResourcesCount,
										total: resources.length,
										defaultValue: "Resources ({{enabled}}/{{total}})",
									})}
								</TabsTrigger>
								<TabsTrigger value="templates">
									{t("detail.directExposure.tabs.templatesWithCounts", {
										enabled: enabledTemplatesCount,
										total: templates.length,
										defaultValue: "Templates ({{enabled}}/{{total}})",
									})}
								</TabsTrigger>
							</TabsList>
						</div>
					</CardHeader>
					<TabsContent value="tools" className={DETAIL_TAB_CONTENT_CLASS}>
						<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
							<div className="mb-3 flex flex-wrap items-center justify-end gap-2">
								<Input placeholder={t("detail.directExposure.searchPlaceholders.tools", { defaultValue: "Search tools..." })} value={toolQuery} onChange={(event) => setToolQuery(event.target.value)} className="w-48 h-9" />
								<Select value={toolStatus} onValueChange={(value) => setToolStatus(value as ToolStatusFilter)}>
									<SelectTrigger className="w-36 h-9"><SelectValue placeholder={t("detail.directExposure.statusPlaceholder", { defaultValue: "Status" })} /></SelectTrigger>
									<SelectContent>
										<SelectItem value="all">{t("detail.directExposure.filters.status.all", { defaultValue: "All" })}</SelectItem>
										<SelectItem value="enabled">{t("detail.directExposure.filters.status.enabled", { defaultValue: "Enabled" })}</SelectItem>
										<SelectItem value="disabled">{t("detail.directExposure.filters.status.disabled", { defaultValue: "Disabled" })}</SelectItem>
									</SelectContent>
								</Select>
								<ButtonGroup className="hidden md:flex ml-2">
									<Button variant="outline" size="sm" onClick={() => setSelectedToolIds(visibleTools.map((tool) => tool.id))}>{t("detail.directExposure.buttons.selectAll", { defaultValue: "Select all" })}</Button>
									<Button variant="outline" size="sm" onClick={() => setSelectedToolIds([])}>{t("detail.directExposure.buttons.clearSelection", { defaultValue: "Clear" })}</Button>
									<Button size="sm" disabled={bulkToolsMutation.isPending || selectedToolIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: true })}>{t("detail.directExposure.buttons.enable", { defaultValue: "Enable" })}</Button>
									<Button size="sm" variant="secondary" disabled={bulkToolsMutation.isPending || selectedToolIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: false })}>{t("detail.directExposure.buttons.disable", { defaultValue: "Disable" })}</Button>
								</ButtonGroup>
							</div>
							<CapabilityList asCard={false} scrollContainedBody title={t("detail.directExposure.tabs.tools", { defaultValue: "Tools" })} kind="tools" context="profile" items={visibleTools} loading={isLoadingCapabilities || isLoadingServer} enableToggle getId={(tool) => tool.id} getEnabled={(tool) => tool.enabled} onToggle={(toolId, next) => toolToggleMutation.mutate({ toolId, enable: next })} emptyText={t("detail.directExposure.empty.tools", { defaultValue: "No tools found for this server." })} filterText={toolQuery} selectable selectedIds={selectedToolIds} onSelectToggle={(toolId) => setSelectedToolIds((prev) => (prev.includes(toolId) ? prev.filter((entry) => entry !== toolId) : [...prev, toolId]))} />
						</CardContent>
					</TabsContent>
					<TabsContent value="prompts" className={DETAIL_TAB_CONTENT_CLASS}>
						<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
							<div className="mb-3 flex flex-wrap items-center justify-end gap-2">
								<Input placeholder={t("detail.directExposure.searchPlaceholders.prompts", { defaultValue: "Search prompts..." })} value={promptQuery} onChange={(event) => setPromptQuery(event.target.value)} className="w-48 h-9" />
								<Select value={promptStatus} onValueChange={(value) => setPromptStatus(value as ToolStatusFilter)}>
									<SelectTrigger className="w-36 h-9"><SelectValue placeholder={t("detail.directExposure.statusPlaceholder", { defaultValue: "Status" })} /></SelectTrigger>
									<SelectContent><SelectItem value="all">{t("detail.directExposure.filters.status.all", { defaultValue: "All" })}</SelectItem><SelectItem value="enabled">{t("detail.directExposure.filters.status.enabled", { defaultValue: "Enabled" })}</SelectItem><SelectItem value="disabled">{t("detail.directExposure.filters.status.disabled", { defaultValue: "Disabled" })}</SelectItem></SelectContent>
								</Select>
								<ButtonGroup className="hidden md:flex ml-2">
									<Button variant="outline" size="sm" onClick={() => setSelectedPromptIds(visiblePrompts.map((entry) => entry.id))}>{t("detail.directExposure.buttons.selectAll", { defaultValue: "Select all" })}</Button>
									<Button variant="outline" size="sm" onClick={() => setSelectedPromptIds([])}>{t("detail.directExposure.buttons.clearSelection", { defaultValue: "Clear" })}</Button>
									<Button size="sm" disabled={bulkToolsMutation.isPending || selectedPromptIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: true })}>{t("detail.directExposure.buttons.enable", { defaultValue: "Enable" })}</Button>
									<Button size="sm" variant="secondary" disabled={bulkToolsMutation.isPending || selectedPromptIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: false })}>{t("detail.directExposure.buttons.disable", { defaultValue: "Disable" })}</Button>
								</ButtonGroup>
							</div>
							<CapabilityList asCard={false} scrollContainedBody title={t("detail.directExposure.tabs.prompts", { defaultValue: "Prompts" })} kind="prompts" context="profile" items={visiblePrompts} loading={isLoadingCapabilities || isLoadingServer} enableToggle getId={(entry) => entry.id} getEnabled={(entry) => entry.enabled} onToggle={(promptId, next) => promptToggleMutation.mutate({ promptId, enable: next })} emptyText={t("detail.directExposure.empty.prompts", { defaultValue: "No prompts found for this server." })} filterText={promptQuery} selectable selectedIds={selectedPromptIds} onSelectToggle={(id) => setSelectedPromptIds((prev) => (prev.includes(id) ? prev.filter((entry) => entry !== id) : [...prev, id]))} />
						</CardContent>
					</TabsContent>
					<TabsContent value="resources" className={DETAIL_TAB_CONTENT_CLASS}>
						<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
							<div className="mb-3 flex flex-wrap items-center justify-end gap-2">
								<Input placeholder={t("detail.directExposure.searchPlaceholders.resources", { defaultValue: "Search resources..." })} value={resourceQuery} onChange={(event) => setResourceQuery(event.target.value)} className="w-48 h-9" />
								<Select value={resourceStatus} onValueChange={(value) => setResourceStatus(value as ToolStatusFilter)}>
									<SelectTrigger className="w-36 h-9"><SelectValue placeholder={t("detail.directExposure.statusPlaceholder", { defaultValue: "Status" })} /></SelectTrigger>
									<SelectContent><SelectItem value="all">{t("detail.directExposure.filters.status.all", { defaultValue: "All" })}</SelectItem><SelectItem value="enabled">{t("detail.directExposure.filters.status.enabled", { defaultValue: "Enabled" })}</SelectItem><SelectItem value="disabled">{t("detail.directExposure.filters.status.disabled", { defaultValue: "Disabled" })}</SelectItem></SelectContent>
								</Select>
								<ButtonGroup className="hidden md:flex ml-2">
									<Button variant="outline" size="sm" onClick={() => setSelectedResourceIds(visibleResources.map((entry) => entry.id))}>{t("detail.directExposure.buttons.selectAll", { defaultValue: "Select all" })}</Button>
									<Button variant="outline" size="sm" onClick={() => setSelectedResourceIds([])}>{t("detail.directExposure.buttons.clearSelection", { defaultValue: "Clear" })}</Button>
									<Button size="sm" disabled={bulkToolsMutation.isPending || selectedResourceIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: true })}>{t("detail.directExposure.buttons.enable", { defaultValue: "Enable" })}</Button>
									<Button size="sm" variant="secondary" disabled={bulkToolsMutation.isPending || selectedResourceIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: false })}>{t("detail.directExposure.buttons.disable", { defaultValue: "Disable" })}</Button>
								</ButtonGroup>
							</div>
							<CapabilityList asCard={false} scrollContainedBody title={t("detail.directExposure.tabs.resources", { defaultValue: "Resources" })} kind="resources" context="profile" items={visibleResources} loading={isLoadingCapabilities || isLoadingServer} enableToggle getId={(entry) => entry.id} getEnabled={(entry) => entry.enabled} onToggle={(resourceId, next) => resourceToggleMutation.mutate({ resourceId, enable: next })} emptyText={t("detail.directExposure.empty.resources", { defaultValue: "No resources found for this server." })} filterText={resourceQuery} selectable selectedIds={selectedResourceIds} onSelectToggle={(id) => setSelectedResourceIds((prev) => (prev.includes(id) ? prev.filter((entry) => entry !== id) : [...prev, id]))} />
						</CardContent>
					</TabsContent>
					<TabsContent value="templates" className={DETAIL_TAB_CONTENT_CLASS}>
						<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
							<div className="mb-3 flex flex-wrap items-center justify-end gap-2">
								<Input placeholder={t("detail.directExposure.searchPlaceholders.templates", { defaultValue: "Search templates..." })} value={templateQuery} onChange={(event) => setTemplateQuery(event.target.value)} className="w-48 h-9" />
								<Select value={templateStatus} onValueChange={(value) => setTemplateStatus(value as ToolStatusFilter)}>
									<SelectTrigger className="w-36 h-9"><SelectValue placeholder={t("detail.directExposure.statusPlaceholder", { defaultValue: "Status" })} /></SelectTrigger>
									<SelectContent><SelectItem value="all">{t("detail.directExposure.filters.status.all", { defaultValue: "All" })}</SelectItem><SelectItem value="enabled">{t("detail.directExposure.filters.status.enabled", { defaultValue: "Enabled" })}</SelectItem><SelectItem value="disabled">{t("detail.directExposure.filters.status.disabled", { defaultValue: "Disabled" })}</SelectItem></SelectContent>
								</Select>
								<ButtonGroup className="hidden md:flex ml-2">
									<Button variant="outline" size="sm" onClick={() => setSelectedTemplateIds(visibleTemplates.map((entry) => entry.id))}>{t("detail.directExposure.buttons.selectAll", { defaultValue: "Select all" })}</Button>
									<Button variant="outline" size="sm" onClick={() => setSelectedTemplateIds([])}>{t("detail.directExposure.buttons.clearSelection", { defaultValue: "Clear" })}</Button>
									<Button size="sm" disabled={bulkToolsMutation.isPending || selectedTemplateIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: true })}>{t("detail.directExposure.buttons.enable", { defaultValue: "Enable" })}</Button>
									<Button size="sm" variant="secondary" disabled={bulkToolsMutation.isPending || selectedTemplateIds.length === 0} onClick={() => bulkToolsMutation.mutate({ enable: false })}>{t("detail.directExposure.buttons.disable", { defaultValue: "Disable" })}</Button>
								</ButtonGroup>
							</div>
							<CapabilityList asCard={false} scrollContainedBody title={t("detail.directExposure.tabs.templates", { defaultValue: "Templates" })} kind="templates" context="profile" items={visibleTemplates} loading={isLoadingCapabilities || isLoadingServer} enableToggle getId={(entry) => entry.id} getEnabled={(entry) => entry.enabled} onToggle={(templateId, next) => templateToggleMutation.mutate({ templateId, enable: next })} emptyText={t("detail.directExposure.empty.templates", { defaultValue: "No templates found for this server." })} filterText={templateQuery} selectable selectedIds={selectedTemplateIds} onSelectToggle={(id) => setSelectedTemplateIds((prev) => (prev.includes(id) ? prev.filter((entry) => entry !== id) : [...prev, id]))} />
						</CardContent>
					</TabsContent>
				</Tabs>
			</Card>
		</div>
	);
}
