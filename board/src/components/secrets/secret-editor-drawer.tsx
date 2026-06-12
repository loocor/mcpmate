import { Copy, Trash2 } from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { SecretKind, SecretUsage } from "../../lib/types";
import { writeClipboardText } from "../../lib/clipboard";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import { isUserCreatableSecretKind } from "../../lib/secret-origin-hints";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../ui/tabs";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "../ui/drawer";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import type { SecretEditorState } from "./secret-editor-state";
import { SecretUsageList } from "./secret-usage-list";

/** Match Vaul drawer slide-out duration so content unmounts after the animation. */
const DRAWER_CLOSE_ANIMATION_MS = 500;

/** Match Client/Server edit drawer form rows. */
const SECRET_FORM_ROW_LABEL_CLASS = "w-20 shrink-0 text-right";

/** Visual mask for write-only secrets in edit mode (not the stored value). */
const STORED_SECRET_VALUE_MASK = "••••••••••••••••••••••••";

function SecretFormRow({
	label,
	htmlFor,
	children,
}: {
	label: string;
	htmlFor?: string;
	children: ReactNode;
}) {
	return (
		<div className="flex items-center gap-4">
			<Label htmlFor={htmlFor} className={SECRET_FORM_ROW_LABEL_CLASS}>
				{label}
			</Label>
			<div className="min-w-0 flex-1">{children}</div>
		</div>
	);
}

interface SecretEditorDrawerProps {
	editor: SecretEditorState | null;
	kindOptions: Array<{ value: SecretKind; label: string }>;
	onChange: (next: SecretEditorState | null) => void;
	onClose: () => void;
	onSave: () => void;
	onDelete?: () => void;
	isSaving: boolean;
	writesDisabled?: boolean;
	placeholder?: string;
	usages?: SecretUsage[];
	usagesLoading?: boolean;
	usedByCount?: number;
	serverNameById?: ReadonlyMap<string, string>;
	initialTab?: "general" | "usage";
	nested?: boolean;
	onNavigateToServer?: (serverId: string) => void;
}

export function SecretEditorDrawer({
	editor,
	kindOptions,
	onChange,
	onClose,
	onSave,
	onDelete,
	isSaving,
	writesDisabled = false,
	placeholder,
	usages = [],
	usagesLoading = false,
	usedByCount,
	serverNameById,
	initialTab = "general",
	nested = false,
	onNavigateToServer,
}: SecretEditorDrawerProps) {
	const { t } = useTranslation("secrets");
	const aliasId = useId();
	const kindId = useId();
	const labelId = useId();
	const valueId = useId();
	const [open, setOpen] = useState(false);
	const [activeTab, setActiveTab] = useState<"general" | "usage">("general");
	const [displayEditor, setDisplayEditor] = useState<SecretEditorState | null>(
		null,
	);
	const [valueFieldFocused, setValueFieldFocused] = useState(false);
	const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const hadEditorRef = useRef(false);

	useEffect(() => {
		if (editor) {
			setDisplayEditor(editor);
			setActiveTab(initialTab);
			setValueFieldFocused(false);
			if (!hadEditorRef.current) {
				setOpen(true);
			}
			hadEditorRef.current = true;
			return;
		}

		hadEditorRef.current = false;
		setOpen(false);
		setValueFieldFocused(false);
	}, [editor, initialTab]);

	useEffect(() => {
		if (open) {
			if (closeTimerRef.current) {
				clearTimeout(closeTimerRef.current);
				closeTimerRef.current = null;
			}
			return;
		}
		if (!displayEditor) return;

		closeTimerRef.current = setTimeout(() => {
			setDisplayEditor(null);
			onClose();
			closeTimerRef.current = null;
		}, DRAWER_CLOSE_ANIMATION_MS);

		return () => {
			if (closeTimerRef.current) {
				clearTimeout(closeTimerRef.current);
				closeTimerRef.current = null;
			}
		};
	}, [displayEditor, onClose, open]);

	const handleCopyPlaceholder = useCallback(async () => {
		if (!placeholder) {
			return;
		}
		try {
			await writeClipboardText(placeholder);
			notifySuccess(
				t("notifications.copySuccess", {
					defaultValue: "Placeholder copied",
				}),
				placeholder,
			);
		} catch (error) {
			notifyError(
				t("notifications.copyError", {
					defaultValue: "Failed to copy placeholder",
				}),
				stringifyError(error),
			);
		}
	}, [placeholder, t]);

	const activeEditor = editor ?? displayEditor;
	if (!activeEditor && !open) return null;
	const canDelete = activeEditor?.mode === "edit" && onDelete;
	const activeKindLabel = activeEditor
		? kindOptions.find((option) => option.value === activeEditor.kind)?.label ??
			activeEditor.kind
		: "";
	const isOAuthSecret = activeEditor ? !isUserCreatableSecretKind(activeEditor.kind) : false;
	const isActiveOAuthSecret = isOAuthSecret && (usedByCount ?? 0) > 0;
	const activeUsageCount =
		usages.length > 0
			? usages.filter((usage) => usage.status !== "stale").length
			: (usedByCount ?? 0);
	const historicalUsageCount = usages.filter(
		(usage) => usage.status === "stale",
	).length;
	const canDeleteFromUsage = activeUsageCount === 0 && !isActiveOAuthSecret;
	const showStoredValueMask =
		activeEditor?.mode === "edit" &&
		!isOAuthSecret &&
		activeEditor.value === "" &&
		!valueFieldFocused;

	let valuePlaceholder = t("editor.placeholders.value", {
		defaultValue: "Secret value",
	});
	if (activeEditor?.mode === "edit" && !showStoredValueMask) {
		valuePlaceholder = t("editor.placeholders.keepValue", {
			defaultValue: "Leave blank to keep existing value",
		});
	}
	if (isOAuthSecret) {
		valuePlaceholder = t("editor.placeholders.oauthManagedValue", {
			defaultValue: "Managed by OAuth; reconnect to update this value",
		});
	}
	const valueInputDisplay = showStoredValueMask
		? STORED_SECRET_VALUE_MASK
		: activeEditor.value;
	const copyPlaceholderLabel = t("editor.actions.copyPlaceholder", {
		defaultValue: "Copy placeholder",
	});
	const copyPlaceholderDescription = t("editor.actions.copyPlaceholderDescription", {
		defaultValue:
			"Copy the [[secret:alias]] placeholder to paste into server env, headers, or args.",
	});
	const kindLockedDescription = t("editor.kindLockedDescription", {
		defaultValue: "Kind is set at creation and cannot be changed.",
	});
	const oauthManagedDescription = isActiveOAuthSecret
		? t("editor.oauthManagedDescription", {
			defaultValue:
				"Managed by OAuth. Reconnect or revoke OAuth to update this credential.",
		})
		: t("editor.oauthOrphanedDescription", {
			defaultValue:
				"Orphaned OAuth credential. No active owner was found; delete it if it is no longer needed.",
		});
	const kindDescription = isOAuthSecret
		? oauthManagedDescription
		: kindLockedDescription;

	const requestClose = () => setOpen(false);

	return (
		<Drawer open={open} nested={nested} onOpenChange={setOpen}>
			<DrawerContent className="flex h-full flex-col">
				<form
					className="flex min-h-0 flex-1 flex-col"
					onSubmit={(event) => {
						event.preventDefault();
						if (writesDisabled) {
							return;
						}
						onSave();
					}}
				>
					<DrawerHeader>
						<div className="flex items-start justify-between gap-3">
							<div className="min-w-0 flex-1 space-y-1 text-left">
								<DrawerTitle>
									{activeEditor.mode === "create"
										? t("editor.createTitle", { defaultValue: "Add Secret" })
										: t("editor.editTitle", { defaultValue: "Edit Secret" })}
								</DrawerTitle>
								<DrawerDescription>
									{t("editor.description", {
										defaultValue:
											"The value is write-only. It will not be shown again after save.",
									})}
								</DrawerDescription>
							</div>
							{placeholder ? (
								<TooltipProvider delayDuration={200}>
									<Tooltip>
										<TooltipTrigger asChild>
											<Button
												type="button"
												variant="ghost"
												size="icon"
												className="-mr-1 -mt-1 h-5 w-5 shrink-0 rounded-md border-0 bg-transparent p-0 text-muted-foreground shadow-none transition-colors hover:bg-transparent hover:text-foreground focus-visible:ring-1 focus-visible:ring-offset-0"
												disabled={isSaving}
												onClick={() => void handleCopyPlaceholder()}
												aria-label={copyPlaceholderLabel}
											>
												<Copy className="h-4 w-4" />
											</Button>
										</TooltipTrigger>
										<TooltipContent side="bottom" align="end" className="max-w-xs">
											<p className="font-medium">{copyPlaceholderLabel}</p>
											<p className="mt-1 text-background/80">
												{copyPlaceholderDescription}
											</p>
										</TooltipContent>
									</Tooltip>
								</TooltipProvider>
							) : null}
						</div>
					</DrawerHeader>
					<div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
						<Tabs
							value={activeTab}
							onValueChange={(value) => setActiveTab(value as "general" | "usage")}
							className="w-full"
						>
							<TabsList className="grid w-full grid-cols-2">
								<TabsTrigger value="general">
									{t("editor.tabs.general", { defaultValue: "General" })}
								</TabsTrigger>
								<TabsTrigger value="usage">
									{t("editor.tabs.usage", { defaultValue: "Usage" })}
									{typeof usedByCount === "number" ? ` (${usedByCount})` : null}
								</TabsTrigger>
							</TabsList>
							<TabsContent value="general" className="space-y-4 pt-4">
								<SecretFormRow
									label={t("editor.fields.alias", { defaultValue: "Alias" })}
									htmlFor={aliasId}
								>
									<Input
										id={aliasId}
										value={activeEditor.alias}
										disabled={activeEditor.mode === "edit"}
										onChange={(event) =>
											onChange({ ...activeEditor, alias: event.target.value })
										}
										placeholder={t("editor.placeholders.alias", {
											defaultValue: "server-context7-url-parameters-token",
										})}
									/>
								</SecretFormRow>
								<SecretFormRow
									label={t("editor.fields.kind", { defaultValue: "Kind" })}
									htmlFor={kindId}
								>
									<div className="space-y-1.5">
										{activeEditor.mode === "create" ? (
											<Select
												value={activeEditor.kind}
												onValueChange={(kind) =>
													onChange({
														...activeEditor,
														kind: kind as SecretKind,
													})
												}
											>
												<SelectTrigger id={kindId} className="h-9">
													<SelectValue />
												</SelectTrigger>
												<SelectContent>
													{kindOptions.map((option) => (
														<SelectItem
															key={option.value}
															value={option.value}
														>
															{option.label}
														</SelectItem>
													))}
												</SelectContent>
											</Select>
										) : (
											<TooltipProvider delayDuration={200}>
												<Tooltip>
													<TooltipTrigger asChild>
														<Input
															id={kindId}
															value={activeKindLabel}
															readOnly
															className="cursor-default bg-muted/50"
														/>
													</TooltipTrigger>
													<TooltipContent
														side="top"
														align="start"
														className="max-w-xs"
													>
														{kindDescription}
													</TooltipContent>
												</Tooltip>
											</TooltipProvider>
										)}
									</div>
								</SecretFormRow>
								<SecretFormRow
									label={t("editor.fields.label", { defaultValue: "Label" })}
									htmlFor={labelId}
								>
									<Input
										id={labelId}
										value={activeEditor.label}
										onChange={(event) =>
											onChange({ ...activeEditor, label: event.target.value })
										}
										placeholder={t("editor.placeholders.label", {
											defaultValue: "context7 · URL parameter · token",
										})}
									/>
								</SecretFormRow>
								<SecretFormRow
									label={t("editor.fields.value", { defaultValue: "Value" })}
									htmlFor={valueId}
								>
									<Input
										id={valueId}
										type={showStoredValueMask ? "text" : "password"}
										value={valueInputDisplay}
										disabled={isOAuthSecret}
										readOnly={showStoredValueMask}
										className={
											showStoredValueMask
												? "cursor-text text-muted-foreground tracking-widest"
												: undefined
										}
										aria-label={
											showStoredValueMask
												? t("editor.fields.storedValue", {
													defaultValue:
														"Stored secret value is hidden. Focus to replace it.",
												})
												: t("editor.fields.value", { defaultValue: "Value" })
										}
										onFocus={() => setValueFieldFocused(true)}
										onBlur={() => setValueFieldFocused(false)}
										onChange={(event) =>
											onChange({ ...activeEditor, value: event.target.value })
										}
										placeholder={valuePlaceholder}
									/>
								</SecretFormRow>
							</TabsContent>
							<TabsContent value="usage" className="space-y-4 pt-4">
								<div className="rounded-lg border bg-muted/30 p-3">
									<div className="flex flex-wrap items-center gap-2">
										<Badge variant={activeUsageCount > 0 ? "success" : "outline"}>
											{t("usage.summary.active", {
												defaultValue: "Active {{count}}",
												count: activeUsageCount,
											})}
										</Badge>
										<Badge
											variant={historicalUsageCount > 0 ? "warning" : "outline"}
										>
											{t("usage.summary.historical", {
												defaultValue: "Historical {{count}}",
												count: historicalUsageCount,
											})}
										</Badge>
									</div>
									<p className="mt-2 text-xs text-muted-foreground">
										{isActiveOAuthSecret
											? t("usage.summary.oauthManaged", {
												defaultValue:
													"OAuth credentials are cleaned up by OAuth revoke or server deletion.",
											})
											: canDeleteFromUsage
												? t("usage.summary.canDelete", {
													defaultValue:
														"No active runtime binding is using this secret.",
												})
												: t("usage.summary.blocked", {
													defaultValue:
														"Remove active bindings before deleting this secret.",
												})}
									</p>
								</div>
								<SecretUsageList
									usages={usages}
									isLoading={usagesLoading}
									serverNameById={serverNameById}
									onNavigateToServer={onNavigateToServer}
								/>
							</TabsContent>
						</Tabs>
					</div>
					<DrawerFooter className="mt-auto border-t px-6 py-4">
						<div className="flex w-full items-center justify-between gap-3">
							<Button
								type="button"
								variant="outline"
								onClick={requestClose}
								disabled={isSaving}
							>
								{t("editor.actions.cancel", { defaultValue: "Cancel" })}
							</Button>
							<div className="flex items-center gap-3">
								{canDelete ? (
									<Button
										type="button"
										variant="destructive"
										className="gap-2"
										disabled={isSaving || writesDisabled || !canDeleteFromUsage}
										title={
											!canDeleteFromUsage
												? isActiveOAuthSecret
													? t("editor.actions.deleteDisabledOAuthTooltip", {
														defaultValue:
															"OAuth-managed credentials are removed by OAuth revoke or server deletion.",
													})
													: t("editor.actions.deleteDisabledTooltip", {
														defaultValue:
															"Cannot delete: secret is actively used by {{count}} location(s)",
														count: activeUsageCount,
													})
												: undefined
										}
										onClick={onDelete}
									>
										<Trash2 className="h-4 w-4" />
										{t("editor.actions.delete", { defaultValue: "Delete" })}
									</Button>
								) : null}
								<Button
									type="submit"
									disabled={
										isSaving || writesDisabled || !activeEditor.alias.trim()
									}
								>
									{activeEditor.mode === "create"
										? t("editor.actions.create", {
											defaultValue: "Create Record",
										})
										: t("editor.actions.save", {
											defaultValue: "Save Changes",
										})}
								</Button>
							</div>
						</div>
					</DrawerFooter>
				</form>
			</DrawerContent>
		</Drawer>
	);
}
