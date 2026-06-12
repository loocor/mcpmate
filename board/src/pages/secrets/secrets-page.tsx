import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	KeyRound,
	Plus,
	RefreshCw,
	ShieldAlert,
} from "lucide-react";
import { useEffect, useMemo, useState, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { ListGridContainer } from "../../components/list-grid-container";
import {
	EmptyState,
	FullHeightEmptyStateCard,
	PageLayout,
} from "../../components/page-layout";
import { StatsCards } from "../../components/stats-cards";
import type { StatCardData } from "../../components/stats-cards";
import { Pagination } from "../../components/pagination";
import { ErrorDisplay } from "../../components/error-display";
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
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { PageToolbar } from "../../components/ui/page-toolbar";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import type {
	Entity,
	PageToolbarCallbacks,
	PageToolbarConfig,
	PageToolbarState,
} from "../../components/ui/page-toolbar";
import { PageLockScreen } from "../../components/lock-screen";
import {
	SecretEditorDrawer,
	SecretCatalogEntry,
	SecretStoreIssueAlert,
	buildCreateEditorStateFromOrigin,
	defaultSecretEditorState,
	originFromSearchParams,
	SECRET_KIND_VALUES,
	stripOriginSearchParams,
	useSecretEditorKindOptions,
	type SecretEditorState,
} from "../../components/secrets";
import { secretsApi, serversApi } from "../../lib/api";
import { requiresEncryptionUnlock } from "../../lib/protection-password";
import {
	classifySecretLifecycle,
	secretHasCleanupAvailable,
	type SecretLifecycleFilter,
	type SecretLifecycleState,
} from "../../lib/secret-lifecycle";
import { useSecretStoreProviderRetryMutation } from "../../lib/hooks/use-secret-store-provider-retry";
import {
	invalidateSecretStoreCatalog,
	invalidateSecretStoreData,
	useSecretStoreStatusQuery,
} from "../../lib/hooks/use-secret-store-status";
import { useSecretsTranslations } from "../../lib/hooks/use-secrets-translations";
import { useUrlView } from "../../lib/hooks/use-url-state";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { SecretMetadata } from "../../lib/types";

const DEFAULT_PAGE_SIZE = 10;

type SecretToolbarEntity = Entity & {
	alias: string;
	kind: string;
	provider_kind: string;
	used_by_count: number;
	historical_usage_count: number;
	version: number;
};

const SECRET_LIFECYCLE_FILTERS: SecretLifecycleFilter[] = [
	"all",
	"active",
	"cleanup_available",
	"unused",
	"oauth_managed",
];

function getSecretDisplay(secret: SecretMetadata) {
	const label = secret.label?.trim();
	return {
		title: label || secret.alias,
		secondary: label ? secret.alias : null,
	};
}

function buildEditEditorState(secret: SecretMetadata): SecretEditorState {
	return {
		mode: "edit",
		alias: secret.alias,
		kind: secret.kind,
		label: secret.label ?? "",
		value: "",
		origin: secret.origin ?? null,
	};
}

export function SecretsPage() {
	const { t, i18n } = useSecretsTranslations();
	const queryClient = useQueryClient();
	const [searchParams, setSearchParams] = useSearchParams();
	const [editor, setEditor] = useState<SecretEditorState | null>(null);
	const [editorInitialTab, setEditorInitialTab] = useState<"general" | "usage">("general");
	const [deleteTarget, setDeleteTarget] = useState<SecretMetadata | null>(null);
	const [expanded, setExpanded] = useState(false);
	const [sortedSecrets, setSortedSecrets] = useState<SecretToolbarEntity[]>([]);
	const [currentPage, setCurrentPage] = useState(1);
	const [itemsPerPage, setItemsPerPage] = useState(DEFAULT_PAGE_SIZE);

	const storedDefaultView = useAppStore(
		(state) => state.dashboardSettings.defaultView,
	);
	const setDashboardSetting = useAppStore((state) => state.setDashboardSetting);
	const { view } = useUrlView({
		paramName: "view",
		defaultView: storedDefaultView,
		validViews: ["grid", "list"],
	});

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
		staleTime: 30_000,
	});
	const editorAlias = editor?.mode === "edit" ? editor.alias : null;
	const serversQuery = useQuery({
		queryKey: ["servers"],
		queryFn: serversApi.getAll,
		staleTime: 30_000,
		enabled: Boolean(editorAlias),
	});
	const usagesQuery = useQuery({
		queryKey: ["secrets", "usages", editorAlias],
		queryFn: () => secretsApi.listUsages(editorAlias ?? ""),
		enabled: Boolean(editorAlias),
	});
	const storeStatusQuery = useSecretStoreStatusQuery();
	const storeReady = storeStatusQuery.data?.status === "ready";
	const needsEncryptionUnlock = requiresEncryptionUnlock(storeStatusQuery.data);

	const providerRetryMutation = useSecretStoreProviderRetryMutation(t, {
		invalidateCatalog: true,
	});

	const serverNameById = useMemo(() => {
		const map = new Map<string, string>();
		for (const server of serversQuery.data?.servers ?? []) {
			const name = server.name?.trim();
			map.set(server.id, name && name.length > 0 ? name : server.id);
		}
		return map;
	}, [serversQuery.data]);

	const handleEncryptionUnlock = async () => {
		await invalidateSecretStoreData(queryClient, { catalog: true });
	};

	const kindOptions = useMemo(
		() =>
			SECRET_KIND_VALUES.map((value) => ({
				value,
				label: t(`kind.${value}`, { defaultValue: value }),
			})),
		[t, i18n.language],
	);
	const kindLabelByKind = useMemo(
		() => new Map(kindOptions.map((option) => [option.value, option.label])),
		[kindOptions],
	);

	const providerLabel = useCallback(
		(providerKind: string): string =>
			t(`provider.${providerKind}`, { defaultValue: providerKind }),
		[t, i18n.language],
	);

	const catalogStatsLabels = useMemo(
		() => ({
			provider: t("list.stats.provider", { defaultValue: "Provider" }),
			usage: t("list.stats.usage", { defaultValue: "Usage" }),
			history: t("list.stats.history", { defaultValue: "History" }),
			version: t("list.stats.version", { defaultValue: "Version" }),
		}),
		[t, i18n.language],
	);

	const editorKindOptions = useSecretEditorKindOptions(editor);

	const lifecycleFilter = useMemo<SecretLifecycleFilter>(() => {
		const raw = searchParams.get("lifecycle");
		return SECRET_LIFECYCLE_FILTERS.includes(raw as SecretLifecycleFilter)
			? (raw as SecretLifecycleFilter)
			: "all";
	}, [searchParams]);

	const setLifecycleFilter = (value: SecretLifecycleFilter) => {
		const next = new URLSearchParams(searchParams);
		if (value === "all") {
			next.delete("lifecycle");
		} else {
			next.set("lifecycle", value);
		}
		setSearchParams(next, { replace: true });
	};

	const lifecycleLabel = useCallback(
		(state: SecretLifecycleState | "all"): string =>
			t(`lifecycle.state.${state}`, { defaultValue: state.replace(/_/g, " ") }),
		[t, i18n.language],
	);

	const lifecycleByAlias = useMemo(() => {
		const map = new Map<string, ReturnType<typeof classifySecretLifecycle>>();
		for (const secret of secretsQuery.data ?? []) {
			map.set(secret.alias, classifySecretLifecycle(secret));
		}
		return map;
	}, [secretsQuery.data]);

	const filteredSecrets = useMemo(() => {
		const secrets = secretsQuery.data ?? [];
		if (lifecycleFilter === "all") return secrets;
		return secrets.filter(
			(secret) => lifecycleByAlias.get(secret.alias)?.state === lifecycleFilter,
		);
	}, [lifecycleByAlias, lifecycleFilter, secretsQuery.data]);

	const secretsAsEntities = useMemo<SecretToolbarEntity[]>(() => {
		return filteredSecrets.map((secret) => ({
			id: secret.alias,
			name: secret.alias,
			description: secret.label ?? secret.placeholder,
			alias: secret.alias,
			kind: secret.kind,
			provider_kind: secret.provider_kind,
			used_by_count: secret.used_by_count,
			historical_usage_count: secret.historical_usage_count,
			version: secret.version,
		}));
	}, [filteredSecrets]);

	const secretsByAlias = useMemo(
		() =>
			new Map(
				(secretsQuery.data ?? []).map((secret) => [secret.alias, secret]),
			),
		[secretsQuery.data],
	);
	const secretsByAliasRef = useRef(secretsByAlias);
	secretsByAliasRef.current = secretsByAlias;

	const totalPages = Math.max(
		1,
		Math.ceil(sortedSecrets.length / itemsPerPage),
	);
	const pagedSecrets = useMemo(() => {
		const start = (currentPage - 1) * itemsPerPage;
		return sortedSecrets.slice(start, start + itemsPerPage);
	}, [currentPage, itemsPerPage, sortedSecrets]);
	const secretDisplayByAlias = useMemo(() => {
		const map = new Map<string, ReturnType<typeof getSecretDisplay>>();
		for (const secret of secretsQuery.data ?? []) {
			map.set(secret.alias, getSecretDisplay(secret));
		}
		return map;
	}, [secretsQuery.data]);
	const catalogRows = useMemo(
		() =>
			pagedSecrets
				.filter((entity) => secretsByAlias.has(entity.alias) && lifecycleByAlias.has(entity.alias))
				.map((entity) => {
					const secret = secretsByAlias.get(entity.alias)!;
					const lifecycle = lifecycleByAlias.get(entity.alias)!;
					return {
						secret,
						display: secretDisplayByAlias.get(secret.alias)!,
						kindLabel:
							kindLabelByKind.get(secret.kind) ?? secret.kind,
						lifecycleState: lifecycle.state,
						providerLabel: providerLabel(secret.provider_kind),
					};
				}),
		[
			kindLabelByKind,
			lifecycleByAlias,
			pagedSecrets,
			providerLabel,
			secretsByAlias,
			secretDisplayByAlias,
		],
	);
	const hasNoSecretRecords = (secretsQuery.data?.length ?? 0) === 0;

	useEffect(() => {
		if (currentPage > totalPages) {
			setCurrentPage(totalPages);
		}
	}, [currentPage, totalPages]);

	useEffect(() => {
		if (searchParams.get("editor") !== "create") return;
		if (!storeReady) return;

		const origin = originFromSearchParams(searchParams);
		const suggestedFromUrl = searchParams.get("suggested_alias")?.trim() ?? "";
		const existingAliases = (secretsQuery.data ?? []).map((secret) => secret.alias);

		if (origin) {
			const nextEditor = buildCreateEditorStateFromOrigin(
				origin,
				existingAliases,
				(key, defaultValue) => t(key, { defaultValue }),
			);
			if (suggestedFromUrl) {
				nextEditor.alias = suggestedFromUrl;
			}
			setEditor(nextEditor);
		} else {
			setEditor({
				...defaultSecretEditorState(),
				alias: suggestedFromUrl,
			});
		}

		const next = new URLSearchParams(searchParams);
		next.delete("editor");
		next.delete("suggested_alias");
		stripOriginSearchParams(next);
		setSearchParams(next, { replace: true });
	}, [searchParams, secretsQuery.data, setSearchParams, storeReady, t]);

	useEffect(() => {
		const alias = searchParams.get("secret")?.trim();
		if (!alias || !secretsQuery.data) return;
		if (!storeReady) return;

		const secret = secretsByAlias.get(alias);
		if (!secret) return;

		setEditorInitialTab(searchParams.get("tab") === "usage" ? "usage" : "general");
		setEditor(buildEditEditorState(secret));

		const next = new URLSearchParams(searchParams);
		next.delete("secret");
		next.delete("tab");
		setSearchParams(next, { replace: true });
	}, [searchParams, secretsByAlias, secretsQuery.data, setSearchParams, storeReady]);

	const saveMutation = useMutation({
		mutationFn: async (state: SecretEditorState) => {
			if (!storeReady) {
				throw new Error("Secret store is not available");
			}
			if (state.mode === "create") {
				return secretsApi.create({
					alias: state.alias.trim(),
					kind: state.kind,
					label: state.label.trim() || null,
					value: state.value,
					origin: state.origin,
				});
			}
			return secretsApi.update({
				alias: state.alias.trim(),
				label: state.label.trim() || null,
				value: state.value.length > 0 ? state.value : undefined,
			});
		},
		onSuccess: async () => {
			setEditor(null);
			await invalidateSecretStoreCatalog(queryClient);
			notifySuccess(
				t("notifications.saveSuccess", { defaultValue: "Secret saved" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("notifications.saveError", { defaultValue: "Failed to save secret" }),
				stringifyError(error),
			);
		},
	});

	const deleteMutation = useMutation({
		mutationFn: (alias: string) => {
			if (!storeReady) {
				throw new Error("Secret store is not available");
			}
			return secretsApi.delete(alias);
		},
		onSuccess: async () => {
			setDeleteTarget(null);
			setEditor(null);
			await invalidateSecretStoreCatalog(queryClient);
			notifySuccess(
				t("notifications.deleteSuccess", { defaultValue: "Secret deleted" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("notifications.deleteError", {
					defaultValue: "Failed to delete secret",
				}),
				stringifyError(error),
			);
		},
	});

	const openCreate = useCallback(() => {
		if (!storeReady) {
			return;
		}
		setEditorInitialTab("general");
		setEditor(defaultSecretEditorState());
	}, [storeReady]);
	const openEdit = useCallback(
		(secret: SecretMetadata, tab: "general" | "usage" = "general") => {
			setEditorInitialTab(tab);
			setEditor(buildEditEditorState(secret));
		},
		[],
	);
	const handleCatalogOpen = useCallback(
		(alias: string) => {
			if (!storeReady) {
				return;
			}
			const secret = secretsByAliasRef.current.get(alias);
			if (secret) {
				openEdit(secret);
			}
		},
		[openEdit, storeReady],
	);
	const handleCatalogViewUsage = useCallback(
		(alias: string) => {
			if (!storeReady) {
				return;
			}
			const secret = secretsByAliasRef.current.get(alias);
			if (secret) {
				openEdit(secret, "usage");
			}
		},
		[openEdit, storeReady],
	);
	const { refetch: refetchSecretsList } = secretsQuery;
	const { refetch: refetchStoreStatus } = storeStatusQuery;
	const refreshSecretsPage = useCallback(() => {
		void Promise.all([refetchSecretsList(), refetchStoreStatus()]);
	}, [refetchSecretsList, refetchStoreStatus]);
	const isRefreshing =
		secretsQuery.isRefetching || storeStatusQuery.isRefetching;
	const viewUsageLabel = t("list.actions.viewUsage", {
		defaultValue: "View usage",
	});
	const closeEditor = () => {
		setEditor(null);
		setEditorInitialTab("general");
	};
	const navigate = useNavigate();
	const handleNavigateToServer = useCallback(
		(serverId: string) => {
			closeEditor();
			navigate(`/servers/${encodeURIComponent(serverId)}`);
		},
		[navigate],
	);

	const editorMeta = useMemo(() => {
		if (!editor || editor.mode !== "edit") {
			return { placeholder: undefined, usedByCount: undefined };
		}
		const secret = secretsByAlias.get(editor.alias);
		return {
			placeholder: secret?.placeholder,
			usedByCount: secret?.used_by_count,
		};
	}, [editor, secretsByAlias]);

	const statsCards = useMemo((): StatCardData[] => {
		const secrets = secretsQuery.data ?? [];
		const inUseCount = secrets.filter((secret) => secret.used_by_count > 0).length;
		const cleanupCount = [...lifecycleByAlias.values()].filter(
			secretHasCleanupAvailable,
		).length;
		const storeStatus = storeStatusQuery.data;

		let storeValue: string | number = "—";
		let storeDescription = t("stats.store.checking", {
			defaultValue: "checking status",
		});
		if (storeStatus) {
			if (storeStatus.status === "ready") {
				storeValue = t("stats.store.ready", { defaultValue: "Ready" });
				storeDescription = t("stats.store.readyDescription", {
					defaultValue: "available for use",
				});
			} else if (storeStatus.issue?.reason_code === "passphrase_unlock_required") {
				storeValue = t("stats.store.locked", { defaultValue: "Locked" });
				storeDescription = t("stats.store.lockedDescription", {
					defaultValue: "unlock required",
				});
			} else {
				storeValue = t("stats.store.issue", { defaultValue: "Issue" });
				storeDescription = t("stats.store.issueDescription", {
					defaultValue: "needs attention",
				});
			}
		}

		return [
			{
				title: t("stats.stored.title", { defaultValue: "Stored Secrets" }),
				value: secretsQuery.isLoading ? "—" : secrets.length,
				description: t("stats.stored.description", {
					defaultValue: "in secure store",
				}),
			},
			{
				title: t("stats.inUse.title", { defaultValue: "In Use" }),
				value: secretsQuery.isLoading ? "—" : inUseCount,
				description: t("stats.inUse.description", {
					defaultValue: "linked to servers",
				}),
			},
			{
				title: t("stats.cleanup.title", { defaultValue: "Cleanup" }),
				value: secretsQuery.isLoading ? "—" : cleanupCount,
				description: t("stats.cleanup.description", {
					defaultValue: "ready to review",
				}),
			},
			{
				title: t("stats.store.title", { defaultValue: "Secure Store" }),
				value: storeValue,
				description: storeDescription,
			},
		];
	}, [
		lifecycleByAlias,
		secretsQuery.data,
		secretsQuery.isLoading,
		storeStatusQuery.data,
		t,
		i18n.language,
	]);

	const filters = (
		<Select
			value={lifecycleFilter}
			onValueChange={(value) =>
				setLifecycleFilter(value as SecretLifecycleFilter)
			}
		>
			<SelectTrigger className="h-9 w-[178px]">
				<SelectValue />
			</SelectTrigger>
			<SelectContent align="end">
				{SECRET_LIFECYCLE_FILTERS.map((filter) => (
					<SelectItem key={filter} value={filter}>
						{lifecycleLabel(filter)}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);

	const toolbarConfig = useMemo<PageToolbarConfig<SecretToolbarEntity>>(
		() => ({
			data: secretsAsEntities,
			search: {
				placeholder: t("toolbar.search.placeholder", {
					defaultValue: "Search secrets...",
				}),
				fields: [
					{
						key: "alias",
						label: t("toolbar.search.fields.alias", { defaultValue: "Alias" }),
						weight: 10,
					},
					{
						key: "description",
						label: t("toolbar.search.fields.label", { defaultValue: "Label" }),
						weight: 8,
					},
					{
						key: "kind",
						label: t("toolbar.search.fields.kind", { defaultValue: "Kind" }),
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
						value: "alias",
						label: t("toolbar.sort.options.alias", { defaultValue: "Alias" }),
						defaultDirection: "asc",
					},
					{
						value: "kind",
						label: t("toolbar.sort.options.kind", { defaultValue: "Kind" }),
						defaultDirection: "asc",
					},
					{
						value: "used_by_count",
						label: t("toolbar.sort.options.usage", { defaultValue: "Usage" }),
						defaultDirection: "desc",
					},
				],
				defaultSort: "alias",
			},
			urlPersistence: {
				enabled: true,
			},
		}),
		[secretsAsEntities, storedDefaultView, t, i18n.language],
	);

	const toolbarState: PageToolbarState = {
		expanded,
	};

	const handleViewModeChange = useCallback(
		(mode: "grid" | "list") => {
			setDashboardSetting("defaultView", mode);
		},
		[setDashboardSetting],
	);

	const handleSortedDataChange = useCallback((data: SecretToolbarEntity[]) => {
		setSortedSecrets(data);
		setCurrentPage(1);
	}, []);

	const toolbarCallbacks = useMemo<PageToolbarCallbacks<SecretToolbarEntity>>(
		() => ({
			onViewModeChange: handleViewModeChange,
			onSortedDataChange: handleSortedDataChange,
			onExpandedChange: setExpanded,
		}),
		[handleSortedDataChange, handleViewModeChange],
	);

	const loadingSkeleton =
		view === "grid"
			? Array.from({ length: 6 }, (_, index) => (
				<Card key={`secret-grid-skeleton-${index}`} className="overflow-hidden">
					<CardContent className="space-y-3 p-4">
						<div className="flex items-center gap-3">
							<div className="h-12 w-12 animate-pulse rounded-[10px] bg-slate-200 dark:bg-slate-800" />
							<div className="flex-1 space-y-2">
								<div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
								<div className="h-4 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
							</div>
						</div>
						<div className="h-8 w-full animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
					</CardContent>
				</Card>
			))
			: Array.from({ length: 3 }, (_, index) => (
				<div
					key={`secret-list-skeleton-${index}`}
					className="flex items-center justify-between rounded-lg border border-slate-200 bg-white px-4 py-4 dark:border-slate-700 dark:bg-slate-900"
				>
					<div className="flex min-w-0 items-center gap-3">
						<div className="h-9 w-9 animate-pulse rounded-md bg-slate-200 dark:bg-slate-800" />
						<div className="space-y-2">
							<div className="h-4 w-40 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
							<div className="h-3 w-64 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
						</div>
					</div>
					<div className="h-9 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
				</div>
			));

	const emptyStateAction = hasNoSecretRecords ? (
		<Button type="button" size="sm" className="mt-4 h-9" onClick={openCreate} disabled={!storeReady}>
			<Plus className="mr-2 h-4 w-4" />
			{t("empty.action", { defaultValue: "Add First Secret" })}
		</Button>
	) : undefined;

	const emptyState = (
		<FullHeightEmptyStateCard>
			<EmptyState
				icon={<KeyRound className="h-12 w-12" />}
				title={
					hasNoSecretRecords
						? t("empty.title", { defaultValue: "No secrets stored" })
						: t("empty.filteredTitle", {
							defaultValue: "No matching secrets",
						})
				}
				description={
					hasNoSecretRecords
						? t("empty.description", {
							defaultValue:
								"Store write-only values for server runtime placeholders.",
						})
						: lifecycleFilter !== "all"
							? t("empty.filteredLifecycleDescription", {
								defaultValue:
									"Adjust the lifecycle filter or search controls to find a secret.",
							})
							: t("empty.filteredDescription", {
								defaultValue:
									"Adjust the search or sort controls to find a secret.",
							})
				}
				action={emptyStateAction}
			/>
		</FullHeightEmptyStateCard>
	);

	const actions = (
		<div className="flex items-center gap-2">
			<Button
				type="button"
				variant="outline"
				size="sm"
				className="h-9 w-9 p-0"
				onClick={refreshSecretsPage}
				disabled={isRefreshing}
				title={t("toolbar.actions.refresh", { defaultValue: "Refresh" })}
			>
				<RefreshCw
					className={`h-4 w-4 ${isRefreshing ? "animate-spin" : ""}`}
				/>
			</Button>
			<Button
				type="button"
				size="sm"
				className="h-9 w-9 p-0"
				onClick={openCreate}
				disabled={!storeReady}
				title={t("toolbar.actions.add", { defaultValue: "Add Secret" })}
			>
				<Plus className="h-4 w-4" />
			</Button>
		</div>
	);

	if (storeStatusQuery.isSuccess && needsEncryptionUnlock) {
		return (
			<PageLockScreen variant="encryption" onSuccess={handleEncryptionUnlock} />
		);
	}

	return (
		<PageLayout
			title={t("title", { defaultValue: "Secure Store" })}
			className="flex h-full min-h-0 flex-col"
			headerActions={
				<PageToolbar<SecretToolbarEntity>
					config={toolbarConfig}
					state={toolbarState}
					callbacks={toolbarCallbacks}
					actions={actions}
					filters={filters}
				/>
			}
			statsCards={<StatsCards cards={statsCards} />}
		>
			<div className="flex min-h-0 flex-1 flex-col gap-4">
				{storeStatusQuery.isError ? (
					<ErrorDisplay
						icon={ShieldAlert}
						title={t("status.error.title", {
							defaultValue: "Store status check failed",
						})}
						error={
							storeStatusQuery.error instanceof Error
								? storeStatusQuery.error
								: t("status.error.description", {
									defaultValue:
										"Could not determine store status. Operations are disabled.",
								})
						}
						onRetry={() => void storeStatusQuery.refetch()}
						retryLabel={t("list.retry", { defaultValue: "Retry" })}
					/>
				) : null}
				{storeStatusQuery.isSuccess && !storeReady && storeStatusQuery.data ? (
					<SecretStoreIssueAlert
						status={storeStatusQuery.data}
						isRetrying={
							providerRetryMutation.isPending || storeStatusQuery.isFetching
						}
						onRetryStatus={() => void storeStatusQuery.refetch()}
						onRetryProvider={(mode) => providerRetryMutation.mutate(mode)}
					/>
				) : null}
				{secretsQuery.isError ? (
					<div className="flex flex-col items-center justify-center gap-3 py-12 text-center">
						<p className="text-sm text-destructive">
							{t("list.error", {
								defaultValue: "Failed to load secrets. The secure store may be unavailable.",
							})}
						</p>
						<Button variant="outline" size="sm" onClick={refreshSecretsPage}>
							<RefreshCw className="mr-2 h-4 w-4" />
							{t("list.retry", { defaultValue: "Retry" })}
						</Button>
					</div>
				) : (
					<div className="flex min-h-0 flex-1 flex-col gap-4">
						<ListGridContainer
							loading={secretsQuery.isLoading}
							loadingSkeleton={loadingSkeleton}
							emptyClassName="h-full"
							emptyState={sortedSecrets.length === 0 ? emptyState : undefined}
						>
							{pagedSecrets.length === 0
								? null
								: catalogRows.map((row) =>
									view === "grid" ? (
										<SecretCatalogEntry
											key={row.secret.alias}
											variant="grid"
											secret={row.secret}
											display={row.display}
											kindLabel={row.kindLabel}
											lifecycleState={row.lifecycleState}
											providerLabel={row.providerLabel}
											statsLabels={catalogStatsLabels}
											onOpen={handleCatalogOpen}
										/>
									) : (
										<SecretCatalogEntry
											key={row.secret.alias}
											variant="list"
											secret={row.secret}
											display={row.display}
											kindLabel={row.kindLabel}
											lifecycleState={row.lifecycleState}
											providerLabel={row.providerLabel}
											statsLabels={catalogStatsLabels}
											viewUsageLabel={viewUsageLabel}
											onOpen={handleCatalogOpen}
											onViewUsage={handleCatalogViewUsage}
										/>
									),
								)}
						</ListGridContainer>
						{sortedSecrets.length > 0 ? (
							<Pagination
								currentPage={currentPage}
								hasPreviousPage={currentPage > 1}
								hasNextPage={currentPage < totalPages}
								itemsPerPage={itemsPerPage}
								currentPageItemCount={pagedSecrets.length}
								totalItemCount={sortedSecrets.length}
								totalPages={totalPages}
								onGoToPage={setCurrentPage}
								onPreviousPage={() =>
									setCurrentPage((page) => Math.max(1, page - 1))
								}
								onFirstPage={() => setCurrentPage(1)}
								onNextPage={() =>
									setCurrentPage((page) => Math.min(totalPages, page + 1))
								}
								onLastPage={() => setCurrentPage(totalPages)}
								onItemsPerPageChange={(next) => {
									setItemsPerPage(next);
									setCurrentPage(1);
								}}
								isLoading={secretsQuery.isRefetching}
							/>
						) : null}
					</div>
				)}
			</div>

			<SecretEditorDrawer
				editor={editor}
				kindOptions={editorKindOptions}
				onChange={setEditor}
				onClose={closeEditor}
				onSave={() => editor && saveMutation.mutate(editor)}
				onDelete={
					editorAlias
						? () => {
							const secret = (secretsQuery.data ?? []).find((s) => s.alias === editorAlias);
							if (secret) setDeleteTarget(secret);
						}
						: undefined
				}
				writesDisabled={!storeReady}
				isSaving={saveMutation.isPending}
				placeholder={editorMeta.placeholder}
				usages={usagesQuery.data ?? []}
				usagesLoading={Boolean(editorAlias) && usagesQuery.isLoading}
				usedByCount={editorMeta.usedByCount}
				serverNameById={serverNameById}
				initialTab={editorInitialTab}
				onNavigateToServer={handleNavigateToServer}
			/>
			<SecretDeleteDialog
				secret={deleteTarget}
				isDeleting={deleteMutation.isPending}
				onClose={() => setDeleteTarget(null)}
				onConfirm={() =>
					deleteTarget && deleteMutation.mutate(deleteTarget.alias)
				}
			/>
		</PageLayout>
	);
}

function SecretDeleteDialog({
	secret,
	isDeleting,
	onClose,
	onConfirm,
}: {
	secret: SecretMetadata | null;
	isDeleting: boolean;
	onClose: () => void;
	onConfirm: () => void;
}) {
	const { t } = useTranslation("secrets");
	const lifecycle = secret ? classifySecretLifecycle(secret) : null;
	const description = resolveDeleteDescription(t, lifecycle?.state);
	return (
		<AlertDialog
			open={Boolean(secret)}
			onOpenChange={(open) => !open && onClose()}
		>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>
						{t("delete.title", { defaultValue: "Delete secret?" })}
					</AlertDialogTitle>
					<AlertDialogDescription>{description}</AlertDialogDescription>
				</AlertDialogHeader>
				<div className="space-y-2 rounded-md bg-muted px-3 py-2 text-xs">
					<div className="font-mono">{secret?.alias}</div>
					{lifecycle ? (
						<div className="text-muted-foreground">
							{t("delete.usageSummary", {
								defaultValue: "Active {{active}} · Historical {{historical}}",
								active: lifecycle.activeCount,
								historical: lifecycle.historicalCount,
							})}
						</div>
					) : null}
				</div>
				<AlertDialogFooter>
					<AlertDialogCancel disabled={isDeleting}>
						{t("delete.actions.cancel", { defaultValue: "Cancel" })}
					</AlertDialogCancel>
					<AlertDialogAction
						disabled={isDeleting}
						onClick={onConfirm}
						className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
					>
						{t("delete.actions.confirm", { defaultValue: "Delete" })}
					</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}

function resolveDeleteDescription(
	t: ReturnType<typeof useTranslation<"secrets">>["t"],
	state?: SecretLifecycleState,
): string {
	switch (state) {
		case "active":
			return t("delete.descriptionActive", {
				defaultValue:
					"This secret is still actively used. Remove active bindings before deleting it.",
			});
		case "cleanup_available":
			return t("delete.descriptionHistorical", {
				defaultValue:
					"This removes the encrypted value. Historical usage metadata remains available without the secret value.",
			});
		case "oauth_managed":
			return t("delete.descriptionOAuth", {
				defaultValue:
					"OAuth-managed credentials are normally removed by OAuth revoke or server deletion. Delete only orphaned OAuth records.",
			});
		case "unused":
			return t("delete.descriptionUnused", {
				defaultValue:
					"This removes the encrypted value. No active or historical usage is recorded.",
			});
		default:
			return t("delete.description", {
				defaultValue:
					"This removes the encrypted value only when no active usage is recorded.",
			});
	}
}
