import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Plus, RefreshCw, ToggleLeft } from "lucide-react";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { EntityListItem } from "../../components/entity-list-item";
import { ListGridContainer } from "../../components/list-grid-container";
import { EmptyState, PageLayout } from "../../components/page-layout";
import { StatsCards } from "../../components/stats-cards";
import { ClientFormDrawer } from "../../components/client-form-drawer";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { PageToolbar } from "../../components/ui/page-toolbar";
import type { Entity } from "../../components/ui/page-toolbar";
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
import { useUrlFilter, useUrlView } from "../../lib/hooks/use-url-state";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { ClientListDefaultFilter } from "../../lib/store";
import type { ClientInfo } from "../../lib/types";
import {
	getClientAttentionClasses,
	getGovernanceStatus,
	type ClientGovernanceStatus,
} from "./client-governance";
import { ClientCard } from "./components/client-card";

const EMPTY_CLIENTS: ClientInfo[] = [];

function renderGovernanceBadge(
	status: ClientGovernanceStatus,
	label: string,
): React.ReactNode {
	switch (status) {
		case "pending":
			return (
				<Badge variant="destructive" className="bg-amber-500 hover:bg-amber-600">
					{label}
				</Badge>
			);
		case "denied":
			return <Badge variant="destructive">{label}</Badge>;
		default:
			return (
				<span className="flex items-center rounded-full bg-emerald-50 px-2 py-1 text-xs font-medium text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400">
					<Check className="mr-1 h-3 w-3" /> {label}
				</span>
			);
	}
}

export function ClientsPage() {
	const navigate = useNavigate();
	const qc = useQueryClient();
	const [refreshing, setRefreshing] = useState(false);
	const [isClientFormOpen, setIsClientFormOpen] = useState(false);
	const [editingClient, setEditingClient] = useState<ClientInfo | null>(null);
	usePageTranslations("clients");
	const { t, i18n } = useTranslation("clients");
	const { defaultFilter, setDashboardSetting } = useAppStore((state) => ({
		defaultFilter: state.dashboardSettings.clientListDefaultFilter,
		setDashboardSetting: state.setDashboardSetting,
	}));

	const storedDefaultView = useAppStore((state) => state.dashboardSettings.defaultView);

	const { view } = useUrlView({
		paramName: "view",
		defaultView: storedDefaultView,
		validViews: ["grid", "list"],
	});

	const { filter, setFilter } = useUrlFilter({
		paramName: "filter",
		defaultValue: defaultFilter,
		validValues: ["all", "allowed", "pending", "denied"],
	});

	const { data, isLoading, isRefetching, refetch } = useQuery({
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

	const clients: ClientInfo[] = data?.client ?? EMPTY_CLIENTS;
	const detectedCount = clients.filter((c) => !!c.detected).length;
	const approvedCount = clients.filter((c) => getGovernanceStatus(c) === "allowed").length;
	const pendingCount = clients.filter((c) => getGovernanceStatus(c) === "pending").length;

	type ClientToolbarEntity = Entity & {
		identifier: string;
		display_name: string;
		detected: boolean;
		approval_status?: string | null;
	};

	const clientsAsEntities = React.useMemo<ClientToolbarEntity[]>(() => {
		const mapped: ClientToolbarEntity[] = clients.map((client: ClientInfo) => ({
			id: client.identifier || client.display_name || "",
			name: client.display_name || client.identifier || "",
			identifier: client.identifier,
			display_name: client.display_name,
			detected: client.detected,
			approval_status: client.approval_status,
			description: client.description ?? undefined,
		}));
		// Default stable sort by name A-Z, tie-breaker by id
		mapped.sort((a, b) => {
			const byName = a.name.localeCompare(b.name, undefined, {
				sensitivity: "base",
			});
			if (byName !== 0) return byName;
			return a.id.localeCompare(b.id, undefined, { sensitivity: "base" });
		});
		return mapped;
	}, [clients]);

	const clientsByIdentifier = React.useMemo(() => {
		return new Map(clients.map((client) => [client.identifier, client]));
	}, [clients]);

	// Apply visibility filter from toolbar
	const filteredClientsAsEntities = React.useMemo<ClientToolbarEntity[]>(() => {
		if (filter === "allowed") {
			return clientsAsEntities.filter((c) => {
				const sourceClient = clientsByIdentifier.get(c.identifier);
				return sourceClient ? getGovernanceStatus(sourceClient) === "allowed" : false;
			});
		}
		if (filter === "pending") {
			return clientsAsEntities.filter((c) => {
				const sourceClient = clientsByIdentifier.get(c.identifier);
				return sourceClient ? getGovernanceStatus(sourceClient) === "pending" : false;
			});
		}
		if (filter === "denied") {
			return clientsAsEntities.filter((c) => {
				const sourceClient = clientsByIdentifier.get(c.identifier);
				return sourceClient ? getGovernanceStatus(sourceClient) === "denied" : false;
			});
		}
		return clientsAsEntities;
	}, [clientsAsEntities, clientsByIdentifier, filter]);

	const getGovernanceStatusLabel = (status: ClientGovernanceStatus) => {
		if (status === "pending") {
			return t("entity.badge.pending", { defaultValue: "Pending" });
		}
		if (status === "denied") {
			return t("entity.badge.denied", { defaultValue: "Denied" });
		}
		return t("entity.badge.allowed", { defaultValue: "Allowed" });
	};

	const [sortedClients, setSortedClients] = React.useState<ClientToolbarEntity[]>(
		filteredClientsAsEntities,
	);

	const governanceMutation = useMutation({
		mutationFn: async ({
			identifier,
			approved,
		}: {
			identifier: string;
			approved: boolean;
		}) => {
			return approved
				? clientsApi.approveRecord({ identifier })
				: clientsApi.suspendRecord({ identifier });
		},
		onSuccess: async () => {
			try {
				// Force backend to refresh detection/config state, then sync cache
				const fresh = await clientsApi.list(true);
				qc.setQueryData(["clients"], fresh);
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

	const renderClientListItem = (client: ClientInfo) => {
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
			client.description ?? client.template?.description ?? undefined;

		const configLabel = t("entity.stats.config", { defaultValue: "Config" });
		const serversTag = t("entity.bottomTags.servers", {
			count: serverCount,
			defaultValue: "Servers: {{count}}",
		});
		const governanceStatus = getGovernanceStatus(client);
		const governanceLabel = getGovernanceStatusLabel(governanceStatus);
		const attentionClasses = getClientAttentionClasses(governanceStatus);

		const recordKindLabel = client.governed_by_default_policy
			? t("entity.badge.defaultPolicy", { defaultValue: "Default Policy" })
			: t("entity.badge.explicitRecord", { defaultValue: "Explicit Record" });

		const writableConfigLabel = client.writable_config
			? t("entity.badge.writableConfig", { defaultValue: "Writable" })
			: null;

		return (
			<EntityListItem
				key={client.identifier}
				id={client.identifier}
				title={displayName}
				description={description}
				avatar={{
					src: client.logo_url ?? undefined,
					alt: displayName,
					fallback: avatarInitial,
				}}
				stats={[{ label: configLabel, value: configPath }]}
				bottomTags={[
					<span key="servers">{serversTag}</span>,
					<span key="recordKind" className="text-slate-500">• {recordKindLabel}</span>,
					writableConfigLabel && <span key="writable" className="text-slate-500">• {writableConfigLabel}</span>
				].filter(Boolean)}
				statusBadge={renderGovernanceBadge(governanceStatus, governanceLabel)}
				enableSwitch={{
					checked: governanceStatus === "allowed",
					onChange: (checked) =>
						governanceMutation.mutate({ identifier, approved: checked }),
					disabled: governanceMutation.isPending || governanceStatus === "pending",
				}}
				className={`${governanceStatus === "pending" ? "opacity-75" : ""} ${attentionClasses.cardClassName}`.trim()}
				onClick={() => navigate(`/clients/${encodeURIComponent(identifier)}`)}
			/>
		);
	};

	const handleRefresh = async () => {
		notifyInfo(
			t("toolbar.actions.refresh.notificationTitle", {
				defaultValue: "Refresh triggered",
			}),
			t("toolbar.actions.refresh.notificationMessage", {
				defaultValue: "Latest client state will sync to the list",
			}),
		);
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
				title: t("statsCards.approved.title", {
					defaultValue: "Approved",
				}),
				value: approvedCount,
				description: t("statsCards.approved.description", {
					defaultValue: "governance allowed",
				}),
			},
			{
				title: t("statsCards.pending.title", {
					defaultValue: "Pending",
				}),
				value: pendingCount,
				description: t("statsCards.pending.description", {
					defaultValue: "awaiting review",
				}),
			},
		],
		[
			clients.length,
			detectedCount,
			approvedCount,
			pendingCount,
			i18n.language,
		],
	);

	// Prepare loading skeleton
	const loadingSkeleton =
		view === "grid"
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
					className="flex items-center justify-between rounded-lg border border-slate-200 bg-white px-4 py-4 dark:border-slate-700 dark:bg-slate-900"
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

	// Toolbar expansion state
	const [expanded, setExpanded] = useState(false);

	// Toolbar configuration
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
				defaultMode: storedDefaultView as "grid" | "list",
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
						value: "approval_status",
						label: t("toolbar.sort.options.approvalStatus", {
							defaultValue: "Governance Status",
						}),
						defaultDirection: "asc" as const,
					},
				],
				defaultSort: "display_name",
			},
			urlPersistence: {
				enabled: true,
			},
		}),
		[filteredClientsAsEntities, i18n.language, t],
	);

	// Toolbar state
	const toolbarState = {
		expanded,
	};

	// Toolbar callbacks
	const toolbarCallbacks: {
		onViewModeChange: (mode: "grid" | "list") => void;
		onSortedDataChange: (sortedData: ClientToolbarEntity[]) => void;
		onExpandedChange: React.Dispatch<React.SetStateAction<boolean>>;
	} = {
		onViewModeChange: (mode: "grid" | "list") => {
			setDashboardSetting("defaultView", mode);
		},
		onSortedDataChange: (sortedData: ClientToolbarEntity[]) => setSortedClients(sortedData),
		onExpandedChange: setExpanded,
	};

	const filterOptions: SegmentOption[] = React.useMemo(
		() => [
			{ value: "all", label: t("toolbar.filters.options.all", { defaultValue: "All" }) },
			{ value: "allowed", label: t("toolbar.filters.options.allowed", { defaultValue: "Allowed" }) },
			{ value: "pending", label: t("toolbar.filters.options.pending", { defaultValue: "Pending" }) },
			{ value: "denied", label: t("toolbar.filters.options.denied", { defaultValue: "Denied" }) },
		],
		[t, i18n.language],
	);

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

	// Toolbar actions
	const actions = (
		<div className="flex items-center gap-2">
			<Button
				onClick={handleRefresh}
				disabled={isRefetching || refreshing}
				variant="outline"
				size="sm"
				className="h-9 w-9 p-0"
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
				onClick={() => {
					setEditingClient(null);
					setIsClientFormOpen(true);
				}}
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
				<PageToolbar<ClientToolbarEntity>
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
				{view === "grid"
					? sortedClients.map((client) => {
						const sourceClient = clientsByIdentifier.get(client.identifier);
						return sourceClient ? (
							<ClientCard
								key={sourceClient.identifier}
								client={sourceClient}
								onNavigate={(identifier) => navigate(`/clients/${encodeURIComponent(identifier)}`)}
								onGovernanceChange={(identifier, approved) => governanceMutation.mutate({ identifier, approved })}
								isGovernancePending={governanceMutation.isPending}
							/>
						) : null;
					})
					: sortedClients.map((client) => {
						const sourceClient = clientsByIdentifier.get(client.identifier);
						return sourceClient ? renderClientListItem(sourceClient) : null;
					})}
			</ListGridContainer>
			<ClientFormDrawer
				open={isClientFormOpen}
				onOpenChange={setIsClientFormOpen}
				mode={editingClient ? "edit" : "create"}
				client={editingClient}
				onSuccess={(identifier) => {
					void refetch();
					navigate(`/clients/${encodeURIComponent(identifier)}`);
				}}
				onDeleteSuccess={() => {
					setEditingClient(null);
				}}
			/>
		</PageLayout>
	);
}

export default ClientsPage;
