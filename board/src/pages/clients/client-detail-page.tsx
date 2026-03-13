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
	Play,
	Plus,
	RefreshCw,
	RotateCcw,
	Square,
	Trash2,
} from "lucide-react";
import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import { ConfirmDialog } from "../../components/confirm-dialog";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
} from "../../components/ui/avatar";
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
import { clientsApi, configSuitsApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import type {
	ClientBackupEntry,
	ClientBackupPolicySetReq,
	ClientConfigImportData,
	ClientConfigMode,
	ClientConfigSelected,
	ClientConfigUpdateData,
	ConfigSuit,
} from "../../lib/types";
import { formatBackupTime } from "../../lib/utils";

type ClientDetailTab = "overview" | "configuration" | "backups";

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
	const { t } = useTranslation("clients");
	const [displayName, setDisplayName] = useState("");
	const [selectedBackups, setSelectedBackups] = useState<string[]>([]);
	const [detected, setDetected] = useState<boolean>(false);
	const [bulkConfirmOpen, setBulkConfirmOpen] = useState(false);
	const [tabValue, setTabValue] = useState<ClientDetailTab>("overview");
	const [selectedProfiles, setSelectedProfiles] = useState<string[]>([]);

	// Try to get display name from list cache
	useEffect(() => {
		const cached = qc.getQueryData<{
			client?: Array<{
				identifier: string;
				display_name?: string;
				detected?: boolean;
			}>;
		}>(["clients"]);
		if (cached?.client) {
			const found = cached.client.find((c) => c.identifier === identifier);
			if (found) {
				setDisplayName(found.display_name || "");
				setDetected(!!found.detected);
				const modeVal = (found as any)?.config_mode;
				if (typeof modeVal === "string") setMode(modeVal as ClientConfigMode);
				const trVal = (found as any)?.transport;
				setTransport(typeof trVal === "string" ? (trVal as string) : "auto");
			}
		}
		if (!displayName) {
			// Fallback: fetch list once to resolve display name
			clientsApi
				.list(false)
				.then((d) => {
					if (d?.client) {
						const f = d.client.find((c) => c.identifier === identifier);
						if (f?.display_name) setDisplayName(f.display_name);
						if (typeof f?.detected === "boolean") setDetected(!!f.detected);
						const modeVal = (f as any)?.config_mode;
						if (typeof modeVal === "string")
							setMode(modeVal as ClientConfigMode);
						const trVal = (f as any)?.transport;
						setTransport(typeof trVal === "string" ? (trVal as string) : "auto");
					}
				})
				.catch(() => {});
		}
	}, [identifier, qc, displayName]);

	const limitId = useId();
	const [mode, setMode] = useState<ClientConfigMode>("hosted");
	const [transport, setTransport] = useState<string>("auto");
	const [selectedConfig, setSelectedConfig] =
		useState<ClientConfigSelected>("default");
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

	const {
		data: backupsData,
		isLoading: loadingBackups,
		refetch: refetchBackups,
	} = useQuery({
		queryKey: ["client-backups", identifier],
		queryFn: () => clientsApi.listBackups(identifier || undefined),
		enabled: !!identifier,
	});

	// Fetch profiles data
	const { data: profilesData, isLoading: loadingProfiles } = useQuery({
		queryKey: ["profiles"],
		queryFn: () => configSuitsApi.getAll(),
		retry: 1,
	});

	const backups: ClientBackupEntry[] = backupsData?.backups || [];
	const visibleBackups = useMemo(
		() => backups.filter((b) => b.identifier === identifier),
		[backups, identifier],
	);

	// Process profiles data
	const profiles: ConfigSuit[] = profilesData?.suits || [];
	const activeProfiles = useMemo(
		() => profiles.filter((profile) => profile.is_active),
		[profiles],
	);
	const sharedProfiles = useMemo(
		() =>
			profiles.filter(
				(profile) => profile.suit_type === "shared" && profile.is_active,
			),
		[profiles],
	);

	// Get profile IDs for fetching capabilities
	const profileIds = useMemo(() => {
		if (selectedConfig === "default") {
			return activeProfiles.map((p) => p.id);
		} else if (selectedConfig === "profile") {
			return sharedProfiles.map((p) => p.id);
		}
		return [];
	}, [selectedConfig, activeProfiles, sharedProfiles]);

	// Fetch capabilities for profiles
	const profileCapabilitiesQueries = useQueries({
		queries: profileIds.map((profileId) => ({
			queryKey: ["profile-capabilities", profileId],
			queryFn: async () => {
				const [serversRes, toolsRes, resourcesRes, promptsRes] =
					await Promise.all([
						configSuitsApi.getServers(profileId),
						configSuitsApi.getTools(profileId),
						configSuitsApi.getResources(profileId),
						configSuitsApi.getPrompts(profileId),
					]);

				return {
					profileId,
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
							toolsRes?.tools?.filter((t: { enabled?: boolean }) => t.enabled)
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
			enabled: profileIds.length > 0,
			retry: 1,
		})),
	});

	// Create capabilities map
	const profileCapabilities = useMemo(() => {
		const map = new Map();
		profileCapabilitiesQueries.forEach((query) => {
			if (query.data) {
				map.set(query.data.profileId, query.data);
			}
		});
		return map;
	}, [profileCapabilitiesQueries]);

	const templateMeta = configDetails?.template;
	const detailDescription =
		configDetails?.description ?? templateMeta?.description ?? "";
	const detailHomepageUrl =
		configDetails?.homepage_url ?? templateMeta?.homepage_url ?? "";
	const detailDocsUrl = configDetails?.docs_url ?? templateMeta?.docs_url ?? "";
	const detailSupportUrl =
		configDetails?.support_url ?? templateMeta?.support_url ?? "";

	const [logEntries, setLogEntries] = useState<
		Array<{
			id: string;
			message: string;
			level: "warning";
			timestamp: string;
		}>
	>([]);
	const [logFilter, setLogFilter] = useState("");

	useEffect(() => {
		if (configDetails?.warnings?.length) {
			setLogEntries((prev) => {
				const existing = new Set(prev.map((entry) => entry.message));
				const next = [...prev];
				for (const warning of configDetails.warnings ?? []) {
					if (!existing.has(warning)) {
						next.push({
							id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
							message: warning,
							level: "warning",
							timestamp: new Date().toISOString(),
						});
					}
				}
				return next;
			});
		}
	}, [configDetails?.warnings]);

	const filteredLogs = useMemo(() => {
		if (!logFilter.trim()) {
			return logEntries;
		}
		const term = logFilter.toLowerCase();
		return logEntries.filter((entry) =>
			entry.message.toLowerCase().includes(term),
		);
	}, [logEntries, logFilter]);

	const { data: policyData, refetch: refetchPolicy } = useQuery({
		queryKey: ["client-policy", identifier],
		queryFn: () => clientsApi.getBackupPolicy(identifier || ""),
		enabled: !!identifier,
	});

	const applyMutation = useMutation<
		{ data: ClientConfigUpdateData | null; preview: boolean },
		unknown,
		{ preview: boolean }
	>({
		mutationFn: async ({ preview }) => {
			if (!identifier) throw new Error("No identifier provided");
			const data = await clientsApi.applyConfig({
				identifier,
				mode,
				selected_config: selectedConfig,
				preview,
			});
			return { data: data ?? null, preview };
		},
		onMutate: () => {
			// No longer needed since we removed the result display
		},
		onSuccess: ({ data, preview }) => {
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
					// Surface backend warnings from preview
					if (Array.isArray(data.warnings) && data.warnings.length) {
						setLogEntries((prev) => {
							const existing = new Set(prev.map((e) => e.message));
							const next = [...prev];
							for (const w of data.warnings!) {
								if (!existing.has(w)) {
									next.push({
										id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
										message: w,
										level: "warning",
										timestamp: new Date().toISOString(),
									});
								}
							}
							return next;
						});
					}
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
			} else {
				notifySuccess(
					t("detail.notifications.applied.title", {
						defaultValue: "Applied",
					}),
					t("detail.notifications.applied.message", {
						defaultValue: "Configuration applied",
					}),
				);
				// Refresh backup records after successful apply
				refetchBackups();
				// Surface backend warnings from apply
				if (data && Array.isArray(data.warnings) && data.warnings.length) {
					setLogEntries((prev) => {
						const existing = new Set(prev.map((e) => e.message));
						const next = [...prev];
						for (const w of data.warnings!) {
							if (!existing.has(w)) {
								next.push({
									id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
									message: w,
									level: "warning",
									timestamp: new Date().toISOString(),
								});
							}
						}
						return next;
					});
				}
			}
			qc.invalidateQueries({ queryKey: ["client-config", identifier] });
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

	const toggleManagedMutation = useMutation({
		mutationFn: async () => {
			if (!identifier) throw new Error("No identifier provided");
			const next = !(configDetails?.managed ?? false);
			await clientsApi.manage(identifier, next ? "enable" : "disable");
			await refetchDetails();
		},
		onSuccess: () =>
			notifySuccess(
				t("detail.notifications.managedUpdated.title", {
					defaultValue: "Updated",
				}),
				t("detail.notifications.managedUpdated.message", {
					defaultValue: "Managed state changed",
				}),
			),
		onError: (e) =>
			notifyError(
				t("detail.notifications.managedFailed.title", {
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
					defaultValue: "Configuration restored from backup",
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
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<div className="flex flex-col gap-1">
					<div className="flex items-center gap-3 flex-wrap">
						<h2 className="text-3xl font-bold tracking-tight">
							{displayName || identifier}
						</h2>
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
				onValueChange={(value) => setTabValue(value as ClientDetailTab)}
			>
				<div className="flex items-center justify-between">
					<TabsList>
						<TabsTrigger value="overview">
							{t("detail.tabs.overview", { defaultValue: "Overview" })}
						</TabsTrigger>
						<TabsTrigger value="configuration">
							{t("detail.tabs.configuration", {
								defaultValue: "Configuration",
							})}
						</TabsTrigger>
						<TabsTrigger value="backups">
							{t("detail.tabs.backups", { defaultValue: "Backups" })}
						</TabsTrigger>
						<TabsTrigger value="logs">
							{t("detail.tabs.logs", { defaultValue: "Logs" })}
						</TabsTrigger>
					</TabsList>
				</div>

				<TabsContent value="overview">
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
												<Avatar className="text-sm">
													{configDetails.logo_url ? (
														<AvatarImage
															src={configDetails.logo_url}
															alt={displayName || identifier}
														/>
													) : null}
													<AvatarFallback>
														{(displayName || identifier || "C")
															.slice(0, 1)
															.toUpperCase()}
													</AvatarFallback>
												</Avatar>
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
															defaultValue: "Supported Transports",
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
													onClick={() => toggleManagedMutation.mutate()}
													disabled={
														toggleManagedMutation.isPending || !configDetails
													}
													className="gap-2"
												>
													{configDetails?.managed ? (
														<Square className="h-4 w-4" />
													) : (
														<Play className="h-4 w-4" />
													)}
													{configDetails?.managed
														? t("detail.overview.buttons.disable", {
																defaultValue: "Disable",
															})
														: t("detail.overview.buttons.enable", {
																defaultValue: "Enable",
															})}
												</Button>
												{(() => {
													const supported =
														(configDetails?.supported_transports ||
															[]) as string[];
													// Show selector only when there are 2+ concrete transports
													if (!supported || supported.length < 2) return null;
													const optionLabel = (v: string) =>
														v === "streamable_http"
															? "Streamable HTTP"
															: v.toUpperCase();
													const allowed = new Set(supported);
													const options = [
														"stdio",
														"sse",
														"streamable_http",
													].filter((v) => allowed.has(v));
													return (
														<ButtonGroupText className="h-9 p-0 select-none shadow-none">
															<Select
																value={transport}
																onValueChange={async (v) => {
																	// Transport must be one of: auto, sse, stdio, streamable_http
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
																	aria-label="Transport selector"
																>
																	<SelectValue />
																</SelectTrigger>
																<SelectContent align="end">
																	<SelectItem value="auto">Auto</SelectItem>
																	{options.map((v) => (
																		<SelectItem key={v} value={v}>
																			{optionLabel(v)}
																		</SelectItem>
																	))}
																</SelectContent>
															</Select>
														</ButtonGroupText>
													);
												})()}
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
					</div>
				</TabsContent>

				<TabsContent value="configuration">
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
								{configDetails?.managed ? (
									<Button
										size="sm"
										variant="default"
										onClick={() => applyMutation.mutate({ preview: false })}
										disabled={applyMutation.isPending}
										className="gap-2"
									>
										<HardDrive
											className={`h-4 w-4 ${applyMutation.isPending ? "animate-pulse" : ""}`}
										/>
										{t("detail.configuration.reapply", {
											defaultValue: "Apply",
										})}
									</Button>
								) : null}
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
																	"MCPMate provisions a dedicated MCP server for this client and enables advanced MCPMate features.",
															},
														)}
													{mode === "transparent" &&
														t(
															"detail.configuration.sections.mode.descriptions.transparent",
															{
																defaultValue:
																	"MCPMate applies the selected original MCP server configurations for this client and performs no additional actions.",
															},
														)}
													{mode === "none" &&
														t(
															"detail.configuration.sections.mode.descriptions.none",
															{
																defaultValue:
																	"MCPMate does not manage this client's configuration.",
															},
														)}
												</p>
											</div>
											<Segment
												value={mode}
												onValueChange={(v) => setMode(v as ClientConfigMode)}
												options={[
													{
														value: "hosted",
														label: t(
															"detail.configuration.sections.mode.options.hosted",
															{ defaultValue: "Hosted Mode" },
														),
													},
													{
														value: "transparent",
														label: t(
															"detail.configuration.sections.mode.options.transparent",
															{ defaultValue: "Transparent Mode" },
														),
													},
													{
														value: "none",
														label: t(
															"detail.configuration.sections.mode.options.none",
															{ defaultValue: "None" },
														),
													},
												]}
												showDots={true}
												className="w-full"
											/>
										</div>

										{/* Source Selection */}
										<div className="space-y-3">
											<div className="space-y-1">
												<h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">
													{t("detail.configuration.sections.source.title", {
														defaultValue: "2. Capability Source",
													})}
												</h4>
												<p className="text-xs text-slate-500 leading-relaxed">
													{selectedConfig === "default" &&
														t(
															"detail.configuration.sections.source.descriptions.default",
															{
																defaultValue:
																	"Use all currently activated profiles.",
															},
														)}
													{selectedConfig === "profile" &&
														t(
															"detail.configuration.sections.source.descriptions.profile",
															{
																defaultValue:
																	"Select specific shared profiles to include.",
															},
														)}
													{selectedConfig === "custom" &&
														t(
															"detail.configuration.sections.source.descriptions.custom",
															{
																defaultValue:
																	"Use customized configuration settings.",
															},
														)}
												</p>
											</div>
											<Segment
												value={selectedConfig}
												onValueChange={(v) =>
													setSelectedConfig(v as ClientConfigSelected)
												}
												options={[
													{
														value: "default",
														label: t(
															"detail.configuration.sections.source.options.default",
															{ defaultValue: "Activated" },
														),
														status:
															t(
																"detail.configuration.sections.source.statusLabel.default",
																{ defaultValue: "" },
															) || undefined,
													},
													{
														value: "profile",
														label: t(
															"detail.configuration.sections.source.options.profile",
															{ defaultValue: "Profiles" },
														),
														status:
															t(
																"detail.configuration.sections.source.statusLabel.profile",
																{ defaultValue: "WIP" },
															) || undefined,
													},
													{
														value: "custom",
														label: t(
															"detail.configuration.sections.source.options.custom",
															{ defaultValue: "Customize" },
														),
														status:
															t(
																"detail.configuration.sections.source.statusLabel.custom",
																{ defaultValue: "WIP" },
															) || undefined,
													},
												]}
												showDots={true}
												className="w-full"
												disabled={mode === "none"}
											/>
										</div>
									</div>

									{/* Right side - Profiles List (6/10) */}
									{(mode === "hosted" ||
										mode === "transparent" ||
										mode === "none") && (
										<div
											className={`col-span-6 ${mode === "none" ? "opacity-50 pointer-events-none" : ""}`}
										>
											<div className="mb-3">
												<h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">
													{t("detail.configuration.sections.profiles.title", {
														defaultValue: "3. Profiles List",
													})}
												</h4>
												{/* Dynamic description based on source */}
												{selectedConfig === "default" && (
													<p className="text-xs text-slate-500 mt-1 leading-relaxed">
														{t(
															"detail.configuration.sections.profiles.descriptions.default",
															{
																defaultValue:
																	"When the activated source is selected, configure all currently activated profiles. Checkboxes are locked to keep the selection consistent.",
															},
														)}
													</p>
												)}
												{selectedConfig === "profile" && (
													<p className="text-xs text-slate-500 mt-1 leading-relaxed">
														{t(
															"detail.configuration.sections.profiles.descriptions.profile",
															{
																defaultValue:
																	"Select which shared profiles to include in this client's configuration.",
															},
														)}
													</p>
												)}
												{selectedConfig === "custom" && (
													<p className="text-xs text-slate-500 mt-1 leading-relaxed">
														{t(
															"detail.configuration.sections.profiles.descriptions.custom",
															{
																defaultValue:
																	"Create and maintain a customized configuration for the current application.",
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
											) : mode === "none" ? (
												<div className="rounded border border-dashed border-slate-200 p-6 text-center text-sm text-slate-500 dark:border-slate-800 dark:text-slate-400">
													{t(
														"detail.configuration.sections.profiles.modeNone",
														{
															defaultValue:
																'Configuration mode is set to "none" - no profiles need to be applied',
														},
													)}
												</div>
											) : (
												<CapsuleStripeList>
													{selectedConfig === "default" ? (
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
																		<div className="flex w-full items-center gap-3">
																			<div className="flex h-6 w-6 items-center justify-center rounded-full border-2 border-slate-300 bg-slate-100 dark:border-slate-600 dark:bg-slate-700">
																				<Check className="h-3 w-3 text-slate-500" />
																			</div>
																			<div className="flex-1 min-w-0">
																				<div className="font-medium text-sm truncate">
																					{profile.name}
																				</div>
																				<div className="text-xs text-slate-500 truncate">
																					{profile.description ||
																						t(
																							"detail.configuration.labels.noDescription",
																							{
																								defaultValue: "No description",
																							},
																						)}
																				</div>
																				{capabilities && (
																					<div className="flex gap-4 mt-1 text-xs text-slate-500">
																						<span>
																							{t(
																								"detail.configuration.labels.servers",
																								{
																									defaultValue: "Servers",
																								},
																							)}
																							: {capabilities.servers.enabled}/
																							{capabilities.servers.total}
																						</span>
																						<span>
																							{t(
																								"detail.configuration.labels.tools",
																								{
																									defaultValue: "Tools",
																								},
																							)}
																							: {capabilities.tools.enabled}/
																							{capabilities.tools.total}
																						</span>
																						<span>
																							{t(
																								"detail.configuration.labels.resources",
																								{
																									defaultValue: "Resources",
																								},
																							)}
																							: {capabilities.resources.enabled}
																							/{capabilities.resources.total}
																						</span>
																						<span>
																							{t(
																								"detail.configuration.labels.prompts",
																								{
																									defaultValue: "Prompts",
																								},
																							)}
																							: {capabilities.prompts.enabled}/
																							{capabilities.prompts.total}
																						</span>
																					</div>
																				)}
																			</div>
																			<div className="ml-auto flex items-center gap-2">
																				<Button
																					variant="ghost"
																					size="sm"
																					className="h-6 w-6 p-0"
																					onClick={() =>
																						navigate(`/profiles/${profile.id}`)
																					}
																				>
																					<Info className="h-3 w-3" />
																				</Button>
																			</div>
																		</div>
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
																		interactive
																		className={`group relative transition-colors ${
																			isSelected
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
																		<div className="flex w-full items-center gap-3">
																			<div
																				className={`flex h-6 w-6 items-center justify-center rounded-full border-2 transition-all duration-200 ${
																					isSelected
																						? "border-primary bg-primary text-white"
																						: "border-slate-300 bg-white dark:border-slate-600 dark:bg-slate-700"
																				}`}
																			>
																				{isSelected && (
																					<Check className="h-3 w-3" />
																				)}
																			</div>
																			<div className="flex-1 min-w-0">
																				<div className="font-medium text-sm truncate">
																					{profile.name}
																				</div>
																				<div className="text-xs text-slate-500 truncate">
																					{profile.description ||
																						t(
																							"detail.configuration.labels.noDescription",
																							{
																								defaultValue: "No description",
																							},
																						)}
																				</div>
																				{capabilities && (
																					<div className="flex gap-4 mt-1 text-xs text-slate-500">
																						<span>
																							{t(
																								"detail.configuration.labels.servers",
																								{
																									defaultValue: "Servers",
																								},
																							)}
																							: {capabilities.servers.enabled}/
																							{capabilities.servers.total}
																						</span>
																						<span>
																							{t(
																								"detail.configuration.labels.tools",
																								{
																									defaultValue: "Tools",
																								},
																							)}
																							: {capabilities.tools.enabled}/
																							{capabilities.tools.total}
																						</span>
																						<span>
																							{t(
																								"detail.configuration.labels.resources",
																								{
																									defaultValue: "Resources",
																								},
																							)}
																							: {capabilities.resources.enabled}
																							/{capabilities.resources.total}
																						</span>
																						<span>
																							{t(
																								"detail.configuration.labels.prompts",
																								{
																									defaultValue: "Prompts",
																								},
																							)}
																							: {capabilities.prompts.enabled}/
																							{capabilities.prompts.total}
																						</span>
																					</div>
																				)}
																			</div>
																			<div className="ml-auto flex items-center gap-2">
																				<Button
																					variant="ghost"
																					size="sm"
																					className="h-6 w-6 p-0"
																					onClick={(e) => {
																						e.stopPropagation();
																						navigate(`/profiles/${profile.id}`);
																					}}
																				>
																					<Info className="h-3 w-3" />
																				</Button>
																			</div>
																		</div>
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

													{/* Ghost item for creating new profile */}
													<CapsuleStripeListItem
														interactive
														className="border-dashed border-slate-300 dark:border-slate-600 hover:border-slate-400 dark:hover:border-slate-500"
														onClick={() => {
															if (selectedConfig === "custom") {
																// Navigate to profile creation with host application type
																navigate("/profiles?type=host_app&mode=create");
															} else {
																navigate("/profiles");
															}
														}}
													>
														<div className="flex w-full items-center gap-3">
															<div className="flex h-6 w-6 items-center justify-center rounded-full border-2 border-dashed border-slate-300 dark:border-slate-600">
																<Plus className="h-3 w-3 text-slate-400" />
															</div>
															<div className="flex-1 min-w-0">
																<div className="font-medium text-sm truncate text-slate-700 dark:text-slate-300">
																	{selectedConfig === "custom"
																		? t(
																				"detail.configuration.sections.profiles.ghost.titleCustom",
																				{
																					defaultValue: "Customize the profile",
																				},
																			)
																		: t(
																				"detail.configuration.sections.profiles.ghost.titleDefault",
																				{ defaultValue: "Add a new profile" },
																			)}
																</div>
																<div className="text-xs text-slate-400 dark:text-slate-600 truncate">
																	{selectedConfig === "custom"
																		? t(
																				"detail.configuration.sections.profiles.ghost.subtitleCustom",
																				{
																					defaultValue:
																						"Create and manage host application profile",
																				},
																			)
																		: t(
																				"detail.configuration.sections.profiles.ghost.subtitleDefault",
																				{
																					defaultValue:
																						"Click to navigate to profile management page",
																				},
																			)}
																</div>
															</div>
														</div>
													</CapsuleStripeListItem>
												</CapsuleStripeList>
											)}
										</div>
									)}
								</div>
							</CardContent>
						</Card>
					</div>
				</TabsContent>

				<TabsContent value="backups">
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
															className={`flex h-6 w-6 items-center justify-center rounded-full border text-[0px] transition-all duration-200 ${
																selected
																	? "border-primary bg-primary text-white shadow-sm"
																	: "border-slate-300 text-transparent group-hover:border-primary/50 group-hover:text-primary/60 dark:border-slate-700 dark:group-hover:border-primary/50"
															}`}
														>
															<Check className="h-3 w-3" />
														</div>
														<div
															className={`font-mono transition-colors duration-200 ${
																selected
																	? "text-primary"
																	: "text-slate-700 dark:text-slate-200"
															}`}
														>
															{b.backup}
														</div>
													</div>
													<div className="flex items-center justify-end gap-4">
														<div
															className={`flex items-center gap-4 text-slate-500 transition-all duration-200 ${
																selected ? "text-primary" : ""
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
									{t("detail.backups.empty", {
										defaultValue: "No backups.",
									})}
								</div>
							)}
						</CardContent>
					</Card>
				</TabsContent>

				<TabsContent value="logs">
					<Card>
						<CardHeader className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
							<div>
								<CardTitle>
									{t("detail.logs.title", { defaultValue: "Logs" })}
								</CardTitle>
								<CardDescription>
									{t("detail.logs.description", {
										defaultValue:
											"Runtime warnings and backend notes for this client.",
									})}
								</CardDescription>
							</div>
							<div className="flex flex-wrap items-center gap-2">
								<Input
									type="search"
									placeholder={t("detail.logs.searchPlaceholder", {
										defaultValue: "Search logs...",
									})}
									value={logFilter}
									onChange={(event) => setLogFilter(event.target.value)}
									className="h-8 w-48"
								/>
								<Button
									size="sm"
									variant="outline"
									onClick={() => setLogEntries([])}
									disabled={!logEntries.length}
								>
									{t("detail.logs.clear", { defaultValue: "Clear Logs" })}
								</Button>
							</div>
						</CardHeader>
						<CardContent className="pt-0">
							{filteredLogs.length ? (
								<div className="space-y-3">
									{filteredLogs.map((entry) => (
										<div
											key={entry.id}
											className="rounded-xl border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-950"
										>
											<div className="flex items-center justify-between border-b border-slate-100 px-4 py-2 text-xs text-slate-500 dark:border-slate-800">
												<span>
													{new Date(entry.timestamp).toLocaleTimeString()}
												</span>
												<span className="rounded-full bg-amber-100 px-2 py-0.5 text-[11px] font-medium uppercase tracking-wide text-amber-800 dark:bg-amber-900/40 dark:text-amber-200">
													{t("detail.logs.warning", {
														defaultValue: "Warning",
													})}
												</span>
											</div>
											<pre className="px-4 py-3 text-[12px] leading-relaxed text-slate-700 dark:text-slate-200 whitespace-pre-wrap">
												{entry.message}
											</pre>
										</div>
									))}
								</div>
							) : (
								<div className="rounded border border-dashed border-slate-200 p-6 text-center text-sm text-slate-500 dark:border-slate-800 dark:text-slate-400">
									{t("detail.logs.empty", {
										defaultValue:
											"No log entries recorded for this client yet.",
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
									"Restore configuration from the selected backup? Current config may be overwritten.",
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
									<SelectItem value="keep_n">keep_n</SelectItem>
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
		</div>
	);
}

export default ClientDetailPage;
