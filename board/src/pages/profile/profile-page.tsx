import {
	useMutation,
	useQueries,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import {
	Activity,
	Plus,
	RefreshCw,
	Server,
	Settings,
	Wrench,
} from "lucide-react";
import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { EntityCard } from "../../components/entity-card";
import { EntityListItem } from "../../components/entity-list-item";
import { ListGridContainer } from "../../components/list-grid-container";
import { EmptyState, PageLayout } from "../../components/page-layout";
import { ProfileFormDrawer } from "../../components/profile-form-drawer";
import { StatsCards } from "../../components/stats-cards";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
} from "../../components/ui/card";
import { PageToolbar } from "../../components/ui/page-toolbar";
import { Switch } from "../../components/ui/switch";
import { configSuitsApi, serversApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type {
	ConfigSuit,
	InstanceSummary,
	ServerSummary,
} from "../../lib/types";

const DEFAULT_ANCHOR_ROLE = "default_anchor";

const arrangeSuitsWithDefaultAnchor = (items: ConfigSuit[] = []) => {
	if (!items.length) {
		return [] as ConfigSuit[];
	}
	const isAnchor = (suit: ConfigSuit) =>
		(suit.role ?? "user") === DEFAULT_ANCHOR_ROLE;
	const anchors = items.filter(isAnchor);
	const remaining = items.filter((suit) => !isAnchor(suit));
	const defaults = remaining.filter((suit) => suit.is_default);
	const others = remaining.filter((suit) => !suit.is_default);
	return [...anchors, ...defaults, ...others];
};

type SuitStats = {
	totalServers: number;
	enabledServers: number;
	totalTools: number;
	enabledTools: number;
	totalResources: number;
	enabledResources: number;
	totalPrompts: number;
	enabledPrompts: number;
};

function formatSuitDisplayName(
	name?: string | null,
	fallback?: string,
): string {
	const raw = typeof name === "string" ? name.trim() : "";
	if (raw.length > 0) {
		return raw
			.split(/\s+/)
			.map((word) =>
				word.length > 0
					? word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
					: word,
			)
			.join(" ");
	}
	return (
		fallback ??
		t("profiles:untitledProfile", { defaultValue: "Untitled Profile" })
	);
}

export function ProfilePage() {
	const { t } = useTranslation();
	usePageTranslations("profiles");
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const [searchParams] = useSearchParams();
	const [isNewSuitDialogOpen, setIsNewSuitDialogOpen] = useState(false);

	// Handle URL parameters for profile creation
	const profileType = searchParams.get("type");
	const defaultView = useAppStore(
		(state) => state.dashboardSettings.defaultView,
	);
	const setDashboardSetting = useAppStore((state) => state.setDashboardSetting);

	// 搜索和排序状态
	const [search, setSearch] = useState("");
	const [expanded, setExpanded] = useState(false);

	// 排序后的数据状态
	const [sortedSuits, setSortedSuits] = React.useState<ConfigSuit[]>([]);

	const {
		data: suitsResponse,
		isLoading: isLoadingSuits,
		refetch: refetchSuits,
		isRefetching: isRefetchingSuits,
		error: suitsError,
	} = useQuery({
		queryKey: ["configSuits"],
		queryFn: async () => {
			console.log("Fetching config suits...");
			const result = await configSuitsApi.getAll();
			console.log("Config suits response:", result);
			return result;
		},
		retry: 1,
		refetchInterval: 30000,
	});

	// Get all active suits for aggregated statistics
	// Use a stable empty array reference to avoid render loops when data is not ready
	const EMPTY_SUITS = React.useMemo<ConfigSuit[]>(() => [], []);
	const rawSuits = Array.isArray(suitsResponse?.suits)
		? (suitsResponse!.suits as ConfigSuit[])
		: EMPTY_SUITS;
	const suits = useMemo(
		() => arrangeSuitsWithDefaultAnchor(rawSuits),
		[rawSuits],
	);
	const activeSuits = suits.filter((suit) => suit.is_active);

	// 当 suits 数据变化时更新 sortedSuits（仅在引用变化时）
	React.useEffect(() => {
		setSortedSuits((prev) => (prev === suits ? prev : suits));
	}, [suits]);

	// Get active suit IDs for query keys
	const activeSuitIds = activeSuits.map((suit) => suit.id);

	// Fetch servers data for all active suits
	const activeSuitServersQueries = useQueries({
		queries: activeSuitIds.map((suitId) => ({
			queryKey: ["activeSuitServers", suitId],
			queryFn: () => configSuitsApi.getServers(suitId),
			retry: 1,
		})),
	});

	// Fetch tools data for all active suits
	const activeSuitToolsQueries = useQueries({
		queries: activeSuitIds.map((suitId) => ({
			queryKey: ["activeSuitTools", suitId],
			queryFn: () => configSuitsApi.getTools(suitId),
			retry: 1,
		})),
	});

	// Fetch resources data for all active suits (currently not displayed in UI)
	// const activeSuitResourcesQueries = useQueries({
	// 	queries: activeSuitIds.map(suitId => ({
	// 		queryKey: ["activeSuitResources", suitId],
	// 		queryFn: () => configSuitsApi.getResources(suitId),
	// 		retry: 1,
	// 	})),
	// });

	// Fetch prompts data for all active suits (currently not displayed in UI)
	// const activeSuitPromptsQueries = useQueries({
	// 	queries: activeSuitIds.map(suitId => ({
	// 		queryKey: ["activeSuitPrompts", suitId],
	// 		queryFn: () => configSuitsApi.getPrompts(suitId),
	// 		retry: 1,
	// 	})),
	// });

	// Fetch all servers data for instance count
	const { data: allServersResponse, isLoading: isLoadingAllServers } = useQuery(
		{
			queryKey: ["allServersStats"],
			queryFn: serversApi.getAll,
			retry: 1,
		},
	);

	// Suit activation mutation
	const activateSuitMutation = useMutation({
		mutationFn: configSuitsApi.activateSuit,
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:messages.profileActivated", {
					defaultValue: "Profile activated",
				}),
				t("profiles:messages.profileActivatedDescription", {
					defaultValue: "Profile has been successfully activated",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:messages.activationFailed", {
					defaultValue: "Activation failed",
				}),
				`${t("profiles:messages.activationFailedDescription", { defaultValue: "Failed to activate profile" })}: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Suit deactivation mutation
	const deactivateSuitMutation = useMutation({
		mutationFn: configSuitsApi.deactivateSuit,
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:messages.profileDeactivated", {
					defaultValue: "Profile deactivated",
				}),
				t("profiles:messages.profileDeactivatedDescription", {
					defaultValue: "Profile has been successfully deactivated",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:messages.deactivationFailed", {
					defaultValue: "Deactivation failed",
				}),
				`${t("profiles:messages.deactivationFailedDescription", { defaultValue: "Failed to deactivate profile" })}: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Handle individual suit toggle
	const handleSuitToggle = (suit: ConfigSuit) => {
		if (suit.is_active) {
			deactivateSuitMutation.mutate(suit.id);
		} else {
			activateSuitMutation.mutate(suit.id);
		}
	};

	const orderedSuits = useMemo(
		() => arrangeSuitsWithDefaultAnchor(suits),
		[suits],
	);

	const suitStatQueries = useQueries({
		queries: orderedSuits.map((suit) => ({
			queryKey: ["configSuitStats", suit.id] as const,
			queryFn: async (): Promise<SuitStats> => {
				const [serversRes, toolsRes, resourcesRes, promptsRes] =
					await Promise.allSettled([
						configSuitsApi.getServers(suit.id),
						configSuitsApi.getTools(suit.id),
						configSuitsApi.getResources(suit.id),
						configSuitsApi.getPrompts(suit.id),
					]);

				const servers =
					serversRes.status === "fulfilled"
						? serversRes.value.servers || []
						: [];
				if (serversRes.status === "rejected") {
					console.error(
						"Failed to fetch profile servers stats",
						serversRes.reason,
					);
				}

				const tools =
					toolsRes.status === "fulfilled" ? toolsRes.value.tools || [] : [];
				if (toolsRes.status === "rejected") {
					console.error("Failed to fetch profile tools stats", toolsRes.reason);
				}

				const resources =
					resourcesRes.status === "fulfilled"
						? resourcesRes.value.resources || []
						: [];
				if (resourcesRes.status === "rejected") {
					console.error(
						"Failed to fetch profile resources stats",
						resourcesRes.reason,
					);
				}

				const prompts =
					promptsRes.status === "fulfilled"
						? promptsRes.value.prompts || []
						: [];
				if (promptsRes.status === "rejected") {
					console.error(
						"Failed to fetch profile prompts stats",
						promptsRes.reason,
					);
				}

				return {
					totalServers: servers.length,
					enabledServers: servers.filter((server) => server.enabled).length,
					totalTools: tools.length,
					enabledTools: tools.filter((tool) => tool.enabled).length,
					totalResources: resources.length,
					enabledResources: resources.filter((resource) => resource.enabled)
						.length,
					totalPrompts: prompts.length,
					enabledPrompts: prompts.filter((prompt) => prompt.enabled).length,
				};
			},
			enabled: !!suit.id,
			staleTime: 30000,
		})),
	});

	// Aggregate data from all active suits using OR logic (deduplication)
	const allServers = allServersResponse?.servers || [];

	// Aggregate servers from all active suits
	const aggregatedServers = useMemo(() => {
		const serverMap = new Map();

		activeSuitServersQueries.forEach((query) => {
			if (query.data?.servers) {
				query.data.servers.forEach((server) => {
					// Use OR logic: if server exists, keep the enabled state if any profile has it enabled
					const existing = serverMap.get(server.id);
					if (!existing || server.enabled) {
						serverMap.set(server.id, { ...server, enabled: server.enabled });
					}
				});
			}
		});

		return Array.from(serverMap.values());
	}, [activeSuitServersQueries]);

	// Aggregate tools from all active suits
	const aggregatedTools = useMemo(() => {
		const toolMap = new Map();

		activeSuitToolsQueries.forEach((query) => {
			if (query.data?.tools) {
				query.data.tools.forEach((tool) => {
					// Use OR logic: if tool exists, keep the enabled state if any profile has it enabled
					const existing = toolMap.get(tool.id);
					if (!existing || tool.enabled) {
						toolMap.set(tool.id, { ...tool, enabled: tool.enabled });
					}
				});
			}
		});

		return Array.from(toolMap.values());
	}, [activeSuitToolsQueries]);

	// Aggregate resources from all active suits (currently not displayed in UI)
	// const aggregatedResources = useMemo(() => {
	// 	const resourceMap = new Map();
	//
	// 	activeSuitResourcesQueries.forEach(query => {
	// 		if (query.data?.resources) {
	// 			query.data.resources.forEach(resource => {
	// 				// Use OR logic: if resource exists, keep the enabled state if any profile has it enabled
	// 				const existing = resourceMap.get(resource.id);
	// 				if (!existing || resource.enabled) {
	// 					resourceMap.set(resource.id, { ...resource, enabled: resource.enabled });
	// 				}
	// 			});
	// 		}
	// 	});
	//
	// 	return Array.from(resourceMap.values());
	// }, [activeSuitResourcesQueries]);

	// Aggregate prompts from all active suits (currently not displayed in UI)
	// const aggregatedPrompts = useMemo(() => {
	// 	const promptMap = new Map();
	//
	// 	activeSuitPromptsQueries.forEach(query => {
	// 		if (query.data?.prompts) {
	// 			query.data.prompts.forEach(prompt => {
	// 				// Use OR logic: if prompt exists, keep the enabled state if any profile has it enabled
	// 				const existing = promptMap.get(prompt.id);
	// 				if (!existing || prompt.enabled) {
	// 					promptMap.set(prompt.id, { ...prompt, enabled: prompt.enabled });
	// 				}
	// 			});
	// 		}
	// 	});
	//
	// 	return Array.from(promptMap.values());
	// }, [activeSuitPromptsQueries]);

	// Calculate aggregated statistics
	const enabledServersCount = aggregatedServers.filter(
		(server) => server.enabled,
	).length;
	const totalServersInSuit = aggregatedServers.length;

	const enabledToolsCount = aggregatedTools.filter(
		(tool) => tool.enabled,
	).length;
	const totalToolsInSuit = aggregatedTools.length;

	// Note: Resources and Prompts statistics are calculated but not displayed in the current UI
	// const enabledResourcesCount = aggregatedResources.filter(resource => resource.enabled).length;
	// const totalResourcesInSuit = aggregatedResources.length;
	// const enabledPromptsCount = aggregatedPrompts.filter(prompt => prompt.enabled).length;
	// const totalPromptsInSuit = aggregatedPrompts.length;

	// Instances statistics: count instances from all enabled servers across all active suits
	const enabledServerNames = aggregatedServers
		.filter((server) => server.enabled)
		.map((server) => server.name);

	const totalInstances = allServers
		.filter((server) => enabledServerNames.includes(server.name))
		.reduce(
			(sum: number, server: ServerSummary) =>
				sum + (server.instances?.length || 0),
			0,
		);

	const readyInstances = allServers
		.filter((server) => enabledServerNames.includes(server.name))
		.reduce((sum: number, server: ServerSummary) => {
			if (server.instances) {
				return (
					sum +
					server.instances.filter(
						(instance: InstanceSummary) =>
							instance.status &&
							["ready", "busy", "running"].includes(
								instance.status.toLowerCase(),
							),
					).length
				);
			}
			return sum;
		}, 0);

	const getSuitStats = (index: number) => {
		const statsQuery = suitStatQueries[index];
		const stats = statsQuery?.data;
		const statsLoading = Boolean(
			statsQuery?.isFetching || statsQuery?.isLoading,
		);
		const formatCount = (enabled?: number, total?: number) => {
			if (statsLoading) return "...";
			const safeEnabled = typeof enabled === "number" ? enabled : 0;
			const safeTotal = typeof total === "number" ? total : 0;
			return `${safeEnabled}/${safeTotal}`;
		};
		return { stats, statsLoading, formatCount };
	};

	const isTogglePending =
		activateSuitMutation.isPending || deactivateSuitMutation.isPending;

	const renderSuitListItem = (suit: ConfigSuit, index: number) => {
		const { stats, formatCount } = getSuitStats(index);
		const displayName = formatSuitDisplayName(suit.name, suit.id);
		const avatarInitial = displayName.charAt(0).toUpperCase() || "P";
		const suitRole = suit.role ?? "user";
		const isDefaultAnchor = suitRole === "default_anchor";
		const isDefaultMember = suit.is_default;

		return (
			<EntityListItem
				key={suit.id}
				id={suit.id}
				title={displayName}
				description={suit.description}
				avatar={{
					fallback: avatarInitial,
				}}
				titleBadges={[
					isDefaultAnchor ? (
						<Badge key="default-anchor" variant="outline">
							{t("profiles:badges.defaultAnchor", {
								defaultValue: "Default Anchor",
							})}
						</Badge>
					) : isDefaultMember ? (
						<Badge key="in-default" variant="outline">
							{t("profiles:badges.inDefault", { defaultValue: "In Default" })}
						</Badge>
					) : null,
				].filter(Boolean)}
				stats={[
					{
						label: t("profiles:badges.multiSelect", {
							defaultValue: "Multi-select",
						}),
						value: suit.multi_select
							? t("yes", { defaultValue: "Yes" })
							: t("no", { defaultValue: "No" }),
					},
					{
						label: t("profiles:badges.priority", { defaultValue: "Priority" }),
						value: suit.priority,
					},
				]}
				bottomTags={[
					<span key="servers">
						{t("profiles:badges.servers", { defaultValue: "Servers" })}:{" "}
						{formatCount(stats?.enabledServers, stats?.totalServers)}
					</span>,
					<span key="tools">
						{t("profiles:badges.tools", { defaultValue: "Tools" })}:{" "}
						{formatCount(stats?.enabledTools, stats?.totalTools)}
					</span>,
					<span key="resources">
						{t("profiles:badges.resources", { defaultValue: "Resources" })}:{" "}
						{formatCount(stats?.enabledResources, stats?.totalResources)}
					</span>,
					<span key="prompts">
						{t("profiles:badges.prompts", { defaultValue: "Prompts" })}:{" "}
						{formatCount(stats?.enabledPrompts, stats?.totalPrompts)}
					</span>,
				]}
				statusBadge={
					<Badge variant={suit.is_active ? "default" : "secondary"}>
						{t(`profiles:suitTypes.${suit.suit_type}`, {
							defaultValue: suit.suit_type,
						})}
					</Badge>
				}
				enableSwitch={{
					checked: suit.is_active,
					onChange: () => handleSuitToggle(suit),
					disabled: isTogglePending || isDefaultAnchor,
				}}
				onClick={() => navigate(`/profiles/${suit.id}`)}
			/>
		);
	};

	const renderSuitCard = (suit: ConfigSuit, index: number) => {
		const { stats, formatCount } = getSuitStats(index);
		const displayName = formatSuitDisplayName(suit.name, suit.id);
		const avatarInitial = displayName.charAt(0).toUpperCase() || "P";
		const suitRole = suit.role ?? "user";
		const isDefaultAnchor = suitRole === "default_anchor";
		const statItems = [
			{
				label: t("profiles:badges.servers", { defaultValue: "Servers" }),
				value: formatCount(stats?.enabledServers, stats?.totalServers),
			},
			{
				label: t("profiles:badges.tools", { defaultValue: "Tools" }),
				value: formatCount(stats?.enabledTools, stats?.totalTools),
			},
			{
				label: t("profiles:badges.resources", { defaultValue: "Resources" }),
				value: formatCount(stats?.enabledResources, stats?.totalResources),
			},
			{
				label: t("profiles:badges.prompts", { defaultValue: "Prompts" }),
				value: formatCount(stats?.enabledPrompts, stats?.totalPrompts),
			},
		];

		return (
			<EntityCard
				key={suit.id}
				id={suit.id}
				title={displayName}
				description={suit.description}
				avatar={{
					fallback: avatarInitial,
				}}
				topRightBadge={
					isDefaultAnchor ? (
						<Badge variant="outline" className="shrink-0">
							{t("profiles:badges.defaultAnchor", {
								defaultValue: "Default Anchor",
							})}
						</Badge>
					) : suit.is_default ? (
						<Badge variant="outline" className="shrink-0">
							{t("profiles:badges.inDefault", { defaultValue: "In Default" })}
						</Badge>
					) : undefined
				}
				stats={statItems}
				bottomLeft={
					<Badge variant={suit.is_active ? "default" : "secondary"}>
						{t(`profiles:suitTypes.${suit.suit_type}`, {
							defaultValue: suit.suit_type,
						})}
					</Badge>
				}
				bottomRight={
					<Switch
						checked={suit.is_active}
						onCheckedChange={() => handleSuitToggle(suit)}
						disabled={isTogglePending || isDefaultAnchor}
						onClick={(e) => e.stopPropagation()}
					/>
				}
				onClick={() => navigate(`/profiles/${suit.id}`)}
			/>
		);
	};

	// 使用排序后的数据，保持默认套件在前的顺序
	const filteredAndSortedSuits = useMemo(
		() => arrangeSuitsWithDefaultAnchor(sortedSuits),
		[sortedSuits],
	);

	// Prepare stats cards data
	const statsCards = [
		{
			title: t("profiles:stats.profiles", { defaultValue: "Profiles" }),
			value: isLoadingSuits ? "..." : `${activeSuits.length}/${suits.length}`,
			description: t("profiles:stats.activeProfiles", {
				defaultValue: "active profiles",
			}),
			icon: <Settings className="h-4 w-4 text-emerald-600" />,
		},
		{
			title: t("profiles:stats.servers", { defaultValue: "Servers" }),
			value:
				activeSuitServersQueries.some((query) => query.isLoading) ||
				isLoadingSuits
					? "..."
					: `${enabledServersCount}/${totalServersInSuit}`,
			description: t("profiles:stats.running", { defaultValue: "running" }),
			icon: <Server className="h-4 w-4 text-blue-600" />,
		},
		{
			title: t("profiles:stats.tools", { defaultValue: "Tools" }),
			value:
				activeSuitToolsQueries.some((query) => query.isLoading) ||
				isLoadingSuits
					? "..."
					: `${enabledToolsCount}/${totalToolsInSuit}`,
			description: t("profiles:stats.enabled", { defaultValue: "enabled" }),
			icon: <Wrench className="h-4 w-4 text-purple-600" />,
		},
		{
			title: t("profiles:stats.instances", { defaultValue: "Instances" }),
			value:
				isLoadingAllServers ||
				activeSuitServersQueries.some((query) => query.isLoading) ||
				isLoadingSuits
					? "..."
					: `${readyInstances}/${totalInstances}`,
			description: t("profiles:stats.ready", { defaultValue: "ready" }),
			icon: <Activity className="h-4 w-4 text-orange-600" />,
		},
	];

	// Prepare loading skeleton
	const loadingSkeleton =
		defaultView === "grid"
			? Array.from({ length: 6 }, (_, index) => {
					const cardId = `loading-grid-${index}`;
					return (
						<Card
							key={cardId}
							className="animate-pulse border border-slate-200 dark:border-slate-800"
						>
							<CardHeader className="space-y-2">
								<div className="h-5 w-32 rounded bg-slate-200 dark:bg-slate-800"></div>
								<div className="h-4 w-48 rounded bg-slate-200 dark:bg-slate-800"></div>
							</CardHeader>
							<CardContent className="space-y-4">
								<div className="grid grid-cols-4 gap-x-6 gap-y-2">
									{Array.from({ length: 4 }, (__, statIndex) => {
										const labelId = `${cardId}-label-${statIndex}`;
										return (
											<div
												key={labelId}
												className="h-3 w-16 rounded bg-slate-200 dark:bg-slate-800"
											></div>
										);
									})}
									{Array.from({ length: 4 }, (__, statIndex) => {
										const valueId = `${cardId}-value-${statIndex}`;
										return (
											<div
												key={valueId}
												className="h-5 w-20 rounded bg-slate-200 dark:bg-slate-800"
											></div>
										);
									})}
								</div>
							</CardContent>
							<CardFooter className="flex items-center justify-between gap-3 border-t border-slate-100 px-4 py-3 dark:border-slate-800">
								<div className="h-5 w-20 rounded-full bg-slate-200 dark:bg-slate-800"></div>
								<div className="flex items-center gap-2">
									<div className="h-3 w-14 rounded bg-slate-200 dark:bg-slate-800"></div>
									<div className="h-6 w-12 rounded bg-slate-200 dark:bg-slate-800"></div>
								</div>
							</CardFooter>
						</Card>
					);
				})
			: Array.from({ length: 3 }, (_, id) => {
					const suitId = `loading-suit-${id}`;
					return (
						<div
							key={suitId}
							className="flex items-center justify-between rounded-lg border border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-950"
						>
							<div className="space-y-1">
								<div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
								<div className="h-4 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
							</div>
							<div className="h-9 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						</div>
					);
				});

	// 工具栏配置
	const toolbarConfig = {
		data: (suitsResponse?.suits || []) as ConfigSuit[],
		search: {
			placeholder: t("profiles:searchPlaceholder", {
				defaultValue: "Search profiles...",
			}),
			fields: [
				{
					key: "name",
					label: t("profiles:fields.name", { defaultValue: "Name" }),
					weight: 10,
				},
				{
					key: "description",
					label: t("profiles:fields.description", {
						defaultValue: "Description",
					}),
					weight: 8,
				},
			],
			debounceMs: 300,
		},
		viewMode: {
			enabled: true,
			defaultMode: defaultView as "grid" | "list",
		},
		sort: {
			enabled: true,
			options: [
				{
					value: "name",
					label: t("profiles:fields.name", { defaultValue: "Name" }),
					defaultDirection: "asc" as const,
				},
				{
					value: "is_active",
					label: t("profiles:sort.activeStatus", {
						defaultValue: "Active Status",
					}),
					defaultDirection: "desc" as const,
				},
			],
			defaultSort: "name",
		},
	};

	// 工具栏状态
	const toolbarState = {
		search,
		viewMode: defaultView,
		sort: "name", // 添加必需的 sort 属性
		expanded,
	};

	// 工具栏回调
	const toolbarCallbacks = {
		onSearchChange: setSearch,
		onViewModeChange: (mode: "grid" | "list") => {
			// 直接更新全局设置
			setDashboardSetting("defaultView", mode);
		},
		onSortedDataChange: (data: ConfigSuit[]) =>
			setSortedSuits(arrangeSuitsWithDefaultAnchor(data)),
		onExpandedChange: setExpanded,
	};

	// 操作按钮
	const actions = (
		<div className="flex items-center gap-2">
			<Button
				onClick={() => refetchSuits()}
				disabled={isRefetchingSuits}
				variant="outline"
				size="sm"
				className="h-9 w-9 p-0"
				title={t("profiles:buttons.refresh", { defaultValue: "Refresh" })}
			>
				<RefreshCw
					className={`h-4 w-4 ${isRefetchingSuits ? "animate-spin" : ""}`}
				/>
			</Button>
			<Button
				size="sm"
				className="h-9 w-9 p-0"
				onClick={() => setIsNewSuitDialogOpen(true)}
				title={t("profiles:buttons.newProfile", {
					defaultValue: "New Profile",
				})}
			>
				<Plus className="h-4 w-4" />
			</Button>
		</div>
	);

	// Prepare empty state
	const emptyState = (
		<Card>
			<CardContent className="flex flex-col items-center justify-center p-6">
				<EmptyState
					icon={<Settings className="h-12 w-12" />}
					title={t("profiles:emptyState.title", {
						defaultValue: "No profiles found",
					})}
					description={t("profiles:emptyState.description", {
						defaultValue:
							"Profiles help organize and manage your MCP servers, tools, and resources",
					})}
					action={
						<Button
							onClick={() => setIsNewSuitDialogOpen(true)}
							size="sm"
							className="mt-4"
						>
							<Plus className="mr-2 h-4 w-4" />
							{t("profiles:buttons.createFirst", {
								defaultValue: "Create First Profile",
							})}
						</Button>
					}
				/>
			</CardContent>
		</Card>
	);

	return (
		<PageLayout
			title={t("profiles:title", { defaultValue: "Profiles" })}
			headerActions={
				<PageToolbar
					config={toolbarConfig as any}
					state={toolbarState}
					callbacks={toolbarCallbacks as any}
					actions={actions}
				/>
			}
			statsCards={<StatsCards cards={statsCards} />}
		>
			{suitsError && (
				<div className="bg-red-50 border border-red-200 rounded-md p-4">
					<h3 className="text-red-800 font-medium">
						{t("profiles:errors.loadingFailed", {
							defaultValue: "Error loading profiles:",
						})}
					</h3>
					<p className="text-red-600 text-sm mt-1">{String(suitsError)}</p>
				</div>
			)}

			<ListGridContainer
				loading={isLoadingSuits}
				loadingSkeleton={loadingSkeleton}
				emptyState={
					filteredAndSortedSuits.length === 0 ? emptyState : undefined
				}
			>
				{defaultView === "grid"
					? filteredAndSortedSuits.map(renderSuitCard)
					: filteredAndSortedSuits.map(renderSuitListItem)}
			</ListGridContainer>

			{/* New Suit Drawer */}
			<ProfileFormDrawer
				open={isNewSuitDialogOpen}
				onOpenChange={setIsNewSuitDialogOpen}
				mode="create"
				restrictProfileType={profileType || undefined}
				onSuccess={() => {
					setIsNewSuitDialogOpen(false);
					refetchSuits();
				}}
			/>
		</PageLayout>
	);
}
