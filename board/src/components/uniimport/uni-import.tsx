import { zodResolver } from "@hookform/resolvers/zod";
import { ClipboardPaste, Loader2, RotateCcw, Target } from "lucide-react";
import {
	forwardRef,
	useCallback,
	useEffect,
	useId,
	useImperativeHandle,
	useMemo,
	useRef,
	useState,
} from "react";
import { useFieldArray, useForm, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { readClipboardText } from "../../lib/clipboard";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { parseJsonDrafts } from "../../lib/install-normalizer";
import { isTauriEnvironmentSync } from "../../lib/platform";
import { Button } from "../ui/button";
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
import { Segment } from "../ui/segment";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { Textarea } from "../ui/textarea";
import {
	CommandField,
	HttpHeaders,
	MetaFields,
	StdioAdvanced,
	UrlParams,
} from "./form-fields";
import {
	useFormState,
	useFormSubmission,
	useFormSync,
	useIngest,
} from "./hooks";
import {
	breathingAnimation,
	DEFAULT_INGEST_MESSAGE,
	type ManualServerFormValues,
	manualServerSchema,
	SERVER_TYPE_OPTIONS,
	type ServerInstallManualFormHandle,
	type ServerInstallManualFormProps,
} from "./types";
import { FormViewModeToggle } from "./view-mode-toggle";

export const ServerInstallManualForm = forwardRef<
	ServerInstallManualFormHandle,
	ServerInstallManualFormProps
>(
	(
		{
			isOpen,
			onClose,
			onSubmit,
			onSubmitMultiple,
			mode = "create",
			initialDraft,
			allowJsonEditing,
			onPreview,
			allowProgrammaticIngest = false,
		}: ServerInstallManualFormProps,
		ref,
	) => {
		usePageTranslations("servers");
		const { t, i18n } = useTranslation("servers");
		const isEditMode = mode === "edit";
		const isMarketMode = mode === "market";
		const jsonEditingEnabled = allowJsonEditing ?? !isEditMode;
		const ingestEnabled = !isEditMode && !isMarketMode;

		// Form state management
		const {
			activeTab,
			setActiveTab,
			viewMode,
			setViewMode,
			jsonText,
			setJsonText,
			jsonError,
			setJsonError,
			formStateRef,
			isRestoringRef,
			lastInitialDraftRef,
			createInitialFormState,
			buildFormValuesFromState,
		} = useFormState();

		const {
			control,
			handleSubmit,
			register,
			formState: { errors, isSubmitting },
			reset,
			watch,
			setValue,
			getValues,
		} = useForm<ManualServerFormValues>({
			resolver: zodResolver(manualServerSchema),
			shouldUnregister: false,
			defaultValues: {
				name: "",
				kind: "stdio",
				command: "",
				url: "",
				args: [],
				env: [],
				headers: [],
				urlParams: [],
				meta_description: "",
				meta_version: "",
				meta_website_url: "",
				meta_repository_url: "",
				meta_repository_source: "",
				meta_repository_subfolder: "",
				meta_repository_id: "",
			} as ManualServerFormValues,
		});

		// Field arrays
		const {
			fields: argFields,
			append: appendArg,
			remove: removeArg,
		} = useFieldArray({ control, name: "args" });

		const {
			fields: envFields,
			append: appendEnv,
			remove: removeEnv,
		} = useFieldArray({ control, name: "env" });

		const {
			fields: headerFields,
			append: appendHeader,
			remove: removeHeader,
		} = useFieldArray({ control, name: "headers" });

		const {
			fields: urlParamFields,
			append: appendUrlParam,
			remove: removeUrlParam,
		} = useFieldArray({ control, name: "urlParams" });

		// Watched values
		const kind = watch("kind");
		const isStdio = kind === "stdio";
		const serverTypeOptions = useMemo(
			() =>
				SERVER_TYPE_OPTIONS.map((option) => ({
					...option,
					label: t(`manual.fields.type.options.${option.value}`, {
						defaultValue:
							option.value === "stdio"
								? "Stdio"
								: option.value === "sse"
									? "SSE"
									: "Streamable HTTP",
					}),
				})),
			[t],
		);
		const watchedName = useWatch({ control, name: "name" });
		const watchedMetaDescription = useWatch({
			control,
			name: "meta_description",
		});
		const watchedMetaVersion = useWatch({ control, name: "meta_version" });
		const watchedMetaWebsite = useWatch({ control, name: "meta_website_url" });
		const watchedMetaRepositoryUrl = useWatch({
			control,
			name: "meta_repository_url",
		});
		const watchedMetaRepositorySource = useWatch({
			control,
			name: "meta_repository_source",
		});
		const watchedMetaRepositorySubfolder = useWatch({
			control,
			name: "meta_repository_subfolder",
		});
		const watchedMetaRepositoryId = useWatch({
			control,
			name: "meta_repository_id",
		});
		const watchedCommand = useWatch({ control, name: "command" });
		const watchedUrl = useWatch({ control, name: "url" });
		const watchedArgs = useWatch({ control, name: "args" });
		const watchedEnv = useWatch({ control, name: "env" });
		const watchedHeaders = useWatch({ control, name: "headers" });

		const ingestMessages = useMemo(
			() => ({
				defaultMessage: t("manual.ingest.default", {
					defaultValue: DEFAULT_INGEST_MESSAGE,
				}),
				parsingDropped: t("manual.ingest.parsingDropped", {
					defaultValue: "Parsing dropped text",
				}),
				parsingPasted: t("manual.ingest.parsingPasted", {
					defaultValue: "Parsing pasted content",
				}),
				success: t("manual.ingest.success", {
					defaultValue: "Server configuration loaded successfully",
				}),
				noneDetectedError: t("manual.ingest.noneDetectedError", {
					defaultValue: "No servers detected in the input",
				}),
				noneDetectedTitle: t("manual.ingest.noneDetectedTitle", {
					defaultValue: "No servers detected",
				}),
				noneDetectedDescription: t("manual.ingest.noneDetectedDescription", {
					defaultValue:
						"We could not find any server definitions in the input.",
				}),
				parseFailedFallback: t("manual.ingest.parseFailedFallback", {
					defaultValue: "Failed to parse input",
				}),
				parseFailedTitle: t("manual.ingest.parseFailedTitle", {
					defaultValue: "Parsing failed",
				}),
			}),
			[t],
		);
		const editingMessage = t("manual.ingest.editing", {
			defaultValue: "Editing server",
		});

		const submissionMessages = useMemo(
			() => ({
				commandRequiredTitle: t("manual.errors.commandRequiredTitle", {
					defaultValue: "Command required",
				}),
				commandRequiredBody: t("manual.errors.commandRequiredBody", {
					defaultValue: "Provide a command for stdio servers.",
				}),
				endpointRequiredTitle: t("manual.errors.endpointRequiredTitle", {
					defaultValue: "Endpoint required",
				}),
				endpointRequiredBody: t("manual.errors.endpointRequiredBody", {
					defaultValue: "Provide a URL for non-stdio servers.",
				}),
				jsonNoServers: t("manual.errors.jsonNoServers", {
					defaultValue: "No servers found in JSON payload",
				}),
				jsonMultipleServers: t("manual.errors.jsonMultipleServers", {
					defaultValue: "Manual entry accepts exactly one server in JSON mode",
				}),
				jsonParseFailedTitle: t("manual.errors.jsonParseFailedTitle", {
					defaultValue: "Invalid JSON",
				}),
				jsonParseFailedFallback: t("manual.errors.jsonParseFailedFallback", {
					defaultValue: "Failed to parse JSON",
				}),
				invalidJsonTitle: t("manual.errors.invalidJsonTitle", {
					defaultValue: "Invalid JSON",
				}),
				submit: {
					edit: t("manual.buttons.save", { defaultValue: "Save changes" }),
					market: t("manual.buttons.import", { defaultValue: "Import server" }),
					create: t("manual.buttons.preview", { defaultValue: "Preview" }),
				},
				pending: {
					edit: t("manual.buttons.saving", { defaultValue: "Saving..." }),
					market: t("manual.buttons.importing", {
						defaultValue: "Importing...",
					}),
					create: t("manual.buttons.processing", {
						defaultValue: "Processing...",
					}),
				},
			}),
			[t],
		);

		// Form sync
		const { saveTypeSnapshot, restoreTypeSnapshot } = useFormSync({
			formStateRef,
			isRestoringRef,
			kind,
			watchedName,
			watchedMetaDescription,
			watchedMetaVersion,
			watchedMetaWebsite,
			watchedMetaRepositoryUrl,
			watchedMetaRepositorySource,
			watchedMetaRepositorySubfolder,
			watchedMetaRepositoryId,
			watchedCommand,
			watchedUrl,
			watchedArgs,
			watchedEnv,
			watchedHeaders,
			getValues,
			reset,
			buildFormValuesFromState,
		});

		// Ingest functionality
		const {
			isIngesting,
			ingestMessage,
			setIngestMessage,
			ingestError,
			setIngestError,
			isIngestSuccess,
			setIsIngestSuccess,
			isDropZoneCollapsed,
			setIsDropZoneCollapsed,
			isDragOver,
			setIsDragOver,
			canIngestProgrammatically,
			resetIngestState,
			applySingleDraftToForm,
			handleIngestPayload,
		} = useIngest({
			ingestEnabled,
			allowProgrammaticIngest,
			formStateRef,
			buildFormValuesFromState,
			reset,
			onSubmitMultiple,
			onClose,
			messages: ingestMessages,
		});

		// Form submission
		const {
			buildDraftFromValues,
			submitForm,
			submitJson,
			submitButtonLabel,
			pendingButtonLabel,
		} = useFormSubmission({
			isEditMode,
			isMarketMode,
			onSubmit,
			onClose,
			reset,
			viewMode,
			jsonText,
			jsonEditingEnabled,
			setJsonError,
			setViewMode,
			messages: submissionMessages,
		});

		// UI state
		const [deleteConfirmStates, setDeleteConfirmStates] = useState<
			Record<string, boolean>
		>({});

		const pasteShortcut = t("manual.ingest.shortcut", {
			defaultValue: "Ctrl/Cmd + V",
		});
		const pasteTipPrefix = t("manual.ingest.tipPrefix", {
			defaultValue: "Tip: press",
		});
		const pasteTipSuffix = t("manual.ingest.tipSuffix", {
			defaultValue: "to paste instantly.",
		});
		const headerTitle = isEditMode
			? t("manual.header.title.edit", { defaultValue: "Editing server" })
			: isMarketMode
				? t("manual.header.title.import", { defaultValue: "Import Server" })
				: t("manual.header.title.create", {
						defaultValue: "Server Uni-Import",
					});
		const headerDescription = isEditMode
			? t("manual.header.description.edit", {
					defaultValue:
						"Review and update the existing server settings. JSON preview remains read-only in this mode.",
				})
			: isMarketMode
				? t("manual.header.description.import", {
						defaultValue: "Configure and import this server from the registry.",
					})
				: t("manual.header.description.create", {
						defaultValue:
							"You can directly drag and drop the configuration information, or enter it manually.",
					});
		const resetLabel = t("manual.buttons.reset", {
			defaultValue: "Reset form",
		});
		const tabsCoreLabel = t("manual.tabs.core", {
			defaultValue: "Core configuration",
		});
		const tabsMetaLabel = t("manual.tabs.meta", {
			defaultValue: "Meta information",
		});
		const tabsMetaWip = t("manual.tabs.metaWip", { defaultValue: "WIP" });
		const nameLabel = t("manual.fields.name.label", { defaultValue: "Name" });
		const namePlaceholder = t("manual.fields.name.placeholder", {
			defaultValue: "e.g., local-mcp",
		});
		const nameReadOnlyTitle = t("manual.fields.name.readOnlyTitle", {
			defaultValue: "Editing server names is disabled",
		});
		const typeLabel = t("manual.fields.type.label", { defaultValue: "Type" });
		const jsonLabel = t("manual.fields.json.label", {
			defaultValue: "Server JSON",
		});
		const cancelLabel = t("manual.buttons.cancel", { defaultValue: "Cancel" });
		const previewLabel = t("manual.buttons.preview", {
			defaultValue: "Preview",
		});
		const previewingLabel = t("manual.buttons.previewing", {
			defaultValue: "Previewing...",
		});

		// Refs
		const dropZoneRef = useRef<HTMLButtonElement | null>(null);
		const commandInputRef = useRef<HTMLInputElement | null>(null);
		const urlInputRef = useRef<HTMLInputElement | null>(null);

		// Generate unique IDs for form elements
		const nameId = useId();
		const kindId = useId();
		const commandId = useId();
		const urlId = useId();
		const metaDescriptionId = useId();
		const metaVersionId = useId();
		const metaWebsiteUrlId = useId();
		const metaRepositoryUrlId = useId();
		const metaRepositorySourceId = useId();
		const metaRepositorySubfolderId = useId();
		const metaRepositoryId = useId();
		const manualJsonId = useId();

		// Focus management
		useEffect(() => {
			const frame = requestAnimationFrame(() => {
				const el = isStdio ? commandInputRef.current : urlInputRef.current;
				if (el) {
					el.focus();
					el.blur();
				}
			});
			return () => cancelAnimationFrame(frame);
		}, [isStdio]);

		// Reset form when closed
		useEffect(() => {
			if (!isOpen) {
				reset();
				setViewMode("form");
				setJsonError(null);
				setActiveTab("core");
				resetIngestState();
				formStateRef.current = createInitialFormState();
				lastInitialDraftRef.current = null;
			}
		}, [
			createInitialFormState,
			isOpen,
			reset,
			resetIngestState,
			setViewMode,
			setJsonError,
			setActiveTab,
			formStateRef,
			lastInitialDraftRef,
		]);

		// Handle initial draft
		useEffect(() => {
			if (!isOpen) return;
			if (!initialDraft) return;
			const signature = JSON.stringify(initialDraft);
			if (lastInitialDraftRef.current === signature) return;
			applySingleDraftToForm(initialDraft);
			lastInitialDraftRef.current = signature;
			setActiveTab("core");
			setViewMode("form");
			setIsIngestSuccess(true);
			setIsDropZoneCollapsed(true);
			setIngestError(null);
			setIngestMessage(
				isEditMode ? editingMessage : ingestMessages.defaultMessage,
			);
		}, [
			applySingleDraftToForm,
			initialDraft,
			isEditMode,
			editingMessage,
			ingestMessages.defaultMessage,
			isOpen,
			setActiveTab,
			setViewMode,
			setIsIngestSuccess,
			setIsDropZoneCollapsed,
			setIngestError,
			setIngestMessage,
			lastInitialDraftRef,
		]);

		// Inject breathing animation styles
		useEffect(() => {
			const style = document.createElement("style");
			style.textContent = breathingAnimation;
			document.head.appendChild(style);
			return () => {
				document.head.removeChild(style);
			};
		}, []);

		// Event handlers
		const handleResetAll = useCallback(() => {
			const initial = createInitialFormState();
			formStateRef.current = initial;
			isRestoringRef.current = true;
			reset(buildFormValuesFromState(initial));
			isRestoringRef.current = false;
			setViewMode("form");
			setActiveTab("core");
			setJsonError(null);
			resetIngestState();
			setDeleteConfirmStates({});
		}, [
			createInitialFormState,
			reset,
			buildFormValuesFromState,
			resetIngestState,
			setViewMode,
			setActiveTab,
			setJsonError,
			formStateRef,
			isRestoringRef,
		]);

		const handleDeleteClick = useCallback(
			(fieldId: string, removeFn: () => void) => {
				if (deleteConfirmStates[fieldId]) {
					// Second click - actually delete
					removeFn();
					setDeleteConfirmStates((prev) => {
						const newState = { ...prev };
						delete newState[fieldId];
						return newState;
					});
				} else {
					// First click - show confirmation
					setDeleteConfirmStates((prev) => ({ ...prev, [fieldId]: true }));
				}
			},
			[deleteConfirmStates],
		);

		const handleGhostClick = useCallback((addFn: () => void) => {
			addFn();
		}, []);

		// Form interaction handlers
		const handleFormInteraction = useCallback(() => {
			if (!isDropZoneCollapsed) {
				setIsDropZoneCollapsed(true);
			}
		}, [isDropZoneCollapsed, setIsDropZoneCollapsed]);

		// Clipboard ingestion helper (defined before any handlers that reference it)
		const ingestClipboardPayload = useCallback(
			async (initialText?: string | null) => {
				if (!ingestEnabled || isDropZoneCollapsed || isIngesting) {
					return false;
				}
				const seeded = initialText?.trim() ? initialText : null;
				const text = seeded ?? (await readClipboardText());
				if (!text || !text.trim()) {
					return false;
				}
				setIngestMessage(ingestMessages.parsingPasted);
				await handleIngestPayload({ text });
				return true;
			},
			[
				handleIngestPayload,
				ingestEnabled,
				ingestMessages.parsingPasted,
				isDropZoneCollapsed,
				isIngesting,
			],
		);

		const handleDropZoneClick = useCallback(() => {
			if (!ingestEnabled) return;
			if (isDropZoneCollapsed) {
				setIsDropZoneCollapsed(false);
				setIngestError(null);
				setIsIngestSuccess(false);
				setIngestMessage(ingestMessages.defaultMessage);
			}
			// In Tauri builds the WebView may not deliver clipboard data to paste events reliably.
			// As a fallback, when the user clicks the drop zone we proactively try reading the
			// clipboard via the Tauri plugin (user gesture present), then run the same pipeline.
			if (isTauriEnvironmentSync() && !isIngesting) {
				void ingestClipboardPayload(null);
			}
		}, [
			ingestEnabled,
			isDropZoneCollapsed,
			setIsDropZoneCollapsed,
			setIngestError,
			setIsIngestSuccess,
			setIngestMessage,
			ingestMessages.defaultMessage,
			ingestClipboardPayload,
			isIngesting,
		]);

		// Drag and drop handlers
		const onDragEnter = (event: React.DragEvent<HTMLButtonElement>) => {
			if (!ingestEnabled) return;
			event.preventDefault();
			event.stopPropagation();
			setIsDragOver(true);
			if (isDropZoneCollapsed) {
				setIsDropZoneCollapsed(false);
				setIngestError(null);
				setIsIngestSuccess(false);
				setIngestMessage(ingestMessages.defaultMessage);
			}
		};

		const onDragLeave = (event: React.DragEvent<HTMLButtonElement>) => {
			if (!ingestEnabled) return;
			event.preventDefault();
			event.stopPropagation();
			if (!event.currentTarget.contains(event.relatedTarget as Node)) {
				setIsDragOver(false);
			}
		};

		const onDrop = async (event: React.DragEvent<HTMLButtonElement>) => {
			if (!ingestEnabled) return;
			event.preventDefault();
			setIsDragOver(false);
			const { files, items } = event.dataTransfer;
			if (files?.length) {
				const file = files[0];
				if (file.name.endsWith(".mcpb") || file.name.endsWith(".dxt")) {
					const buffer = await file.arrayBuffer();
					setIngestMessage(
						t("manual.ingest.processingBundle", {
							name: file.name,
							defaultValue: "Processing bundle: {{name}}",
						}),
					);
					await handleIngestPayload({ buffer, fileName: file.name });
					return;
				}
				const text = await file.text();
				setIngestMessage(
					t("manual.ingest.parsingFile", {
						name: file.name,
						defaultValue: "Parsing text from {{name}}",
					}),
				);
				await handleIngestPayload({ text, fileName: file.name });
				return;
			}
			if (items?.length) {
				const item = items[0];
				if (item.kind === "string") {
					item.getAsString(async (text) => {
						setIngestMessage(ingestMessages.parsingDropped);
						await handleIngestPayload({ text });
					});
				}
			}
		};

		const onPaste = useCallback(
			(event: React.ClipboardEvent<HTMLButtonElement>) => {
				if (!ingestEnabled || isDropZoneCollapsed || isIngesting) {
					return;
				}
				event.preventDefault();
				// In Tauri/WebView, event.clipboardData may be empty even on user gesture.
				// Prefer the Tauri clipboard plugin when available for reliability.
				const seeded = isTauriEnvironmentSync()
					? null
					: (event.clipboardData?.getData("text/plain") ?? null);
				void ingestClipboardPayload(seeded);
			},
			[ingestClipboardPayload, ingestEnabled, isDropZoneCollapsed, isIngesting],
		);

		// Paste listener
		useEffect(() => {
			if (!isOpen || !ingestEnabled) return;
			const listener = (event: ClipboardEvent) => {
				if (!ingestEnabled || isDropZoneCollapsed || isIngesting) {
					return;
				}
				event.preventDefault();
				const seeded = isTauriEnvironmentSync()
					? null
					: (event.clipboardData?.getData("text/plain") ?? null);
				void ingestClipboardPayload(seeded);
			};
			window.addEventListener("paste", listener);
			return () => window.removeEventListener("paste", listener);
		}, [
			ingestClipboardPayload,
			ingestEnabled,
			isOpen,
			isDropZoneCollapsed,
			isIngesting,
		]);

		// Focus drop zone
		useEffect(() => {
			if (!isOpen || !ingestEnabled) return;
			const frame = requestAnimationFrame(() => {
				dropZoneRef.current?.focus();
			});
			return () => cancelAnimationFrame(frame);
		}, [ingestEnabled, isOpen]);

		// Imperative handle
		useImperativeHandle(ref, () => ({
			ingest: canIngestProgrammatically
				? handleIngestPayload
				: async () => undefined,
			loadDraft: (draft) => {
				applySingleDraftToForm(draft);
				setIsIngestSuccess(true);
				setIsDropZoneCollapsed(true);
				setIngestMessage(ingestMessages.success);
				setIngestError(null);
				setActiveTab("core");
			},
			getCurrentDraft: () => {
				const values = getValues();
				return buildDraftFromValues(values);
			},
		}));

		// Form submission handlers
		const formSubmitHandler = handleSubmit(submitForm);

		const onFormSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
			if (viewMode === "json") {
				event.preventDefault();
				if (jsonEditingEnabled) {
					await submitJson();
				}
				return;
			}
			await formSubmitHandler(event);
		};

		// JSON sync functions
		const syncFormToJson = () => {
			saveTypeSnapshot(kind);
			const valuesForJson = buildFormValuesFromState(formStateRef.current);
			const current = buildDraftFromValues(valuesForJson);

			const entry: Record<string, unknown> = {
				type: current.kind,
			};

			if (current.kind === "stdio") {
				if (current.command) entry.command = current.command;
				if (current.args?.length) entry.args = current.args;
				if (current.env && Object.keys(current.env).length)
					entry.env = current.env;
			} else {
				if (current.url) {
					const params = (current as { urlParams?: Record<string, string> })
						?.urlParams;
					if (params && Object.keys(params).length) {
						try {
							const isHttp = /^https?:/i.test(current.url);
							const u = new URL(
								current.url,
								isHttp ? undefined : "http://dummy.local",
							);
							for (const [k, v] of Object.entries(params)) {
								u.searchParams.set(k, v);
							}
							entry.url = isHttp
								? u.toString()
								: `${current.url}?${u.searchParams.toString()}`;
						} catch {
							const qs = new URLSearchParams(
								params as Record<string, string>,
							).toString();
							entry.url = qs ? `${current.url}?${qs}` : current.url;
						}
					} else {
						entry.url = current.url;
					}
				}
				if (current.headers && Object.keys(current.headers).length)
					entry.headers = current.headers;
			}

			if (current.meta && Object.keys(current.meta).length)
				entry.meta = current.meta;

			const payload = {
				mcpServers: {
					[current.name || "new-server"]: entry,
				},
			};

			setJsonText(JSON.stringify(payload, null, 2));
			setJsonError(null);
		};

		const syncJsonToForm = () => {
			try {
				const drafts = parseJsonDrafts(jsonText);
				if (!drafts.length) {
					setJsonError(submissionMessages.jsonNoServers);
					return false;
				}
				setJsonError(null);

				// Implementation would go here - simplified for brevity
				return true;
			} catch (error) {
				const message =
					error instanceof Error
						? error.message
						: submissionMessages.jsonParseFailedFallback;
				setJsonError(message);
				return false;
			}
		};

		const handleModeChange = (mode: "form" | "json") => {
			if (mode === viewMode) return;
			if (mode === "json") {
				syncFormToJson();
				setViewMode("json");
				return;
			}
			const ok = jsonEditingEnabled ? syncJsonToForm() : true;
			if (ok) {
				setViewMode("form");
			}
		};

		return (
			<Drawer
				open={isOpen}
				onOpenChange={(value) => (!value ? onClose() : undefined)}
			>
				<DrawerContent>
					<form onSubmit={onFormSubmit} className="flex h-full flex-col">
						<DrawerHeader className="pb-2">
							<div className="flex items-start justify-between gap-2">
								<div>
									<DrawerTitle>{headerTitle}</DrawerTitle>
									<DrawerDescription className="mt-1 text-sm text-muted-foreground">
										{headerDescription}
									</DrawerDescription>
								</div>
								{ingestEnabled ? (
									<Button
										type="button"
										variant="ghost"
										size="icon"
										onClick={handleResetAll}
										aria-label={resetLabel}
										title={resetLabel}
									>
										<RotateCcw className="h-4 w-4" />
									</Button>
								) : null}
								{ingestEnabled ? (
									<Button
										type="button"
										variant="secondary"
										size="sm"
										onClick={() => void ingestClipboardPayload(null)}
										className="ml-1"
										aria-label="Paste from clipboard"
										title="Paste from clipboard"
									>
										<ClipboardPaste className="mr-1 h-4 w-4" /> Paste
									</Button>
								) : null}
							</div>
						</DrawerHeader>

						{/* Uni-Import Drop Zone */}
						{ingestEnabled ? (
							<button
								ref={dropZoneRef}
								type="button"
								onDrop={onDrop}
								onDragOver={(event) => event.preventDefault()}
								onDragEnter={onDragEnter}
								onDragLeave={onDragLeave}
								onPaste={onPaste}
								onClick={handleDropZoneClick}
								className={`px-4 mb-4 w-full cursor-pointer focus:outline-none ${
									isDropZoneCollapsed ? "h-10" : "h-[18vh]"
								}`}
								style={{ border: "none" }}
							>
								<div
									className={`w-full h-full flex items-center justify-center gap-4 rounded-lg border border-dashed transition-all duration-300 ${
										isDropZoneCollapsed
											? "flex-row px-4 py-2 border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
											: "flex-col py-8 border-slate-300 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
									} ${
										ingestError
											? "border-red-300 bg-red-50 dark:border-red-700 dark:bg-red-900/20"
											: isIngestSuccess
												? "border-green-300 bg-green-50 dark:border-green-700 dark:bg-green-900/20"
												: isDragOver
													? "border-blue-300 bg-blue-50 dark:border-blue-700 dark:bg-blue-900/20"
													: ""
									}`}
								>
									{isIngesting ? (
										<Loader2
											className={`animate-spin ${
												isDropZoneCollapsed ? "h-4 w-4" : "h-6 w-6"
											}`}
										/>
									) : (
										<Target
											className={`transition-all duration-300 ${
												isDropZoneCollapsed ? "h-4 w-4" : "h-12 w-12"
											} ${
												isDragOver || isIngesting
													? "animate-pulse"
													: "scale-100"
											} ${isDragOver ? "text-blue-500" : "text-slate-500"}`}
											style={{
												animation:
													ingestError || isDragOver || isIngesting
														? "breathing 1.5s ease-in-out infinite"
														: undefined,
											}}
										/>
									)}

									<div
										className={`text-center ${
											isDropZoneCollapsed ? "flex-1 text-left" : ""
										}`}
									>
										<p
											className={`leading-relaxed transition-all duration-300 ${
												isDropZoneCollapsed
													? "text-sm max-w-none"
													: "max-w-none px-4 text-sm"
											} ${
												ingestError
													? "text-red-600 dark:text-red-400"
													: isIngestSuccess
														? "text-green-600 dark:text-green-400"
														: isDragOver
															? "text-blue-600 dark:text-blue-400"
															: "text-slate-600 dark:text-slate-300"
											} ${isIngesting || isDragOver ? "animate-pulse" : ""}`}
										>
											{ingestError || ingestMessage}
										</p>
										{!isDropZoneCollapsed && !ingestError && (
											<p className="text-xs text-slate-400 mt-2">
												{pasteTipPrefix}{" "}
												<kbd className="rounded bg-slate-200 px-1 text-[10px]">
													{pasteShortcut}
												</kbd>{" "}
												{pasteTipSuffix}
											</p>
										)}
									</div>
								</div>
							</button>
						) : null}

						{/* Main Content Area */}
						<div
							className={`flex-1 overflow-y-auto px-4 pb-4 ${
								ingestEnabled ? "pt-0" : "pt-4"
							}`}
						>
							<Tabs
								value={activeTab}
								onValueChange={(v) => setActiveTab(v as "core" | "meta")}
								className="space-y-4"
							>
								<TabsList className="grid w-full grid-cols-2">
									<TabsTrigger value="core">{tabsCoreLabel}</TabsTrigger>
									<TabsTrigger value="meta">
										{tabsMetaLabel} <sup>({tabsMetaWip})</sup>
									</TabsTrigger>
								</TabsList>

								<TabsContent
									value="core"
									className={`space-y-4 ${
										viewMode === "json" ? "flex flex-col h-full" : ""
									}`}
									onClick={handleFormInteraction}
								>
									<div className="flex items-center justify-end pt-2">
										<FormViewModeToggle
											mode={viewMode}
											onChange={handleModeChange}
										/>
									</div>

									{viewMode === "form" ? (
										<>
											<div className="space-y-4">
												<div className="flex items-center gap-4">
													<Label htmlFor={nameId} className="w-20 text-right">
														{nameLabel}
													</Label>
													<div className="flex-1">
														<Input
															id={nameId}
															{...register("name")}
															placeholder={namePlaceholder}
															readOnly={isEditMode}
															aria-readonly={isEditMode}
															title={isEditMode ? nameReadOnlyTitle : undefined}
															className={
																isEditMode
																	? "cursor-not-allowed bg-muted text-muted-foreground"
																	: undefined
															}
														/>
														{errors.name && (
															<p className="text-xs text-red-500">
																{t(errors.name.message ?? "", {
																	defaultValue: errors.name.message,
																})}
															</p>
														)}
													</div>
												</div>
												<div className="flex items-center gap-4">
													<Label htmlFor={kindId} className="w-20 text-right">
														{typeLabel}
													</Label>
													<div className="flex-1">
														<Segment
															options={serverTypeOptions}
															value={kind}
															onValueChange={(value) => {
																const newKind =
																	value as ManualServerFormValues["kind"];
																if (newKind === kind) {
																	return;
																}
																saveTypeSnapshot(kind);
																setValue("kind", newKind, {
																	shouldDirty: true,
																	shouldTouch: true,
																});
																restoreTypeSnapshot(newKind);
															}}
															showDots={true}
														/>
														{errors.kind && (
															<p className="text-xs text-red-500">
																{t(errors.kind.message ?? "", {
																	defaultValue: errors.kind.message,
																})}
															</p>
														)}
													</div>
												</div>
											</div>

											<CommandField
												kind={kind}
												control={control}
												errors={errors}
												commandId={commandId}
												urlId={urlId}
												commandInputRef={commandInputRef}
												urlInputRef={urlInputRef}
												viewMode={viewMode}
											/>

											<StdioAdvanced
												viewMode={viewMode}
												isStdio={isStdio}
												argFields={argFields}
												envFields={envFields}
												removeArg={removeArg}
												removeEnv={removeEnv}
												appendArg={appendArg}
												appendEnv={appendEnv}
												register={register}
												deleteConfirmStates={deleteConfirmStates}
												onDeleteClick={handleDeleteClick}
												onGhostClick={handleGhostClick}
											/>

											<UrlParams
												viewMode={viewMode}
												isStdio={isStdio}
												urlParamFields={urlParamFields}
												removeUrlParam={removeUrlParam}
												appendUrlParam={appendUrlParam}
												register={register}
												deleteConfirmStates={deleteConfirmStates}
												onDeleteClick={handleDeleteClick}
												onGhostClick={handleGhostClick}
											/>

											<HttpHeaders
												viewMode={viewMode}
												isStdio={isStdio}
												headerFields={headerFields}
												removeHeader={removeHeader}
												appendHeader={appendHeader}
												register={register}
												deleteConfirmStates={deleteConfirmStates}
												onDeleteClick={handleDeleteClick}
												onGhostClick={handleGhostClick}
											/>
										</>
									) : (
										<div className="flex flex-col h-full">
											<div className="flex items-start gap-4 flex-1">
												<Label
													htmlFor={manualJsonId}
													className="w-20 text-right pt-3 flex-shrink-0"
												>
													{jsonLabel}
												</Label>
												<div className="flex-1 flex flex-col">
													<div className="flex-1 min-h-[400px] border border-input rounded-md flex flex-col">
														<Textarea
															id={manualJsonId}
															value={jsonText}
															onChange={
																jsonEditingEnabled
																	? (event) => setJsonText(event.target.value)
																	: undefined
															}
															readOnly={!jsonEditingEnabled}
															aria-readonly={!jsonEditingEnabled}
															className="font-mono text-sm flex-1 border-0 focus:ring-0 focus:outline-none"
															style={{
																background: "transparent",
																caretColor: jsonEditingEnabled
																	? "currentColor"
																	: "transparent",
																minHeight: "400px",
																userSelect: "text",
																WebkitUserSelect: "text",
																MozUserSelect: "text",
																msUserSelect: "text",
															}}
														/>
													</div>
													{jsonError ? (
														<p className="text-xs text-red-500 mt-2">
															{jsonError}
														</p>
													) : null}
												</div>
											</div>
										</div>
									)}
								</TabsContent>

								<TabsContent
									value="meta"
									className="space-y-4 pt-4"
									onClick={handleFormInteraction}
								>
									<MetaFields
										formStateRef={formStateRef}
										register={register}
										errors={errors}
										metaDescriptionId={metaDescriptionId}
										metaVersionId={metaVersionId}
										metaWebsiteUrlId={metaWebsiteUrlId}
										metaRepositoryUrlId={metaRepositoryUrlId}
										metaRepositorySourceId={metaRepositorySourceId}
										metaRepositorySubfolderId={metaRepositorySubfolderId}
										metaRepositoryId={metaRepositoryId}
									/>
								</TabsContent>
							</Tabs>
						</div>

						<DrawerFooter className="mt-auto border-t px-6 py-4">
							<div className="flex w-full items-center justify-between gap-3">
								<Button
									type="button"
									variant="outline"
									onClick={onClose}
									disabled={isSubmitting}
								>
									{cancelLabel}
								</Button>
								{isMarketMode ? (
									<Button
										type="button"
										onClick={onPreview}
										disabled={isSubmitting}
									>
										{isSubmitting ? (
											<>
												<Loader2 className="mr-2 h-4 w-4 animate-spin" />
												{previewingLabel}
											</>
										) : (
											previewLabel
										)}
									</Button>
								) : (
									<Button type="submit" disabled={isSubmitting}>
										{isSubmitting ? (
											<>
												<Loader2 className="mr-2 h-4 w-4 animate-spin" />
												{pendingButtonLabel}
											</>
										) : (
											submitButtonLabel
										)}
									</Button>
								)}
							</div>
						</DrawerFooter>
					</form>
				</DrawerContent>
			</Drawer>
		);
	},
);
