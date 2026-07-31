import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import {
	useBulkSelection,
	useEnableDisableBulkActions,
} from "../../components/bulk-selection";
import CapabilityList, {
	type CapabilityKind,
} from "../../components/capability-list";
import { CapabilityManagementPanel } from "../../components/capability-management-panel";
import type { CapabilityPreviewFlatItem } from "../../components/capability-preview-list";
import { CAPABILITY_SCROLL_CARD_CLASS } from "../../components/capability-scroll-card-layout";
import { CachedAvatar } from "../../components/cached-avatar";
import { SurfaceReviewDialog } from "../../components/surface-review-dialog";
import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { clientsApi, serversApi } from "../../lib/api";
import { capabilityRecordMatchesSearch } from "../../lib/capability-search";
import { capabilityKey, splitCapabilityKey } from "../../lib/capability-keys";
import { useCapabilityKindFilters } from "../../hooks/use-capability-kind-filters";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import type { CapabilityRecord } from "../../types/capabilities";
import type {
	ClientCapabilityConfigData,
	ClientCapabilityConfigReq,
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitResourceTemplate,
	ConfigSuitTool,
	ServerDetail,
	UnifyDirectCapabilityRefs,
} from "../../lib/types";
import {
	getDirectAuthenticationNotice,
	getCapabilityId,
	projectDirectCapabilities,
} from "./client-direct-capabilities";

type CapabilityStatusFilter = "all" | "enabled" | "disabled";

type DirectFlatCapabilityItem = CapabilityRecord & {
	__directCapabilityKind: CapabilityKind;
};

function getServerIconSrc(server?: ServerDetail): string | undefined {
	const icon = server?.icons?.find(
		(entry: { src?: string | null }) =>
			typeof entry?.src === "string" && entry.src.length > 0,
	);
	return icon?.src;
}

function requireCapabilityConfig(
	config: ClientCapabilityConfigData | null,
): ClientCapabilityConfigData {
	if (!config) {
		throw new Error("Client capability configuration is not available.");
	}
	return config;
}

function normalizeCapabilityIds(ids: string[] = []): string[] {
	return Array.from(new Set(ids.map((id) => id.trim()).filter(Boolean))).sort();
}

function getSelectedCapabilityRefs(
	capabilityConfig: ClientCapabilityConfigData,
): UnifyDirectCapabilityRefs {
	const capabilityRefs = capabilityConfig.unify_direct_exposure?.capability_refs;
	return {
		tool_refs: normalizeCapabilityIds(capabilityRefs?.tool_refs),
		prompt_refs: normalizeCapabilityIds(capabilityRefs?.prompt_refs),
		resource_refs: normalizeCapabilityIds(capabilityRefs?.resource_refs),
		template_refs: normalizeCapabilityIds(capabilityRefs?.template_refs),
	};
}

function getCapabilityDetailKey(
	item: Record<string, unknown>,
	kind: CapabilityKind,
): string | null {
	switch (kind) {
		case "tools":
		case "prompts":
			return getCapabilityId(item, ["unique_name"]);
		case "resources":
			return getCapabilityId(item, ["unique_uri"]);
		case "templates":
			return getCapabilityId(item, ["unique_uri_template"]);
	}
}

function resolveNextCapabilityIdList(
	currentIds: string[] = [],
	capabilityId: string,
	enable: boolean,
): string[] {
	const remainingIds = currentIds.filter((id) => id !== capabilityId);
	return enable
		? normalizeCapabilityIds([...remainingIds, capabilityId])
		: remainingIds;
}

function createCapabilityConfigPayload(
	identifier: string,
	existingConfig: ClientCapabilityConfigData,
	nextCapabilityRefs: UnifyDirectCapabilityRefs,
): ClientCapabilityConfigReq {
	return {
		identifier,
		capability_source: existingConfig.capability_source,
		selected_profile_ids: existingConfig.selected_profile_ids,
		source_revision_set: existingConfig.source_revision_set,
		unify_direct_exposure: {
			route_mode: "capability_level",
			server_ids: [],
			capability_refs: nextCapabilityRefs,
		},
	};
}

function directCapabilityDetailsCacheToken(items: DirectFlatCapabilityItem[]) {
	return items
		.map((item) =>
			[
				item.__directCapabilityKind,
				item.id,
				item.enabled ? "1" : "0",
				item.description ?? "",
			].join(":"),
		)
		.join("|");
}

export function ClientDirectCapabilitiesPage() {
	usePageTranslations("clients");
	usePageTranslations("servers");
	usePageTranslations("profiles");
	const { t, i18n } = useTranslation(["clients", "servers"]);
	const navigate = useNavigate();
	const { identifier, serverId } = useParams<{
		identifier: string;
		serverId: string;
	}>();
	const [searchParams, setSearchParams] = useSearchParams();
	const reviewItemId = searchParams.get("review_item");
	const reviewRefId = searchParams.get("ref_id");
	const queryClient = useQueryClient();
	const [capabilityQuery, setCapabilityQuery] = useState("");
	const {
		kindFilters: capabilityKindFilters,
		kindMatches: capabilityKindMatches,
		kindFilterLabel: capabilityKindFilterLabel,
		kindFilterOptions: capabilityKindFilterOptions,
		toggleKindFilter: toggleCapabilityKindFilter,
		clearKindFilters: clearCapabilityKindFilters,
	} = useCapabilityKindFilters(t);
	const [capabilityStatus, setCapabilityStatus] =
		useState<CapabilityStatusFilter>("all");
	const capabilityBulk = useBulkSelection<string>();

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
		isError: isCapabilitiesError,
		error: capabilitiesError,
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
					failures: [],
					failureState: "none" as const,
					authentication: null,
				};
			}
			const [
				capabilityLists,
				clientCapabilityConfig,
			] = await Promise.all([
				serversApi.listAllCapabilities(serverId),
				clientsApi.getCapabilityConfig(identifier),
			]);
			const selectedCapabilityRefs = getSelectedCapabilityRefs(
				requireCapabilityConfig(clientCapabilityConfig),
			);
			return projectDirectCapabilities(
				capabilityLists,
				selectedCapabilityRefs,
				serverId,
				serverDetails?.name ?? serverId,
			);
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
	const capabilityFailureState = capabilityResponse?.failureState ?? "none";
	const authenticationFailure = capabilityResponse?.authentication ?? null;
	const authenticationNotice =
		getDirectAuthenticationNotice(authenticationFailure);
	const openServerLogs = () => {
		if (serverId) {
			navigate(`/audit?server_id=${encodeURIComponent(serverId)}`);
		}
	};

	const loadCapabilityDetails = useCallback(
		async (
			item: Record<string, unknown>,
			kind: CapabilityKind,
		): Promise<CapabilityRecord | null> => {
			if (!serverId) return null;
			const key = getCapabilityDetailKey(item, kind);
			if (!key) return null;
			const detail = await serversApi.getCapabilityDetail(serverId, kind, key);
			return (detail.item ?? null) as CapabilityRecord | null;
		},
		[serverId],
	);

	useEffect(() => {
		if (!reviewRefId) return;
		setCapabilityQuery(reviewRefId);
	}, [reviewRefId]);

	const capabilityStatusFilter = useCallback(
		(item: { enabled: boolean }) =>
			capabilityStatus === "all" ||
			(capabilityStatus === "enabled" ? item.enabled : !item.enabled),
		[capabilityStatus],
	);

	const filteredTools = useMemo(
		() => tools.filter(capabilityStatusFilter),
		[capabilityStatusFilter, tools],
	);
	const filteredPrompts = useMemo(
		() => prompts.filter(capabilityStatusFilter),
		[capabilityStatusFilter, prompts],
	);
	const filteredResources = useMemo(
		() => resources.filter(capabilityStatusFilter),
		[capabilityStatusFilter, resources],
	);
	const filteredTemplates = useMemo(
		() => templates.filter(capabilityStatusFilter),
		[capabilityStatusFilter, templates],
	);

	const visibleCapabilityKeys = useMemo(
		() => [
			...(capabilityKindMatches("tools")
				? filteredTools
					.filter((tool) =>
						capabilityRecordMatchesSearch(
							tool as CapabilityRecord,
							capabilityQuery,
						),
					)
					.map((tool) => capabilityKey("tools", tool.id))
				: []),
			...(capabilityKindMatches("resources")
				? filteredResources
					.filter((resource) =>
						capabilityRecordMatchesSearch(
							resource as CapabilityRecord,
							capabilityQuery,
						),
					)
					.map((resource) => capabilityKey("resources", resource.id))
				: []),
			...(capabilityKindMatches("prompts")
				? filteredPrompts
					.filter((prompt) =>
						capabilityRecordMatchesSearch(
							prompt as CapabilityRecord,
							capabilityQuery,
						),
					)
					.map((prompt) => capabilityKey("prompts", prompt.id))
				: []),
			...(capabilityKindMatches("templates")
				? filteredTemplates
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
			capabilityKindMatches,
			capabilityQuery,
			filteredPrompts,
			filteredResources,
			filteredTemplates,
			filteredTools,
		],
	);

	const capabilityStatusLabel = useMemo(() => {
		if (capabilityStatus === "enabled") {
			return t("servers:detail.filters.status.enabled", {
				defaultValue: "Enabled",
			});
		}
		if (capabilityStatus === "disabled") {
			return t("servers:detail.filters.status.disabled", {
				defaultValue: "Disabled",
			});
		}
		return t("servers:detail.filters.status.all", {
			defaultValue: "All",
		});
	}, [capabilityStatus, i18n.language, t]);

	const updateCapabilityRefs = useCallback(
		async (nextCapabilityRefs: UnifyDirectCapabilityRefs) => {
			if (!identifier) return;
			const existingConfig = requireCapabilityConfig(
				await loadCapabilityConfig(),
			);
			await clientsApi.updateCapabilityConfig(
				createCapabilityConfigPayload(
					identifier,
					existingConfig,
					nextCapabilityRefs,
				),
			);
		},
		[identifier],
	);

	const toolToggleMutation = useMutation<
		unknown,
		unknown,
		{ toolId: string; enable: boolean }
	>({
		mutationFn: async ({ toolId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig = requireCapabilityConfig(
				await loadCapabilityConfig(),
			);
			const currentCapabilityRefs = getSelectedCapabilityRefs(existingConfig);
			await updateCapabilityRefs({
				...currentCapabilityRefs,
				tool_refs: resolveNextCapabilityIdList(
					currentCapabilityRefs.tool_refs,
					toolId,
					enable,
				),
			});
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
			notifySuccess(
				t("clients:detail.directExposure.notifications.savedTitle", {
					defaultValue: "Direct capabilities updated",
				}),
				t("clients:detail.directExposure.notifications.savedMessage", {
					defaultValue: "The direct capability exposure list has been updated.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("clients:detail.directExposure.notifications.saveFailedTitle", {
					defaultValue: "Unable to update direct capabilities",
				}),
				String(error),
			);
		},
	});

	const promptToggleMutation = useMutation<
		unknown,
		unknown,
		{ promptId: string; enable: boolean }
	>({
		mutationFn: async ({ promptId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig = requireCapabilityConfig(
				await loadCapabilityConfig(),
			);
			const currentCapabilityRefs = getSelectedCapabilityRefs(existingConfig);
			await updateCapabilityRefs({
				...currentCapabilityRefs,
				prompt_refs: resolveNextCapabilityIdList(
					currentCapabilityRefs.prompt_refs,
					promptId,
					enable,
				),
			});
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
		},
	});

	const resourceToggleMutation = useMutation<
		unknown,
		unknown,
		{ resourceId: string; enable: boolean }
	>({
		mutationFn: async ({ resourceId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig = requireCapabilityConfig(
				await loadCapabilityConfig(),
			);
			const currentCapabilityRefs = getSelectedCapabilityRefs(existingConfig);
			await updateCapabilityRefs({
				...currentCapabilityRefs,
				resource_refs: resolveNextCapabilityIdList(
					currentCapabilityRefs.resource_refs,
					resourceId,
					enable,
				),
			});
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
		},
	});

	const templateToggleMutation = useMutation<
		unknown,
		unknown,
		{ templateId: string; enable: boolean }
	>({
		mutationFn: async ({ templateId, enable }) => {
			if (!identifier || !serverId) return null;
			const existingConfig = requireCapabilityConfig(
				await loadCapabilityConfig(),
			);
			const currentCapabilityRefs = getSelectedCapabilityRefs(existingConfig);
			await updateCapabilityRefs({
				...currentCapabilityRefs,
				template_refs: resolveNextCapabilityIdList(
					currentCapabilityRefs.template_refs,
					templateId,
					enable,
				),
			});
			return null;
		},
		onSuccess: () => {
			invalidateDirectQueries();
			void refetchCapabilities();
		},
	});

	const bulkCapabilitiesMutation = useMutation<
		unknown,
		unknown,
		{ enable: boolean; ids: string[] }
	>({
		mutationFn: async ({ enable, ids }) => {
			if (!identifier || !serverId) return null;
			const existingConfig = requireCapabilityConfig(
				await loadCapabilityConfig(),
			);
			const currentCapabilityRefs = getSelectedCapabilityRefs(existingConfig);
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

			const resolveBulkIds = (
				currentIds: string[] = [],
				selectedIds: string[],
			) => {
				const selectedIdSet = new Set(selectedIds);
				const remainingIds = currentIds.filter((id) => !selectedIdSet.has(id));
				return enable
					? normalizeCapabilityIds([...remainingIds, ...selectedIds])
					: remainingIds;
			};

			await updateCapabilityRefs({
				tool_refs: resolveBulkIds(
					currentCapabilityRefs.tool_refs,
					grouped.tools,
				),
				prompt_refs: resolveBulkIds(
					currentCapabilityRefs.prompt_refs,
					grouped.prompts,
				),
				resource_refs: resolveBulkIds(
					currentCapabilityRefs.resource_refs,
					grouped.resources,
				),
				template_refs: resolveBulkIds(
					currentCapabilityRefs.template_refs,
					grouped.templates,
				),
			});
			return null;
		},
		onSuccess: () => {
			capabilityBulk.clearSelection();
			capabilityBulk.exitBulkMode();
			invalidateDirectQueries();
			void refetchCapabilities();
			notifySuccess(
				t("clients:detail.directExposure.notifications.savedTitle", {
					defaultValue: "Direct capabilities updated",
				}),
				t("clients:detail.directExposure.notifications.savedMessage", {
					defaultValue: "The direct capability exposure list has been updated.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("clients:detail.directExposure.notifications.saveFailedTitle", {
					defaultValue: "Unable to update direct capabilities",
				}),
				String(error),
			);
		},
	});

	const capabilityBulkActions = useEnableDisableBulkActions(
		capabilityBulk,
		visibleCapabilityKeys,
		bulkCapabilitiesMutation,
	);

	const renderDirectFlatCapabilityList = useCallback(
		(items: CapabilityPreviewFlatItem[]) => {
			const flatItems: DirectFlatCapabilityItem[] = items.map(
				({ kind, item }) => ({
					...item,
					__directCapabilityKind: kind,
				}),
			);

			return (
				<CapabilityList<DirectFlatCapabilityItem>
					asCard={false}
					kind="tools"
					getKind={(item) => item.__directCapabilityKind}
					context="profile"
					leadingIcon="kind"
					items={flatItems}
					scrollContainedBody
					enableToggle
					getId={(item) =>
						capabilityKey(item.__directCapabilityKind, item.id)
					}
					getEnabled={(item) => !!item.enabled}
					getItemClassName={(item) =>
						reviewRefId && item.id === reviewRefId
							? "border border-dashed border-amber-500 bg-amber-50/60 dark:border-amber-600 dark:bg-amber-950/30"
							: undefined
					}
					onToggle={(_, next, item) => {
						if (item.__directCapabilityKind === "tools") {
							toolToggleMutation.mutate({
								toolId: item.id,
								enable: next,
							});
							return;
						}
						if (item.__directCapabilityKind === "resources") {
							resourceToggleMutation.mutate({
								resourceId: item.id,
								enable: next,
							});
							return;
						}
						if (item.__directCapabilityKind === "prompts") {
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
					emptyText={t("clients:detail.directExposure.empty.tools", {
						defaultValue: "No capabilities found for this server.",
					})}
					selectable={capabilityBulk.isBulkMode}
					selectedIds={capabilityBulk.selectedIds}
					onSelectToggle={(id) => capabilityBulk.toggleItem(id)}
					loadDetails={loadCapabilityDetails}
					detailsCacheScope={directCapabilityDetailsCacheToken(flatItems)}
				/>
			);
		},
		[
			capabilityBulk,
			loadCapabilityDetails,
			promptToggleMutation,
			resourceToggleMutation,
			reviewRefId,
			t,
			templateToggleMutation,
			toolToggleMutation,
		],
	);

	const panelTitle =
		serverDetails?.name ??
		serverId ??
		t("clients:detail.directExposure.title", {
			defaultValue: "Capability Level",
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
						<div className="min-w-0 flex-1">
							<div className="flex items-center gap-2">
								<CardTitle>{panelTitle}</CardTitle>
								<Badge variant="outline">
									{t("clients:detail.directExposure.badge", {
										defaultValue: "Direct Exposure",
									})}
								</Badge>
							</div>
							<CardDescription>
								{serverDetails?.meta?.description ||
									t("clients:detail.directExposure.serverDescriptionFallback", {
										defaultValue:
											"Choose which capabilities from this server should be exposed directly to the client.",
									})}
							</CardDescription>
						</div>
					</div>
				</CardHeader>
			</Card>

				{isCapabilitiesError || capabilityFailureState === "complete" ? (
					<Alert variant="destructive">
						<AlertTitle>
							{t("clients:detail.directExposure.loadFailed", {
								defaultValue: "Unable to load capabilities",
							})}
						</AlertTitle>
						<AlertDescription className="flex flex-wrap items-center justify-between gap-3">
							<span>
								{capabilitiesError instanceof Error
									? capabilitiesError.message
									: t("clients:detail.directExposure.loadFailedDescription", {
											defaultValue:
												"No capability kind could be loaded. Open logs for diagnostic details.",
										})}
							</span>
							<Button type="button" size="sm" variant="outline" onClick={openServerLogs}>
								{t("servers:detail.capabilityList.viewLogs", {
									defaultValue: "View logs",
								})}
							</Button>
						</AlertDescription>
					</Alert>
			) : null}

			{capabilityFailureState === "partial" ? (
				<Alert className="border-amber-300 bg-amber-50 text-amber-950 dark:border-amber-700 dark:bg-amber-950/30 dark:text-amber-100">
					<AlertTriangle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
					<AlertTitle>
						{t("servers:detail.capabilityList.partialFailure", {
							defaultValue: "Some capabilities could not be discovered",
						})}
					</AlertTitle>
						<AlertDescription className="flex flex-wrap items-center justify-between gap-3">
							<span>
								{t("servers:detail.capabilityList.partialFailureDescription", {
									defaultValue:
										"Capability discovery was incomplete. Open logs for diagnostic details.",
								})}
							</span>
							<Button type="button" size="sm" variant="outline" onClick={openServerLogs}>
								{t("servers:detail.capabilityList.viewLogs", {
									defaultValue: "View logs",
								})}
							</Button>
						</AlertDescription>
				</Alert>
			) : null}

			{authenticationNotice ? (
				<Alert variant="destructive">
					<AlertTitle>
						{t(authenticationNotice.titleKey, {
							defaultValue: authenticationNotice.titleDefault,
						})}
					</AlertTitle>
					<AlertDescription>
						{t(authenticationNotice.descriptionKey, {
							defaultValue: authenticationNotice.descriptionDefault,
						})}
					</AlertDescription>
				</Alert>
			) : null}

			<Card className={CAPABILITY_SCROLL_CARD_CLASS}>
				<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
					<CapabilityManagementPanel
						headerVariant="toolbar-inline"
						isBulkMode={capabilityBulk.isBulkMode}
						onToggleBulkMode={capabilityBulk.toggleMode}
						bulkActions={capabilityBulkActions}
						searchValue={capabilityQuery}
						onSearchChange={setCapabilityQuery}
						searchPlaceholder={t("servers:wizard.preview.filterCapabilities", {
							defaultValue: "Filter capabilities...",
						})}
						kindFilter={{
							label: capabilityKindFilterLabel,
							allLabel: t("servers:detail.filters.kind.all", {
								defaultValue: "All Types",
							}),
							options: capabilityKindFilterOptions,
							selectedValues: capabilityKindFilters,
							onClear: clearCapabilityKindFilters,
							onToggle: (value, checked) =>
								toggleCapabilityKindFilter(value as CapabilityKind, checked),
						}}
						statusFilter={{
							label: capabilityStatusLabel,
							value: capabilityStatus,
							placeholder: t("clients:detail.directExposure.statusPlaceholder", {
								defaultValue: "Status",
							}),
							options: [
								{
									value: "all",
									label: t("clients:detail.directExposure.filters.status.all", {
										defaultValue: "All",
									}),
								},
								{
									value: "enabled",
									label: t(
										"clients:detail.directExposure.filters.status.enabled",
										{ defaultValue: "Enabled" },
									),
								},
								{
									value: "disabled",
									label: t(
										"clients:detail.directExposure.filters.status.disabled",
										{ defaultValue: "Disabled" },
									),
								},
							],
							onValueChange: (value) =>
								setCapabilityStatus(value as CapabilityStatusFilter),
						}}
						hasSource={Boolean(serverId)}
						isLoading={isLoadingCapabilities || isLoadingServer}
						tools={filteredTools as CapabilityRecord[]}
						resources={filteredResources as CapabilityRecord[]}
						prompts={filteredPrompts as CapabilityRecord[]}
						templates={filteredTemplates as CapabilityRecord[]}
						kindFilters={capabilityKindFilters}
						emptyText={t("clients:detail.directExposure.empty.tools", {
							defaultValue: "No capabilities found for this server.",
						})}
						emptySearchText={t("servers:detail.capabilityList.emptyAll", {
							defaultValue: "No capabilities from this server",
						})}
						renderFlatList={renderDirectFlatCapabilityList}
					/>
				</CardContent>
			</Card>

			<SurfaceReviewDialog
				reviewItemId={reviewItemId}
				open={!!reviewItemId}
				onOpenChange={(open) => {
					if (open) return;
					const next = new URLSearchParams(searchParams);
					next.delete("review_item");
					next.delete("ref_id");
					setSearchParams(next, { replace: true });
				}}
			/>
		</div>
	);
}
