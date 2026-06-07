import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	Copy,
	KeyRound,
	Pencil,
	Plus,
	RefreshCw,
	ShieldAlert,
	ShieldCheck,
	Trash2,
} from "lucide-react";
import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { EntityListItem } from "../../components/entity-list-item";
import { EmptyState, PageLayout } from "../../components/page-layout";
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
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "../../components/ui/dialog";
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
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { PageToolbar } from "../../components/ui/page-toolbar";
import type {
	Entity,
	PageToolbarCallbacks,
	PageToolbarConfig,
	PageToolbarState,
} from "../../components/ui/page-toolbar";
import { secretsApi } from "../../lib/api";
import { writeClipboardText } from "../../lib/clipboard";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import type {
	SecretKind,
	SecretMetadata,
	SecretOrigin,
	SecretUsage,
} from "../../lib/types";

const SECRET_KIND_VALUES: SecretKind[] = [
	"generic",
	"token",
	"api_key",
	"password",
	"oauth_access_token",
	"oauth_refresh_token",
	"url_credential",
	"header_value",
];

const DEFAULT_PAGE_SIZE = 10;

interface SecretEditorState {
	mode: "create" | "edit";
	alias: string;
	kind: SecretKind;
	label: string;
	value: string;
	origin: SecretOrigin | null;
}

type SecretToolbarEntity = Entity & {
	alias: string;
	kind: string;
	provider_kind: string;
	used_by_count: number;
	version: number;
};

const defaultEditorState = (): SecretEditorState => ({
	mode: "create",
	alias: "",
	kind: "token",
	label: "",
	value: "",
	origin: null,
});

const ORIGIN_QUERY_KEYS = [
	"server_id",
	"server_name",
	"server_kind",
	"source",
	"field_group",
	"field_key",
	"field_index",
	"field_path",
] as const;

function originFromSearchParams(params: URLSearchParams): SecretOrigin | null {
	const origin: SecretOrigin = {};
	for (const key of ORIGIN_QUERY_KEYS) {
		const value = params.get(`origin_${key}`);
		if (!value) continue;
		if (key === "field_index") {
			const parsed = Number.parseInt(value, 10);
			if (Number.isFinite(parsed)) {
				origin.field_index = parsed;
			}
			continue;
		}
		origin[key] = value;
	}
	return Object.keys(origin).length > 0 ? origin : null;
}

function stripOriginSearchParams(params: URLSearchParams) {
	for (const key of ORIGIN_QUERY_KEYS) {
		params.delete(`origin_${key}`);
	}
}

function usageLabel(
	usage: SecretUsage,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	const location = usage.location;
	if (typeof location === "string") {
		const keyMap: Record<string, string> = {
			stdio_command: "stdioCommand",
			streamable_http_url: "httpUrl",
			oauth_token: "oauthToken",
		};
		return t(`usage.location.${keyMap[location] ?? location}`, {
			defaultValue: location,
		});
	}
	if ("stdio_env" in location && typeof location.stdio_env === "object") {
		return t("usage.location.stdioEnv", {
			defaultValue: "stdio env {{name}}",
			name: (location.stdio_env as { name?: string }).name ?? "",
		});
	}
	if (
		"stdio_argument" in location &&
		typeof location.stdio_argument === "object"
	) {
		return t("usage.location.stdioArgument", {
			defaultValue: "stdio arg {{index}}",
			index: (location.stdio_argument as { index?: number }).index ?? "",
		});
	}
	if (
		"streamable_http_header" in location &&
		typeof location.streamable_http_header === "object"
	) {
		return t("usage.location.httpHeader", {
			defaultValue: "http header {{name}}",
			name: (location.streamable_http_header as { name?: string }).name ?? "",
		});
	}
	if ("stdio_command" in location) {
		return t("usage.location.stdioCommand", { defaultValue: "stdio command" });
	}
	if ("streamable_http_url" in location) {
		return t("usage.location.httpUrl", { defaultValue: "http url" });
	}
	if ("oauth_token" in location) {
		return t("usage.location.oauthToken", { defaultValue: "oauth token" });
	}
	return JSON.stringify(location);
}

export function SecretsPage() {
	usePageTranslations("secrets");
	const { t } = useTranslation("secrets");
	const queryClient = useQueryClient();
	const [searchParams, setSearchParams] = useSearchParams();
	const [editor, setEditor] = useState<SecretEditorState | null>(null);
	const [usageTarget, setUsageTarget] = useState<SecretMetadata | null>(null);
	const [deleteTarget, setDeleteTarget] = useState<SecretMetadata | null>(null);
	const [expanded, setExpanded] = useState(false);
	const [sortedSecrets, setSortedSecrets] = useState<SecretToolbarEntity[]>([]);
	const [currentPage, setCurrentPage] = useState(1);
	const [itemsPerPage, setItemsPerPage] = useState(DEFAULT_PAGE_SIZE);

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
	});
	const usagesQuery = useQuery({
		queryKey: ["secrets", "usages", usageTarget?.alias],
		queryFn: () => secretsApi.listUsages(usageTarget?.alias ?? ""),
		enabled: Boolean(usageTarget?.alias),
	});
	const storeStatusQuery = useQuery({
		queryKey: ["secrets", "status"],
		queryFn: secretsApi.status,
	});
	const storeReady = storeStatusQuery.data?.status === "ready";

	const kindOptions = SECRET_KIND_VALUES.map((value) => ({
		value,
		label: t(`kind.${value}`, { defaultValue: value }),
	}));

	const kindLabel = (kind: string): string =>
		kindOptions.find((option) => option.value === kind)?.label ?? kind;

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
		setEditor({
			...defaultEditorState(),
			origin: originFromSearchParams(searchParams),
		});
		const next = new URLSearchParams(searchParams);
		next.delete("editor");
		stripOriginSearchParams(next);
		setSearchParams(next, { replace: true });
	}, [searchParams, setSearchParams]);

	const saveMutation = useMutation({
		mutationFn: async (state: SecretEditorState) => {
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
				kind: state.kind,
				label: state.label.trim() || null,
				value: state.value.length > 0 ? state.value : undefined,
				origin: state.origin,
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
		mutationFn: (alias: string) => secretsApi.delete(alias),
		onSuccess: async () => {
			setDeleteTarget(null);
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

	const openCreate = () => setEditor(defaultEditorState());
	const openEdit = (secret: SecretMetadata) =>
		setEditor({
			mode: "edit",
			alias: secret.alias,
			kind: (secret.kind as SecretKind) || "generic",
			label: secret.label ?? "",
			value: "",
			origin: secret.origin ?? null,
		});

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
			enabled: false,
			defaultMode: "list",
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
		onSortedDataChange: (data) => {
			setSortedSecrets(data);
			setCurrentPage(1);
		},
		onExpandedChange: setExpanded,
	};

	const loadingSkeleton = Array.from({ length: 3 }, (_, index) => (
		<div
			key={`secret-skeleton-${index}`}
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
		<Button type="button" size="sm" className="mt-4 h-9" onClick={openCreate}>
			<Plus className="mr-2 h-4 w-4" />
			{t("empty.action", { defaultValue: "Add First Secret" })}
		</Button>
	) : undefined;

	const emptyState = (
		<Card>
			<CardContent className="flex flex-col items-center justify-center p-6">
				<EmptyState
					icon={<KeyRound className="h-4 w-4" />}
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
			</CardContent>
		</Card>
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

		return (
			<EntityListItem
				key={secret.alias}
				id={secret.alias}
				title={secret.alias}
				description={
					<div className="min-w-0">
						{secret.label ? (
							<div className="truncate">{secret.label}</div>
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
						value: secret.provider_kind,
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
						onClick={() => setUsageTarget(secret)}
					>
						<ShieldCheck className="mr-2 h-4 w-4" />
						{secret.used_by_count}
					</Button>,
					<Button
						key="copy"
						type="button"
						variant="ghost"
						size="icon"
						className="h-9 w-9"
						onClick={() => void writeClipboardText(secret.placeholder)}
						aria-label={t("list.actions.copy", {
							defaultValue: "Copy placeholder",
						})}
					>
						<Copy className="h-4 w-4" />
					</Button>,
					<Button
						key="edit"
						type="button"
						variant="ghost"
						size="icon"
						className="h-9 w-9"
						onClick={() => openEdit(secret)}
						aria-label={t("list.actions.edit", { defaultValue: "Edit secret" })}
					>
						<Pencil className="h-4 w-4" />
					</Button>,
					<Button
						key="delete"
						type="button"
						variant="ghost"
						size="icon"
						className="h-9 w-9"
						onClick={() => setDeleteTarget(secret)}
						aria-label={t("list.actions.delete", {
							defaultValue: "Delete secret",
						})}
					>
						<Trash2 className="h-4 w-4 text-destructive" />
					</Button>,
				]}
				onClick={() => openEdit(secret)}
			/>
		);
	};

	return (
		<PageLayout
			title={t("title", { defaultValue: "Secure Store" })}
			headerActions={
				<PageToolbar<SecretToolbarEntity>
					config={toolbarConfig}
					state={toolbarState}
					callbacks={toolbarCallbacks}
					actions={actions}
				/>
			}
		>
			<div className="space-y-4">
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
				{secretsQuery.isLoading ? (
					<div className="space-y-4">{loadingSkeleton}</div>
				) : secretsQuery.isError ? (
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
				) : sortedSecrets.length === 0 ? (
					emptyState
				) : (
					<>
						<div className="space-y-4">{pagedSecrets.map(renderSecretRow)}</div>
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
					</>
				)}
			</div>

			<SecretEditorDrawer
				editor={editor}
				kindOptions={kindOptions}
				onChange={setEditor}
				onClose={() => setEditor(null)}
				onSave={() => editor && saveMutation.mutate(editor)}
				isSaving={saveMutation.isPending}
			/>
			<SecretUsageDialog
				secret={usageTarget}
				usages={usagesQuery.data ?? []}
				isLoading={usagesQuery.isLoading}
				onClose={() => setUsageTarget(null)}
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

function SecretEditorDrawer({
	editor,
	kindOptions,
	onChange,
	onClose,
	onSave,
	isSaving,
}: {
	editor: SecretEditorState | null;
	kindOptions: Array<{ value: SecretKind; label: string }>;
	onChange: (next: SecretEditorState | null) => void;
	onClose: () => void;
	onSave: () => void;
	isSaving: boolean;
}) {
	const { t } = useTranslation("secrets");
	const aliasId = useId();
	const kindId = useId();
	const labelId = useId();
	const valueId = useId();
	if (!editor) return null;

	return (
		<Drawer open={Boolean(editor)} onOpenChange={(open) => !open && onClose()}>
			<DrawerContent className="h-full flex flex-col">
				<form
					className="flex min-h-0 flex-1 flex-col"
					onSubmit={(event) => {
						event.preventDefault();
						onSave();
					}}
				>
					<DrawerHeader>
						<DrawerTitle>
							{editor.mode === "create"
								? t("editor.createTitle", { defaultValue: "Add Secret" })
								: t("editor.editTitle", { defaultValue: "Edit Secret" })}
						</DrawerTitle>
						<DrawerDescription>
							{t("editor.description", {
								defaultValue:
									"The value is write-only. It will not be shown again after save.",
							})}
						</DrawerDescription>
					</DrawerHeader>
					<div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
						<div className="grid gap-4">
							<div className="grid gap-2">
								<Label htmlFor={aliasId}>
									{t("editor.fields.alias", { defaultValue: "Alias" })}
								</Label>
								<Input
									id={aliasId}
									value={editor.alias}
									disabled={editor.mode === "edit"}
									onChange={(event) =>
										onChange({ ...editor, alias: event.target.value })
									}
									placeholder="server/github/token"
								/>
							</div>
							<div className="grid gap-2">
								<Label htmlFor={kindId}>
									{t("editor.fields.kind", { defaultValue: "Kind" })}
								</Label>
								<Select
									value={editor.kind}
									onValueChange={(kind) =>
										onChange({ ...editor, kind: kind as SecretKind })
									}
								>
									<SelectTrigger id={kindId} className="h-9">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										{kindOptions.map((option) => (
											<SelectItem key={option.value} value={option.value}>
												{option.label}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
							<div className="grid gap-2">
								<Label htmlFor={labelId}>
									{t("editor.fields.label", { defaultValue: "Label" })}
								</Label>
								<Input
									id={labelId}
									value={editor.label}
									onChange={(event) =>
										onChange({ ...editor, label: event.target.value })
									}
									placeholder="GitHub token"
								/>
							</div>
							<div className="grid gap-2">
								<Label htmlFor={valueId}>
									{t("editor.fields.value", { defaultValue: "Value" })}
								</Label>
								<Input
									id={valueId}
									type="password"
									value={editor.value}
									onChange={(event) =>
										onChange({ ...editor, value: event.target.value })
									}
									placeholder={
										editor.mode === "edit"
											? t("editor.placeholders.keepValue", {
													defaultValue: "Leave blank to keep existing value",
												})
											: t("editor.placeholders.value", {
													defaultValue: "Secret value",
												})
									}
								/>
							</div>
						</div>
					</div>
					<DrawerFooter className="mt-auto border-t bg-background px-6 py-4">
						<Button
							type="button"
							variant="outline"
							onClick={onClose}
							disabled={isSaving}
						>
							{t("editor.actions.cancel", { defaultValue: "Cancel" })}
						</Button>
						<Button type="submit" disabled={isSaving || !editor.alias.trim()}>
							<KeyRound className="mr-2 h-4 w-4" />
							{t("editor.actions.save", { defaultValue: "Save" })}
						</Button>
					</DrawerFooter>
				</form>
			</DrawerContent>
		</Drawer>
	);
}

function SecretUsageDialog({
	secret,
	usages,
	isLoading,
	onClose,
}: {
	secret: SecretMetadata | null;
	usages: SecretUsage[];
	isLoading: boolean;
	onClose: () => void;
}) {
	const { t } = useTranslation("secrets");
	return (
		<Dialog open={Boolean(secret)} onOpenChange={(open) => !open && onClose()}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						{t("usage.title", { defaultValue: "Secret Usage" })}
					</DialogTitle>
					<DialogDescription className="font-mono">
						{secret?.alias}
					</DialogDescription>
				</DialogHeader>
				<div className="rounded-md border">
					{isLoading ? (
						<div className="p-4 text-sm text-muted-foreground">
							{t("usage.loading", { defaultValue: "Loading usages" })}
						</div>
					) : usages.length === 0 ? (
						<div className="p-4 text-sm text-muted-foreground">
							{t("usage.empty", { defaultValue: "No server usage recorded" })}
						</div>
					) : (
						<table className="w-full text-sm">
							<thead className="bg-muted/50 text-left">
								<tr>
									<th className="px-3 py-2 font-medium">
										{t("usage.columns.server", { defaultValue: "Server" })}
									</th>
									<th className="px-3 py-2 font-medium">
										{t("usage.columns.location", { defaultValue: "Location" })}
									</th>
								</tr>
							</thead>
							<tbody>
								{usages.map((usage, index) => (
									<tr key={`${usage.server_id}-${index}`} className="border-t">
										<td className="px-3 py-2 font-mono text-xs">
											{usage.server_id}
										</td>
										<td className="px-3 py-2">{usageLabel(usage, t)}</td>
									</tr>
								))}
							</tbody>
						</table>
					)}
				</div>
			</DialogContent>
		</Dialog>
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
								"This removes the encrypted value only when no server usage is recorded.",
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
