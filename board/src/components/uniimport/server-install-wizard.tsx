import { zodResolver } from "@hookform/resolvers/zod";
import {
	AlertTriangle,
	ChevronDown,
	ChevronRight,
	Copy,
	Loader2,
	RefreshCw,
	Target,
} from "lucide-react";
import type { FocusEvent } from "react";
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
import { useFieldArray, useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import {
	type InstallSource,
	type ServerInstallDraft,
	useServerInstallPipeline,
	type WizardStep,
} from "../../hooks/use-server-install-pipeline";
import { readClipboardText, writeClipboardText } from "../../lib/clipboard";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { useAppStore } from "../../lib/store";
import { Alert, AlertDescription, AlertTitle } from "../ui/alert";
import { Badge } from "../ui/badge";
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
import { useFormState, useFormSync, useIngest } from "./hooks";
import {
	breathingAnimation,
	DEFAULT_INGEST_MESSAGE,
	type ManualServerFormValues,
	manualServerSchema,
	SERVER_TYPE_OPTIONS,
	type ServerInstallManualFormHandle,
} from "./types";
import { FormViewModeToggle } from "./view-mode-toggle";

// Step definitions

const STEP_ORDER: WizardStep[] = ["form", "preview", "result"];

interface ServerInstallWizardProps {
	isOpen: boolean;
	onClose: () => void;
	// Supported modes: legacy aliases kept for compatibility
	mode?: "new" | "import" | "create" | "edit" | "market";
	initialDraft?: ServerInstallDraft;
	onPreview?: (drafts: ServerInstallDraft[]) => void;
	onImport?: (drafts: ServerInstallDraft[]) => void;
	allowProgrammaticIngest?: boolean;
	// Optional shared pipeline instance from parent page (recommended)
	pipeline?: ReturnType<typeof useServerInstallPipeline>;
}

export const ServerInstallWizard = forwardRef(
	(
		{
			isOpen,
			onClose,
			mode = "create",
			initialDraft,
			onPreview,
			onImport,
			allowProgrammaticIngest = false,
			pipeline: externalPipeline,
		}: ServerInstallWizardProps,
		ref: React.Ref<ServerInstallManualFormHandle>,
	) => {
		usePageTranslations("servers");
		const { t } = useTranslation("servers");
		// Normalize modes: "create"->"new", "market"->"import"
		const normalizedMode =
			mode === "create" ? "new" : mode === "market" ? "import" : mode;
		const isEditMode = normalizedMode === "edit";
		const isImportMode = normalizedMode === "import";
		const jsonEditingEnabled = !isEditMode;
		const ingestEnabled = !isEditMode && !isImportMode;

		// Wizard state
		const [isClosing, setIsClosing] = useState(false);
		const [uiActiveTab, setUiActiveTab] = useState<"core" | "meta">("core");
		const steps = useMemo(
			() =>
				STEP_ORDER.map((id) => ({
					id,
					label: t(`wizard.steps.${id}.label`, {
						defaultValue:
							id === "form"
								? "Configuration"
								: id === "preview"
									? "Preview"
									: "Import & Profile",
					}),
					hint: t(`wizard.steps.${id}.hint`, {
						defaultValue:
							id === "form"
								? "Setup"
								: id === "preview"
									? "Review"
									: "Complete",
					}),
				})),
			[t],
		);

		// Install pipeline (prefer external shared instance to keep state in sync with parent page)
		const installPipeline = externalPipeline ?? useServerInstallPipeline();
		const currentStep = installPipeline.state.currentStep ?? "form";
		const navigate = useNavigate();

		// Form state management
		const {
			viewMode,
			setViewMode,
			jsonText,
			setJsonText,
			jsonError,
			setJsonError,
			formStateRef,
			isRestoringRef,
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
			trigger,
		} = useForm<ManualServerFormValues>({
			resolver: zodResolver(manualServerSchema),
			defaultValues: buildFormValuesFromState(createInitialFormState()),
		});

		const viewModeRef = useRef(viewMode);

		useEffect(() => {
			viewModeRef.current = viewMode;
		}, [viewMode]);

		// Form field arrays
		const argFields = useFieldArray({
			control,
			name: "args",
		});

		const envFields = useFieldArray({
			control,
			name: "env",
		});

		const headerFields = useFieldArray({
			control,
			name: "headers",
		});

		const paramFields = useFieldArray({
			control,
			name: "urlParams",
		});

		// Field array methods
		const appendArg = useCallback(() => {
			argFields.append({ value: "" });
		}, [argFields]);

		const removeArg = useCallback(
			(index: number) => {
				argFields.remove(index);
			},
			[argFields],
		);

		const appendEnv = useCallback(() => {
			envFields.append({ key: "", value: "" });
		}, [envFields]);

		const removeEnv = useCallback(
			(index: number) => {
				envFields.remove(index);
			},
			[envFields],
		);

		const appendHeader = useCallback(() => {
			headerFields.append({ key: "", value: "" });
		}, [headerFields]);

		const removeHeader = useCallback(
			(index: number) => {
				headerFields.remove(index);
			},
			[headerFields],
		);

		const appendUrlParam = useCallback(() => {
			paramFields.append({ key: "", value: "" });
		}, [paramFields]);

		const removeUrlParam = useCallback(
			(index: number) => {
				paramFields.remove(index);
			},
			[paramFields],
		);

		// Form refs
		const dropZoneRef = useRef<HTMLButtonElement | null>(null);
		const commandInputRef = useRef<HTMLInputElement>(null);
		const urlInputRef = useRef<HTMLInputElement>(null);

		// Form field IDs
		const nameId = useId();
		const kindId = useId();
		const commandId = useId();
		const urlId = useId();
		const manualJsonId = useId();
		const metaDescriptionId = useId();
		const metaVersionId = useId();
		const metaWebsiteUrlId = useId();
		const metaRepositoryUrlId = useId();
		const metaRepositorySourceId = useId();
		const metaRepositorySubfolderId = useId();
		const metaRepositoryId = useId();

		// Watch form values
		const kind = watch("kind");
		const isStdio = kind === "stdio";

		// Form interaction handlers
		const handleFormInteraction = useCallback(() => {
			// Track form interaction for auto-save or validation
		}, []);

		const handleModeChange = useCallback(
			(mode: "form" | "json") => {
				setViewMode(mode);
			},
			[setViewMode],
		);

		// Type snapshot management (for form state restoration)
		const saveTypeSnapshot = useCallback((_currentKind: string) => {
			// Save current form state for restoration
		}, []);

		const restoreTypeSnapshot = useCallback((_newKind: string) => {
			// Restore form state based on type
		}, []);

		// Delete confirmation states
		const [deleteConfirmStates, setDeleteConfirmStates] = useState<
			Record<string, boolean>
		>({});

		const handleDeleteClick = useCallback(
			(id: string, removeFn: () => void) => {
				setDeleteConfirmStates((prev) => {
					if (prev[id]) {
						removeFn();
						const { [id]: _omit, ...rest } = prev;
						return rest;
					}
					return { ...prev, [id]: true };
				});
				setTimeout(() => {
					setDeleteConfirmStates((prev) => {
						const { [id]: _omit, ...rest } = prev;
						return rest;
					});
				}, 2000);
			},
			[],
		);

		const handleGhostClick = useCallback((addFn: () => void) => {
			addFn();
		}, []);

		// Sync form state with our JSON snapshot and watchers
		const watchedName = watch("name");
		const watchedMetaDescription = watch("meta_description");
		const watchedMetaVersion = watch("meta_version");
		const watchedMetaWebsite = watch("meta_website_url");
		const watchedMetaRepositoryUrl = watch("meta_repository_url");
		const watchedMetaRepositorySource = watch("meta_repository_source");
		const watchedMetaRepositorySubfolder = watch("meta_repository_subfolder");
		const watchedMetaRepositoryId = watch("meta_repository_id");
		const watchedCommand = watch("command");
		const watchedUrl = watch("url");
		const watchedArgs = watch("args");
		const watchedEnv = watch("env");
		const watchedHeaders = watch("headers");
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
		const previewInFlightRef = useRef(false);

		const toKeyValueRecord = useCallback(
			(items?: Array<{ key?: string | null; value?: string | null }>) => {
				if (!Array.isArray(items)) return {} as Record<string, string>;
				return items.reduce<Record<string, string>>((acc, entry) => {
					const key = typeof entry?.key === "string" ? entry.key.trim() : "";
					if (!key) return acc;
					const rawValue = typeof entry?.value === "string" ? entry.value : "";
					acc[key] = rawValue.trim();
					return acc;
				}, {});
			},
			[],
		);

		const toArgsArray = useCallback(
			(items?: Array<{ value?: string | null }>) => {
				if (!Array.isArray(items)) return [] as string[];
				return items
					.map((entry) =>
						typeof entry?.value === "string" ? entry.value.trim() : "",
					)
					.filter((value): value is string => value.length > 0);
			},
			[],
		);

		const buildJsonPayloadFromValues = useCallback(
			(values: ManualServerFormValues) => {
				const trim = (input?: string | null) =>
					typeof input === "string" ? input.trim() : "";
				const serverName = (() => {
					const name = trim(values.name);
					return name.length > 0 ? name : "example";
				})();
				const serverPayload: Record<string, unknown> = {
					type: values.kind,
				};

				if (values.kind === "stdio") {
					serverPayload.command = trim(values.command);
					serverPayload.args = toArgsArray(values.args);
					const envRecord = toKeyValueRecord(values.env);
					if (Object.keys(envRecord).length > 0) {
						serverPayload.env = envRecord;
					}
					if (!Array.isArray(serverPayload.args)) {
						serverPayload.args = [];
					}
				} else {
					serverPayload.url = trim(values.url);
					const headersRecord = toKeyValueRecord(values.headers);
					if (Object.keys(headersRecord).length > 0) {
						serverPayload.headers = headersRecord;
					}
					const urlParamsRecord = toKeyValueRecord((values as any).urlParams);
					if (Object.keys(urlParamsRecord).length > 0) {
						serverPayload.urlParams = urlParamsRecord;
					}
				}

				const repository: Record<string, string> = {};
				const meta: Record<string, unknown> = {};

				const description = trim(values.meta_description);
				if (description) meta.description = description;
				const version = trim(values.meta_version);
				if (version) meta.version = version;
				const websiteUrl = trim(values.meta_website_url);
				if (websiteUrl) meta.websiteUrl = websiteUrl;

				const repoUrl = trim(values.meta_repository_url);
				if (repoUrl) repository.url = repoUrl;
				const repoSource = trim(values.meta_repository_source);
				if (repoSource) repository.source = repoSource;
				const repoSubfolder = trim(values.meta_repository_subfolder);
				if (repoSubfolder) repository.subfolder = repoSubfolder;
				const repoId = trim(values.meta_repository_id);
				if (repoId) repository.id = repoId;
				if (Object.keys(repository).length > 0) {
					meta.repository = repository;
				}

				if (Object.keys(meta).length > 0) {
					serverPayload.meta = meta;
				}

				return JSON.stringify(
					{
						mcpServers: {
							[serverName]: serverPayload,
						},
					},
					null,
					2,
				);
			},
			[toArgsArray, toKeyValueRecord],
		);

		const updateJsonFromValues = useCallback(
			(values?: ManualServerFormValues) => {
				const currentValues = values ?? getValues();
				const nextJson = buildJsonPayloadFromValues(currentValues);
				setJsonError(null);
				setJsonText((prev) => (prev === nextJson ? prev : nextJson));
			},
			[buildJsonPayloadFromValues, getValues, setJsonError, setJsonText],
		);

		useEffect(() => {
			if (viewMode !== "json") return;
			updateJsonFromValues();
			const subscription = watch((formValues) => {
				if (viewModeRef.current !== "json") return;
				updateJsonFromValues(formValues as ManualServerFormValues);
			});
			return () => subscription.unsubscribe();
		}, [viewMode, watch, updateJsonFromValues]);

		const previewPrereqsMet = useMemo(() => {
			const normalize = (value?: string | null) =>
				typeof value === "string" ? value.trim() : "";
			const hasName = normalize(watchedName).length > 0;
			if (!hasName) return false;
			if (!kind) return false;
			if (kind === "stdio") {
				return normalize(watchedCommand).length > 0;
			}
			return normalize(watchedUrl).length > 0;
		}, [watchedName, kind, watchedCommand, watchedUrl]);

		const hasBlockingErrors = useMemo(
			() => Boolean(errors.name || errors.kind || errors.command || errors.url),
			[errors.name, errors.kind, errors.command, errors.url],
		);

		useFormSync({
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

		// Ingest functionality (programmatic and tab button)
		const {
			isIngesting,
			ingestMessage,
			ingestError,
			isIngestSuccess,
			isDropZoneCollapsed,
			isDragOver,
			setIsDragOver,
			setIsDropZoneCollapsed,
			resetIngestState,
			handleIngestPayload,
		} = useIngest({
			ingestEnabled,
			allowProgrammaticIngest,
			formStateRef,
			buildFormValuesFromState,
			reset,
			onSubmitMultiple: (drafts) => {
				if (onPreview) {
					onPreview(drafts);
				} else {
					installPipeline.begin(drafts, "ingest");
				}
			},
			onClose,
			messages: ingestMessages,
		});

		// Drag & drop/paste handlers for the top drop zone (new mode only)
		const canIngestFromDataTransfer = (dt: DataTransfer | null): boolean => {
			if (!dt) return false;
			const types = Array.from(dt.types ?? []);
			return (
				types.includes("Files") ||
				types.includes("text/plain") ||
				types.includes("text/uri-list")
			);
		};

		const extractPayloadFromDataTransfer = async (
			dt: DataTransfer,
		): Promise<{
			text?: string;
			buffer?: ArrayBuffer;
			fileName?: string;
		} | null> => {
			if (dt.files && dt.files.length > 0) {
				const file = dt.files[0];
				if (file.name.endsWith(".mcpb") || file.name.endsWith(".dxt")) {
					return { buffer: await file.arrayBuffer(), fileName: file.name };
				}
				return { text: await file.text(), fileName: file.name };
			}
			const plain = dt.getData("text/plain");
			if (plain) return { text: plain };
			const uri = dt.getData("text/uri-list");
			if (uri) return { text: uri };
			if (dt.items && dt.items.length) {
				for (const item of Array.from(dt.items)) {
					if (item.kind === "string") {
						const v = await new Promise<string | null>((resolve) =>
							item.getAsString((t) => resolve(t ?? null)),
						);
						if (v) return { text: v };
					}
				}
			}
			return null;
		};

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
				await handleIngestPayload({ text });
				return true;
			},
			[handleIngestPayload, ingestEnabled, isDropZoneCollapsed, isIngesting],
		);

		const handleDropZoneActivate = useCallback(() => {
			if (!ingestEnabled) return;
			if (isDropZoneCollapsed || ingestError || isIngestSuccess) {
				resetIngestState();
			}
			setIsDropZoneCollapsed(false);
		}, [
			ingestEnabled,
			isDropZoneCollapsed,
			ingestError,
			isIngestSuccess,
			resetIngestState,
			setIsDropZoneCollapsed,
		]);

		const handleContentFocus = useCallback(
			(event: FocusEvent<HTMLDivElement>) => {
				if (!ingestEnabled) return;
				const target = event.target as Node;
				if (dropZoneRef.current && dropZoneRef.current.contains(target)) {
					return;
				}
				if (!isDropZoneCollapsed) {
					setIsDropZoneCollapsed(true);
				}
			},
			[ingestEnabled, isDropZoneCollapsed, setIsDropZoneCollapsed],
		);

		// Step navigation logic
		const canNavigateToStep = useCallback(
			(step: WizardStep): boolean => {
				switch (step) {
					case "form":
						return true;
					case "preview":
						return previewPrereqsMet && !hasBlockingErrors && !jsonError;
					case "result":
						// Can navigate to result if preview is completed
						return installPipeline.state.previewState !== null;
					default:
						return false;
				}
			},
			[
				previewPrereqsMet,
				hasBlockingErrors,
				jsonError,
				installPipeline.state.previewState,
			],
		);

		// Sync current step with pipeline state

		// Handle preview action
		const toDraftFromValues = useCallback(
			(values: ManualServerFormValues): ServerInstallDraft => {
				const trim = (v?: string | null) => {
					if (v == null) return undefined;
					const t = v.trim();
					return t.length ? t : undefined;
				};
				const args = (values.args ?? [])
					.map((it) => trim(it.value))
					.filter((v): v is string => Boolean(v));
				const kvToRecord = (
					items?: Array<{ key?: string; value?: string }>,
				): Record<string, string> | undefined => {
					const pairs = (items ?? [])
						.map((e) => {
							const k = trim(e.key);
							const val = trim(e.value);
							return k ? { key: k, value: val ?? "" } : null;
						})
						.filter((x): x is { key: string; value: string } => Boolean(x));
					return pairs.length
						? pairs.reduce<Record<string, string>>((acc, e) => {
								acc[e.key] = e.value;
								return acc;
							}, {})
						: undefined;
				};
				const urlParams = kvToRecord((values as any).urlParams);
				const headers = kvToRecord(values.headers);
				const env = kvToRecord(values.env);
				const repository = (() => {
					const payload: Record<string, string> = {};
					const url = trim(values.meta_repository_url);
					const source = trim(values.meta_repository_source);
					const subfolder = trim(values.meta_repository_subfolder);
					const id = trim(values.meta_repository_id);
					if (url) payload.url = url;
					if (source) payload.source = source;
					if (subfolder) payload.subfolder = subfolder;
					if (id) payload.id = id;
					return Object.keys(payload).length ? (payload as any) : undefined;
				})();
				const meta: any = {};
				const description = trim(values.meta_description);
				const version = trim(values.meta_version);
				const websiteUrl = trim(values.meta_website_url);
				if (description) meta.description = description;
				if (version) meta.version = version;
				if (websiteUrl) meta.websiteUrl = websiteUrl;
				if (repository) meta.repository = repository;

				return {
					name: values.name.trim(),
					kind: values.kind,
					command: values.kind === "stdio" ? trim(values.command) : undefined,
					url: values.kind === "stdio" ? undefined : trim(values.url),
					args: values.kind === "stdio" && args.length ? args : undefined,
					env: values.kind === "stdio" ? env : undefined,
					headers: values.kind !== "stdio" ? headers : undefined,
					...(values.kind !== "stdio" && urlParams ? { urlParams } : {}),
					meta: Object.keys(meta).length ? meta : undefined,
				};
			},
			[],
		);

		const handlePreview = useCallback(
			async (opts?: { shouldFocus?: boolean; skipValidation?: boolean }) => {
				if (previewInFlightRef.current) return;
				if (!opts?.skipValidation) {
					const isValid = await trigger(undefined, {
						shouldFocus: opts?.shouldFocus ?? true,
					});
					if (!isValid) return;
				}
				previewInFlightRef.current = true;

				const formValues = getValues();
				const draft = toDraftFromValues(formValues);
				const drafts = [draft];
				const origin = isImportMode
					? ("market" as InstallSource)
					: ("manual" as InstallSource);

				if (currentStep !== "preview") {
					installPipeline.setCurrentStep("preview");
				}

				try {
					if (onPreview) {
						await Promise.resolve(onPreview(drafts));
					} else {
						await installPipeline.begin(drafts, origin);
					}
				} finally {
					previewInFlightRef.current = false;
				}
			},
			[
				trigger,
				getValues,
				toDraftFromValues,
				isImportMode,
				onPreview,
				currentStep,
				installPipeline,
			],
		);

		// Auto-trigger preview when navigating to preview step
		useEffect(() => {
			if (
				currentStep === "preview" &&
				installPipeline.state.previewState === null &&
				!installPipeline.state.isPreviewLoading &&
				!previewInFlightRef.current
			) {
				void handlePreview({ shouldFocus: false });
			}
		}, [
			currentStep,
			installPipeline.state.previewState,
			installPipeline.state.isPreviewLoading,
			handlePreview,
		]);

		const handleStepChange = useCallback(
			(step: WizardStep) => {
				if (isSubmitting) return;
				if (step === "preview") {
					void handlePreview();
					return;
				}
				if (step === "result") {
					// Just navigate to result step, don't trigger import yet
					if (canNavigateToStep(step)) {
						installPipeline.setCurrentStep(step);
					}
					return;
				}
				if (canNavigateToStep(step)) {
					installPipeline.setCurrentStep(step);
				}
			},
			[isSubmitting, handlePreview, canNavigateToStep, installPipeline],
		);

		// Overlay close handler (immediate, no delay)
		const handleOverlayClose = useCallback(() => {
			if (!isClosing) {
				setIsClosing(true);
				onClose();
				setIsClosing(false);
				installPipeline.setCurrentStep("form");
				installPipeline.reset();
			}
		}, [onClose, isClosing, installPipeline]);

		// Handle import action
		const handleImport = useCallback(async () => {
			if (onImport) {
				const formValues = getValues();
				const draft = toDraftFromValues(formValues);
				await Promise.resolve(onImport([draft]));
				handleOverlayClose();
				return;
			}

			// Use install pipeline for import; close drawer on success
			const didSucceed = await installPipeline.confirmImport();
			if (didSucceed) {
				handleOverlayClose();
			}
		}, [
			getValues,
			onImport,
			toDraftFromValues,
			handleOverlayClose,
			installPipeline,
		]);

		// Cancel close handler (with delay for complete reset)
		const handleCancelClose = useCallback(() => {
			if (!isClosing) {
				setIsClosing(true);
				setTimeout(() => {
					onClose();
					setIsClosing(false);

					// Complete reset for Cancel button
					installPipeline.setCurrentStep("form");
					installPipeline.reset();

					// Reset form to initial state
					const initialFormState = createInitialFormState();
					formStateRef.current = initialFormState;
					reset(buildFormValuesFromState(initialFormState));

					// Reset ingest state (drag zone)
					resetIngestState();

					// Reset UI state
					setUiActiveTab("core");
					setViewMode("form");
				}, 50);
			}
		}, [
			onClose,
			isClosing,
			installPipeline,
			createInitialFormState,
			reset,
			buildFormValuesFromState,
			resetIngestState,
			setViewMode,
		]);

		type NextStepAction = "close" | "servers" | "profiles" | "preview" | "none";

		const handleNextStepAction = useCallback(
			(action: NextStepAction) => {
				switch (action) {
					case "close":
						handleOverlayClose();
						break;
					case "servers":
						handleOverlayClose();
						window.setTimeout(() => navigate("/servers"), 0);
						break;
					case "profiles":
						handleOverlayClose();
						window.setTimeout(() => navigate("/profiles"), 0);
						break;
					case "preview":
						handleStepChange("preview");
						break;
					case "none":
					default:
						break;
				}
			},
			[handleOverlayClose, navigate, handleStepChange],
		);

		// Reset wizard when opening (only on transition from closed to open)
		useEffect(() => {
			if (isOpen) {
				installPipeline.reset();
			}
			// eslint-disable-next-line react-hooks/exhaustive-deps
		}, [isOpen]);

		// Hydrate form when an initial draft is provided (e.g., Market mode)
		// Create a stable key that only changes when the actual draft content changes
		const draftKey = useMemo(() => {
			if (!initialDraft) return null;
			return JSON.stringify({
				name: initialDraft.name,
				kind: initialDraft.kind,
				command: initialDraft.command,
				url: initialDraft.url,
			});
		}, [initialDraft]);

		const processedDraftRef = useRef<string | null>(null);

		useEffect(() => {
			if (!initialDraft || !isOpen || !draftKey) return;

			// Skip if we've already processed this exact draft
			if (processedDraftRef.current === draftKey) return;
			processedDraftRef.current = draftKey;

			try {
				const payload = {
					mcpServers: {
						[initialDraft.name]: {
							type: initialDraft.kind,
							command: initialDraft.command,
							args: initialDraft.args,
							env: initialDraft.env,
							url: initialDraft.url,
							headers: initialDraft.headers,
							meta: initialDraft.meta,
						},
					},
				};
				void handleIngestPayload({ text: JSON.stringify(payload) });
			} catch {}
			// eslint-disable-next-line react-hooks/exhaustive-deps
		}, [draftKey, isOpen]);

		// Reset processed draft ref when drawer closes
		useEffect(() => {
			if (!isOpen) {
				processedDraftRef.current = null;
			}
		}, [isOpen]);

		// Inject breathing animation styles
		useEffect(() => {
			const style = document.createElement("style");
			style.textContent = breathingAnimation;
			document.head.appendChild(style);
			return () => {
				document.head.removeChild(style);
			};
		}, []);

		// Perform dry-run when entering result step
		useEffect(() => {
			if (
				currentStep === "result" &&
				!installPipeline.state.importResult &&
				!installPipeline.state.isImporting
			) {
				// Only perform dry-run if we haven't already done it or if the drafts have changed
				if (
					!installPipeline.state.dryRunResult &&
					!installPipeline.state.isDryRunLoading
				) {
					void installPipeline.performDryRun();
				}
			}
		}, [currentStep, installPipeline]);

		// Expose methods via ref
		useImperativeHandle(ref, () => ({
			ingest: async (payload) => {
				await handleIngestPayload(payload);
			},
			loadDraft: async (draft: ServerInstallDraft) => {
				// Apply a single draft to the form using the ingest helper logic path
				await handleIngestPayload({
					text: JSON.stringify({
						mcpServers: {
							[draft.name]: {
								type: draft.kind,
								command: draft.command,
								args: draft.args,
								env: draft.env,
								url: draft.url,
								headers: draft.headers,
								meta: draft.meta,
							},
						},
					}),
				});
			},
			getCurrentDraft: () => {
				const values = getValues();
				return toDraftFromValues(values);
			},
			reset: () => {
				reset();
				installPipeline.reset();
			},
		}));

		// Render step content
		const renderStepContent = () => {
			switch (currentStep) {
				case "form":
					return renderFormStep();
				case "preview":
					return renderPreviewStep();
				case "result":
					return renderResultStep();
				default:
					return null;
			}
		};

		const renderFormStep = () => {
			const contentPadding = ingestEnabled
				? "px-4 pb-4 pt-0"
				: "px-4 pb-4 pt-4";
			return (
				<div className="flex flex-col">
					<form
						onSubmit={handleSubmit(() =>
							handlePreview({ skipValidation: true, shouldFocus: false }),
						)}
						className="flex flex-col"
						onClick={handleFormInteraction}
						onKeyDown={handleFormInteraction}
					>
						{/* New-mode drop zone (top) */}
						{ingestEnabled ? (
							<div className="px-4 py-4">
								<button
									type="button"
									ref={dropZoneRef}
									onFocus={handleDropZoneActivate}
									onClick={handleDropZoneActivate}
									onMouseDown={handleDropZoneActivate}
									onDragOver={(e) => {
										if (!canIngestFromDataTransfer(e.dataTransfer)) return;
										e.preventDefault();
										setIsDragOver(true);
									}}
									onDragEnter={(e) => {
										if (!canIngestFromDataTransfer(e.dataTransfer)) return;
										e.preventDefault();
										// Auto-expand and reset drop zone if collapsed
										if (isDropZoneCollapsed) {
											resetIngestState();
										}
										setIsDragOver(true);
									}}
									onDragLeave={(e) => {
										e.preventDefault();
										setIsDragOver(false);
									}}
									onDrop={async (e) => {
										if (!canIngestFromDataTransfer(e.dataTransfer)) return;
										e.preventDefault();
										setIsDragOver(false);
										const payload = await extractPayloadFromDataTransfer(
											e.dataTransfer!,
										);
										if (payload) await handleIngestPayload(payload);
									}}
									onPaste={async (e) => {
										if (isDropZoneCollapsed) return;
										e.preventDefault();
										await ingestClipboardPayload(
											e.clipboardData?.getData("text/plain") ??
												e.clipboardData?.getData("text") ??
												null,
										);
									}}
									className="w-full"
								>
									<div
										className={`w-full ${
											isDropZoneCollapsed ? "h-10" : "h-[18vh]"
										} flex items-center justify-center gap-1 rounded-lg border border-dashed transition-all duration-300 ${
											isDropZoneCollapsed
												? "flex-row px-4 py-0 border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
												: "flex-col py-2 border-slate-300 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
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
												className={`${isDropZoneCollapsed ? "h-4 w-4" : "h-6 w-6"} animate-spin`}
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
										<p
											className={`text-sm ${
												ingestError
													? "text-red-600 dark:text-red-400"
													: isIngestSuccess
														? "text-green-600 dark:text-green-400"
														: isDragOver
															? "text-blue-600 dark:text-blue-400"
															: "text-slate-600 dark:text-slate-300"
											}`}
										>
											{ingestError || ingestMessage}
										</p>
										{!isDropZoneCollapsed && !ingestError && (
											<p className="mt-0 text-xs text-slate-400">
												{t("manual.ingest.tipPrefix", {
													defaultValue: "Tip: press",
												})}{" "}
												<kbd className="rounded bg-slate-200 px-1 text-[10px]">
													{t("manual.ingest.shortcut", {
														defaultValue: "Ctrl/Cmd + V",
													})}
												</kbd>{" "}
												{t("manual.ingest.tipSuffix", {
													defaultValue: "to paste instantly.",
												})}
											</p>
										)}
									</div>
								</button>
							</div>
						) : null}

						<div
							className={`relative z-0 ${contentPadding}`}
							onFocusCapture={handleContentFocus}
						>
							<Tabs
								value={uiActiveTab}
								onValueChange={(v) => setUiActiveTab(v as "core" | "meta")}
								className="h-full flex flex-col"
							>
								<TabsList className="grid w-full grid-cols-2">
									<TabsTrigger value="core">
										{t("manual.tabs.core", {
											defaultValue: "Core configuration",
										})}
									</TabsTrigger>
									<TabsTrigger value="meta">
										{t("manual.tabs.meta", {
											defaultValue: "Meta information",
										})}{" "}
										<sup>
											({t("manual.tabs.metaWip", { defaultValue: "WIP" })})
										</sup>
									</TabsTrigger>
								</TabsList>

								<TabsContent
									value="core"
									className="space-y-4 flex-1 min-h-0"
									onClick={handleFormInteraction}
								>
									<div className="space-y-4">
										{/* View Mode Toggle */}
										<div className="flex justify-end pt-2">
											<FormViewModeToggle
												mode={viewMode}
												onChange={handleModeChange}
												variant="compact"
											/>
										</div>

										{viewMode === "form" ? (
											<>
												<div className="space-y-4">
													<div className="flex items-center gap-4">
														<Label htmlFor={nameId} className="w-20 text-right">
															{t("manual.fields.name.label", {
																defaultValue: "Name",
															})}
														</Label>
														<div className="flex-1">
															<Input
																id={nameId}
																{...register("name")}
																placeholder={t(
																	"manual.fields.name.placeholder",
																	{
																		defaultValue: "e.g., local-mcp",
																	},
																)}
																readOnly={isEditMode}
																aria-readonly={isEditMode}
																title={
																	isEditMode
																		? t("manual.fields.name.readOnlyTitle", {
																				defaultValue:
																					"Editing server names is disabled",
																			})
																		: undefined
																}
																className={
																	isEditMode
																		? "cursor-not-allowed bg-muted text-muted-foreground"
																		: undefined
																}
															/>
															{errors.name && (
																<p className="text-xs text-red-500">
																	{errors.name.message}
																</p>
															)}
														</div>
													</div>
													<div className="flex items-center gap-4">
														<Label htmlFor={kindId} className="w-20 text-right">
															{t("manual.fields.type.label", {
																defaultValue: "Type",
															})}
														</Label>
														<div className="flex-1">
															<Segment
																options={SERVER_TYPE_OPTIONS}
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
																	{errors.kind.message}
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
													argFields={argFields.fields}
													envFields={envFields.fields}
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
													urlParamFields={paramFields.fields}
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
													headerFields={headerFields.fields}
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
														{t("manual.fields.json.label", {
															defaultValue: "Server JSON",
														})}
													</Label>
													<div className="flex-1 flex flex-col">
														<div className="group relative flex-1 min-h-[400px] border border-input rounded-md flex flex-col">
															{jsonText && (
																<div className="pointer-events-none absolute top-0 right-0 z-10 flex w-full justify-end p-2">
																	<Button
																		type="button"
																		variant="outline"
																		size="sm"
																		className="pointer-events-auto h-7 w-7 p-0 bg-white/95 backdrop-blur-sm opacity-0 shadow-sm transition-opacity group-hover:opacity-100 dark:bg-slate-900/95"
																		onClick={async (event) => {
																			event.stopPropagation();
																			await writeClipboardText(jsonText);
																		}}
																		title={t("manual.fields.json.copy", {
																			defaultValue: "Copy JSON",
																		})}
																	>
																		<Copy className="h-3.5 w-3.5" />
																	</Button>
																</div>
															)}
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
									</div>
								</TabsContent>

								<TabsContent
									value="meta"
									className="space-y-4 pt-2"
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
					</form>
				</div>
			);
		};

		// Preview items by name mapping (moved outside render function)
		const previewItemsByName = useMemo(() => {
			const map = new Map<string, Record<string, unknown>>();
			const items = (installPipeline.state.previewState as any)?.data?.items;
			if (Array.isArray(items)) {
				for (const entry of items) {
					if (entry && typeof entry === "object" && "name" in entry) {
						const name = (entry as { name?: unknown }).name;
						if (typeof name === "string") {
							map.set(name, entry as Record<string, unknown>);
						}
					}
				}
			}
			return map;
		}, [installPipeline.state.previewState]);

		// Expand/collapse details per draft (moved outside render function)
		const [expanded, setExpanded] = useState<Record<string, boolean>>({});
		const toggleExpanded = (name: string) =>
			setExpanded((e) => ({ ...e, [name]: !e[name] }));

		const renderPreviewStep = () => {
			const { state } = installPipeline;
			const { drafts, previewState, previewError, isPreviewLoading, source } =
				state;

			// Capability rendering helper
			const renderCapabilitySection = (
				kind: string,
				items: Record<string, unknown>[],
			) => {
				if (!items.length) return null;
				return (
					<div className="space-y-3">
						<div className="text-xs font-semibold text-slate-700 dark:text-slate-200 uppercase tracking-wide">
							{kind}
						</div>
						<div className="space-y-2.5">
							{items.map((item, idx) => {
								const name = String(
									(item as any).name ??
										(item as any).title ??
										`Item ${idx + 1}`,
								);
								const description = String(
									(item as any).description ?? (item as any).summary ?? "",
								);
								const uniqueKey = `${kind}-${name}-${idx}`;
								return (
									<div key={uniqueKey} className="text-sm leading-relaxed">
										<div className="font-semibold text-slate-800 dark:text-slate-100">
											{name}
										</div>
										{description && (
											<div className="text-slate-600 dark:text-slate-400 mt-1 text-xs leading-relaxed">
												{description}
											</div>
										)}
									</div>
								);
							})}
						</div>
					</div>
				);
			};

			// Helper to convert items to record list
			const asRecordList = (
				items: unknown[] | undefined,
			): Record<string, unknown>[] => {
				if (!Array.isArray(items)) return [];
				return items.filter(
					(item): item is Record<string, unknown> =>
						item !== null && typeof item === "object",
				);
			};

			return (
				<div className="px-4 py-4">
					<div className="space-y-4">
						{previewError ? (
							<Alert variant="destructive">
								<AlertTriangle className="h-4 w-4" />
								<AlertTitle>Preview failed</AlertTitle>
								<AlertDescription>{previewError}</AlertDescription>
							</Alert>
						) : null}

						{isPreviewLoading ? (
							<div className="flex items-center justify-center gap-2 rounded border border-slate-200 bg-white px-3 py-2 text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
								<Loader2 className="h-4 w-4 animate-spin" />{" "}
								{t("wizard.preview.generating", {
									defaultValue: "Generating capability preview…",
								})}
							</div>
						) : null}

						{previewState?.success === false && previewState?.error ? (
							<Alert variant="default">
								<AlertTriangle className="h-4 w-4" />
								<AlertTitle>Preview reported issues</AlertTitle>
								<AlertDescription>
									Some servers could not be contacted during preview. You can
									still proceed—the proxy will retry after installation.
								</AlertDescription>
							</Alert>
						) : null}

						<div className="space-y-3">
							{drafts.map((draft) => {
								const item = previewItemsByName.get(draft.name) as any;
								const ok = item?.ok !== false;
								const tools = asRecordList(item?.tools?.items as any);
								const resources = asRecordList(item?.resources?.items as any);
								const templates = asRecordList(
									item?.resource_templates?.items as any,
								);
								const prompts = asRecordList(item?.prompts?.items as any);
								const summaryParts = [] as string[];
								if (tools.length)
									summaryParts.push(
										`${tools.length} ${tools.length === 1 ? t("wizard.preview.capabilities.tool", { defaultValue: "tool" }) : t("wizard.preview.capabilities.tools", { defaultValue: "tools" })}`,
									);
								if (resources.length)
									summaryParts.push(
										`${resources.length} ${resources.length === 1 ? t("wizard.preview.capabilities.resource", { defaultValue: "resource" }) : t("wizard.preview.capabilities.resources", { defaultValue: "resources" })}`,
									);
								if (templates.length)
									summaryParts.push(
										`${templates.length} ${templates.length === 1 ? t("wizard.preview.capabilities.template", { defaultValue: "template" }) : t("wizard.preview.capabilities.templates", { defaultValue: "templates" })}`,
									);
								if (prompts.length)
									summaryParts.push(
										`${prompts.length} ${prompts.length === 1 ? t("wizard.preview.capabilities.prompt", { defaultValue: "prompt" }) : t("wizard.preview.capabilities.prompts", { defaultValue: "prompts" })}`,
									);
								const summaryText = summaryParts.join(" · ");
								const isOpen = !!expanded[draft.name];
								return (
									<div key={draft.name} className="rounded border px-4 py-3">
										<div className="flex items-center justify-between gap-2">
											<div className="flex items-center gap-2 min-w-0">
												<button
													type="button"
													className="p-0 text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
													onClick={() => toggleExpanded(draft.name)}
													onKeyDown={(e) => {
														if (e.key === "Enter" || e.key === " ") {
															e.preventDefault();
															toggleExpanded(draft.name);
														}
													}}
													aria-label={isOpen ? "Collapse" : "Expand"}
												>
													{isOpen ? (
														<ChevronDown className="h-4 w-4" />
													) : (
														<ChevronRight className="h-4 w-4" />
													)}
												</button>
												<div
													className="font-medium text-sm truncate"
													title={draft.name}
												>
													{draft.name}
													{summaryText ? (
														<span className="ml-2 text-xs text-slate-500">
															{summaryText}
														</span>
													) : null}
												</div>
											</div>
											<Badge variant="secondary" className="text-xs">
												{draft.kind}
											</Badge>
										</div>

										{!ok && item?.error ? (
											<div className="mt-2 text-xs text-red-500 break-words overflow-hidden">
												{String(item.error)}
											</div>
										) : null}

										{/* Details */}
										{isOpen ? (
											<div className="mt-4 pt-3 border-t space-y-5">
												{renderCapabilitySection("tools", tools)}
												{renderCapabilitySection("resources", resources)}
												{renderCapabilitySection("templates", templates)}
												{renderCapabilitySection("prompts", prompts)}
											</div>
										) : null}
									</div>
								);
							})}
						</div>
					</div>
				</div>
			);
		};

		const renderResultStep = () => {
			const { state } = installPipeline;
			const {
				importResult,
				isImporting,
				dryRunResult,
				isDryRunLoading,
				dryRunError,
				dryRunStats,
				dryRunWarning,
			} = state;
			const summary = importResult?.summary as
				| { imported_count?: number | null; skipped_count?: number | null }
				| undefined;
			const importedCount = summary?.imported_count ?? 0;
			const skippedCount = summary?.skipped_count ?? 0;
			const onlySkipped = importedCount === 0 && skippedCount > 0;

			// Get auto-add setting from store
			const autoAddToDefault =
				useAppStore.getState().dashboardSettings.autoAddServerToDefaultProfile;
			const selectedProfileName = autoAddToDefault ? "Default" : null;

			// Show ready state UI before import is completed
			const showReadyState = !importResult && !isImporting;

			// Determine if we can proceed with import based on dry-run
			const dryRunImportableCount = dryRunStats?.importedCount ?? 0;
			const dryRunSkippedCount = dryRunStats?.skippedCount ?? 0;
			const dryRunFailedCount = dryRunStats?.failedCount ?? 0;
			const canProceedWithImport =
				!isDryRunLoading &&
				!dryRunError &&
				dryRunResult &&
				dryRunImportableCount > 0;
			const validationOverviewItems = [
				{
					label: t("wizard.result.validation.ready", {
						defaultValue: "Ready to import",
					}),
					value: dryRunImportableCount,
					accent:
						dryRunImportableCount > 0
							? "text-green-600"
							: "text-muted-foreground",
				},
				{
					label: t("wizard.result.validation.skipped", {
						defaultValue: "Will be skipped",
					}),
					value: dryRunSkippedCount,
					accent:
						dryRunSkippedCount > 0 ? "text-amber-600" : "text-muted-foreground",
				},
				{
					label: t("wizard.result.validation.failed", {
						defaultValue: "Failed validation",
					}),
					value: dryRunFailedCount,
					accent:
						dryRunFailedCount > 0 ? "text-red-600" : "text-muted-foreground",
				},
			];

			const successSteps: Array<{ label: string; action: NextStepAction }> =
				selectedProfileName
					? [
							{
								label: t("wizard.result.success.close", {
									defaultValue:
										"Close this drawer to continue browsing or queue another server for import.",
								}),
								action: "close",
							},
							{
								label: t("wizard.result.success.servers", {
									defaultValue:
										"Open the Servers dashboard to review and manage the new server.",
								}),
								action: "servers",
							},
							{
								label: t("wizard.result.success.profilesWithName", {
									profile: selectedProfileName,
									defaultValue:
										'Open Profiles to verify "{{profile}}" reflects the new server.',
								}),
								action: "profiles",
							},
						]
					: [
							{
								label: t("wizard.result.success.close", {
									defaultValue:
										"Close this drawer to continue browsing or queue another server for import.",
								}),
								action: "close",
							},
							{
								label: t("wizard.result.success.servers", {
									defaultValue:
										"Open the Servers dashboard to review and manage the new server.",
								}),
								action: "servers",
							},
							{
								label: t("wizard.result.success.profiles", {
									defaultValue:
										"Visit Profiles to add this server to the appropriate activation sets.",
								}),
								action: "profiles",
							},
						];
			const failureSteps: Array<{ label: string; action: NextStepAction }> = [
				{
					label: t("wizard.result.failure.adjustServers", {
						defaultValue:
							"Return to the Servers dashboard to adjust or remove the configuration before retrying.",
					}),
					action: "servers",
				},
				{
					label: t("wizard.result.failure.reviewPreview", {
						defaultValue:
							"Review the preview output above for errors and apply the necessary fixes before confirming again.",
					}),
					action: "preview",
				},
				{
					label: t("wizard.result.failure.rerunPreview", {
						defaultValue:
							"Keep this drawer open, update the configuration, and rerun Preview before another import attempt.",
					}),
					action: "preview",
				},
			];

			const renderNextSteps = (
				items: Array<{ label: string; action: NextStepAction }>,
			) => (
				<div className="space-y-2">
					<h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
						{t("wizard.result.nextSteps.title", { defaultValue: "Next steps" })}
					</h4>
					<ul className="space-y-1 text-sm text-slate-600 dark:text-slate-300">
						{items.map(({ label, action }) => {
							const interactive = action !== "none";
							return (
								<li key={label} className="flex items-start gap-2">
									{interactive ? (
										<button
											type="button"
											onClick={() => handleNextStepAction(action)}
											className="group flex items-start gap-2 text-left text-slate-600 hover:text-primary focus:outline-none dark:text-slate-300"
										>
											<ChevronRight className="mt-1 h-3 w-3 text-slate-400 group-hover:text-primary" />
											<span className="underline decoration-dotted underline-offset-2">
												{label}
											</span>
										</button>
									) : (
										<div className="flex items-start gap-2">
											<ChevronRight className="mt-1 h-3 w-3 text-slate-400" />
											<span>{label}</span>
										</div>
									)}
								</li>
							);
						})}
					</ul>
				</div>
			);

			const readySteps: Array<{ label: string; action: NextStepAction }> = [
				{
					label: t("wizard.result.readySteps.reviewConfig", {
						defaultValue:
							"Review the server configuration and capabilities from the previous step.",
					}),
					action: "none",
				},
				{
					label: autoAddToDefault
						? t("wizard.result.readySteps.autoAdd", {
								defaultValue:
									"The server will be automatically added to the Default profile based on your settings.",
							})
						: t("wizard.result.readySteps.manualAssign", {
								defaultValue:
									"The server will remain unassigned. You can add it to profiles later from the Profiles page.",
							}),
					action: "none",
				},
				{
					label: t("wizard.result.readySteps.importAction", {
						defaultValue:
							"Click the Import button below to install the server to your system.",
					}),
					action: "none",
				},
			];

			return (
				<div className="flex flex-col">
					<div className="p-4 space-y-4">
						{showReadyState ? (
							isDryRunLoading ? (
								<div className="flex items-center justify-center gap-2 rounded border border-slate-200 bg-white px-3 py-2 text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
									<Loader2 className="h-4 w-4 animate-spin" />
									{t("wizard.result.validating", {
										defaultValue: "Validating import...",
									})}
								</div>
							) : (
								<div className="rounded-lg border p-4 space-y-4">
									{(() => {
										let badgeColor = "bg-green-500";
										let statusTitle = t("wizard.result.validatedTitle", {
											defaultValue: "Import Validated",
										});
										let detailMessage: string | null = null;
										let detailTone: "error" | "warning" | "muted" = "muted";
										if (dryRunError) {
											badgeColor = "bg-red-500";
											statusTitle = t("wizard.result.validationFailedTitle", {
												defaultValue: "Import Validation Failed",
											});
											detailMessage = dryRunError;
											detailTone = "error";
										} else if (canProceedWithImport) {
											if (dryRunWarning) {
												badgeColor = "bg-amber-500";
												statusTitle = t(
													"wizard.result.validatedWithWarningsTitle",
													{
														defaultValue: "Import Validated With Warnings",
													},
												);
												detailMessage = dryRunWarning;
												detailTone = "warning";
											}
										} else {
											badgeColor = "bg-amber-500";
											statusTitle = t("wizard.result.alreadyInstalledTitle", {
												defaultValue: "Already Installed",
											});
											detailMessage = dryRunWarning;
											detailTone = "warning";
										}

										const detailClass =
											detailTone === "error"
												? "text-sm text-red-600"
												: detailTone === "warning"
													? "text-sm text-amber-600"
													: "text-sm text-muted-foreground";

										const skippedOnly =
											!dryRunError &&
											!canProceedWithImport &&
											dryRunSkippedCount > 0;

										const nextSteps = dryRunError
											? failureSteps
											: canProceedWithImport
												? readySteps
												: skippedOnly
													? [
															{
																label: t(
																	"wizard.result.skipSteps.useExisting",
																	{
																		defaultValue:
																			"Close this drawer and start using the existing server.",
																	},
																),
																action: "close" as NextStepAction,
															},
															{
																label: t(
																	"wizard.result.skipSteps.chooseAnother",
																	{
																		defaultValue:
																			"Go back to the previous step to choose a different server if needed.",
																	},
																),
																action: "preview" as NextStepAction,
															},
														]
													: readySteps;

										return (
											<>
												<div className="flex items-center gap-2">
													<span
														className={`font-medium ${detailTone === "warning" && !dryRunError ? "text-amber-600" : detailTone === "error" ? "text-red-600" : "text-green-600"}`}
													>
														{statusTitle}
													</span>
												</div>
												<div className="space-y-3">
													<div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
														{validationOverviewItems.map(
															({ label, value, accent }) => (
																<div
																	key={label}
																	className="rounded-md border p-3"
																>
																	<div className="text-xs font-semibold uppercase text-muted-foreground">
																		{label}
																	</div>
																	<div
																		className={`text-2xl font-bold ${accent}`}
																	>
																		{value}
																	</div>
																</div>
															),
														)}
													</div>
													{detailMessage ? (
														<p className={detailClass}>{detailMessage}</p>
													) : null}
													{renderNextSteps(nextSteps)}
												</div>
											</>
										);
									})()}
								</div>
							)
						) : isImporting ? (
							<div className="flex items-center justify-center gap-2 rounded border border-slate-200 bg-white px-3 py-2 text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
								<Loader2 className="h-4 w-4 animate-spin" />
								{t("wizard.result.importingStatus", {
									defaultValue: "Importing servers…",
								})}
							</div>
						) : importResult ? (
							<div className="space-y-4">
								{/* Success/Error Status */}
								<div className="rounded-lg border p-4">
									<div className="flex items-center gap-2 mb-2">
										{importResult.success !== false ? (
											<div className="h-2 w-2 rounded-full bg-green-500" />
										) : (
											<div className="h-2 w-2 rounded-full bg-red-500" />
										)}
										<span className="font-medium">
											{importResult.success !== false
												? t("wizard.result.successTitle", {
														defaultValue: "Import Successful",
													})
												: t("wizard.result.failureTitle", {
														defaultValue: "Import Failed",
													})}
										</span>
									</div>
									{importResult.success !== false ? (
										<>
											<p className="text-sm text-muted-foreground">
												{onlySkipped
													? t("wizard.result.successAllSkipped", {
															defaultValue:
																"All selected servers were already installed. No changes were applied.",
														})
													: t("wizard.result.successInstalled", {
															defaultValue:
																"The server has been successfully installed and is ready to use.",
														})}
											</p>
											{selectedProfileName ? (
												<p className="mt-2 text-xs text-muted-foreground">
													{t("wizard.result.successAutoEnabled", {
														profile: selectedProfileName,
														defaultValue:
															'Enabled automatically in "{{profile}}".',
													})}
												</p>
											) : null}
										</>
									) : (
										<p className="text-sm text-red-600">
											{importResult.error ||
												t("wizard.result.failureGeneric", {
													defaultValue: "An error occurred during import",
												})}
										</p>
									)}
								</div>

								{/* Import Statistics */}
								{importResult.summary && (
									<div className="grid grid-cols-2 gap-4">
										<div className="rounded-lg border p-3">
											<div className="text-sm font-medium text-muted-foreground">
												{t("wizard.result.stats.imported", {
													defaultValue: "Imported",
												})}
											</div>
											<div className="text-2xl font-bold text-green-600">
												{importResult.summary.imported_count || 0}
											</div>
										</div>
										<div className="rounded-lg border p-3">
											<div className="text-sm font-medium text-muted-foreground">
												{t("wizard.result.stats.skipped", {
													defaultValue: "Skipped",
												})}
											</div>
											<div className="text-2xl font-bold text-yellow-600">
												{importResult.summary.skipped_count || 0}
											</div>
										</div>
									</div>
								)}

								{/* Server Details */}
								{importResult.servers && (
									<div className="space-y-2">
										<h4 className="font-medium">
											{t("wizard.result.installedServersTitle", {
												defaultValue: "Installed Servers",
											})}
										</h4>
										<div className="space-y-2">
											{Object.entries(
												importResult.servers as Record<string, any>,
											).map(
												([name, server]: [string, Record<string, unknown>]) => {
													const status = String(
														(server as any)?.status ?? "unknown",
													);
													return (
														<div
															key={name}
															className="flex items-center justify-between rounded border p-2"
														>
															<span className="font-medium">{name}</span>
															<Badge
																variant={
																	status === "success"
																		? "default"
																		: "destructive"
																}
															>
																{status}
															</Badge>
														</div>
													);
												},
											)}
										</div>
									</div>
								)}

								{renderNextSteps(
									importResult.success !== false ? successSteps : failureSteps,
								)}
							</div>
						) : (
							<div className="flex items-center justify-center h-full">
								<div className="text-center">
									<div className="text-lg font-medium mb-2">
										{t("wizard.result.readyTitle", {
											defaultValue: "Ready to Import",
										})}
									</div>
									<div className="text-sm text-muted-foreground">
										{t("wizard.result.readyDescription", {
											defaultValue:
												"Click the Import button to proceed with installation",
										})}
									</div>
								</div>
							</div>
						)}
					</div>
				</div>
			);
		};

		return (
			<Drawer
				open={isOpen}
				onOpenChange={(open) => !open && handleOverlayClose()}
			>
				<DrawerContent className="h-full flex flex-col">
					<DrawerHeader>
						<DrawerTitle className="flex items-center gap-2">
							{isEditMode
								? t("wizard.header.editTitle", { defaultValue: "Edit Server" })
								: t("wizard.header.addTitle", {
										defaultValue: "Add MCP Server",
									})}
						</DrawerTitle>
						<DrawerDescription>
							{isEditMode
								? t("wizard.header.editDescription", {
										defaultValue: "Update server configuration",
									})
								: t("wizard.header.addDescription", {
										defaultValue: "Configure and install a new MCP server",
									})}
						</DrawerDescription>
					</DrawerHeader>

					{/* Step Navigation */}
					<div className="relative z-10 p-4 pb-0 bg-background">
						<div className="flex items-center justify-between gap-2">
							<div className="flex items-center gap-2">
								{steps.map((step, index) => {
									const isActive = currentStep === step.id;
									const canNavigate = canNavigateToStep(step.id);

									return (
										<div key={step.id} className="flex items-center gap-2">
											<button
												type="button"
												onClick={() => handleStepChange(step.id)}
												disabled={!canNavigate || isSubmitting}
												className={`flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold transition-colors ${
													isActive
														? "bg-primary text-primary-foreground"
														: canNavigate
															? "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700 cursor-pointer"
															: "bg-slate-100 text-slate-400 dark:bg-slate-900 dark:text-slate-500 cursor-not-allowed"
												}`}
											>
												{index + 1}
											</button>
											<button
												type="button"
												onClick={() => handleStepChange(step.id)}
												disabled={!canNavigate || isSubmitting}
												className="flex flex-col text-left transition-colors hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50"
											>
												<span
													className={`text-sm font-medium ${
														isActive
															? "text-primary"
															: canNavigate
																? "text-slate-600 dark:text-slate-300"
																: "text-slate-400 dark:text-slate-500"
													}`}
												>
													{step.label}
												</span>
												<span className="text-xs text-muted-foreground">
													{step.hint}
												</span>
											</button>
											{index < steps.length - 1 && (
												<span className="hidden h-px w-10 bg-slate-200 md:block dark:bg-slate-800" />
											)}
										</div>
									);
								})}
							</div>
							{/* Refresh button for preview step - only show after preview is completed */}
							{currentStep === "preview" &&
								installPipeline.state.previewState !== null &&
								!installPipeline.state.isPreviewLoading && (
									<Button
										variant="ghost"
										className="h-9 w-9 p-0"
										aria-label={t("wizard.preview.retry", {
											defaultValue: "Retry preview",
										})}
										title={t("wizard.preview.retry", {
											defaultValue: "Retry preview",
										})}
										disabled={installPipeline.state.isImporting}
										onClick={() => {
											const { drafts, source } = installPipeline.state;
											installPipeline.setPreviewState(null);
											installPipeline.begin(drafts, source ?? "manual");
										}}
									>
										<RefreshCw className="h-4 w-4" />
									</Button>
								)}
						</div>
					</div>

					{/* Step Content - with spacing and bottom padding to avoid footer overlap */}
					<div className="flex-1 min-h-0 overflow-y-auto py-2 pb-20">
						{renderStepContent()}
					</div>

					{/* Footer - fixed at bottom with subtle shadow for separation */}
					<DrawerFooter className="absolute bottom-0 left-0 right-0 z-10 border-t p-4 bg-background">
						<div className="flex w-full items-center justify-between gap-3">
							{currentStep === "result" &&
							installPipeline.state.importResult ? (
								<div />
							) : (
								<Button
									type="button"
									variant="outline"
									onClick={
										currentStep === "preview"
											? () => handleStepChange("form")
											: currentStep === "result"
												? () => handleStepChange("preview")
												: handleCancelClose
									}
									disabled={
										isSubmitting ||
										(currentStep === "result" &&
											installPipeline.state.isImporting)
									}
								>
									{currentStep === "preview" || currentStep === "result"
										? t("wizard.buttons.back", { defaultValue: "Back" })
										: t("wizard.buttons.cancel", { defaultValue: "Cancel" })}
								</Button>
							)}
							<div className="flex gap-2">
								{currentStep === "form" && (
									<Button
										type="button"
										onClick={() => handlePreview()}
										disabled={isSubmitting || !canNavigateToStep("preview")}
									>
										{isSubmitting ? (
											<>
												<Loader2 className="mr-2 h-4 w-4 animate-spin" />
												{t("wizard.buttons.previewing", {
													defaultValue: "Previewing...",
												})}
											</>
										) : (
											t("wizard.buttons.preview", { defaultValue: "Preview" })
										)}
									</Button>
								)}
								{currentStep === "preview" && (
									<Button
										type="button"
										onClick={() => handleStepChange("result")}
										disabled={isSubmitting || !canNavigateToStep("result")}
									>
										{t("wizard.buttons.next", { defaultValue: "Next" })}
									</Button>
								)}
								{currentStep === "result" &&
									!installPipeline.state.importResult && (
										<Button
											type="button"
											onClick={handleImport}
											disabled={
												installPipeline.state.isImporting ||
												installPipeline.state.isDryRunLoading ||
												!!installPipeline.state.dryRunError ||
												!(
													installPipeline.state.dryRunStats &&
													installPipeline.state.dryRunStats.importedCount > 0
												)
											}
										>
											{installPipeline.state.isImporting ? (
												<>
													<Loader2 className="mr-2 h-4 w-4 animate-spin" />
													{t("wizard.buttons.importing", {
														defaultValue: "Importing...",
													})}
												</>
											) : installPipeline.state.isDryRunLoading ? (
												<>
													<Loader2 className="mr-2 h-4 w-4 animate-spin" />
													{t("wizard.buttons.validating", {
														defaultValue: "Validating...",
													})}
												</>
											) : (
												t("wizard.buttons.import", { defaultValue: "Import" })
											)}
										</Button>
									)}
								{currentStep === "result" &&
									installPipeline.state.importResult && (
										<Button type="button" onClick={handleOverlayClose}>
											{t("wizard.buttons.done", { defaultValue: "Done" })}
										</Button>
									)}
							</div>
						</div>
					</DrawerFooter>
				</DrawerContent>
			</Drawer>
		);
	},
);

ServerInstallWizard.displayName = "ServerInstallWizard";
