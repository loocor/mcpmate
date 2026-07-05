import { zodResolver } from "@hookform/resolvers/zod";
import { useQueryClient } from "@tanstack/react-query";
import {
	AlertTriangle,
	ArrowLeft,
	ChevronRight,
	Loader2,
	Radar,
	RefreshCw,
	RotateCcw,
} from "lucide-react";
import type { FocusEvent, MouseEvent } from "react";
import {
	forwardRef,
	useCallback,
	useEffect,
	useId,
	useImperativeHandle,
	useLayoutEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { useFieldArray, useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { clientsApi, serversApi } from "../../lib/api";
import {
	resolveAutoAddTargetProfileId,
	useAutoAddTargetProfile,
} from "../../lib/default-profile";
import { startOAuthAccessFlow } from "../../lib/oauth-callback-access";
import {
	type InstallSource,
	type ServerInstallDraft,
	useServerInstallPipeline,
	type WizardStep,
} from "../../hooks/use-server-install-pipeline";
import { readClipboardText } from "../../lib/clipboard";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError } from "../../lib/notify";
import { cn, toTitleCase } from "../../lib/utils";
import { onboardingApi } from "../../lib/onboarding-api";
import {
	canIngestFromDataTransfer,
	extractPayloadFromDataTransfer,
	formatServerUniImportTransferError,
} from "../../lib/server-uni-import-transfer";
import {
	compactKeyValueFields,
	shouldAppendKeyValueRow,
} from "../../lib/key-value-fields";
import { useAppStore } from "../../lib/store";
import type { ClientInfo, SecretOrigin } from "../../lib/types";
import type { CapabilityRecord } from "../../types/capabilities";
import CapabilityList from "../capability-list";
import { CapabilityToolbar } from "../capability-toolbar";
import {
	CapabilityPreviewList,
	type CapabilityPreviewFlatItem,
	type CapabilityPreviewKind,
} from "../capability-preview-list";
import {
	InlineSecretCreate,
	useInlineSecretCreateField,
} from "../secrets";
import {
	BulkSelectionCheckbox,
	BulkSelectionHeader,
	buildIncludeExcludeBulkActions,
	useBulkSelectionLabels,
	useBulkSelection,
} from "../bulk-selection";
import CapabilityCombobox from "../capability-combobox";
import { CachedAvatar } from "../cached-avatar";
import { CardListScrollBody } from "../card-list-scroll-body";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../capsule-stripe-list";
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
import { Spinner } from "../ui/spinner";
import { Switch } from "../ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import {
	CommandField,
	HttpHeaders,
	MetaFields,
	StdioAdvanced,
	UrlParams,
} from "./form-fields";
import { ServerAuthSection } from "./server-auth-section";
import {
	buildImportValidationItems,
	ImportValidationSummary,
} from "./import-validation-summary";
import { draftToServerConfig } from "./draft-to-server-config";
import {
	draftToFormState,
	useFormState,
	useFormSync,
	useIngest,
	useSecretFieldInsert,
	useServerTypeOptions,
} from "./hooks";
import {
	FORM_FILL_SHELL_CLASS,
	FORM_TAB_SHELL_CLASS,
	INSTALL_DRAWER_CONTENT_CLASS,
	installFormBodyClass,
	isCoreJsonView,
	SECONDARY_TAB_CONTENT_CLASS,
} from "./form-tab-layout";
import { CoreConfigTabPanel } from "./core-config-tab-panel";
import { SERVER_INSTALL_FORM_ROW_LABEL_CLASS } from "./field-list";
import { ServerConfigJsonPanel } from "./server-config-json-panel";
import {
	breathingAnimation,
	DEFAULT_INGEST_MESSAGE,
	type ManualServerFormValues,
	manualServerSchema,
	type ServerInstallManualFormHandle,
} from "./types";

// Step definitions

const STEP_ORDER: WizardStep[] = ["form", "preview", "result"];

type BulkDraftView = "list" | "detail";

type ImportPreviewFlatCapabilityItem = CapabilityRecord & {
	__importCapabilityKind: CapabilityPreviewKind;
};

type ImportPreviewCapabilityGroup = {
	items?: unknown[];
};

type ImportPreviewItem = {
	name?: unknown;
	ok?: boolean;
	error?: unknown;
	tools?: ImportPreviewCapabilityGroup;
	resources?: ImportPreviewCapabilityGroup;
	resource_templates?: ImportPreviewCapabilityGroup;
	prompts?: ImportPreviewCapabilityGroup;
};

const IMPORT_PREVIEW_KIND_ORDER: CapabilityPreviewKind[] = [
	"tools",
	"resources",
	"templates",
	"prompts",
];

function importPreviewKindDefaultLabel(kind: CapabilityPreviewKind): string {
	if (kind === "templates") {
		return "Resource Templates";
	}
	return toTitleCase(kind);
}

function draftEndpointSummary(draft: ServerInstallDraft): string {
	if (draft.kind === "stdio") {
		return [draft.command, ...(draft.args ?? [])].filter(Boolean).join(" ");
	}
	return draft.url ?? "";
}

function importPreviewCapabilityItemId(
	item: ImportPreviewFlatCapabilityItem,
): string {
	const record = item as CapabilityRecord;
	const identifier =
		record.unique_name ??
		record.tool_name ??
		record.prompt_name ??
		record.resource_uri ??
		record.uri ??
		record.uriTemplate ??
		record.uri_template ??
		record.name;
	return `${item.__importCapabilityKind}:${String(identifier ?? "capability")}`;
}

function draftListAvatar(draft: ServerInstallDraft) {
	const draftIcon = draft.meta?.icons?.[0]?.src;
	const avatarFallback = (draft.name || "S").slice(0, 1).toUpperCase();
	return (
		<CachedAvatar
			src={draftIcon}
			alt={draft.name ? `${draft.name} icon` : undefined}
			fallback={avatarFallback}
			size="sm"
			shape="rounded"
			className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
		/>
	);
}

function clientHasScannableConfig(client: ClientInfo): boolean {
	return Boolean(client.detected && client.config_path?.trim());
}

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
		// Live snapshot of the "Auto Add To Default Profile" setting and the
		// real default-anchor profile name (when enabled). Used to render
		// accurate result-step labels — the imperative resolve in
		// `handleImport` remains the source of truth for the API call.
		const autoAddTargetProfile = useAutoAddTargetProfile();
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
		const [bulkDraftView, setBulkDraftView] = useState<BulkDraftView>("list");
		const bulkSelection = useBulkSelection<string>();
		const { bulkModeDescription } = useBulkSelectionLabels();
		const { serverTypeOptions, transportLabel } = useServerTypeOptions();
		const [activeDraftName, setActiveDraftName] = useState<string | null>(null);
		const [activePreviewName, setActivePreviewName] = useState<string | null>(
			null,
		);
		const [isLocalScanLoading, setLocalScanLoading] = useState(false);

		const resetBulkUIState = useCallback(() => {
			setBulkDraftView("list");
			bulkSelection.exitBulkMode();
			setActiveDraftName(null);
			setActivePreviewName(null);
		}, [bulkSelection]);

		const [capabilitySearch, setCapabilitySearch] = useState("");
		const [capabilityKindFilters, setCapabilityKindFilters] = useState<
			CapabilityPreviewKind[]
		>([]);
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
		const internalPipeline = useServerInstallPipeline();
		const installPipeline = externalPipeline ?? internalPipeline;
		const currentStep = installPipeline.state.currentStep ?? "form";
		const navigate = useNavigate();
		const queryClient = useQueryClient();

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

		const handleSecretSelect = useSecretFieldInsert(getValues, setValue);

		const { onCreateSecret, controller } =
			useInlineSecretCreateField(handleSecretSelect);

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
			const current = getValues("env") ?? [];
			if (!shouldAppendKeyValueRow(current)) return;
			envFields.append({ key: "", value: "" });
		}, [envFields, getValues]);

		const removeEnv = useCallback(
			(index: number) => {
				envFields.remove(index);
				queueMicrotask(() => {
					const current = getValues("env") ?? [];
					const compacted = compactKeyValueFields(current);
					if (compacted.length !== current.length) {
						envFields.replace(compacted);
					}
				});
			},
			[envFields, getValues],
		);

		const appendHeader = useCallback(() => {
			const current = getValues("headers") ?? [];
			if (!shouldAppendKeyValueRow(current)) return;
			headerFields.append({ key: "", value: "" });
		}, [headerFields, getValues]);

		const removeHeader = useCallback(
			(index: number) => {
				headerFields.remove(index);
				queueMicrotask(() => {
					const current = getValues("headers") ?? [];
					const compacted = compactKeyValueFields(current);
					if (compacted.length !== current.length) {
						headerFields.replace(compacted);
					}
				});
			},
			[headerFields, getValues],
		);

		const appendUrlParam = useCallback(() => {
			const current = getValues("urlParams") ?? [];
			if (!shouldAppendKeyValueRow(current)) return;
			paramFields.append({ key: "", value: "" });
		}, [paramFields, getValues]);

		const removeUrlParam = useCallback(
			(index: number) => {
				paramFields.remove(index);
				queueMicrotask(() => {
					const current = getValues("urlParams") ?? [];
					const compacted = compactKeyValueFields(current);
					if (compacted.length !== current.length) {
						paramFields.replace(compacted);
					}
				});
			},
			[paramFields, getValues],
		);

		// Form refs
		const dropZoneRef = useRef<HTMLDivElement | null>(null);
		// Form field IDs
		const nameId = useId();
		const kindId = useId();
		const commandId = useId();
		const urlId = useId();
		const manualJsonId = useId();
		const metaIconUrlId = useId();
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

		const handleModeChange = useCallback(
			(mode: "form" | "json") => {
				setViewMode(mode);
			},
			[setViewMode],
		);

		// Type snapshot management (for form state restoration)

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
		const watchedMetaIconUrl = watch("meta_icon_url");
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
		const watchedUrlParams = watch("urlParams");
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
		const importInFlightRef = useRef(false);
		const wizardSessionEpochRef = useRef(0);
		const pendingImportServerRef = useRef<string | null>(null);
		const [pendingImportServerId, setPendingImportServerId] =
			useState<string | null>(null);
		const [isImportActionPending, setIsImportActionPending] = useState(false);
		const [selectedAuthMode, setSelectedAuthMode] =
			useState<"header" | "oauth">("header");
		const suggestedAuthMode = useMemo<"header" | "oauth">(() => {
			if (!isImportMode || isStdio) {
				return "header";
			}
			const hasAuthorizationHeader = (watchedHeaders ?? []).some((entry) => {
				const key = typeof entry?.key === "string" ? entry.key.trim().toLowerCase() : "";
				return key === "authorization";
			});
			return hasAuthorizationHeader ? "header" : "oauth";
		}, [isImportMode, isStdio, watchedHeaders]);

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

		const cleanupPendingImportServer = useCallback(() => {
			const pendingId = pendingImportServerRef.current;
			if (!pendingId) {
				return;
			}
			pendingImportServerRef.current = null;
			setPendingImportServerId(null);
			void serversApi.deleteServer(pendingId).catch(() => { });
		}, []);

		const clearPendingImportState = useCallback(() => {
			const publishedServerId = pendingImportServerRef.current;
			if (!publishedServerId) {
				return null;
			}
			pendingImportServerRef.current = null;
			setPendingImportServerId(null);
			return publishedServerId;
		}, []);

		const resolveImportTargetProfileId = useCallback(async () => {
			const autoAddTargetProfileId = await resolveAutoAddTargetProfileId({
				autoAddEnabled:
					useAppStore.getState().dashboardSettings.autoAddServerToDefaultProfile,
			});
			return installPipeline.state.targetProfileId ?? autoAddTargetProfileId;
		}, [installPipeline.state.targetProfileId]);

		const completePendingPublishImport = useCallback(
			async (
				draft: ServerInstallDraft,
				publishedServerId: string,
				targetProfileId: string | null,
			) => {
				await serversApi.updateServer(
					publishedServerId,
					draftToServerConfig(draft, {
						enabled: true,
						pending_import: false,
						profile_ids: targetProfileId ? [targetProfileId] : undefined,
					}),
				);
				await queryClient.invalidateQueries({ queryKey: ["servers"] });
				if (targetProfileId) {
					await queryClient.invalidateQueries({
						queryKey: ["configSuits"],
					});
				}
				installPipeline.setImportResult({
					success: true,
					summary: {
						imported_count: 1,
						skipped_count: 0,
					},
					servers: {
						[draft.name]: {
							id: publishedServerId,
							status: "success",
						},
					},
				});
			},
			[queryClient, installPipeline],
		);

		const runImportPipeline = useCallback(
			async (targetProfileId: string | null) => {
				const didSucceed = await installPipeline.confirmImport(targetProfileId);
				if (!didSucceed) {
					return false;
				}
				if (targetProfileId) {
					await queryClient.invalidateQueries({
						queryKey: ["configSuits"],
					});
				}
				return true;
			},
			[installPipeline, queryClient],
		);

		const tryFinalizePublishImport = useCallback(
			async (draft: ServerInstallDraft, targetProfileId: string | null) => {
				const publishedServerId = pendingImportServerRef.current;
				if (!publishedServerId) {
					return false;
				}
				try {
					await completePendingPublishImport(draft, publishedServerId, targetProfileId);
					clearPendingImportState();
				} catch (error) {
					pendingImportServerRef.current = publishedServerId;
					setPendingImportServerId(publishedServerId);
					throw error;
				}
				return true;
			},
			[completePendingPublishImport, clearPendingImportState],
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
				const iconUrl = trim(values.meta_icon_url);
				if (iconUrl) meta.icons = [{ src: iconUrl }];

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

		const formStateFromDraft = useCallback(
			(draft: ServerInstallDraft) => {
				return draftToFormState(draft);
			},
			[],
		);

		const loadDraftIntoForm = useCallback(
			(draft: ServerInstallDraft) => {
				const nextState = formStateFromDraft(draft);
				formStateRef.current = nextState;
				reset(buildFormValuesFromState(nextState));
				setViewMode("form");
				setJsonError(null);
				setUiActiveTab("core");
			},
			[
				buildFormValuesFromState,
				formStateFromDraft,
				formStateRef,
				reset,
				setJsonError,
				setViewMode,
			],
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

		const secretOriginBase = useMemo<SecretOrigin>(
			() => ({
				server_id: pendingImportServerId,
				server_name: watchedName?.trim() || null,
				server_kind: kind,
				source: isEditMode ? "server_edit" : "server_install",
			}),
			[isEditMode, kind, pendingImportServerId, watchedName],
		);

		const { saveTypeSnapshot, restoreTypeSnapshot } = useFormSync({
			formStateRef,
			isRestoringRef,
			kind,
			watchedName,
			watchedMetaDescription,
			watchedMetaIconUrl,
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
			setIngestMessage,
			ingestError,
			setIngestError,
			isIngestSuccess,
			isDropZoneCollapsed,
			isDragOver,
			setIsDragOver,
			setIsDropZoneCollapsed,
			resetIngestState,
			markIngestSuccess,
			handleIngestPayload,
		} = useIngest({
			ingestEnabled,
			allowProgrammaticIngest,
			formStateRef,
			buildFormValuesFromState,
			reset,
			sessionEpochRef: wizardSessionEpochRef,
			onSubmitMultiple: (drafts) => {
				if (onPreview) {
					onPreview(drafts);
				} else {
					installPipeline.setDraftCollection(drafts, "ingest");
					installPipeline.setCurrentStep("form");
					resetBulkUIState();
				}
			},
			messages: ingestMessages,
		});

		const resetWizardSession = useCallback(() => {
			wizardSessionEpochRef.current += 1;
			previewInFlightRef.current = false;
			installPipeline.setCurrentStep("form");
			installPipeline.reset();
			const initialFormState = createInitialFormState();
			formStateRef.current = initialFormState;
			isRestoringRef.current = true;
			reset(buildFormValuesFromState(initialFormState));
			isRestoringRef.current = false;
			resetIngestState();
			setUiActiveTab("core");
			setViewMode("form");
			setJsonError(null);
			resetBulkUIState();
			setPendingImportServerId(null);
		}, [
			installPipeline,
			createInitialFormState,
			formStateRef,
			isRestoringRef,
			reset,
			buildFormValuesFromState,
			resetIngestState,
			setViewMode,
			resetBulkUIState,
		]);

		const handleResetForm = useCallback(() => {
			if (initialDraft) {
				loadDraftIntoForm(initialDraft);
			} else {
				const initial = createInitialFormState();
				formStateRef.current = initial;
				isRestoringRef.current = true;
				reset(buildFormValuesFromState(initial));
				isRestoringRef.current = false;
				setViewMode("form");
				setUiActiveTab("core");
				setJsonError(null);
			}

			resetIngestState();
			resetBulkUIState();

			if (installPipeline.state.drafts.length > 0) {
				installPipeline.setDraftCollection([], null);
			}
			installPipeline.setPreviewState(null);
			installPipeline.setPreviewError(null);
		}, [
			initialDraft,
			loadDraftIntoForm,
			createInitialFormState,
			formStateRef,
			isRestoringRef,
			reset,
			buildFormValuesFromState,
			setViewMode,
			setJsonError,
			resetIngestState,
			installPipeline,
			resetBulkUIState,
		]);

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

		const handleLocalConfigScan = useCallback(async () => {
			if (
				!ingestEnabled ||
				isDropZoneCollapsed ||
				isLocalScanLoading ||
				isIngesting
			) {
				return;
			}
			try {
				setLocalScanLoading(true);
				const clientsResp = await clientsApi.detect(true);
				const scannableClients = (clientsResp?.client ?? []).filter(
					clientHasScannableConfig,
				);
				if (!scannableClients.length) {
					notifyError(
						t("manual.scan.noneTitle", {
							defaultValue: "No local configs found",
						}),
						t("manual.scan.noneDescription", {
							defaultValue:
								"No detected MCP clients have a local configuration file to scan.",
						}),
					);
					return;
				}

				const scanResp = await onboardingApi.scanServers(
					scannableClients.map((client) => ({
						identifier: client.identifier,
						display_name: client.display_name || client.identifier,
						config_path: client.config_path,
						config_file_parse:
							client.config_file_parse_override ??
							client.config_file_parse_effective ??
							null,
					})),
				);
				if (!scanResp.success || !scanResp.data) {
					throw new Error(
						String(scanResp.error?.message ?? "Local config scan failed"),
					);
				}

				const drafts: ServerInstallDraft[] = scanResp.data.candidates.map(
					(candidate) => ({
						name: candidate.name,
						kind:
							candidate.kind === "sse" ||
								candidate.kind === "streamable_http"
								? candidate.kind
								: "stdio",
						command:
							candidate.kind === "stdio"
								? candidate.command ?? undefined
								: undefined,
						args: candidate.args?.length ? candidate.args : undefined,
						env:
							candidate.env && Object.keys(candidate.env).length
								? candidate.env
								: undefined,
						url:
							candidate.kind !== "stdio" && candidate.url
								? candidate.url
								: undefined,
					}),
				);
				if (!drafts.length) {
					notifyError(
						t("manual.scan.noServersTitle", {
							defaultValue: "No servers detected",
						}),
						t("manual.scan.noServersDescription", {
							defaultValue:
								"The local scan did not find importable MCP server entries.",
						}),
					);
					return;
				}

				if (drafts.length === 1) {
					loadDraftIntoForm(drafts[0]);
					installPipeline.setDraftCollection(drafts, "ingest");
					setBulkDraftView("detail");
					setActiveDraftName(drafts[0].name);
					markIngestSuccess();
					return;
				}

				installPipeline.setDraftCollection(drafts, "ingest");
				installPipeline.setCurrentStep("form");
				resetBulkUIState();
				markIngestSuccess();
			} catch (error) {
				notifyError(
					t("manual.scan.failedTitle", {
						defaultValue: "Local scan failed",
					}),
					error instanceof Error ? error.message : String(error ?? ""),
				);
			} finally {
				setLocalScanLoading(false);
			}
		}, [
			ingestEnabled,
			installPipeline,
			isDropZoneCollapsed,
			isIngesting,
			isLocalScanLoading,
			loadDraftIntoForm,
			markIngestSuccess,
			resetBulkUIState,
			t,
		]);

		const scanActionLabel = t("manual.scan.action", {
			defaultValue: "Scan local configs",
		});
		const scanActionHint = t("manual.scan.actionHint", {
			defaultValue: "Click to scan local configs",
		});

		const handleDropZoneClick = useCallback(
			(event: MouseEvent<HTMLDivElement>) => {
				event.stopPropagation();
				if (!ingestEnabled) return;
				if (isDropZoneCollapsed || ingestError || isIngestSuccess) {
					resetIngestState();
					setIsDropZoneCollapsed(false);
				}
			},
			[ingestEnabled, ingestError, isDropZoneCollapsed, isIngestSuccess, resetIngestState, setIsDropZoneCollapsed],
		);

		const handleDropZoneFocus = useCallback(() => {
			if (!ingestEnabled || !isDropZoneCollapsed) return;
			resetIngestState();
			setIsDropZoneCollapsed(false);
		}, [ingestEnabled, isDropZoneCollapsed, resetIngestState, setIsDropZoneCollapsed]);

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

		const handleFormInteraction = useCallback(() => {
			if (!ingestEnabled) return;
			if (!isDropZoneCollapsed) {
				setIsDropZoneCollapsed(true);
			}
		}, [ingestEnabled, isDropZoneCollapsed, setIsDropZoneCollapsed]);

		// Step navigation logic
		const canNavigateToStep = useCallback(
			(step: WizardStep): boolean => {
				switch (step) {
					case "form":
						return true;
					case "preview":
						if (installPipeline.state.drafts.length > 1) {
							return installPipeline.state.selectedDraftNames.length > 0;
						}
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
				installPipeline.state.drafts.length,
				installPipeline.state.selectedDraftNames.length,
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
				const iconUrl = trim(values.meta_icon_url);
				if (iconUrl) meta.icons = [{ src: iconUrl }];

				return {
					name: values.name.trim(),
					serverId: pendingImportServerRef.current ?? undefined,
					source: initialDraft?.source,
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
			[initialDraft?.source],
		);

		const persistActiveDraft = useCallback(() => {
			if (!activeDraftName) return;
			const nextDraft = toDraftFromValues(getValues());
			installPipeline.updateDraft(nextDraft, activeDraftName);
			if (nextDraft.name !== activeDraftName) {
				setActiveDraftName(nextDraft.name);
			}
		}, [activeDraftName, getValues, installPipeline, toDraftFromValues]);

		const handlePreview = useCallback(
			async (opts?: { shouldFocus?: boolean; skipValidation?: boolean }) => {
				if (previewInFlightRef.current) return;
				const isBulkCollection = installPipeline.state.drafts.length > 1;
				if (isBulkCollection) {
					const selectedNames = new Set(
						installPipeline.state.selectedDraftNames,
					);
					const activeDraft =
						activeDraftName && bulkDraftView === "detail"
							? toDraftFromValues(getValues())
							: null;
					if (activeDraft && activeDraftName) {
						const isValid = await trigger(undefined, { shouldFocus: true });
						if (!isValid) return;
						installPipeline.updateDraft(activeDraft, activeDraftName);
						if (activeDraft.name !== activeDraftName) {
							setActiveDraftName(activeDraft.name);
							selectedNames.delete(activeDraftName);
							selectedNames.add(activeDraft.name);
						}
					}
					const nextDrafts = installPipeline.state.drafts.map((draft) =>
						activeDraft && draft.name === activeDraftName ? activeDraft : draft,
					);
					const selectedDrafts = nextDrafts.filter((draft) =>
						selectedNames.has(draft.name),
					);
					if (!selectedDrafts.length) {
						notifyError(
							t("manual.bulk.noSelectionTitle", {
								defaultValue: "No servers selected",
							}),
							t("manual.bulk.noSelectionDescription", {
								defaultValue: "Select at least one server to preview.",
							}),
						);
						return;
					}
					installPipeline.setDraftCollection(nextDrafts, "ingest");
					installPipeline.setSelectedDraftNames(Array.from(selectedNames));
					previewInFlightRef.current = true;
					try {
						installPipeline.setCurrentStep("preview");
						setActivePreviewName(selectedDrafts[0]?.name ?? null);
						await installPipeline.previewDrafts(
							selectedDrafts[0] ? [selectedDrafts[0]] : [],
						);
					} finally {
						previewInFlightRef.current = false;
					}
					return;
				}
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
				installPipeline.setDraftCollection(drafts, origin);

				if (currentStep !== "preview") {
					installPipeline.setCurrentStep("preview");
				}

				try {
					if (pendingImportServerRef.current && !isEditMode) {
						const previewEpoch = wizardSessionEpochRef.current;
						installPipeline.setPreviewError(null);
						installPipeline.setPreviewState(null);
						installPipeline.setPreviewLoading(true);
						const hiddenServerId = pendingImportServerRef.current;
						try {
							const [tools, resources, prompts, resourceTemplates] = await Promise.all([
								serversApi.listTools(hiddenServerId, "force"),
								serversApi.listResources(hiddenServerId, "force"),
								serversApi.listPrompts(hiddenServerId, "force"),
								serversApi.listResourceTemplates(hiddenServerId, "force"),
							]);

							if (previewEpoch !== wizardSessionEpochRef.current) {
								return;
							}

							installPipeline.setPreviewState({
								success: true,
								data: {
									items: [
										{
											name: draft.name,
											ok: true,
											error: null,
											tools,
											resources,
											prompts,
											resource_templates: resourceTemplates,
										},
									],
								},
							});
						} catch (error) {
							if (previewEpoch !== wizardSessionEpochRef.current) {
								return;
							}
							const message =
								error instanceof Error ? error.message : "Preview request failed";
							installPipeline.setPreviewError(message);
							notifyError("Preview failed", message);
						} finally {
							if (previewEpoch === wizardSessionEpochRef.current) {
								installPipeline.setPreviewLoading(false);
							}
						}
						return;
					}

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
				isEditMode,
				isImportMode,
				onPreview,
				currentStep,
				installPipeline,
				activeDraftName,
				bulkDraftView,
				t,
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
				cleanupPendingImportServer();
				onClose();
				setIsClosing(false);
			}
		}, [cleanupPendingImportServer, onClose, isClosing]);

		// Handle import action
		const handleImport = useCallback(async () => {
			if (importInFlightRef.current) {
				return;
			}

			importInFlightRef.current = true;
			setIsImportActionPending(true);

			try {
				const draft = toDraftFromValues(getValues());
				const effectiveTargetProfileId = await resolveImportTargetProfileId();
				if (
					!isEditMode &&
					(await tryFinalizePublishImport(draft, effectiveTargetProfileId))
				) {
					return;
				}

				if (onImport) {
					await Promise.resolve(onImport([draft]));
					clearPendingImportState();
					handleOverlayClose();
					return;
				}

				const didSucceed = await runImportPipeline(effectiveTargetProfileId);
				if (!didSucceed) {
					return;
				}

				clearPendingImportState();
				handleOverlayClose();
			} finally {
				importInFlightRef.current = false;
				setIsImportActionPending(false);
			}
		}, [
			getValues,
			onImport,
			toDraftFromValues,
			resolveImportTargetProfileId,
			runImportPipeline,
			tryFinalizePublishImport,
			clearPendingImportState,
			handleOverlayClose,
			isEditMode,
		]);

		// Cancel close handler (with delay for complete reset)
		const handleCancelClose = useCallback(() => {
			if (!isClosing) {
				setIsClosing(true);
				cleanupPendingImportServer();

				setTimeout(() => {
					onClose();
					setIsClosing(false);
				}, 50);
			}
		}, [cleanupPendingImportServer, onClose, isClosing]);

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

		// Reset wizard whenever the drawer opens or closes
		const wasOpenRef = useRef(false);
		useLayoutEffect(() => {
			const wasOpen = wasOpenRef.current;
			if (isOpen !== wasOpen) {
				resetWizardSession();
			}
			wasOpenRef.current = isOpen;
		}, [isOpen, resetWizardSession]);

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
			} catch {
				// Draft parsing is best-effort; ignore failures
			}
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
			const skipDryRunForHiddenPreview =
				Boolean(pendingImportServerId) &&
				!isEditMode &&
				installPipeline.state.previewState !== null &&
				installPipeline.state.previewState.success !== false &&
				!installPipeline.state.previewError;
			if (
				currentStep === "result" &&
				!installPipeline.state.importResult &&
				!installPipeline.state.isImporting &&
				!skipDryRunForHiddenPreview
			) {
				// Only perform dry-run if we haven't already done it or if the drafts have changed
				if (
					!installPipeline.state.dryRunResult &&
					!installPipeline.state.isDryRunLoading
				) {
					void installPipeline.performDryRun();
				}
			}
		}, [
			currentStep,
			isEditMode,
			installPipeline,
			pendingImportServerId,
		]);

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

		const selectedDraftNameSet = useMemo(
			() => new Set(installPipeline.state.selectedDraftNames),
			[installPipeline.state.selectedDraftNames],
		);

		const wizardBulkActions = useMemo(
			() =>
				buildIncludeExcludeBulkActions({
					bulk: bulkSelection,
					visibleIds: installPipeline.state.drafts.map((draft) => draft.name),
					onInclude: () =>
						installPipeline.setSelectedDraftNames(
							Array.from(
								new Set([
									...installPipeline.state.selectedDraftNames,
									...bulkSelection.selectedIds,
								]),
							),
						),
					onExclude: () =>
						installPipeline.setSelectedDraftNames(
							installPipeline.state.selectedDraftNames.filter(
								(name) => !bulkSelection.selectedIdSet.has(name),
							),
						),
					t,
				}),
			[bulkSelection, installPipeline.state.drafts, installPipeline.state.selectedDraftNames, installPipeline.setSelectedDraftNames, t],
		);

		const toggleDraftSelection = useCallback(
			(name: string) => {
				const next = new Set(installPipeline.state.selectedDraftNames);
				if (next.has(name)) {
					next.delete(name);
				} else {
					next.add(name);
				}
				installPipeline.setSelectedDraftNames(Array.from(next));
			},
			[installPipeline],
		);

		const openDraftDetail = useCallback(
			(draft: ServerInstallDraft) => {
				if (activeDraftName && bulkDraftView === "detail") {
					persistActiveDraft();
				}
				loadDraftIntoForm(draft);
				setActiveDraftName(draft.name);
				setBulkDraftView("detail");
			},
			[
				activeDraftName,
				bulkDraftView,
				loadDraftIntoForm,
				persistActiveDraft,
			],
		);

		const returnToBulkList = useCallback(() => {
			persistActiveDraft();
			setBulkDraftView("list");
			setActiveDraftName(null);
		}, [persistActiveDraft]);

		const previewDraftByName = useCallback(
			async (name: string) => {
				const draft = installPipeline.state.drafts.find(
					(item) => item.name === name,
				);
				if (!draft) return;
				setActivePreviewName(name);
				await installPipeline.previewDrafts([draft]);
			},
			[installPipeline],
		);

		const renderDraftListItem = (
			draft: ServerInstallDraft,
			options: {
				isActive?: boolean;
				mode: "configure" | "preview";
			},
		) => {
			const includedForImport = selectedDraftNameSet.has(draft.name);
			const bulkSelected =
				options.mode === "configure" &&
				bulkSelection.isBulkMode &&
				bulkSelection.selectedIdSet.has(draft.name);
			const endpoint = draftEndpointSummary(draft);
			const draftIcon = draft.meta?.icons?.[0]?.src;
			const draftDescription = draft.meta?.description?.trim();
			const avatarFallback = (draft.name || "S").slice(0, 1).toUpperCase();
			const isConfigureBulkMode =
				options.mode === "configure" && bulkSelection.isBulkMode;
			const handleRowActivate = () => {
				if (isConfigureBulkMode) {
					bulkSelection.toggleItem(draft.name);
					return;
				}
				if (options.mode === "configure") {
					openDraftDetail(draft);
					return;
				}
				void previewDraftByName(draft.name);
			};
			return (
				<CapsuleStripeListItem
					key={draft.name}
					interactive
					className={cn(
						"group relative transition-colors",
						(bulkSelected || options.isActive) &&
						"bg-primary/10 ring-1 ring-slate-200/80 dark:ring-slate-700/60",
					)}
					onClick={handleRowActivate}
					onKeyDown={(event) => {
						if (event.key === "Enter" || event.key === " ") {
							event.preventDefault();
							handleRowActivate();
						}
					}}
				>
					<div className="flex w-full items-center justify-between gap-4">
						<div className="flex min-w-0 flex-1 items-center gap-3">
							{options.mode === "configure" ? (
								<BulkSelectionCheckbox
									visible={bulkSelection.isBulkMode}
									checked={bulkSelected}
									onToggle={() => bulkSelection.toggleItem(draft.name)}
									ariaLabel={t("manual.bulk.selectServer", {
										name: draft.name,
										defaultValue: "Select {{name}}",
									})}
								/>
							) : null}
							<CachedAvatar
								src={draftIcon}
								alt={draft.name ? `${draft.name} icon` : undefined}
								fallback={avatarFallback}
								size="sm"
								shape="rounded"
								className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
							/>
							<div className="min-w-0">
								<h3 className="font-medium text-slate-900 dark:text-slate-100">
									{toTitleCase(draft.name)}
								</h3>
								<p className="truncate text-sm text-slate-500">
									{endpoint ||
										t("manual.bulk.missingEndpoint", {
											defaultValue: "Missing command or URL",
										})}
								</p>
								{draftDescription ? (
									<p className="line-clamp-2 text-xs text-slate-500">
										{draftDescription}
									</p>
								) : null}
							</div>
						</div>
						<div className="ml-auto flex shrink-0 items-center gap-2">
							{endpoint ? (
								<Badge variant="secondary">{transportLabel[draft.kind]}</Badge>
							) : (
								<Badge variant="outline">
									{t("manual.bulk.missingEndpoint", {
										defaultValue: "Missing command or URL",
									})}
								</Badge>
							)}
							{options.mode === "configure" ? (
								<Switch
									checked={includedForImport}
									onClick={(event) => event.stopPropagation()}
									onCheckedChange={() => toggleDraftSelection(draft.name)}
									aria-label={t("manual.bulk.includeForImport", {
										name: draft.name,
										defaultValue: "Include {{name}} in import",
									})}
								/>
							) : null}
							{!isConfigureBulkMode ? (
								<ChevronRight
									className="h-4 w-4 shrink-0 text-slate-400 dark:text-slate-500"
									aria-hidden
								/>
							) : null}
						</div>
					</div>
				</CapsuleStripeListItem>
			);
		};

		const renderBulkDraftList = () => {
			const drafts = installPipeline.state.drafts;
			return (
				<div
					className="flex min-h-0 flex-1 flex-col"
					onClick={handleFormInteraction}
				>
					<BulkSelectionHeader
						title={t("manual.bulk.title", {
							count: drafts.length,
							defaultValue: "{{count}} servers detected",
						})}
						description={
							bulkSelection.isBulkMode
								? bulkModeDescription(bulkSelection.selectedCount)
								: t("manual.bulk.description", {
									count: installPipeline.state.selectedDraftNames.length,
									defaultValue:
										"Select the servers to preview and import. {{count}} selected.",
								})
						}
						isBulkMode={bulkSelection.isBulkMode}
						onToggleBulkMode={bulkSelection.toggleMode}
						actions={wizardBulkActions}
					/>
					<CardListScrollBody>
						<CapsuleStripeList className="rounded-none border-0 overflow-visible">
							{drafts.map((draft) =>
								renderDraftListItem(draft, { mode: "configure" }),
							)}
						</CapsuleStripeList>
					</CardListScrollBody>
				</div>
			);
		};

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
			const showBulkDraftList =
				installPipeline.state.drafts.length > 1 && bulkDraftView === "list";
			const isCoreJsonPanel = isCoreJsonView(uiActiveTab, viewMode);
			return (
				<div className={FORM_FILL_SHELL_CLASS}>
					<form
						onSubmit={handleSubmit(() =>
							handlePreview({ skipValidation: true, shouldFocus: false }),
						)}
						className={FORM_FILL_SHELL_CLASS}
					>
						{/* New-mode drop zone (top) */}
						{ingestEnabled ? (
							<div
								className="relative shrink-0 px-4 py-4"
								onClick={(event) => event.stopPropagation()}
							>
								<div
									data-desktop-drop-target="server-import"
									role="button"
									tabIndex={0}
									ref={dropZoneRef}
									onFocus={handleDropZoneFocus}
									onClick={handleDropZoneClick}
									onKeyDown={(e) => {
										if (e.key === "Enter" || e.key === " ") {
											e.preventDefault();
											handleDropZoneClick(e as unknown as MouseEvent<HTMLDivElement>);
										}
									}}
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
										e.stopPropagation();
										setIsDragOver(false);
										try {
											const payload = await extractPayloadFromDataTransfer(
												e.dataTransfer!,
											);
											if (payload) await handleIngestPayload(payload);
										} catch (error) {
											setIngestError(
												formatServerUniImportTransferError(error, t),
											);
											setIngestMessage(ingestMessages.defaultMessage);
										}
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
										className={`w-full ${isDropZoneCollapsed ? "h-10" : "h-[18vh]"
											} flex items-center justify-center gap-1 rounded-lg border border-dashed transition-all duration-300 ${isDropZoneCollapsed
												? "flex-row px-4 py-0 border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
												: "flex-col py-2 border-slate-300 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
											} ${ingestError
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
										) : isDropZoneCollapsed ? (
											<Radar
												className="h-4 w-4 shrink-0 text-slate-500"
												aria-hidden
											/>
										) : (
											<TooltipProvider delayDuration={200}>
												<Tooltip>
													<TooltipTrigger asChild>
														<button
															type="button"
															className={cn(
																"inline-flex h-14 w-14 shrink-0 items-center justify-center rounded-full transition-colors",
																"hover:bg-slate-200/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring dark:hover:bg-slate-800/60",
															)}
															disabled={isLocalScanLoading || isIngesting}
															aria-label={scanActionLabel}
															onClick={(event) => {
																event.stopPropagation();
																void handleLocalConfigScan();
															}}
														>
															{isLocalScanLoading ? (
																<Loader2 className="h-6 w-6 animate-spin" />
															) : (
																<Radar
																	className={cn(
																		"h-12 w-12 scale-100 text-slate-500 transition-all duration-300",
																		isDragOver && "animate-pulse text-blue-500",
																	)}
																	style={{
																		animation:
																			ingestError || isDragOver
																				? "breathing 1.5s ease-in-out infinite"
																				: undefined,
																	}}
																/>
															)}
														</button>
													</TooltipTrigger>
													<TooltipContent side="bottom">
														{scanActionHint}
													</TooltipContent>
												</Tooltip>
											</TooltipProvider>
										)}
										<p
											className={`text-sm ${ingestError
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
								</div>
							</div>
						) : null}

						<div
							className={installFormBodyClass(ingestEnabled, isCoreJsonPanel)}
							onFocusCapture={handleContentFocus}
						>
							{showBulkDraftList ? (
								renderBulkDraftList()
							) : (
								<>
									{installPipeline.state.drafts.length > 1 ? (
										<div className="mb-3 shrink-0">
											<Button
												type="button"
												variant="ghost"
												size="sm"
												onClick={returnToBulkList}
											>
												<ArrowLeft className="mr-2 h-4 w-4" />
												{t("manual.bulk.backToList", {
													defaultValue: "Back to detected servers",
												})}
											</Button>
										</div>
									) : null}
									<Tabs
										value={uiActiveTab}
										onValueChange={(v) => setUiActiveTab(v as "core" | "meta")}
										className="flex min-h-0 flex-1 flex-col"
									>
										<TabsList className="grid w-full shrink-0 grid-cols-2">
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
											className={FORM_TAB_SHELL_CLASS}
										>
											<CoreConfigTabPanel
												viewMode={viewMode}
												onViewModeChange={handleModeChange}
												onContentClick={handleFormInteraction}
												formContent={
													<>
														<div className="space-y-4">
															<div className="flex items-center gap-3">
																<Label htmlFor={nameId} className={SERVER_INSTALL_FORM_ROW_LABEL_CLASS}>
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
																		readOnly={isEditMode || Boolean(pendingImportServerId)}
																		aria-readonly={isEditMode || Boolean(pendingImportServerId)}
																		title={
																			isEditMode || Boolean(pendingImportServerId)
																				? pendingImportServerId
																					? t("manual.fields.name.readOnlyTitleAfterOAuth", {
																						defaultValue:
																							"Editing server names is disabled after OAuth setup starts",
																					})
																					: t("manual.fields.name.readOnlyTitle", {
																						defaultValue: "Editing server names is disabled",
																					})
																				: undefined
																		}
																		className={
																			isEditMode || Boolean(pendingImportServerId)
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
															<div className="flex items-center gap-3">
																<Label htmlFor={kindId} className={SERVER_INSTALL_FORM_ROW_LABEL_CLASS}>
																	{t("manual.fields.type.label", {
																		defaultValue: "Type",
																	})}
																</Label>
																<div className="flex-1">
																	<Segment
																		options={serverTypeOptions}
																		value={kind}
																		onValueChange={(value) => {
																			if (pendingImportServerId) {
																				return;
																			}
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
															viewMode={viewMode}
															onCreateSecret={onCreateSecret}
															secretOriginBase={secretOriginBase}
														/>

														<ServerAuthSection
															serverId={pendingImportServerId ?? undefined}
															isStdio={isStdio}
															viewMode={viewMode}
															isNewServer={!isEditMode}
															suggestedAuthMode={suggestedAuthMode}
															onAuthModeChange={setSelectedAuthMode}
															onOAuthConnected={(serverId) => {
																if (isEditMode || pendingImportServerRef.current !== serverId) {
																	return;
																}
																void handlePreview({
																	skipValidation: true,
																	shouldFocus: false,
																});
															}}
															onInitiateOAuth={async (config) => {
																const formValues = getValues();
																const draft = toDraftFromValues(formValues);
																if (!draft.name) {
																	throw new Error(
																		t("manual.errors.nameRequired", {
																			defaultValue: "Name is required",
																		}),
																	);
																}

																let targetServerId = pendingImportServerRef.current;
																if (!isEditMode) {
																	if (targetServerId) {
																		await serversApi.updateServer(
																			targetServerId,
																			draftToServerConfig(draft, {
																				pending_import: true,
																				enabled: false,
																			}),
																		);
																	} else {
																		const created = await serversApi.createServer(
																			draftToServerConfig(draft, {
																				pending_import: true,
																				enabled: false,
																			}),
																		);
																		targetServerId = created.data?.id ?? null;
																		if (!targetServerId) {
																			throw new Error(
																				t("manual.errors.oauthDraftServerFailed", {
																					defaultValue: "Failed to create OAuth draft server",
																				}),
																			);
																		}
																		pendingImportServerRef.current = targetServerId;
																		setPendingImportServerId(targetServerId);
																	}
																} else if (!targetServerId) {
																	throw new Error(
																		t("manual.errors.oauthServerIdRequired", {
																			defaultValue: "Server ID is required to initiate OAuth",
																		}),
																	);
																}

																await startOAuthAccessFlow(targetServerId, config);
															}}
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
															control={control}
															deleteConfirmStates={deleteConfirmStates}
															onDeleteClick={handleDeleteClick}
															onGhostClick={handleGhostClick}
															onCreateSecret={onCreateSecret}
															secretOriginBase={secretOriginBase}
															getEnvRowKeyAt={(index) =>
																watchedEnv?.[index]?.key?.trim() || undefined
															}
														/>

														<UrlParams
															viewMode={viewMode}
															isStdio={isStdio}
															urlParamFields={paramFields.fields}
															removeUrlParam={removeUrlParam}
															appendUrlParam={appendUrlParam}
															register={register}
															control={control}
															deleteConfirmStates={deleteConfirmStates}
															onDeleteClick={handleDeleteClick}
															onGhostClick={handleGhostClick}
															onCreateSecret={onCreateSecret}
															secretOriginBase={secretOriginBase}
															getRowKeyAt={(index) =>
																watchedUrlParams?.[index]?.key?.trim() || undefined
															}
														/>

														<HttpHeaders
															viewMode={viewMode}
															isStdio={isStdio}
															labelTooltip={
																!isStdio && selectedAuthMode === "oauth"
																	? t("manual.auth.transportHint", {
																		defaultValue:
																			"URL Parameters and HTTP Headers are optional transport extras. They still apply after OAuth if this server needs them.",
																	})
																	: undefined
															}
															headerFields={headerFields.fields}
															removeHeader={removeHeader}
															appendHeader={appendHeader}
															register={register}
															control={control}
															deleteConfirmStates={deleteConfirmStates}
															onDeleteClick={handleDeleteClick}
															onGhostClick={handleGhostClick}
															onCreateSecret={onCreateSecret}
															secretOriginBase={secretOriginBase}
															getRowKeyAt={(index) =>
																watchedHeaders?.[index]?.key?.trim() || undefined
															}
														/>
													</>
												}
												jsonContent={
													<ServerConfigJsonPanel
														id={manualJsonId}
														label={t("manual.fields.json.label", {
															defaultValue: "Server JSON",
														})}
														jsonText={jsonText}
														jsonError={jsonError}
														jsonEditingEnabled={jsonEditingEnabled}
														onJsonChange={setJsonText}
														copyLabel={t("manual.fields.json.copy", {
															defaultValue: "Copy JSON",
														})}
													/>
												}
											/>
										</TabsContent>

										<TabsContent
											value="meta"
											className={SECONDARY_TAB_CONTENT_CLASS}
											onClick={handleFormInteraction}
										>
											<MetaFields
												formStateRef={formStateRef}
												register={register}
												errors={errors}
												iconUrl={watchedMetaIconUrl}
												metaIconUrlId={metaIconUrlId}
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
								</>
							)}
						</div>
					</form>
				</div>
			);
		};

		const previewItemsByName = useMemo(() => {
			const map = new Map<string, ImportPreviewItem>();
			const previewData = installPipeline.state.previewState as
				| { data?: { items?: unknown[] } }
				| null;
			const items = previewData?.data?.items;
			if (Array.isArray(items)) {
				for (const entry of items) {
					if (entry && typeof entry === "object" && "name" in entry) {
						const previewItem = entry as ImportPreviewItem;
						const name = previewItem.name;
						if (typeof name === "string") {
							map.set(name, previewItem);
						}
					}
				}
			}
			return map;
		}, [installPipeline.state.previewState]);

		const hasPendingImportPublishFlow =
			Boolean(pendingImportServerId) && !isEditMode;
		const hiddenPreviewReady =
			hasPendingImportPublishFlow &&
			installPipeline.state.previewState !== null &&
			installPipeline.state.previewState.success !== false &&
			!installPipeline.state.previewError;

		const renderPreviewStep = () => {
			const { state } = installPipeline;
			const { drafts, previewState, previewError, isPreviewLoading } = state;

			const asRecordList = (
				items: unknown[] | undefined,
			): Record<string, unknown>[] => {
				if (!Array.isArray(items)) return [];
				return items.filter(
					(item): item is Record<string, unknown> =>
						item !== null && typeof item === "object",
				);
			};
			const capabilityKindSet = new Set(capabilityKindFilters);
			const isCapabilityKindVisible = (kind: CapabilityPreviewKind): boolean =>
				capabilityKindSet.size === 0 || capabilityKindSet.has(kind);
			const capabilityKindOptions = IMPORT_PREVIEW_KIND_ORDER.map((kind) => ({
				value: kind,
				label: t(`detail.filters.kind.${kind}`, {
					defaultValue: importPreviewKindDefaultLabel(kind),
				}),
			}));
			const allCapabilityKindsLabel = t("detail.filters.kind.all", {
				defaultValue: "All Types",
			});
			let capabilityKindLabel = allCapabilityKindsLabel;
			if (capabilityKindFilters.length === 1) {
				capabilityKindLabel =
					capabilityKindOptions.find(
						(option) => option.value === capabilityKindFilters[0],
					)?.label ?? allCapabilityKindsLabel;
			} else if (capabilityKindFilters.length > 1) {
				capabilityKindLabel = t("detail.filters.kind.selected", {
					count: capabilityKindFilters.length,
					defaultValue: "{{count}} Types",
				});
			}
			const toggleCapabilityKindFilter = (
				value: string,
				checked: boolean,
			) => {
				setCapabilityKindFilters((current) => {
					const next = new Set(current);
					if (checked) {
						next.add(value as CapabilityPreviewKind);
					} else {
						next.delete(value as CapabilityPreviewKind);
					}
					return IMPORT_PREVIEW_KIND_ORDER.filter((kind) => next.has(kind));
				});
			};
			const renderImportFlatCapabilityList = (
				items: CapabilityPreviewFlatItem[],
			) => {
				const flatItems: ImportPreviewFlatCapabilityItem[] = items.map(
					({ kind, item }) => ({
						...item,
						__importCapabilityKind: kind,
					}),
				);

				return (
					<CapabilityList<ImportPreviewFlatCapabilityItem>
						asCard={false}
						kind="tools"
						getKind={(item) => item.__importCapabilityKind}
						context="server"
						leadingIcon="kind"
						items={flatItems}
						clickToToggleDetails
						scrollContainedBody
						getId={importPreviewCapabilityItemId}
						emptyText={t("wizard.preview.emptyCapabilityFilters", {
							defaultValue: "No capabilities match the selected filters.",
						})}
					/>
				);
			};

			return (
				<div className="flex min-h-0 flex-1 flex-col px-4 py-4">
					<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
						{previewError ? (
							<Alert variant="destructive" className="shrink-0">
								<AlertTriangle className="h-4 w-4" />
								<AlertTitle>Preview failed</AlertTitle>
								<AlertDescription>{previewError}</AlertDescription>
							</Alert>
						) : null}

						{previewState?.success === false && previewState?.error ? (
							<Alert variant="default" className="shrink-0">
								<AlertTriangle className="h-4 w-4" />
								<AlertTitle>Preview reported issues</AlertTitle>
								<AlertDescription>
									Some servers could not be contacted during preview. You can
									still proceed—the proxy will retry after installation.
								</AlertDescription>
							</Alert>
						) : null}

						{(() => {
							const renderPreviewCard = (draft: ServerInstallDraft) => {
								const item = previewItemsByName.get(draft.name);
								const ok = item?.ok !== false;
								const tools = asRecordList(item?.tools?.items);
								const resources = asRecordList(item?.resources?.items);
								const templates = asRecordList(item?.resource_templates?.items);
								const prompts = asRecordList(item?.prompts?.items);
								return (
									<div
										key={draft.name}
										className="flex min-h-0 flex-1 flex-col overflow-hidden"
									>
										<div className="shrink-0 pb-3">
											<CapabilityToolbar
												searchValue={capabilitySearch}
												onSearchChange={setCapabilitySearch}
												searchPlaceholder={t(
													"wizard.preview.filterCapabilities",
													{ defaultValue: "Filter capabilities..." },
												)}
												kindFilter={{
													label: capabilityKindLabel,
													allLabel: allCapabilityKindsLabel,
													options: capabilityKindOptions,
													selectedValues: capabilityKindFilters,
													onClear: () => setCapabilityKindFilters([]),
													onToggle: toggleCapabilityKindFilter,
												}}
												containedFocus
											/>
										</div>
										<CapabilityPreviewList
											className="min-h-0 flex-1"
											contentClassName="flex min-h-0 flex-1 flex-col p-0"
											framed={false}
											showHeader={false}
											tools={isCapabilityKindVisible("tools") ? tools : []}
											resources={
												isCapabilityKindVisible("resources") ? resources : []
											}
											templates={
												isCapabilityKindVisible("templates") ? templates : []
											}
											prompts={
												isCapabilityKindVisible("prompts") ? prompts : []
											}
											hasSource={Boolean(item)}
											isLoading={isPreviewLoading}
											error={!ok && item?.error ? String(item.error) : null}
											searchValue={capabilitySearch}
											emptyText={
												capabilityKindFilters.length
													? t("wizard.preview.emptyCapabilityFilters", {
														defaultValue:
															"No capabilities match the selected filters.",
													})
													: undefined
											}
											renderFlatList={renderImportFlatCapabilityList}
										/>
									</div>
								);
							};

							const selectedDrafts = drafts.filter((draft) =>
								selectedDraftNameSet.has(draft.name),
							);
							if (!selectedDrafts.length) {
								return null;
							}

							const activeName =
								activePreviewName ??
								selectedDrafts[0]?.name ??
								null;
							const activeDraft =
								selectedDrafts.find((draft) => draft.name === activeName) ??
								selectedDrafts[0] ??
								null;

							return (
								<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
									<div className="shrink-0">
										<CapabilityCombobox
											kind="tool"
											items={selectedDrafts}
											value={activeName ?? undefined}
											onChange={(name) => {
												void previewDraftByName(name);
											}}
											placeholder={t("wizard.preview.selectServer", {
												defaultValue: "Select server",
											})}
											triggerClassName="h-10 rounded-lg border border-dashed border-slate-200 bg-slate-50 px-4 py-0 shadow-none transition-all duration-300 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40 dark:hover:bg-slate-900/40"
											triggerLabelClassName="font-semibold text-slate-900 dark:text-slate-100"
											renderItemLeading={draftListAvatar}
											renderTriggerTrailing={(draft) => (
												<Badge variant="secondary" className="shrink-0 text-xs">
													{transportLabel[draft.kind]}
												</Badge>
											)}
											renderItemTrailing={(draft) => (
												<Badge variant="secondary" className="shrink-0 text-xs">
													{transportLabel[draft.kind]}
												</Badge>
											)}
											getKey={(draft) => draft.name}
											getLabel={(draft) => toTitleCase(draft.name)}
											getDescription={(draft) =>
												draftEndpointSummary(draft) || undefined
											}
										/>
									</div>
									{activeDraft ? renderPreviewCard(activeDraft) : null}
								</div>
							);
						})()}
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
				selectedDraftNames,
			} = state;
			const summary = importResult?.summary as
				| { imported_count?: number | null; skipped_count?: number | null }
				| undefined;
			const importedCount = summary?.imported_count ?? 0;
			const skippedCount = summary?.skipped_count ?? 0;
			const onlySkipped = importedCount === 0 && skippedCount > 0;

			// Reflect the live setting + actual default-anchor profile name so
			// the UI matches whatever the user has renamed the anchor to.
			const autoAddToDefault = autoAddTargetProfile.enabled;
			const selectedProfileName = autoAddToDefault
				? (autoAddTargetProfile.profileName ?? "Default")
				: null;

			// Show ready state UI before import is completed
			const showReadyState = !importResult && !isImporting;

			// Determine if we can proceed with import based on dry-run
			const dryRunImportableCount = dryRunStats?.importedCount ?? 0;
			const dryRunSkippedCount = dryRunStats?.skippedCount ?? 0;
			const effectiveSkippedCount = hiddenPreviewReady ? 0 : dryRunSkippedCount;
			const canProceedWithImport =
				hiddenPreviewReady ||
				!isDryRunLoading &&
				!dryRunError &&
				dryRunResult &&
				dryRunImportableCount > 0;
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
								(() => {
									const validationItems = buildImportValidationItems({
										selectedNames: selectedDraftNames,
										stats: dryRunStats,
										hiddenPreviewReady,
									});
									const hasPerServerBreakdown =
										validationItems.length > 0 &&
										(hiddenPreviewReady || dryRunStats !== null);
									const showValidationSummary =
										!dryRunError || (dryRunStats?.failedCount ?? 0) > 0;

									const skippedOnly =
										!dryRunError &&
										!hiddenPreviewReady &&
										!canProceedWithImport &&
										effectiveSkippedCount > 0;

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
										<div className="flex flex-col gap-20">
											<div className="space-y-3">
												{hasPerServerBreakdown && showValidationSummary ? (
													<ImportValidationSummary
														items={validationItems}
														hiddenPreviewReady={hiddenPreviewReady}
													/>
												) : dryRunError ? (
													<p className="text-sm text-red-600">{dryRunError}</p>
												) : null}
											</div>
											{renderNextSteps(nextSteps)}
										</div>
									);
								})()
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

								<div className="pt-20">
									{renderNextSteps(
										importResult.success !== false ? successSteps : failureSteps,
									)}
								</div>
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

		const isFlexFillStep =
			currentStep === "form" || currentStep === "preview";
		const detectedServerCount = installPipeline.state.drafts.length;
		const headerPluralCount = detectedServerCount > 1 ? detectedServerCount : 1;
		const isImportBusy =
			isImportActionPending || installPipeline.state.isImporting;
		const hasImportableDrafts =
			hiddenPreviewReady ||
			(installPipeline.state.dryRunStats?.importedCount ?? 0) > 0;
		const isImportButtonDisabled =
			isImportBusy ||
			installPipeline.state.isDryRunLoading ||
			!!installPipeline.state.dryRunError ||
			!hasImportableDrafts;
		const importButtonContent = isImportBusy ? (
			<>
				<Spinner size="sm" className="mr-2" />
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
		);

		return (
			<>
				<Drawer
					open={isOpen}
					onOpenChange={(open) => !open && handleOverlayClose()}
				>
					<DrawerContent className={INSTALL_DRAWER_CONTENT_CLASS}>
						<DrawerHeader className="shrink-0">
							<div className="flex items-start justify-between gap-3">
								<div className="min-w-0 flex-1 space-y-1 text-left">
									<DrawerTitle className="flex items-center gap-2">
										{isEditMode
											? t("wizard.header.editTitle", { defaultValue: "Edit Server" })
											: t("wizard.header.addTitle", {
												count: headerPluralCount,
												defaultValue:
													detectedServerCount > 1
														? "Add MCP Servers"
														: "Add MCP Server",
											})}
									</DrawerTitle>
									<DrawerDescription>
										{isEditMode
											? t("wizard.header.editDescription", {
												defaultValue: "Update server configuration",
											})
											: t("wizard.header.addDescription", {
												count: headerPluralCount,
												defaultValue:
													detectedServerCount > 1
														? `Review and install ${detectedServerCount} detected MCP servers`
														: "Configure and install a new MCP server",
											})}
									</DrawerDescription>
								</div>
								{currentStep === "form" ? (
									<TooltipProvider delayDuration={200}>
										<Tooltip>
											<TooltipTrigger asChild>
												<Button
													type="button"
													variant="ghost"
													size="icon"
													className="-mr-1 -mt-1 h-5 w-5 shrink-0 rounded-md border-0 bg-transparent p-0 text-muted-foreground shadow-none transition-colors hover:bg-transparent hover:text-foreground focus-visible:ring-1 focus-visible:ring-offset-0"
													disabled={isSubmitting}
													onClick={handleResetForm}
													aria-label={t("wizard.buttons.reset", {
														defaultValue: "Reset form",
													})}
												>
													<RotateCcw className="h-4 w-4" />
												</Button>
											</TooltipTrigger>
											<TooltipContent side="bottom" align="end" className="max-w-xs">
												<p className="font-medium">
													{t("wizard.buttons.reset", {
														defaultValue: "Reset form",
													})}
												</p>
												<p className="mt-1 text-background/80">
													{t("wizard.buttons.resetDescription", {
														defaultValue:
															"Clear all fields and restore the initial configuration.",
													})}
												</p>
											</TooltipContent>
										</Tooltip>
									</TooltipProvider>
								) : currentStep === "preview" &&
									(installPipeline.state.previewState !== null ||
										installPipeline.state.isPreviewLoading) ? (
									<TooltipProvider delayDuration={200}>
										<Tooltip>
											<TooltipTrigger asChild>
												<Button
													type="button"
													variant="ghost"
													size="icon"
													className="-mr-1 -mt-1 h-5 w-5 shrink-0 rounded-md border-0 bg-transparent p-0 text-muted-foreground shadow-none transition-colors hover:bg-transparent hover:text-foreground focus-visible:ring-1 focus-visible:ring-offset-0"
													disabled={
														installPipeline.state.isImporting ||
														installPipeline.state.isPreviewLoading
													}
													aria-label={
														installPipeline.state.isPreviewLoading
															? t("wizard.buttons.previewing", {
																defaultValue: "Previewing...",
															})
															: t("wizard.preview.retry", {
																defaultValue: "Retry preview",
															})
													}
													onClick={() => {
														installPipeline.setPreviewState(null);
														if (
															installPipeline.state.drafts.length > 1 &&
															activePreviewName
														) {
															void previewDraftByName(activePreviewName);
														} else {
															void handlePreview({
																skipValidation: true,
																shouldFocus: false,
															});
														}
													}}
												>
													{installPipeline.state.isPreviewLoading ? (
														<Loader2 className="h-4 w-4 motion-safe:animate-spin" />
													) : (
														<RefreshCw className="h-4 w-4" />
													)}
												</Button>
											</TooltipTrigger>
											<TooltipContent side="bottom" align="end" className="max-w-xs">
												<p className="font-medium">
													{installPipeline.state.isPreviewLoading
														? t("wizard.buttons.previewing", {
															defaultValue: "Previewing...",
														})
														: t("wizard.preview.retry", {
															defaultValue: "Retry preview",
														})}
												</p>
												<p className="mt-1 text-background/80">
													{t("wizard.preview.retryDescription", {
														defaultValue:
															"Regenerate capability preview for the selected server.",
													})}
												</p>
											</TooltipContent>
										</Tooltip>
									</TooltipProvider>
								) : null}
							</div>
						</DrawerHeader>

						{/* Step Navigation */}
						<div className="relative z-10 shrink-0 bg-background p-4 pb-0">
							<div className="flex items-center gap-2">
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
													className={`flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold transition-colors ${isActive
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
														className={`text-sm font-medium ${isActive
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
							</div>
						</div>

						{/* Step Content - with spacing and bottom padding to avoid footer overlap */}
						<div
							className={cn(
								"flex-1 min-h-0 py-2",
								isFlexFillStep
									? "flex flex-col overflow-hidden"
									: "overflow-y-auto",
							)}
						>
							{renderStepContent()}
						</div>

						{/* Footer - fixed at bottom with subtle shadow for separation */}
						<DrawerFooter className="shrink-0 border-t bg-background p-4">
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
												disabled={isImportButtonDisabled}
											>
												{importButtonContent}
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
				<InlineSecretCreate controller={controller} nested />
			</>
		);
	},
);

ServerInstallWizard.displayName = "ServerInstallWizard";
