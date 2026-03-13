import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Globe, Plus, RefreshCw, ToggleLeft } from "lucide-react";
import React, { type MouseEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { EntityCard } from "../../components/entity-card";
import { EntityListItem } from "../../components/entity-list-item";
import { ListGridContainer } from "../../components/list-grid-container";
import { EmptyState, PageLayout } from "../../components/page-layout";
import { StatsCards } from "../../components/stats-cards";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { PageToolbar } from "../../components/ui/page-toolbar";
import { Switch } from "../../components/ui/switch";
import type { SegmentOption } from "../../components/ui/segment";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "../../components/ui/select";
import { clientsApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { ClientListDefaultFilter } from "../../lib/store";

export function ClientsPage() {
	const navigate = useNavigate();
	const qc = useQueryClient();
	const [refreshing, setRefreshing] = useState(false);
	usePageTranslations("clients");
	const { t, i18n } = useTranslation("clients");
    const { defaultView, defaultFilter, setDashboardSetting } = useAppStore((state) => ({
        defaultView: state.dashboardSettings.defaultView,
        defaultFilter: state.dashboardSettings.clientListDefaultFilter,
        setDashboardSetting: state.setDashboardSetting,
    }));

    const { data, isLoading, isRefetching, refetch, error, isError } = useQuery({
		queryKey: ["clients"],
		queryFn: async () => {
			const resp = await clientsApi.list(false);
			return resp;
		},
		staleTime: 10_000,
		retry: false,
		refetchOnWindowFocus: false,
		refetchOnReconnect: false,
	});

	// Use a stable empty array reference to avoid re-render loops when data is undefined or on errors
	const EMPTY: any[] = React.useMemo(() => [], []);
	const clients =
		data && Array.isArray((data as any).client) ? (data as any).client : EMPTY;
	const detectedCount = clients.filter((c: any) => !!c.detected).length;
	const managedCount = clients.filter((c: any) => !!c.managed).length;
	const configuredCount = clients.filter((c: any) => !!c.has_mcp_config).length;

    const clientsAsEntities = React.useMemo(() => {
        const mapped = clients.map((client: any) => ({
            id: client.identifier || client.display_name || "",
            name: client.display_name || client.identifier || "",
            description: client.description || "",
            ...client,
        }));
        // Default stable sort by name A→Z, tie-breaker by id
        mapped.sort((a, b) => {
            const byName = a.name.localeCompare(b.name, undefined, {
                sensitivity: "base",
            });
            if (byName !== 0) return byName;
            return a.id.localeCompare(b.id, undefined, { sensitivity: "base" });
        });
        return mapped;
    }, [clients]);

    // Temporary filter state in toolbar; initialized from Settings
    const [filter, setFilter] = React.useState<ClientListDefaultFilter>(defaultFilter);

    // Keep in sync with Settings if changed elsewhere
    React.useEffect(() => {
        setFilter(defaultFilter);
    }, [defaultFilter]);

    // Apply visibility filter from toolbar
    const filteredClientsAsEntities = React.useMemo(() => {
        if (filter === "detected") {
            return clientsAsEntities.filter((c: any) => !!c.detected);
        }
        if (filter === "managed") {
            return clientsAsEntities.filter((c: any) => !!c.managed);
        }
        return clientsAsEntities;
    }, [clientsAsEntities, filter]);

	// 排序后的数据状态
    const [sortedClients, setSortedClients] = React.useState(
        filteredClientsAsEntities,
    );

	// 同步最新数据源
    React.useEffect(() => {
        // Only update when reference actually changed to prevent update depth issues
        setSortedClients((prev) =>
            prev === filteredClientsAsEntities ? prev : filteredClientsAsEntities,
        );
    }, [filteredClientsAsEntities]);
	const [search, setSearch] = React.useState("");

	const manageMutation = useMutation({
		mutationFn: async ({
			identifier,
			managed,
		}: {
			identifier: string;
			managed: boolean;
		}) => {
			const result = await clientsApi.manage(
				identifier,
				managed ? "enable" : "disable",
			);
			return result;
		},
		onSuccess: async () => {
			try {
				// Force backend to refresh detection/config state, then sync cache
				const fresh = await clientsApi.list(true);
				qc.setQueryData(["clients"], fresh as any);
			} catch {
				/* noop */
			}
			qc.invalidateQueries({ queryKey: ["clients"] });
			notifySuccess(
				t("notifications.managementUpdated.title", {
					defaultValue: "Updated",
				}),
				t("notifications.managementUpdated.message", {
					defaultValue: "Client management state updated",
				}),
			);
		},
		onError: (err) =>
			notifyError(
				t("notifications.operationFailed.title", {
					defaultValue: "Operation failed",
				}),
				String(err),
			),
	});

	const renderClientListItem = (client: any) => {
		const displayName =
			client.display_name ||
			client.identifier ||
			t("entity.fallbackName", { defaultValue: "Client" });
		const identifier = client.identifier || "—";
		const avatarInitial =
			(displayName.trim() || identifier).charAt(0).toUpperCase() || "C";
		const serverCount = client.mcp_servers_count ?? 0;
		const configPath =
			client.config_path ||
			t("entity.config.notConfigured", { defaultValue: "Not configured" });
		const description =
			client.description ?? client.template?.description ?? "";

		const configLabel = t("entity.stats.config", { defaultValue: "Config" });
		const serversTag = t("entity.bottomTags.servers", {
			count: serverCount,
			defaultValue: "Servers: {{count}}",
		});
		const detectedLabel = t("entity.badge.detected", {
			defaultValue: "Detected",
		});
		const notDetectedLabel = t("entity.badge.notDetected", {
			defaultValue: "Not Detected",
		});

		return (
			<EntityListItem
				key={client.identifier}
				id={client.identifier}
				title={displayName}
				description={description}
				avatar={{
					src: client.logo_url,
					alt: displayName,
					fallback: avatarInitial,
				}}
				stats={[{ label: configLabel, value: configPath }]}
				bottomTags={[<span key="servers">{serversTag}</span>]}
				statusBadge={
					client.detected ? (
						<span className="flex items-center rounded-full bg-emerald-50 px-2 py-1 text-xs font-medium text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400">
							<Check className="mr-1 h-3 w-3" /> {detectedLabel}
						</span>
					) : (
						<Badge variant="secondary">{notDetectedLabel}</Badge>
					)
				}
				enableSwitch={{
					checked: client.managed,
					onChange: (checked) =>
						manageMutation.mutate({ identifier, managed: checked }),
					disabled: manageMutation.isPending,
				}}
				onClick={() => navigate(`/clients/${encodeURIComponent(identifier)}`)}
			/>
		);
	};

	const renderClientCard = (client: any) => {
		const displayName =
			client.display_name ||
			client.identifier ||
			t("entity.fallbackName", { defaultValue: "Client" });
		const identifier = client.identifier || "—";
		const avatarInitial =
			(displayName.trim() || identifier).charAt(0).toUpperCase() || "C";
		const description =
			client.description ?? client.template?.description ?? "";
		const homepageUrl =
			client.homepage_url ?? client.template?.homepage_url ?? null;
		const statItems = [
			{
				label: t("entity.stats.servers", { defaultValue: "Servers" }),
				value: (client.mcp_servers_count ?? 0).toString(),
			},
			{
				label: t("entity.stats.managed", { defaultValue: "Managed" }),
				value: client.managed
					? t("states.on", { defaultValue: "On" })
					: t("states.off", { defaultValue: "Off" }),
			},
			{
				label: t("entity.stats.detected", { defaultValue: "Detected" }),
				value: client.detected
					? t("states.yes", { defaultValue: "Yes" })
					: t("states.no", { defaultValue: "No" }),
			},
			{
				label: t("entity.stats.config", { defaultValue: "Config" }),
				value: client.has_mcp_config
					? t("states.present", { defaultValue: "Present" })
					: t("states.missing", { defaultValue: "Missing" }),
			},
		];
		const detectedBadge = client.detected ? (
			<Badge variant="default">
				{t("entity.badge.detected", { defaultValue: "Detected" })}
			</Badge>
		) : (
			<Badge variant="secondary">
				{t("entity.badge.notDetected", { defaultValue: "Not Detected" })}
			</Badge>
		);
		const quickLinks = (
			[
				{
					label: t("detail.overview.labels.homepage", {
						defaultValue: "Homepage",
					}),
					url: homepageUrl,
					icon: Globe,
				},
			] as const
		).filter((link) => !!link.url);

		const handleQuickLink = (event: MouseEvent, url: string) => {
			event.stopPropagation();
			if (!url) return;
			try {
				window.open(url, "_blank", "noopener,noreferrer");
			} catch {
				/* noop */
			}
		};

		return (
			<EntityCard
				key={`client-card-${identifier}`}
				id={identifier}
				title={displayName}
				description={description}
				avatar={{
					src: client.logo_url,
					alt: displayName,
					fallback: avatarInitial,
				}}
				avatarShape="rounded"
				stats={statItems}
				bottomLeft={
					<div className="flex flex-wrap items-center gap-2">
						{detectedBadge}
						{quickLinks.map((link) => (
							<button
								key={`${identifier}-${link.label}`}
								type="button"
								className="inline-flex items-center justify-center rounded-full border border-transparent bg-transparent h-5 w-5 text-slate-400 transition hover:text-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 dark:text-slate-500 dark:hover:text-slate-300"
								onClick={(event) => handleQuickLink(event, link.url!)}
								title={link.label}
							>
								<link.icon className="w-4" />
								<span className="sr-only">{link.label}</span>
							</button>
						))}
					</div>
				}
				bottomRight={
					<Switch
						checked={client.managed}
						onCheckedChange={(checked) =>
							manageMutation.mutate({ identifier, managed: checked })
						}
						onClick={(e) => e.stopPropagation()}
						disabled={manageMutation.isPending}
					/>
				}
				onClick={() => navigate(`/clients/${encodeURIComponent(identifier)}`)}
			/>
		);
	};

	const handleRefresh = async () => {
		setRefreshing(true);
		try {
			await clientsApi.list(true);
		} catch {
			/* noop */
		}
		await refetch();
		setRefreshing(false);
	};

	// Prepare stats cards data
	const statsCards = React.useMemo(
		() => [
			{
				title: t("statsCards.total.title", {
					defaultValue: "Total Clients",
				}),
				value: clients.length,
				description: t("statsCards.total.description", {
					defaultValue: "discovered",
				}),
			},
			{
				title: t("statsCards.detected.title", {
					defaultValue: "Detected",
				}),
				value: `${detectedCount}/${clients.length}`,
				description: t("statsCards.detected.description", {
					defaultValue: "installed",
				}),
			},
			{
				title: t("statsCards.managed.title", {
					defaultValue: "Managed",
				}),
				value: managedCount,
				description: t("statsCards.managed.description", {
					defaultValue: "management enabled",
				}),
			},
			{
				title: t("statsCards.configured.title", {
					defaultValue: "Configured",
				}),
				value: configuredCount,
				description: t("statsCards.configured.description", {
					defaultValue: "has MCP config",
				}),
			},
		],
		[
			clients.length,
			detectedCount,
			managedCount,
			configuredCount,
			i18n.language,
		],
	);

	// Prepare loading skeleton
	const loadingSkeleton =
		defaultView === "grid"
			? Array.from({ length: 6 }, (_, index) => (
					<Card key={`client-skeleton-grid-${index}`} className="p-4">
						<div className="flex items-start gap-3">
							<div className="h-12 w-12 animate-pulse rounded-[10px] bg-slate-200 dark:bg-slate-800" />
							<div className="flex-1 space-y-2">
								<div className="h-4 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
								<div className="h-3 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
							</div>
						</div>
						<div className="mt-4 grid grid-cols-2 gap-3">
							{Array.from({ length: 4 }, (__, sIdx) => (
								<div
									key={`client-skeleton-stat-grid-${index}-${sIdx}`}
									className="space-y-2"
								>
									<div className="h-3 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
									<div className="h-4 w-20 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
								</div>
							))}
						</div>
					</Card>
				))
			: Array.from({ length: 3 }, (_, index) => (
					<div
						key={`client-skeleton-list-${index}`}
						className="flex items-center justify-between rounded-lg border border-slate-200 bg-white px-4 py-4 dark:border-slate-800 dark:bg-slate-950"
					>
						<div className="flex items-center gap-3">
							<div className="h-12 w-12 animate-pulse rounded-[10px] bg-slate-200 dark:bg-slate-800" />
							<div className="space-y-2">
								<div className="h-4 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
								<div className="h-3 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
							</div>
						</div>
						<div className="h-9 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
					</div>
				));

	// Prepare empty state
	const emptyState = (
		<Card>
			<CardContent className="flex flex-col items-center justify-center p-6">
				<EmptyState
					icon={<ToggleLeft className="h-12 w-12" />}
					title={t("emptyState.title", {
						defaultValue: "No clients found",
					})}
					description={t("emptyState.description", {
						defaultValue:
							"Make sure MCPMate backend is running and detection is enabled",
					})}
				/>
			</CardContent>
		</Card>
	);

	// 展开状态
	const [expanded, setExpanded] = useState(false);

	// 工具栏配置
    const toolbarConfig = React.useMemo(
        () => ({
        data: filteredClientsAsEntities,
			search: {
				placeholder: t("toolbar.search.placeholder", {
					defaultValue: "Search clients...",
				}),
				fields: [
					{
						key: "display_name",
						label: t("toolbar.search.fields.displayName", {
							defaultValue: "Display Name",
						}),
						weight: 10,
					},
					{
						key: "identifier",
						label: t("toolbar.search.fields.identifier", {
							defaultValue: "Identifier",
						}),
						weight: 8,
					},
					{
						key: "description",
						label: t("toolbar.search.fields.description", {
							defaultValue: "Description",
						}),
						weight: 5,
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
						value: "display_name",
						label: t("toolbar.sort.options.displayName", {
							defaultValue: "Name",
						}),
						defaultDirection: "asc" as const,
					},
					{
						value: "detected",
						label: t("toolbar.sort.options.detected", {
							defaultValue: "Detection Status",
						}),
						defaultDirection: "desc" as const,
					},
					{
						value: "managed",
						label: t("toolbar.sort.options.managed", {
							defaultValue: "Management Status",
						}),
						defaultDirection: "desc" as const,
					},
				],
				defaultSort: "display_name",
			},
        }),
        [filteredClientsAsEntities, defaultView, i18n.language, t],
    );

	// 工具栏状态
	const toolbarState = {
		search,
		viewMode: defaultView,
		sort: "display_name", // 添加必需的 sort 属性
		expanded,
	};

	// 工具栏回调
	const toolbarCallbacks = {
		onSearchChange: setSearch,
		onViewModeChange: (mode: "grid" | "list") => {
			// 直接更新全局设置
			setDashboardSetting("defaultView", mode);
		},
		onSortedDataChange: setSortedClients,
		onExpandedChange: setExpanded,
	};

    // Toolbar: quick filter options (All / Detected / Managed)
    const filterOptions: SegmentOption[] = React.useMemo(
        () => [
            { value: "all", label: t("toolbar.filters.options.all", { defaultValue: "All" }) },
            { value: "detected", label: t("toolbar.filters.options.detected", { defaultValue: "Detected" }) },
            { value: "managed", label: t("toolbar.filters.options.managed", { defaultValue: "Managed" }) },
        ],
        [t, i18n.language],
    );

    // 过滤器节点（由 PageToolbar 控制显示时机）
    const filterNode = (
        <div className="w-32">
            <Select
                value={filter}
                onValueChange={(value) => setFilter(value as ClientListDefaultFilter)}
            >
                <SelectTrigger className="h-9 w-full" aria-label={t("toolbar.filters.title", { defaultValue: "Filter" })}>
                    <SelectValue placeholder={t("toolbar.filters.title", { defaultValue: "Filter" })} />
                </SelectTrigger>
                <SelectContent align="end">
                    {filterOptions.map((opt) => (
                        <SelectItem key={opt.value as string} value={opt.value as string}>
                            {opt.label}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
        </div>
    );

    // 操作按钮（刷新 / 新增）
    const actions = (
        <div className="flex items-center gap-2">
            <Button
                onClick={handleRefresh}
                disabled={isRefetching || refreshing}
                variant="outline"
                size="sm"
                className="h-9 w-9 p-0"
                onMouseUp={() =>
                    notifyInfo(
                        t("toolbar.actions.refresh.notificationTitle", {
                            defaultValue: "Refresh triggered",
                        }),
                        t("toolbar.actions.refresh.notificationMessage", {
                            defaultValue: "Latest client state will sync to the list",
                        }),
                    )
                }
                title={t("toolbar.actions.refresh.title", {
                    defaultValue: "Refresh",
                })}
            >
                <RefreshCw
                    className={`h-4 w-4 ${isRefetching || refreshing ? "animate-spin" : ""}`}
                />
            </Button>
            <Button
                size="sm"
                className="h-9 w-9 p-0"
                onClick={() =>
                    notifyInfo(
                        t("toolbar.actions.add.notificationTitle", {
                            defaultValue: "Feature in Development",
                        }),
                        t("toolbar.actions.add.notificationMessage", {
                            defaultValue:
                                "This feature is being implemented, please stay tuned",
                        }),
                    )
                }
                title={t("toolbar.actions.add.title", {
                    defaultValue: "Add Client",
                })}
            >
                <Plus className="h-4 w-4" />
            </Button>
        </div>
    );

	return (
		<PageLayout
			title={t("title", { defaultValue: "Clients" })}
			headerActions={
                    <PageToolbar
                        config={toolbarConfig}
                        state={toolbarState}
                        callbacks={toolbarCallbacks}
                        filters={filterNode}
                        actions={actions}
                    />
			}
			statsCards={<StatsCards cards={statsCards} />}
		>
			<ListGridContainer
				loading={isLoading}
				loadingSkeleton={loadingSkeleton}
				emptyState={sortedClients.length === 0 ? emptyState : undefined}
			>
				{defaultView === "grid"
					? sortedClients.map((client) => renderClientCard(client))
					: sortedClients.map((client) => renderClientListItem(client))}
			</ListGridContainer>
		</PageLayout>
	);
}

export default ClientsPage;
