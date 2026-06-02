import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, KeyRound, Pencil, Plus, RefreshCw, ShieldCheck, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PageLayout } from "../../components/page-layout";
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
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../components/ui/dialog";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { secretsApi } from "../../lib/api";
import { writeClipboardText } from "../../lib/clipboard";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import type { SecretKind, SecretMetadata, SecretUsage } from "../../lib/types";

const SECRET_KIND_OPTIONS: Array<{ value: SecretKind; label: string }> = [
	{ value: "generic", label: "Generic" },
	{ value: "token", label: "Token" },
	{ value: "api_key", label: "API key" },
	{ value: "password", label: "Password" },
	{ value: "oauth_access_token", label: "OAuth access" },
	{ value: "oauth_refresh_token", label: "OAuth refresh" },
	{ value: "url_credential", label: "URL credential" },
	{ value: "header_value", label: "Header value" },
];

interface SecretEditorState {
	mode: "create" | "edit";
	alias: string;
	kind: SecretKind;
	label: string;
	value: string;
}

const defaultEditorState = (): SecretEditorState => ({
	mode: "create",
	alias: "",
	kind: "token",
	label: "",
	value: "",
});

function kindLabel(kind: string): string {
	return SECRET_KIND_OPTIONS.find((option) => option.value === kind)?.label ?? kind;
}

function usageLabel(usage: SecretUsage): string {
	const location = usage.location;
	if (typeof location === "string") {
		return location;
	}
	if ("stdio_env" in location && typeof location.stdio_env === "object") {
		return `stdio env ${(location.stdio_env as { name?: string }).name ?? ""}`;
	}
	if ("stdio_argument" in location && typeof location.stdio_argument === "object") {
		return `stdio arg ${(location.stdio_argument as { index?: number }).index ?? ""}`;
	}
	if (
		"streamable_http_header" in location &&
		typeof location.streamable_http_header === "object"
	) {
		return `http header ${(location.streamable_http_header as { name?: string }).name ?? ""}`;
	}
	if ("stdio_command" in location) return "stdio command";
	if ("streamable_http_url" in location) return "http url";
	if ("oauth_token" in location) return "oauth token";
	return JSON.stringify(location);
}

export function SecretsPage() {
	const { t } = useTranslation();
	const queryClient = useQueryClient();
	const [editor, setEditor] = useState<SecretEditorState | null>(null);
	const [usageTarget, setUsageTarget] = useState<SecretMetadata | null>(null);
	const [deleteTarget, setDeleteTarget] = useState<SecretMetadata | null>(null);

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
	});
	const usagesQuery = useQuery({
		queryKey: ["secrets", "usages", usageTarget?.alias],
		queryFn: () => secretsApi.listUsages(usageTarget?.alias ?? ""),
		enabled: Boolean(usageTarget?.alias),
	});

	const sortedSecrets = useMemo(
		() =>
			[...(secretsQuery.data ?? [])].sort((left, right) =>
				left.alias.localeCompare(right.alias),
			),
		[secretsQuery.data],
	);

	const saveMutation = useMutation({
		mutationFn: async (state: SecretEditorState) => {
			if (state.mode === "create") {
				return secretsApi.create({
					alias: state.alias.trim(),
					kind: state.kind,
					label: state.label.trim() || null,
					value: state.value,
				});
			}
			return secretsApi.update({
				alias: state.alias.trim(),
				kind: state.kind,
				label: state.label.trim() || null,
				value: state.value.length > 0 ? state.value : undefined,
			});
		},
		onSuccess: async () => {
			setEditor(null);
			await queryClient.invalidateQueries({ queryKey: ["secrets"] });
			notifySuccess(t("secrets.saveSuccess", { defaultValue: "Secret saved" }));
		},
		onError: (error) => {
			notifyError(t("secrets.saveError", { defaultValue: "Failed to save secret" }), stringifyError(error));
		},
	});

	const deleteMutation = useMutation({
		mutationFn: (alias: string) => secretsApi.delete(alias),
		onSuccess: async () => {
			setDeleteTarget(null);
			await queryClient.invalidateQueries({ queryKey: ["secrets"] });
			notifySuccess(t("secrets.deleteSuccess", { defaultValue: "Secret deleted" }));
		},
		onError: (error) => {
			notifyError(t("secrets.deleteError", { defaultValue: "Failed to delete secret" }), stringifyError(error));
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
		});

	return (
		<PageLayout
			title={t("secrets.pageDescription", {
				defaultValue: "Manage runtime secrets used by managed MCP server configuration.",
			})}
			headerActions={
				<>
					<Button
						type="button"
						variant="outline"
						size="icon"
						onClick={() => void secretsQuery.refetch()}
						aria-label={t("common.refresh", { defaultValue: "Refresh" })}
					>
						<RefreshCw className="h-4 w-4" />
					</Button>
					<Button type="button" onClick={openCreate}>
						<Plus className="mr-2 h-4 w-4" />
						{t("secrets.add", { defaultValue: "Add Secret" })}
					</Button>
				</>
			}
		>
			<Card>
				<CardHeader className="pb-3">
					<CardTitle className="flex items-center gap-2 text-lg">
						<ShieldCheck className="h-5 w-5" />
						{t("secrets.storeTitle", { defaultValue: "Secure Store" })}
					</CardTitle>
					<CardDescription>
						{t("secrets.storeDescription", {
							defaultValue: "Values are write-only after save. Server configs use placeholders.",
						})}
					</CardDescription>
				</CardHeader>
				<CardContent>
					<div className="overflow-x-auto rounded-md border">
						<table className="w-full text-sm">
							<thead className="bg-muted/50 text-left">
								<tr>
									<th className="px-3 py-2 font-medium">
										{t("secrets.columns.alias", { defaultValue: "Alias" })}
									</th>
									<th className="px-3 py-2 font-medium">
										{t("secrets.columns.kind", { defaultValue: "Kind" })}
									</th>
									<th className="px-3 py-2 font-medium">
										{t("secrets.columns.provider", { defaultValue: "Provider" })}
									</th>
									<th className="px-3 py-2 font-medium">
										{t("secrets.columns.usage", { defaultValue: "Usage" })}
									</th>
									<th className="px-3 py-2 font-medium">
										{t("secrets.columns.version", { defaultValue: "Version" })}
									</th>
									<th className="px-3 py-2 text-right font-medium">
										{t("secrets.columns.actions", { defaultValue: "Actions" })}
									</th>
								</tr>
							</thead>
							<tbody>
								{sortedSecrets.length === 0 ? (
									<tr>
										<td colSpan={6} className="px-3 py-8 text-center text-muted-foreground">
											{secretsQuery.isLoading
												? t("secrets.loading", { defaultValue: "Loading secrets" })
												: t("secrets.empty", { defaultValue: "No secrets stored" })}
										</td>
									</tr>
								) : (
									sortedSecrets.map((secret) => (
										<tr key={secret.alias} className="border-t">
											<td className="max-w-[320px] px-3 py-3">
												<div className="truncate font-medium">{secret.alias}</div>
												<div className="truncate font-mono text-xs text-muted-foreground">
													{secret.placeholder}
												</div>
											</td>
											<td className="px-3 py-3">
												<Badge variant="secondary">{kindLabel(secret.kind)}</Badge>
											</td>
											<td className="px-3 py-3 text-muted-foreground">{secret.provider_kind}</td>
											<td className="px-3 py-3">
												<Button type="button" variant="ghost" size="sm" onClick={() => setUsageTarget(secret)}>
													{secret.used_by_count}
												</Button>
											</td>
											<td className="px-3 py-3 font-mono text-xs">{secret.version}</td>
											<td className="px-3 py-3">
												<div className="flex justify-end gap-1">
													<Button
														type="button"
														variant="ghost"
														size="icon"
														onClick={() => void writeClipboardText(secret.placeholder)}
														aria-label={t("secrets.copyPlaceholder", { defaultValue: "Copy placeholder" })}
													>
														<Copy className="h-4 w-4" />
													</Button>
													<Button
														type="button"
														variant="ghost"
														size="icon"
														onClick={() => openEdit(secret)}
														aria-label={t("secrets.edit", { defaultValue: "Edit secret" })}
													>
														<Pencil className="h-4 w-4" />
													</Button>
													<Button
														type="button"
														variant="ghost"
														size="icon"
														onClick={() => setDeleteTarget(secret)}
														aria-label={t("secrets.delete", { defaultValue: "Delete secret" })}
													>
														<Trash2 className="h-4 w-4 text-destructive" />
													</Button>
												</div>
											</td>
										</tr>
									))
								)}
							</tbody>
						</table>
					</div>
				</CardContent>
			</Card>

			<SecretEditorDialog
				editor={editor}
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
				onConfirm={() => deleteTarget && deleteMutation.mutate(deleteTarget.alias)}
			/>
		</PageLayout>
	);
}

function SecretEditorDialog({
	editor,
	onChange,
	onClose,
	onSave,
	isSaving,
}: {
	editor: SecretEditorState | null;
	onChange: (next: SecretEditorState | null) => void;
	onClose: () => void;
	onSave: () => void;
	isSaving: boolean;
}) {
	const { t } = useTranslation();
	if (!editor) return null;

	return (
		<Dialog open={Boolean(editor)} onOpenChange={(open) => !open && onClose()}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						{editor.mode === "create"
							? t("secrets.createTitle", { defaultValue: "Add Secret" })
							: t("secrets.editTitle", { defaultValue: "Edit Secret" })}
					</DialogTitle>
					<DialogDescription>
						{t("secrets.editorDescription", {
							defaultValue: "The value is write-only. It will not be shown again after save.",
						})}
					</DialogDescription>
				</DialogHeader>
				<div className="grid gap-4">
					<div className="grid gap-2">
						<Label htmlFor="secret-alias">Alias</Label>
						<Input
							id="secret-alias"
							value={editor.alias}
							disabled={editor.mode === "edit"}
							onChange={(event) => onChange({ ...editor, alias: event.target.value })}
							placeholder="server/github/token"
						/>
					</div>
					<div className="grid gap-2">
						<Label>Kind</Label>
						<Select
							value={editor.kind}
							onValueChange={(kind) => onChange({ ...editor, kind: kind as SecretKind })}
						>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{SECRET_KIND_OPTIONS.map((option) => (
									<SelectItem key={option.value} value={option.value}>
										{option.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>
					<div className="grid gap-2">
						<Label htmlFor="secret-label">Label</Label>
						<Input
							id="secret-label"
							value={editor.label}
							onChange={(event) => onChange({ ...editor, label: event.target.value })}
							placeholder="GitHub token"
						/>
					</div>
					<div className="grid gap-2">
						<Label htmlFor="secret-value">Value</Label>
						<Input
							id="secret-value"
							type="password"
							value={editor.value}
							onChange={(event) => onChange({ ...editor, value: event.target.value })}
							placeholder={editor.mode === "edit" ? "Leave blank to keep existing value" : "Secret value"}
						/>
					</div>
				</div>
				<DialogFooter>
					<Button type="button" variant="outline" onClick={onClose}>
						{t("common.cancel", { defaultValue: "Cancel" })}
					</Button>
					<Button type="button" onClick={onSave} disabled={isSaving || !editor.alias.trim()}>
						<KeyRound className="mr-2 h-4 w-4" />
						{t("common.save", { defaultValue: "Save" })}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
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
	const { t } = useTranslation();
	return (
		<Dialog open={Boolean(secret)} onOpenChange={(open) => !open && onClose()}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>{t("secrets.usageTitle", { defaultValue: "Secret Usage" })}</DialogTitle>
					<DialogDescription className="font-mono">{secret?.alias}</DialogDescription>
				</DialogHeader>
				<div className="rounded-md border">
					{isLoading ? (
						<div className="p-4 text-sm text-muted-foreground">
							{t("secrets.loadingUsages", { defaultValue: "Loading usages" })}
						</div>
					) : usages.length === 0 ? (
						<div className="p-4 text-sm text-muted-foreground">
							{t("secrets.noUsages", { defaultValue: "No server usage recorded" })}
						</div>
					) : (
						<table className="w-full text-sm">
							<thead className="bg-muted/50 text-left">
								<tr>
									<th className="px-3 py-2 font-medium">Server</th>
									<th className="px-3 py-2 font-medium">
										{t("secrets.columns.location", { defaultValue: "Location" })}
									</th>
								</tr>
							</thead>
							<tbody>
								{usages.map((usage, index) => (
									<tr key={`${usage.server_id}-${index}`} className="border-t">
										<td className="px-3 py-2 font-mono text-xs">{usage.server_id}</td>
										<td className="px-3 py-2">{usageLabel(usage)}</td>
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
	const { t } = useTranslation();
	return (
		<AlertDialog open={Boolean(secret)} onOpenChange={(open) => !open && onClose()}>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>
						{t("secrets.deleteTitle", { defaultValue: "Delete secret?" })}
					</AlertDialogTitle>
					<AlertDialogDescription>
						{t("secrets.deleteDescription", {
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
						{t("common.cancel", { defaultValue: "Cancel" })}
					</AlertDialogCancel>
					<AlertDialogAction
						disabled={isDeleting}
						onClick={onConfirm}
						className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
					>
						{t("common.delete", { defaultValue: "Delete" })}
					</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}
