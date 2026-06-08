import { Copy, Trash2 } from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { SecretKind, SecretUsage } from "../../lib/types";
import { writeClipboardText } from "../../lib/clipboard";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
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
	const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const hadEditorRef = useRef(false);

	useEffect(() => {
		if (editor) {
			setDisplayEditor(editor);
			setActiveTab(initialTab);
			if (!hadEditorRef.current) {
				setOpen(true);
			}
			hadEditorRef.current = true;
			return;
		}

		hadEditorRef.current = false;
		setOpen(false);
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
	const copyPlaceholderLabel = t("editor.actions.copyPlaceholder", {
		defaultValue: "Copy placeholder",
	});
	const copyPlaceholderDescription = t("editor.actions.copyPlaceholderDescription", {
		defaultValue:
			"Copy the [[secret:alias]] placeholder to paste into server env, headers, or args.",
	});

	const requestClose = () => setOpen(false);

	return (
		<Drawer open={open} nested={nested} onOpenChange={setOpen}>
			<DrawerContent className="flex h-full flex-col">
				<form
					className="flex min-h-0 flex-1 flex-col"
					onSubmit={(event) => {
						event.preventDefault();
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
									<Select
										value={activeEditor.kind}
										onValueChange={(kind) =>
											onChange({ ...activeEditor, kind: kind as SecretKind })
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
										type="password"
										value={activeEditor.value}
										onChange={(event) =>
											onChange({ ...activeEditor, value: event.target.value })
										}
										placeholder={
											activeEditor.mode === "edit"
												? t("editor.placeholders.keepValue", {
													defaultValue: "Leave blank to keep existing value",
												})
												: t("editor.placeholders.value", {
													defaultValue: "Secret value",
												})
										}
									/>
								</SecretFormRow>
							</TabsContent>
							<TabsContent value="usage" className="pt-4">
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
										disabled={isSaving || (usedByCount != null && usedByCount > 0)}
										title={
											usedByCount != null && usedByCount > 0
												? t("editor.actions.deleteDisabledTooltip", {
													defaultValue:
														"Cannot delete: secret is actively used by {{count}} location(s)",
													count: usedByCount,
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
									disabled={isSaving || !activeEditor.alias.trim()}
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
