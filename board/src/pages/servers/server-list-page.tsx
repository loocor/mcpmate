import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	AlertCircle,
	Bug,
	Plug,
	Plus,
	RefreshCw,
	Server,
	Target,
} from "lucide-react";
import React, { useCallback, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { ConfirmDialog } from "../../components/confirm-dialog";
import { EntityCard } from "../../components/entity-card";
import { EntityListItem } from "../../components/entity-list-item";
import { ErrorDisplay } from "../../components/error-display";
import { ListGridContainer } from "../../components/list-grid-container";
import { EmptyState, PageLayout } from "../../components/page-layout";
import { ServerEditDrawer } from "../../components/server-edit-drawer";
import { ServerAuthBadge } from "../../components/server-auth-badge";
import { ServerInstallWizard, type ServerInstallManualFormHandle } from "../../components/server-install";
import { StatsCards } from "../../components/stats-cards";
import { StatusBadge } from "../../components/status-badge";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
// Dropdown removed in favor of a single combined add flow
import {
	PageToolbar,
	type PageToolbarConfig,
	type PageToolbarCallbacks,
	type PageToolbarState,
} from "../../components/ui/page-toolbar";
import { Switch } from "../../components/ui/switch";
import { useServerInstallPipeline } from "../../hooks/use-server-install-pipeline";
import { serversApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { useUrlSort, useUrlView } from "../../lib/hooks/use-url-state";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type {
	MCPServerConfig,
	ServerDetail,
	ServerListResponse,
	ServerSummary,
} from "../../lib/types";

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

// Helper function to get the instance count for a server
function getCapabilitySummary(server: ServerSummary) {
	return server.capability ?? server.capabilities ?? undefined;
}

function canIngestFromDataTransfer(dataTransfer: DataTransfer | null): boolean {
	if (!dataTransfer) return false;
	const types = Array.from(dataTransfer.types ?? []);
	return (
		types.includes("Files") ||
		types.includes("text/plain") ||
		types.includes("text/uri-list")
	);
}

async function extractPayloadFromDataTransfer(
	dataTransfer: DataTransfer,
): Promise<{ text?: string; buffer?: ArrayBuffer; fileName?: string } | null> {
	if (dataTransfer.files && dataTransfer.files.length > 0) {
		const file = dataTransfer.files[0];
		if (file.name.endsWith(".mcpb") || file.name.endsWith(".dxt")) {
			// Try bundle-style parsing first (.mcpb, optionally .dxt if it matches the same layout)
			return { buffer: await file.arrayBuffer(), fileName: file.name };
		}
		return { text: await file.text(), fileName: file.name };
	}

	const plainText = dataTransfer.getData("text/plain");
	if (plainText) {
		return { text: plainText };
	}

	const uriList = dataTransfer.getData("text/uri-list");
	if (uriList) {
		return { text: uriList };
	}

	if (dataTransfer.items && dataTransfer.items.length > 0) {
		for (const item of Array.from(dataTransfer.items)) {
			if (item.kind === "string") {
				const value = await new Promise<string | null>((resolve) => {
					item.getAsString((text) => resolve(text ?? null));
				});
				if (value) {
					return { text: value };
				}
			}
		}
	}

	return null;
}

export function ServerListPage() {
	usePageTranslations("servers");
	const { t } = useTranslation("servers");
	const navigate = useNavigate();
	const [debugInfo, setDebugInfo] = useState<string | null>(null);
	const [manualOpen, setManualOpen] = useState(false);
	const manualRef = useRef<ServerInstallManualFormHandle | null>(null);
	const [isAddDragActive, setAddDragActive] = useState(false);
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

	const queryClient = useQueryClient();

	const installPipeline = useServerInstallPipeline({
		onImported: () => {
			queryClient.invalidateQueries({ queryKey: ["servers"] });
			refetch();
		},
	});

	const handleAddDragEnter = useCallback(
		(event: React.DragEvent<HTMLDivElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			setAddDragActive(true);
		},
		[],
	);

	const handleAddDragOver = useCallback(
		(event: React.DragEvent<HTMLDivElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			if (event.dataTransfer) {
				event.dataTransfer.dropEffect = "copy";
			}
			if (!isAddDragActive) {
				setAddDragActive(true);
			}
		},
		[isAddDragActive],
	);

	const handleAddDragLeave = useCallback(
		(event: React.DragEvent<HTMLDivElement>) => {
			event.preventDefault();
			event.stopPropagation();
			const nextTarget = event.relatedTarget as Node | null;
			if (nextTarget && event.currentTarget.contains(nextTarget)) {
				return;
			}
			setAddDragActive(false);
		},
		[],
	);

	const handleAddDragEnd = useCallback(() => {
		setAddDragActive(false);
	}, []);

	const storedDefaultView = useAppStore((state) => state.dashboardSettings.defaultView);
	const setDashboardSetting = useAppStore((state) => state.setDashboardSetting);

	const { view } = useUrlView({
		paramName: "view",
		defaultView: storedDefaultView,
		validViews: ["grid", "list"],
	});
	const viewMode = view;
	const { sortState } = useUrlSort({
		paramName: "sort",
		defaultField: "name",
		defaultDirection: "asc",
		validFields: ["name", "enabled"],
	});

	const pendingServerDeepLinkImport = useAppStore(
		(state) => state.pendingServerDeepLinkImport,
	);
	const setPendingServerDeepLinkImport = useAppStore(
		(state) => state.setPendingServerDeepLinkImport,
	);
	const enableServerDebug = useAppStore(
		(state) => state.dashboardSettings.enableServerDebug,
	);
	const openDebugInNewWindow = useAppStore(
		(state) => state.dashboardSettings.openDebugInNewWindow,
	);
	const syncServerStateToClients = useAppStore(
		(state) => state.dashboardSettings.syncServerStateToClients,
	);

	React.useEffect(() => {
		if (!pendingServerDeepLinkImport) {
			return;
		}
		const { text, format } = pendingServerDeepLinkImport;
		setPendingServerDeepLinkImport(null);
		const fileName =
			format === "json"
				? "snippet.json"
				: format === "toml"
					? "snippet.toml"
					: "snippet.txt";
		setManualOpen(true);
		requestAnimationFrame(() => {
			manualRef.current?.ingest({ text, fileName });
		});
		notifyInfo(
			t("notifications.deepLinkImport.title", {
				defaultValue: "Configuration received",
			}),
			t("notifications.deepLinkImport.message", {
				defaultValue:
					"Review the imported server snippet in the drawer before saving.",
			}),
		);
	}, [pendingServerDeepLinkImport, setPendingServerDeepLinkImport, t]);

	const handleAddDrop = useCallback(
		async (event: React.DragEvent<HTMLDivElement>) => {
			event.preventDefault();
			event.stopPropagation();
			setAddDragActive(false);
			const dataTransfer = event.dataTransfer;
			if (!dataTransfer || !canIngestFromDataTransfer(dataTransfer)) {
				notifyError(
					t("notifications.importUnsupported.title", {
						defaultValue: "Unsupported content",
					}),
					t("notifications.importUnsupported.message", {
						defaultValue:
							"Drop text, JSON snippets, URLs, or MCP bundles to use Uni-Import.",
					}),
				);
				return;
			}
			const payload = await extractPayloadFromDataTransfer(dataTransfer);
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
			setManualOpen(true);
			requestAnimationFrame(() => {
				manualRef.current?.ingest(payload);
			});
		},
		[t],
	);

	const {
		data: serverListResponse,
		isLoading,
		refetch,
		isRefetching,
		error,
		isError,
	} = useQuery<ServerListResponse>({
		queryKey: ["servers"],
		queryFn: async () => {
			try {
				// Append inspect information
				console.log("Fetching servers...");
				const result = await serversApi.getAll();
				console.log("Servers fetched:", result);
				return result;
			} catch (err) {
				console.error("Error fetching servers:", err);
				// Capture error information for display
				setDebugInfo(
					err instanceof Error ? `${err.message}\n\n${err.stack}` : String(err),
				);
				throw err;
			}
		},
		refetchInterval: (query) => {
			const servers = query.state.data?.servers ?? [];
			const hasTransitionalServer = servers.some((server) =>
				isTransitionalServerStatus(server.status),
			);
			return hasTransitionalServer ? 5000 : 30000;
		},
		refetchIntervalInBackground: true,
		retry: 1, // Reduce retry count to show errors more quickly
	});

	React.useEffect(() => {
		if (sortedServers.length === 0 && serverListResponse?.servers) {
			const initialSorted = [...serverListResponse.servers].sort((a, b) => {
				const aValue = a[sortState.field as keyof ServerSummary];
				const bValue = b[sortState.field as keyof ServerSummary];

				let comparison = 0;
				if (typeof aValue === "string" && typeof bValue === "string") {
					comparison = aValue.localeCompare(bValue);
				} else if (typeof aValue === "boolean" && typeof bValue === "boolean") {
					comparison = Number(aValue) - Number(bValue);
				} else {
					comparison = String(aValue).localeCompare(String(bValue));
				}

				return sortState.direction === "desc" ? -comparison : comparison;
			});
			setSortedServers(initialSorted);
		}
	}, [serverListResponse?.servers, sortedServers.length, sortState]);

	// Enable/disable server
	async function toggleServerAsync(
		serverId: string,
		enable: boolean,
		sync?: boolean,
	) {
		setPending((p) => ({ ...p, [serverId]: true }));
		try {
			if (enable) {
				await serversApi.enableServer(serverId, sync);
			} else {
				await serversApi.disableServer(serverId, sync);
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
				: t("notifications.toggle.disableAction", { defaultValue: "disable" });
			const errorMessage = error instanceof Error ? error.message : String(error);
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
	}

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
			return await serversApi.updateServer(serverId, config);
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
					defaultValue: "Server {{serverId}}",
				}),
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

	const handleServerToggle = async (serverId: string, enabled: boolean) => {
		setIsTogglePending(true);
		try {
			if (enabled) {
				await serversApi.enableServer(serverId, syncServerStateToClients);
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
				await serversApi.disableServer(serverId, syncServerStateToClients);
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
	};

	const getServerDescription = (server: ServerSummary) => {
		const serverTypeRaw = server.server_type || "";
		const serverType = serverTypeRaw.toLowerCase();

		let technicalLine = "";
		if (serverType.includes("stdio") || serverType.includes("process")) {
			technicalLine = `stdio://${server.name || server.id}`;
		} else if (serverType.includes("http") || serverType.includes("sse")) {
			technicalLine = `http://localhost:3000/${server.id}`;
		} else {
			technicalLine = t("entity.description.serverLabel", {
				name: server.name || server.id,
				defaultValue: "Server: {{name}}",
			});
		}

		const metaDescription = server.meta?.description?.trim();
		const firstLine = metaDescription
			? `${metaDescription}${serverTypeRaw ? ` · ${serverTypeRaw}` : ""}`
			: technicalLine;

		return (
			<div
				className="text-sm text-slate-500 truncate max-w-[200px]"
				title={firstLine}
			>
				{firstLine}
			</div>
		);
	};

	const getUnifyEligibilityTag = (server: ServerSummary) => {
	if (!server.unify_direct_exposure_eligible) return null;
	return (
		<Badge
			variant="secondary"
			className="border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200"
		>
			{t("entity.tags.unifyEligible", { defaultValue: "Unify Direct" })}
		</Badge>
	);
};

const getConnectionTypeTags = (server: ServerSummary) => {
		const tags = [];
		const lower = (server.server_type || "").toLowerCase();
		const isStdio = lower.includes("stdio") || lower.includes("process");
		const isStreamable =
			lower.includes("stream") ||
			lower.includes("sse") ||
			lower.includes("streamable");
		const isGenericHttp = lower.includes("http") || lower.includes("rest");

		if (isStdio) {
			tags.push(
				<span
					key="stdio"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.stdio", { defaultValue: "STDIO" })}
				</span>,
			);
		} else if (isStreamable) {
			tags.push(
				<span
					key="streamable_http"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.streamableHttp", {
						defaultValue: "Streamable HTTP",
					})}
				</span>,
			);
		} else if (isGenericHttp) {
			tags.push(
				<span
					key="http"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.http", { defaultValue: "HTTP" })}
				</span>,
			);
		}

		if (tags.length === 0) {
			tags.push(
				<span
					key="default"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.http", { defaultValue: "HTTP" })}
				</span>,
			);
		}

		return tags;
	};

	const renderServerListItem = (server: ServerSummary) => {
		const serverInitial = (server.name || server.id || "S")
			.slice(0, 1)
			.toUpperCase();
		const iconSrc = server.icons?.[0]?.src;
		const iconAlt = server.name
			? t("entity.iconAlt.named", {
				name: server.name,
				defaultValue: "{{name}} icon",
			})
			: t("entity.iconAlt.fallback", { defaultValue: "Server icon" });
		const capabilitySummary = getCapabilitySummary(server);
		const capabilityStats = capabilitySummary
			? [
				{
					label: t("entity.stats.tools", { defaultValue: "Tools" }),
					value: capabilitySummary.tools_count,
				},
				{
					label: t("entity.stats.prompts", {
						defaultValue: "Prompts",
					}),
					value: capabilitySummary.prompts_count,
				},
				{
					label: t("entity.stats.resources", {
						defaultValue: "Resources",
					}),
					value: capabilitySummary.resources_count,
				},
				{
					label: t("entity.stats.templates", {
						defaultValue: "Templates",
					}),
					value: capabilitySummary.resource_templates_count,
				},
			]
			: [
				{
					label: t("entity.stats.tools", { defaultValue: "Tools" }),
					value: 0,
				},
				{
					label: t("entity.stats.prompts", { defaultValue: "Prompts" }),
					value: 0,
				},
				{
					label: t("entity.stats.resources", {
						defaultValue: "Resources",
					}),
					value: 0,
				},
				{
					label: t("entity.stats.templates", {
						defaultValue: "Templates",
					}),
					value: 0,
				},
			];

		return (
			<EntityListItem
				key={server.id}
				id={server.id}
				title={server.name}
				description={
					<div className="flex items-center gap-2">
						{getConnectionTypeTags(server)}
						{getUnifyEligibilityTag(server)}
						<ServerAuthBadge
							authMode={server.auth_mode}
							oauthStatus={server.oauth_status}
						/>
					</div>
				}
				avatar={{
					src: iconSrc,
					alt: iconSrc ? iconAlt : undefined,
					fallback: serverInitial,
				}}
				titleBadges={getUnifyEligibilityTag(server) ? [getUnifyEligibilityTag(server)] : []}
				stats={capabilityStats}
				statusBadge={
					<StatusBadge
						status={server.status}
						instances={server.instances}
						blinkOnError={["error", "unhealthy", "stopped", "failed"].includes(
							(server.status || "").toLowerCase(),
						)}
						isServerEnabled={server.enabled}
					/>
				}
				enableSwitch={{
					checked: server.enabled || false,
					onChange: (checked: boolean) =>
						handleServerToggle(server.id, checked),
					disabled: isTogglePending,
				}}
				actionButtons={
					enableServerDebug
						? [
							<Button
								key="debug"
								size="sm"
								variant="outline"
								className="p-2"
								onClick={(ev) => {
									ev.stopPropagation();
									const url = `/servers/${encodeURIComponent(server.id)}?view=debug&channel=native`;
									if (openDebugInNewWindow) {
										if (typeof window !== "undefined") {
											window.open(url, "_blank", "noopener,noreferrer");
										}
										return;
									}
									navigate(url);
								}}
								title={t("actions.debug.open", {
									defaultValue: "Open inspect view",
								})}
							>
								<Bug className="h-4 w-4" />
							</Button>,
						]
						: []
				}
				onClick={() => navigate(`/servers/${encodeURIComponent(server.id)}`)}
			/>
		);
	};

	const renderServerCard = (server: ServerSummary) => {
		const serverInitial = (server.name || server.id || "S")
			.slice(0, 1)
			.toUpperCase();
		const iconSrc = server.icons?.[0]?.src;
		const iconAlt = server.name
			? t("entity.iconAlt.named", {
				name: server.name,
				defaultValue: "{{name}} icon",
			})
			: t("entity.iconAlt.fallback", { defaultValue: "Server icon" });
		const capabilitySummary = getCapabilitySummary(server);
		const cardStats = capabilitySummary
			? [
				{
					label: t("entity.stats.tools", { defaultValue: "Tools" }),
					value: capabilitySummary.tools_count,
				},
				{
					label: t("entity.stats.prompts", { defaultValue: "Prompts" }),
					value: capabilitySummary.prompts_count,
				},
				{
					label: t("entity.stats.resources", {
						defaultValue: "Resources",
					}),
					value: capabilitySummary.resources_count,
				},
				{
					label: t("entity.stats.templates", {
						defaultValue: "Templates",
					}),
					value: capabilitySummary.resource_templates_count,
				},
			]
			: [
				{
					label: t("entity.stats.tools", { defaultValue: "Tools" }),
					value: 0,
				},
				{
					label: t("entity.stats.prompts", { defaultValue: "Prompts" }),
					value: 0,
				},
				{
					label: t("entity.stats.resources", {
						defaultValue: "Resources",
					}),
					value: 0,
				},
				{
					label: t("entity.stats.templates", {
						defaultValue: "Templates",
					}),
					value: 0,
				},
			];

		return (
			<EntityCard
				key={server.id}
				id={server.id}
				title={server.name}
				description={getServerDescription(server)}
				avatar={{
					src: iconSrc,
					alt: iconSrc ? iconAlt : undefined,
					fallback: serverInitial,
				}}
				topRightBadge={
					<div className="flex items-center gap-2">
						{getConnectionTypeTags(server)}
						{getUnifyEligibilityTag(server)}
						<ServerAuthBadge
							authMode={server.auth_mode}
							oauthStatus={server.oauth_status}
							showLabel={false}
						/>
					</div>
				}
				stats={cardStats}
				bottomLeft={
					<StatusBadge
						status={server.status}
						instances={server.instances}
						blinkOnError={["error", "unhealthy", "stopped", "failed"].includes(
							(server.status || "").toLowerCase(),
						)}
						isServerEnabled={server.enabled}
					/>
				}
				bottomRight={
					<Switch
						checked={server.enabled || false}
						onCheckedChange={(checked) => {
							toggleServerAsync(
								server.id,
								checked,
								syncServerStateToClients,
							);
						}}
						disabled={!!pending[server.id]}
						onClick={(e) => e.stopPropagation()}
					/>
				}
				onClick={() => navigate(`/servers/${encodeURIComponent(server.id)}`)}
			/>
		);
	};

	// Add inspect button handler
	const toggleDebugInfo = () => {
		if (debugInfo) {
			setDebugInfo(null);
		} else {
			const debugLines = [
				`${t("debug.info.baseUrl", { defaultValue: "API Base URL" })}: ${window.location.origin}`,
				`${t("debug.info.currentTime", { defaultValue: "Current Time" })}: ${new Date().toLocaleString()}`,
				`${t("debug.info.error", { defaultValue: "Error" })}: ${error instanceof Error ? error.message : String(error)}`,
				`${t("debug.info.data", { defaultValue: "Servers Data" })}: ${JSON.stringify(serverListResponse, null, 2)}`,
			];
			setDebugInfo(debugLines.join("\n"));
		}
	};

	// Use sorted data
	const filteredAndSortedServers = useMemo(() => {
		return sortedServers;
	}, [sortedServers]);

	const statsCards = useMemo(() => {
		const list = serverListResponse?.servers ?? [];
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
	}, [serverListResponse, t]);

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
		data: (serverListResponse?.servers || []) as ToolbarServer[],
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
		onSortedDataChange: (data) => setSortedServers(data as ServerSummary[]),
		onExpandedChange: setExpanded,
	};

	// Action buttons
	const actions = (
		<div className="flex items-center gap-2">
			{isError && enableServerDebug && (
				<Button
					onClick={toggleDebugInfo}
					variant="outline"
					size="sm"
					className="h-9 w-9 p-0"
					title={t("actions.debug.title", { defaultValue: "Inspect" })}
				>
					<AlertCircle className="h-4 w-4" />
				</Button>
			)}
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
			<div
				onDragEnter={handleAddDragEnter}
				onDragOver={handleAddDragOver}
				onDragLeave={handleAddDragLeave}
				onDrop={handleAddDrop}
				onDragEnd={handleAddDragEnd}
				className={`rounded-md ${isAddDragActive ? "ring-2 ring-slate-300 dark:ring-slate-600" : ""
					}`}
			>
				<Button
					size="icon"
					className={`h-9 w-9 transition-colors ${isAddDragActive
						? "bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900"
						: ""
						}`}
					title={t("actions.add.title", { defaultValue: "Add Server" })}
					onClick={() => setManualOpen(true)}
				>
					{isAddDragActive ? (
						<Target className="h-4 w-4" />
					) : (
						<Plus className="h-4 w-4" />
					)}
				</Button>
			</div>
		</div>
	);

	// Prepare empty state
	const emptyState = (
		<Card>
			<CardContent className="flex flex-col items-center justify-center p-6">
				<EmptyState
					icon={<Server className="h-12 w-12" />}
					title={t("emptyState.title", { defaultValue: "No servers found" })}
					description={t("emptyState.description", {
						defaultValue: "Add your first MCP server to get started",
					})}
					action={
						<Button
							onClick={() => setManualOpen(true)}
							size="sm"
							className="mt-4"
						>
							<Plus className="mr-2 h-4 w-4" />
							{t("emptyState.action", {
								defaultValue: "Add First Server",
							})}
						</Button>
					}
				/>
			</CardContent>
		</Card>
	);

	return (
		<PageLayout
			title={t("title", { defaultValue: "Servers" })}
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
			{isError && enableServerDebug && (
				<Button onClick={toggleDebugInfo} variant="outline" size="sm">
					<AlertCircle className="mr-2 h-4 w-4" />
					{debugInfo
						? t("actions.debug.hide", { defaultValue: "Hide Inspect" })
						: t("actions.debug.show", { defaultValue: "Inspect" })}
				</Button>
			)}

			{/* Display error information */}
			{isError && (
				<ErrorDisplay
					title={t("errors.loadFailed", {
						defaultValue: "Failed to load servers",
					})}
					error={error as Error}
					onRetry={() => refetch()}
				/>
			)}

			{/* Display inspect information */}
			{debugInfo && (
				<Card className="overflow-hidden">
					<CardHeader className="bg-slate-100 dark:bg-slate-800 p-4">
						<CardTitle className="text-lg flex justify-between">
							{t("debug.cardTitle", {
								defaultValue: "Inspect Details",
							})}
							<Button
								onClick={() => setDebugInfo(null)}
								variant="ghost"
								size="sm"
							>
								{t("debug.close", { defaultValue: "Close" })}
							</Button>
						</CardTitle>
					</CardHeader>
					<CardContent className="p-4">
						<pre className="whitespace-pre-wrap text-xs overflow-auto max-h-96">
							{debugInfo}
						</pre>
					</CardContent>
				</Card>
			)}

			<ListGridContainer
				loading={isLoading}
				loadingSkeleton={loadingSkeleton}
				emptyState={
					filteredAndSortedServers.length === 0 ? emptyState : undefined
				}
			>
				{viewMode === "grid"
					? filteredAndSortedServers.map(renderServerCard)
					: filteredAndSortedServers.map(renderServerListItem)}
			</ListGridContainer>

			{/* Server install pipeline */}
			<ServerInstallWizard
				ref={manualRef}
				isOpen={manualOpen}
				onClose={() => setManualOpen(false)}
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
