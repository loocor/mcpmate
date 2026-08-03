import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCw, Server } from "lucide-react";
import React, { useCallback, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { ConfirmDialog } from "../../components/confirm-dialog";
import { ListGridContainer } from "../../components/list-grid-container";
import { Pagination } from "../../components/pagination";
import {
	EmptyState,
	FullHeightEmptyStateCard,
	PageLayout,
} from "../../components/page-layout";
import { ServerImportDropButton } from "../../components/server-import-drop-button";
import { ServerEditDrawer } from "../../components/server-edit-drawer";
import { ServerCatalogEntry } from "../../components/servers";
import { ServerInstallWizard, type ServerInstallManualFormHandle } from "../../components/server-install";
import { StatsCards } from "../../components/stats-cards";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
} from "../../components/ui/card";
// Dropdown removed in favor of a single combined add flow
import {
	PageToolbar,
	type PageToolbarConfig,
	type PageToolbarCallbacks,
	type PageToolbarState,
} from "../../components/ui/page-toolbar";
import { useServerInstallPipeline } from "../../hooks/use-server-install-pipeline";
import { serversApi } from "../../lib/api";
import {
	CATALOG_PAGE_SIZE_OPTIONS,
	useResponsiveCatalogPagination,
} from "../../lib/hooks/use-responsive-catalog-pagination";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { useUrlView } from "../../lib/hooks/use-url-state";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import type { ServerIngestPayload } from "../../lib/install-normalizer";
import {
	canIngestFromDataTransfer,
	extractPayloadFromDataTransfer,
	formatServerUniImportTransferError,
} from "../../lib/server-uni-import-transfer";
import { useAppStore } from "../../lib/store";
import type {
	MCPServerConfig,
	ServerDetail,
	ServerListResponse,
	ServerSummary,
} from "../../lib/types";
import { getServerListRefetchInterval } from "./server-list-polling";

const EMPTY_SERVERS: ServerSummary[] = [];

export function ServerListPage() {
	usePageTranslations("servers");
	const { t, i18n } = useTranslation("servers");
	const navigate = useNavigate();
	const [manualOpen, setManualOpen] = useState(false);
	const [pendingIngestPayload, setPendingIngestPayload] =
		useState<ServerIngestPayload | null>(null);
	const manualRef = useRef<ServerInstallManualFormHandle | null>(null);
	const hasNotifiedServerListErrorRef = useRef(false);
	const [editingServer, setEditingServer] = useState<ServerDetail | null>(null);
	const [deletingServer, setDeletingServer] = useState<string | null>(null);
	const [isDeleteConfirmOpen, setIsDeleteConfirmOpen] = useState(false);
	const [isDeleteLoading, setIsDeleteLoading] = useState(false);
	const [deleteError, setDeleteError] = useState<string | null>(null);
	const [pending, setPending] = useState<Record<string, boolean>>({});
	const [isTogglePending, setIsTogglePending] = useState(false);

	const [expanded, setExpanded] = useState(false);

	// Sorted data state
	const [sortedServers, setSortedServers] = React.useState<ServerSummary[]>([]);
	const [isCatalogDataReady, setIsCatalogDataReady] = useState(false);

	const queryClient = useQueryClient();

	const installPipeline = useServerInstallPipeline({
		onImported: () => {
			queryClient.invalidateQueries({ queryKey: ["servers"] });
			refetch();
		},
	});

	const storedDefaultView = useAppStore((state) => state.dashboardSettings.defaultView);
	const setDashboardSetting = useAppStore((state) => state.setDashboardSetting);

	const { view } = useUrlView({
		paramName: "view",
		defaultView: storedDefaultView,
		validViews: ["grid", "list"],
	});
	const viewMode = view;

	const pendingServerDeepLinkImport = useAppStore(
		(state) => state.pendingServerDeepLinkImport,
	);
	const setPendingServerDeepLinkImport = useAppStore(
		(state) => state.setPendingServerDeepLinkImport,
	);
	const syncServerStateToClients = useAppStore(
		(state) => state.dashboardSettings.syncServerStateToClients,
	);

	const openManualIngest = useCallback((payload: ServerIngestPayload) => {
		setPendingIngestPayload(payload);
		setManualOpen(true);
	}, []);

	React.useEffect(() => {
		if (!manualOpen || pendingIngestPayload === null) {
			return;
		}
		const payload = pendingIngestPayload;
		setPendingIngestPayload(null);
		void manualRef.current?.ingest(payload);
	}, [manualOpen, pendingIngestPayload]);

	React.useEffect(() => {
		if (!pendingServerDeepLinkImport) {
			return;
		}
		const { text, format, source } = pendingServerDeepLinkImport;
		setPendingServerDeepLinkImport(null);
		const fileName =
			format === "json"
				? "snippet.json"
				: format === "toml"
					? "snippet.toml"
					: "snippet.txt";
		openManualIngest({ text, fileName, source });
		notifyInfo(
			t("notifications.deepLinkImport.title", {
				defaultValue: "Configuration received",
			}),
			t("notifications.deepLinkImport.message", {
				defaultValue:
					"Review the imported server snippet in the drawer before saving.",
			}),
		);
	}, [
		i18n.language,
		openManualIngest,
		pendingServerDeepLinkImport,
		setPendingServerDeepLinkImport,
		t,
	]);

	const ingestServerImportDataTransfer = useCallback(
		async (dataTransfer: DataTransfer | null) => {
			if (!dataTransfer || !canIngestFromDataTransfer(dataTransfer)) {
				notifyError(
					t("notifications.importUnsupported.title", {
						defaultValue: "Unsupported content",
					}),
					t("notifications.importUnsupported.message", {
						defaultValue:
							"Drop text, JSON snippets, URLs, or config files to use Uni-Import.",
					}),
				);
				return;
			}

			let payload;
			try {
				payload = await extractPayloadFromDataTransfer(dataTransfer);
			} catch (error) {
				notifyError(
					t("notifications.importUnsupported.title", {
						defaultValue: "Unsupported content",
					}),
					formatServerUniImportTransferError(error, t),
				);
				return;
			}

			if (!payload) {
				notifyError(
					t("notifications.importEmpty.title", {
						defaultValue: "Nothing to import",
					}),
					t("notifications.importEmpty.message", {
						defaultValue:
							"We could not detect any usable configuration from the dropped content.",
					}),
				);
				return;
			}

			openManualIngest(payload);
		},
		[i18n.language, openManualIngest, t],
	);

	const {
		data: serverData,
		isLoading,
		refetch,
		isRefetching,
		error,
		isError,
	} = useQuery<ServerListResponse>({
		queryKey: ["servers"],
		queryFn: async () => {
			try {
				console.log("Fetching servers...");
				const result = await serversApi.getAll();
				console.log("Servers fetched:", result);
				return result;
			} catch (err) {
				console.error("Error fetching servers:", err);
				throw err;
			}
		},
		refetchInterval: (query) => {
			const servers = query.state.data?.servers ?? [];
			return getServerListRefetchInterval(servers);
		},
		refetchIntervalInBackground: true,
		retry: 1, // Reduce retry count to show errors more quickly
	});

	React.useEffect(() => {
		if (!isError || !error) {
			hasNotifiedServerListErrorRef.current = false;
			return;
		}
		if (hasNotifiedServerListErrorRef.current) {
			return;
		}

		hasNotifiedServerListErrorRef.current = true;
		notifyError(
			t("errors.loadFailed", { defaultValue: "Failed to load servers" }),
			error.message,
		);
	}, [error, i18n.language, isError, t]);

	// Enable/disable server
	const toggleServerAsync = useCallback(
		async (serverId: string, enable: boolean, sync?: boolean) => {
			setPending((p) => ({ ...p, [serverId]: true }));
			try {
				const sourceRevisionSet = serverData?.servers.find(
					(server) => server.id === serverId,
				)?.source_revision_set;
				if (!sourceRevisionSet) {
					throw new Error(
						"Capability catalog revisions are not loaded. Refresh servers and retry.",
					);
				}
				if (enable) {
					await serversApi.enableServer(serverId, sourceRevisionSet, sync);
				} else {
					await serversApi.disableServer(serverId, sourceRevisionSet, sync);
				}
				const successTitle = enable
					? t("notifications.toggle.enabledTitle", {
						defaultValue: "Server enabled",
					})
					: t("notifications.toggle.disabledTitle", {
						defaultValue: "Server disabled",
					});
				const successMessage = t("notifications.toggle.message", {
					serverId,
					defaultValue: "Server {{serverId}}",
				});
				notifySuccess(successTitle, successMessage);
				queryClient.invalidateQueries({ queryKey: ["servers"] });
				setTimeout(
					() => queryClient.invalidateQueries({ queryKey: ["servers"] }),
					1000,
				);
			} catch (error) {
				const actionLabel = enable
					? t("notifications.toggle.enableAction", { defaultValue: "enable" })
					: t("notifications.toggle.disableAction", {
						defaultValue: "disable",
					});
				const errorMessage =
					error instanceof Error ? error.message : String(error);
				notifyError(
					t("notifications.genericError.title", {
						defaultValue: "Operation failed",
					}),
					t("notifications.toggle.error", {
						action: actionLabel,
						message: errorMessage,
						defaultValue: "Unable to {{action}} server: {{message}}",
					}),
				);
			} finally {
				setPending((p) => ({ ...p, [serverId]: false }));
			}
		},
		[i18n.language, queryClient, serverData?.servers, t],
	);

	// Note: Reconnect functionality is moved to instance-level pages

	// Update server
	const updateServerMutation = useMutation({
		mutationFn: async ({
			serverId,
			config,
		}: {
			serverId: string;
			config: Partial<MCPServerConfig>;
		}) => {
			const {
				unify_direct_exposure_eligible: requestedEligibility,
				...crudConfig
			} = config;
			const result = await serversApi.updateServer(serverId, crudConfig);
			const currentServer = serverData?.servers.find(
				(server) => server.id === serverId,
			);
			if (
				requestedEligibility !== undefined &&
				requestedEligibility !==
					currentServer?.unify_direct_exposure_eligible
			) {
				if (!currentServer?.source_revision_set) {
					throw new Error(
						"Capability catalog revisions are not loaded. Refresh servers and retry.",
					);
				}
				await serversApi.setDirectExposureEligibility(
					serverId,
					requestedEligibility,
					currentServer.source_revision_set,
				);
			}
			return result;
		},
		onSuccess: (_, variables) => {
			notifySuccess(
				t("notifications.update.title", {
					defaultValue: "Server updated",
				}),
				t("notifications.update.message", {
					serverId: variables.serverId,
					defaultValue: "Server {{serverId}}",
				}),
			);
			queryClient.invalidateQueries({ queryKey: ["servers"] });
		},
		onError: (error, variables) => {
			notifyError(
				t("notifications.update.errorTitle", {
					defaultValue: "Update failed",
				}),
				t("notifications.update.errorMessage", {
					serverId: variables.serverId,
					message: error instanceof Error ? error.message : String(error),
					defaultValue:
						"Unable to update {{serverId}}: {{message}}",
				}),
			);
		},
	});

	// Handle update server
	const handleUpdateServer = async (config: Partial<MCPServerConfig>) => {
		if (editingServer) {
			console.log("Updating server:", editingServer.id, "with config:", config);
			try {
				await updateServerMutation.mutateAsync({
					serverId: editingServer.id,
					config,
				});
				console.log("Server update successful");
				setEditingServer(null);
			} catch (error) {
				console.error("Server update failed:", error);
				throw error; // Re-throw to let the mutation handle it
			}
		}
	};

	// Handle delete server
	const handleDeleteServer = async () => {
		if (!deletingServer) return;

		setIsDeleteLoading(true);
		setDeleteError(null);

		try {
			await serversApi.deleteServer(deletingServer);
			notifySuccess(
				t("notifications.delete.title", {
					defaultValue: "Server deleted",
				}),
				t("notifications.delete.message", {
					serverId: deletingServer,
					defaultValue:
						"Server {{serverId}}. Review Secure Store cleanup if it used stored secrets.",
				}),
				"/secrets?lifecycle=unused",
			);
			queryClient.invalidateQueries({ queryKey: ["servers"] });
			setIsDeleteConfirmOpen(false);
			setDeletingServer(null);
		} catch (error) {
			setDeleteError(
				error instanceof Error
					? error.message
					: t("notifications.delete.errorFallback", {
						defaultValue: "Error deleting server",
					}),
			);
		} finally {
			setIsDeleteLoading(false);
		}
	};

	const handleServerToggle = useCallback(
		async (serverId: string, enabled: boolean) => {
			setIsTogglePending(true);
			try {
				const sourceRevisionSet = serverData?.servers.find(
					(server) => server.id === serverId,
				)?.source_revision_set;
				if (!sourceRevisionSet) {
					throw new Error(
						"Capability catalog revisions are not loaded. Refresh servers and retry.",
					);
				}
				if (enabled) {
					await serversApi.enableServer(
						serverId,
						sourceRevisionSet,
						syncServerStateToClients,
					);
					notifySuccess(
						t("notifications.toggle.enabledTitle", {
							defaultValue: "Server enabled",
						}),
						t("notifications.toggle.enabledDetail", {
							serverId,
							defaultValue: "Server {{serverId}} has been enabled",
						}),
					);
				} else {
					await serversApi.disableServer(
						serverId,
						sourceRevisionSet,
						syncServerStateToClients,
					);
					notifySuccess(
						t("notifications.toggle.disabledTitle", {
							defaultValue: "Server disabled",
						}),
						t("notifications.toggle.disabledDetail", {
							serverId,
							defaultValue: "Server {{serverId}} has been disabled",
						}),
					);
				}
				queryClient.invalidateQueries({ queryKey: ["servers"] });
			} catch (error) {
				notifyError(
					t("notifications.toggle.failedTitle", {
						defaultValue: "Failed to toggle server",
					}),
					error instanceof Error
						? error.message
						: t("notifications.genericError.unknown", {
							defaultValue: "Unknown error",
						}),
				);
			} finally {
				setIsTogglePending(false);
			}
		},
		[
			i18n.language,
			queryClient,
			serverData?.servers,
			syncServerStateToClients,
			t,
		],
	);

	const catalogStatsLabels = useMemo(
		() => ({
			tools: t("entity.stats.tools", { defaultValue: "Tools" }),
			prompts: t("entity.stats.prompts", { defaultValue: "Prompts" }),
			resources: t("entity.stats.resources", { defaultValue: "Resources" }),
			templates: t("entity.stats.templates", { defaultValue: "Templates" }),
		}),
		[i18n.language, t],
	);

	const handleCatalogOpen = useCallback(
		(serverId: string) => {
			navigate(`/servers/${encodeURIComponent(serverId)}`);
		},
		[navigate],
	);

	const handleCatalogListToggle = useCallback(
		(serverId: string, enabled: boolean) => {
			void handleServerToggle(serverId, enabled);
		},
		[handleServerToggle],
	);

	const handleCatalogGridToggle = useCallback(
		(serverId: string, enabled: boolean) => {
			void toggleServerAsync(serverId, enabled, syncServerStateToClients);
		},
		[syncServerStateToClients, toggleServerAsync],
	);

	const serverPagination = useResponsiveCatalogPagination(
		sortedServers,
		viewMode as "grid" | "list",
		isCatalogDataReady,
	);
	const catalogScrollRef = useRef<HTMLDivElement | null>(null);

	React.useEffect(() => {
		catalogScrollRef.current?.scrollTo({ top: 0 });
	}, [serverPagination.currentPage]);
	const hasNoServerRecords = serverData?.servers?.length === 0;

	const statsCards = useMemo(() => {
		const list = serverData?.servers ?? [];
		return [
			{
				title: t("statsCards.total.title", {
					defaultValue: "Total Servers",
				}),
				value: list.length,
				description: t("statsCards.total.description", {
					defaultValue: "registered",
				}),
			},
			{
				title: t("statsCards.enabled.title", {
					defaultValue: "Enabled",
				}),
				value: list.filter((s) => s.enabled).length,
				description: t("statsCards.enabled.description", {
					defaultValue: "feature toggled",
				}),
			},
			{
				title: t("statsCards.connected.title", {
					defaultValue: "Connected",
				}),
				value: list.filter(
					(s) => String(s.status || "").toLowerCase() === "connected",
				).length,
				description: t("statsCards.connected.description", {
					defaultValue: "active connections",
				}),
			},
			{
				title: t("statsCards.instances.title", {
					defaultValue: "Instances",
				}),
				value: list.reduce(
					(sum, s) => sum + (s.instances?.length || 0),
					0,
				),
				description: t("statsCards.instances.description", {
					defaultValue: "total across servers",
				}),
			},
		];
	}, [i18n.language, serverData, t]);

	// Prepare loading skeleton
	const loadingSkeleton =
		viewMode === "grid"
			? Array.from({ length: 6 }, (_, index) => (
				<Card
					key={`loading-grid-skeleton-${Date.now()}-${index}`}
					className="overflow-hidden"
				>
					<CardHeader className="p-4">
						<div className="h-6 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						<div className="h-4 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
					</CardHeader>
					<CardContent className="p-4 pt-0">
						<div className="mt-2 flex justify-between">
							<div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
							<div className="h-9 w-20 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						</div>
					</CardContent>
				</Card>
			))
			: Array.from({ length: 3 }, (_, index) => (
				<div
					key={`loading-list-skeleton-${Date.now()}-${index}`}
					className="flex items-center justify-between rounded-lg border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900"
				>
					<div className="flex items-center gap-3">
						<div className="h-12 w-12 animate-pulse rounded-[10px] bg-slate-200 dark:bg-slate-800"></div>
						<div className="space-y-2">
							<div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
							<div className="h-4 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
						</div>
					</div>
					<div className="h-9 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
				</div>
			));

	// Toolbar config
	type ToolbarServer = ServerSummary & { [key: string]: unknown };
	const toolbarConfig: PageToolbarConfig<ToolbarServer> = {
		data: (serverData?.servers ?? EMPTY_SERVERS) as ToolbarServer[],
		isDataReady: serverData !== undefined,
		search: {
			placeholder: t("toolbar.search.placeholder", {
				defaultValue: "Search servers...",
			}),
			fields: [
				{
					key: "name",
					label: t("toolbar.search.fields.name", { defaultValue: "Name" }),
					weight: 10,
				},
				{
					key: "description",
					label: t("toolbar.search.fields.description", {
						defaultValue: "Description",
					}),
					weight: 8,
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
					value: "name",
					label: t("toolbar.sort.options.name", { defaultValue: "Name" }),
					defaultDirection: "asc" as const,
				},
				{
					value: "enabled",
					label: t("toolbar.sort.options.enabled", {
						defaultValue: "Enable Status",
					}),
					defaultDirection: "desc" as const,
				},
			],
			defaultSort: "name",
		},
		urlPersistence: {
			enabled: true,
		},
	};

	// Toolbar state
	const toolbarState: PageToolbarState = {
		expanded,
	};

	// Toolbar callbacks
	const toolbarCallbacks: PageToolbarCallbacks<ToolbarServer> = {
		onViewModeChange: (mode: "grid" | "list") => {
			setDashboardSetting("defaultView", mode);
		},
		onSortedDataChange: (data) => {
			setSortedServers(data as ServerSummary[]);
			setIsCatalogDataReady(true);
		},
		onExpandedChange: setExpanded,
	};

	// Action buttons
	const actions = (
		<div className="flex items-center gap-2">
			<Button
				onClick={() => refetch()}
				disabled={isRefetching}
				variant="outline"
				size="sm"
				className="h-9 w-9 p-0"
				title={t("actions.refresh.title", { defaultValue: "Refresh" })}
			>
				<RefreshCw
					className={`h-4 w-4 ${isRefetching ? "animate-spin" : ""}`}
				/>
			</Button>
			<div className="rounded-md">
				<ServerImportDropButton
					title={t("actions.add.title", { defaultValue: "Add Server" })}
					onClick={() => setManualOpen(true)}
					onDrop={ingestServerImportDataTransfer}
				/>
			</div>
		</div>
	);

	// Prepare empty state
	const emptyStateAction = hasNoServerRecords ? (
		<ServerImportDropButton
			variant="labeled"
			className="mt-4"
			onClick={() => setManualOpen(true)}
			onDrop={ingestServerImportDataTransfer}
			title={t("emptyState.action", {
				defaultValue: "Add First Server",
			})}
			label={t("emptyState.action", {
				defaultValue: "Add First Server",
			})}
		/>
	) : undefined;

	const emptyState = (
		<FullHeightEmptyStateCard>
			<EmptyState
				icon={<Server className="h-12 w-12" />}
				title={t("emptyState.title", { defaultValue: "No servers found" })}
				description={t("emptyState.description", {
					defaultValue: "Add your first MCP server to get started",
				})}
				action={emptyStateAction}
			/>
		</FullHeightEmptyStateCard>
	);

	return (
		<PageLayout
			title={t("title", { defaultValue: "Servers" })}
			className="flex h-full min-h-0 flex-col"
			headerActions={
				<PageToolbar<ToolbarServer>
					config={toolbarConfig}
					state={toolbarState}
					callbacks={toolbarCallbacks}
					actions={actions}
				/>
			}
			statsCards={<StatsCards cards={statsCards} />}
		>
			<div className="flex min-h-0 flex-1 flex-col">
				<div ref={catalogScrollRef} className="min-h-0 flex-1 overflow-y-auto pr-1">
					<ListGridContainer
						loading={isLoading}
						loadingSkeleton={loadingSkeleton}
						emptyClassName="h-full"
						emptyState={
							sortedServers.length === 0 ? emptyState : undefined
						}
					>
						{viewMode === "grid"
							? serverPagination.pageItems.map((server) => (
								<ServerCatalogEntry
									key={server.id}
									variant="grid"
									server={server}
									statsLabels={catalogStatsLabels}
									onOpen={handleCatalogOpen}
									onToggle={handleCatalogGridToggle}
									isToggleDisabled={!!pending[server.id]}
								/>
							))
							: serverPagination.pageItems.map((server) => (
								<ServerCatalogEntry
									key={server.id}
									variant="list"
									server={server}
									statsLabels={catalogStatsLabels}
									onOpen={handleCatalogOpen}
									onToggle={handleCatalogListToggle}
									isToggleDisabled={isTogglePending}
								/>
							))}
					</ListGridContainer>
				</div>
				<Pagination
					currentPage={serverPagination.currentPage}
					hasPreviousPage={serverPagination.hasPreviousPage}
					hasNextPage={serverPagination.hasNextPage}
					isLoading={isLoading || !isCatalogDataReady}
					itemsPerPage={serverPagination.pageSize}
					currentPageItemCount={serverPagination.pageItems.length}
					totalItemCount={sortedServers.length}
					totalPages={serverPagination.totalPages}
					onGoToPage={serverPagination.goToPage}
					onItemsPerPageChange={serverPagination.onItemsPerPageChange}
					onPreviousPage={serverPagination.goToPreviousPage}
					onFirstPage={serverPagination.goToFirstPage}
					onNextPage={serverPagination.goToNextPage}
					onLastPage={serverPagination.goToLastPage}
					hasFirstPage={serverPagination.hasPreviousPage}
					hasLastPage={serverPagination.hasNextPage}
					pageSizeOptions={[...CATALOG_PAGE_SIZE_OPTIONS]}
					className="shrink-0 pt-3 pb-1"
				/>
			</div>

			{/* Server install pipeline */}
			<ServerInstallWizard
				ref={manualRef}
				isOpen={manualOpen}
				onClose={() => {
					setManualOpen(false);
					setPendingIngestPayload(null);
				}}
				mode="new"
				pipeline={installPipeline}
			/>

			{/* Edit server drawer */}
			{editingServer ? (
				<ServerEditDrawer
					server={editingServer}
					isOpen={!!editingServer}
					onClose={() => setEditingServer(null)}
					onSubmit={handleUpdateServer}
				/>
			) : null}

			{/* Delete confirmation dialog */}
			<ConfirmDialog
				isOpen={isDeleteConfirmOpen}
				onClose={() => {
					setIsDeleteConfirmOpen(false);
					setDeleteError(null);
				}}
				onConfirm={handleDeleteServer}
				title={t("confirmDelete.title", { defaultValue: "Delete Server" })}
				description={t("confirmDelete.description", {
					serverId: deletingServer ?? "",
					defaultValue:
						"Are you sure you want to delete the server \"{{serverId}}\"? This action cannot be undone.",
				})}
				confirmLabel={t("confirmDelete.confirm", { defaultValue: "Delete" })}
				cancelLabel={t("confirmDelete.cancel", { defaultValue: "Cancel" })}
				variant="destructive"
				isLoading={isDeleteLoading}
				error={deleteError}
			/>
		</PageLayout>
	);
}
