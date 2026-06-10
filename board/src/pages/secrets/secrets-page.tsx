import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	KeyRound,
	Plus,
	RefreshCw,
	ShieldAlert,
	ShieldCheck,
} from "lucide-react";
import { useEffect, useMemo, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { EntityCard } from "../../components/entity-card";
import { EntityListItem } from "../../components/entity-list-item";
import { ListGridContainer } from "../../components/list-grid-container";
import {
	EmptyState,
	FullHeightEmptyStateCard,
	PageLayout,
} from "../../components/page-layout";
import { StatsCards } from "../../components/stats-cards";
import type { StatCardData } from "../../components/stats-cards";
import { Pagination } from "../../components/pagination";
import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
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
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { PageToolbar } from "../../components/ui/page-toolbar";
import type {
	Entity,
	PageToolbarCallbacks,
	PageToolbarConfig,
	PageToolbarState,
} from "../../components/ui/page-toolbar";
import { LockScreen } from "../../components/lock-screen";
import {
	SecretEditorDrawer,
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
import { useUrlView } from "../../lib/hooks/use-url-state";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type {
	SecretKind,
	SecretMetadata,
} from "../../lib/types";

const DEFAULT_PAGE_SIZE = 10;

type SecretToolbarEntity = Entity & {
	alias: string;
	kind: string;
	provider_kind: string;
	used_by_count: number;
	version: number;
};

const defaultEditorState = defaultSecretEditorState;

function getSecretDisplay(secret: SecretMetadata) {
	const label = secret.label?.trim();
	return {
		title: label || secret.alias,
		secondary: label ? secret.alias : null,
	};
}

export function SecretsPage() {
	usePageTranslations("secrets");
	const { t } = useTranslation("secrets");
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
	const viewMode = view;

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
	});
	const serversQuery = useQuery({
		queryKey: ["servers"],
		queryFn: serversApi.getAll,
		staleTime: 30_000,
	});
	const serverNameById = useMemo(() => {
		const map = new Map<string, string>();
		for (const server of serversQuery.data?.servers ?? []) {
			const name = server.name?.trim();
			map.set(server.id, name && name.length > 0 ? name : server.id);
		}
		return map;
	}, [serversQuery.data]);
	const editorAlias = editor?.mode === "edit" ? editor.alias : null;
	const usagesQuery = useQuery({
		queryKey: ["secrets", "usages", editorAlias],
		queryFn: () => secretsApi.listUsages(editorAlias ?? ""),
		enabled: Boolean(editorAlias),
	});
	const storeStatusQuery = useQuery({
		queryKey: ["secrets", "status"],
		queryFn: secretsApi.status,
	});
	const storeReady = storeStatusQuery.data?.status === "ready";
	const needsEncryptionUnlock = requiresEncryptionUnlock(storeStatusQuery.data);

	const handleEncryptionUnlock = async () => {
		await queryClient.invalidateQueries({ queryKey: ["secrets", "status"] });
		await queryClient.invalidateQueries({ queryKey: ["secrets"] });
	};

	const kindOptions = SECRET_KIND_VALUES.map((value) => ({
		value,
		label: t(`kind.${value}`, { defaultValue: value }),
	}));

	const editorKindOptions = useSecretEditorKindOptions(editor);

	const kindLabel = (kind: string): string =>
		kindOptions.find((option) => option.value === kind)?.label ?? kind;

	const providerLabel = (providerKind: string): string =>
		t(`provider.${providerKind}`, { defaultValue: providerKind });

	const secretsAsEntities = useMemo<SecretToolbarEntity[]>(() => {
		const mapped = (secretsQuery.data ?? []).map((secret) => ({
			id: secret.alias,
			name: secret.alias,
			description: secret.label ?? secret.placeholder,
			alias: secret.alias,
			kind: secret.kind,
			provider_kind: secret.provider_kind,
			used_by_count: secret.used_by_count,
			version: secret.version,
		}));
		mapped.sort((left, right) => left.alias.localeCompare(right.alias));
		return mapped;
	}, [secretsQuery.data]);

	const secretsByAlias = useMemo(
		() =>
			new Map(
				(secretsQuery.data ?? []).map((secret) => [secret.alias, secret]),
			),
		[secretsQuery.data],
	);

	const totalPages = Math.max(
		1,
		Math.ceil(sortedSecrets.length / itemsPerPage),
	);
	const pagedSecrets = useMemo(() => {
		const start = (currentPage - 1) * itemsPerPage;
		return sortedSecrets.slice(start, start + itemsPerPage);
	}, [currentPage, itemsPerPage, sortedSecrets]);
	const hasNoSecretRecords = (secretsQuery.data?.length ?? 0) === 0;

	useEffect(() => {
		if (currentPage > totalPages) {
			setCurrentPage(totalPages);
		}
	}, [currentPage, totalPages]);

	useEffect(() => {
		if (searchParams.get("editor") !== "create") return;
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
				...defaultEditorState(),
				alias: suggestedFromUrl,
			});
		}

		const next = new URLSearchParams(searchParams);
		next.delete("editor");
		next.delete("suggested_alias");
		stripOriginSearchParams(next);
		setSearchParams(next, { replace: true });
	}, [searchParams, secretsQuery.data, setSearchParams, t]);

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
			await queryClient.invalidateQueries({ queryKey: ["secrets"] });
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
			await queryClient.invalidateQueries({ queryKey: ["secrets"] });
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

	const openCreate = () => {
		setEditorInitialTab("general");
		setEditor(defaultEditorState());
	};
	const openEdit = (secret: SecretMetadata, tab: "general" | "usage" = "general") => {
		setEditorInitialTab(tab);
		setEditor({
			mode: "edit",
			alias: secret.alias,
			kind: (secret.kind as SecretKind) || "generic",
			label: secret.label ?? "",
			value: "",
			origin: secret.origin ?? null,
		});
	};
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

	const editorPlaceholder = useMemo(() => {
		if (!editor || editor.mode !== "edit") {
			return undefined;
		}
		return secretsByAlias.get(editor.alias)?.placeholder;
	}, [editor, secretsByAlias]);

	const editorUsedByCount = useMemo(() => {
		if (!editor || editor.mode !== "edit") {
			return undefined;
		}
		return secretsByAlias.get(editor.alias)?.used_by_count;
	}, [editor, secretsByAlias]);

	const statsCards = useMemo((): StatCardData[] => {
		const secrets = secretsQuery.data ?? [];
		const inUseCount = secrets.filter((secret) => secret.used_by_count > 0).length;
		const usageRefs = secrets.reduce((sum, secret) => sum + secret.used_by_count, 0);
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
				title: t("stats.usageRefs.title", { defaultValue: "Usage References" }),
				value: secretsQuery.isLoading ? "—" : usageRefs,
				description: t("stats.usageRefs.description", {
					defaultValue: "runtime bindings",
				}),
			},
			{
				title: t("stats.store.title", { defaultValue: "Secure Store" }),
				value: storeValue,
				description: storeDescription,
			},
		];
	}, [
		secretsQuery.data,
		secretsQuery.isLoading,
		storeStatusQuery.data,
		t,
	]);

	const toolbarConfig: PageToolbarConfig<SecretToolbarEntity> = {
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
	};

	const toolbarState: PageToolbarState = {
		expanded,
	};

	const toolbarCallbacks: PageToolbarCallbacks<SecretToolbarEntity> = {
		onViewModeChange: (mode: "grid" | "list") => {
			setDashboardSetting("defaultView", mode);
		},
		onSortedDataChange: (data) => {
			setSortedSecrets(data);
			setCurrentPage(1);
		},
		onExpandedChange: setExpanded,
	};

	const loadingSkeleton =
		viewMode === "grid"
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
				onClick={() => void secretsQuery.refetch()}
				disabled={secretsQuery.isRefetching}
				title={t("toolbar.actions.refresh", { defaultValue: "Refresh" })}
			>
				<RefreshCw
					className={`h-4 w-4 ${secretsQuery.isRefetching ? "animate-spin" : ""}`}
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

	const renderSecretRow = (entity: SecretToolbarEntity) => {
		const secret = secretsByAlias.get(entity.alias);
		if (!secret) return null;
		const display = getSecretDisplay(secret);

		return (
			<EntityListItem
				key={secret.alias}
				id={secret.alias}
				title={display.title}
				description={
					<div className="min-w-0">
						{display.secondary ? (
							<div className="truncate font-mono text-xs">
								{display.secondary}
							</div>
						) : null}
						<div className="truncate font-mono text-xs text-muted-foreground">
							{secret.placeholder}
						</div>
					</div>
				}
				avatar={{
					fallback: secret.alias.slice(0, 2).toUpperCase(),
				}}
				titleBadges={[
					<Badge key="kind" variant="secondary">
						{kindLabel(secret.kind)}
					</Badge>,
				]}
				stats={[
					{
						label: t("list.stats.provider", { defaultValue: "Provider" }),
						value: providerLabel(secret.provider_kind),
						valueTitle: secret.provider_kind,
					},
					{
						label: t("list.stats.usage", { defaultValue: "Usage" }),
						value: secret.used_by_count,
					},
					{
						label: t("list.stats.version", { defaultValue: "Version" }),
						value: secret.version,
					},
				]}
				actionButtons={[
					<Button
						key="usage"
						type="button"
						variant="ghost"
						size="sm"
						className="h-9 px-2"
						onClick={() => openEdit(secret, "usage")}
						aria-label={t("list.actions.viewUsage", {
							defaultValue: "View usage",
						})}
					>
						<ShieldCheck className="mr-2 h-4 w-4" />
						{secret.used_by_count}
					</Button>,
				]}
				onClick={() => openEdit(secret)}
			/>
		);
	};

	const renderSecretCard = (entity: SecretToolbarEntity) => {
		const secret = secretsByAlias.get(entity.alias);
		if (!secret) return null;
		const display = getSecretDisplay(secret);

		return (
			<EntityCard
				key={secret.alias}
				id={secret.alias}
				title={display.title}
				description={
					<div className="min-w-0">
						{display.secondary ? (
							<div className="truncate font-mono text-xs">
								{display.secondary}
							</div>
						) : null}
						<div className="truncate font-mono text-xs text-muted-foreground">
							{secret.placeholder}
						</div>
					</div>
				}
				avatar={{
					fallback: secret.alias.slice(0, 2).toUpperCase(),
				}}
				avatarShape="rounded"
				topRightBadge={
					<Badge variant="secondary">{kindLabel(secret.kind)}</Badge>
				}
				stats={[
					{
						label: t("list.stats.provider", { defaultValue: "Provider" }),
						value: providerLabel(secret.provider_kind),
						valueTitle: secret.provider_kind,
					},
					{
						label: t("list.stats.usage", { defaultValue: "Usage" }),
						value: String(secret.used_by_count),
					},
					{
						label: t("list.stats.version", { defaultValue: "Version" }),
						value: String(secret.version),
					},
				]}
				onClick={() => openEdit(secret)}
			/>
		);
	};

	if (storeStatusQuery.isSuccess && needsEncryptionUnlock) {
		return <LockScreen variant="encryption" onSuccess={handleEncryptionUnlock} />;
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
				/>
			}
			statsCards={<StatsCards cards={statsCards} />}
		>
			<div className="flex min-h-0 flex-1 flex-col gap-4">
				{storeStatusQuery.isError && (
					<Alert variant="destructive">
						<ShieldAlert className="h-4 w-4" />
						<AlertTitle>
							{t("status.error.title", {
								defaultValue: "Store status check failed",
							})}
						</AlertTitle>
						<AlertDescription>
							{storeStatusQuery.error instanceof Error
								? storeStatusQuery.error.message
								: t("status.error.description", {
									defaultValue:
										"Could not determine store status. Operations are disabled.",
								})}
						</AlertDescription>
					</Alert>
				)}
				{storeStatusQuery.isSuccess && !storeReady && (
					<Alert variant="destructive">
						<ShieldAlert className="h-4 w-4" />
						<AlertTitle>
							{t("status.unavailable.title", {
								defaultValue: "Secure store unavailable",
							})}
						</AlertTitle>
						<AlertDescription>
							{storeStatusQuery.data?.issue?.message ??
								t("status.unavailable.description", {
									defaultValue:
										"The secret store is not ready. Create and update operations are disabled until the issue is resolved.",
								})}
						</AlertDescription>
					</Alert>
				)}
				{secretsQuery.isError ? (
					<div className="flex flex-col items-center justify-center gap-3 py-12 text-center">
						<p className="text-sm text-destructive">
							{t("list.error", {
								defaultValue: "Failed to load secrets. The secure store may be unavailable.",
							})}
						</p>
						<Button variant="outline" size="sm" onClick={() => secretsQuery.refetch()}>
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
							{viewMode === "grid"
								? pagedSecrets.map(renderSecretCard)
								: pagedSecrets.map(renderSecretRow)}
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
				isSaving={saveMutation.isPending}
				placeholder={editorPlaceholder}
				usages={usagesQuery.data ?? []}
				usagesLoading={Boolean(editorAlias) && usagesQuery.isLoading}
				usedByCount={editorUsedByCount}
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
					<AlertDialogDescription>
						{t("delete.description", {
							defaultValue:
								"This removes the encrypted value only when no active usage is recorded.",
						})}
					</AlertDialogDescription>
				</AlertDialogHeader>
				<div className="rounded-md bg-muted px-3 py-2 font-mono text-xs">
					{secret?.alias}
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
