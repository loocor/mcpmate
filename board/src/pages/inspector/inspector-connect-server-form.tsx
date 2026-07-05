import {
	useCallback,
	useEffect,
	useId,
	useMemo,
	useRef,
	useState,
	type ClipboardEvent,
	type DragEvent,
	type MouseEvent,
} from "react";
import {
	ChevronsUpDown,
	Database,
	Loader2,
	Pencil,
	Plug,
	RotateCcw,
	Unplug,
} from "lucide-react";
import { useFieldArray, useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Button } from "../../components/ui/button";
import {
	Command,
	CommandEmpty,
	CommandInput,
	CommandItem,
	CommandList,
} from "../../components/ui/command";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "../../components/ui/popover";
import { Segment } from "../../components/ui/segment";
import { CoreConfigTabPanel } from "../../components/server-install/core-config-tab-panel";
import { draftToServerConfig } from "../../components/server-install/draft-to-server-config";
import {
	CommandField,
	HttpHeaders,
	StdioAdvanced,
	UrlParams,
} from "../../components/server-install/form-fields";
import { ServerAuthSection } from "../../components/server-install/server-auth-section";
import { ServerConfigJsonPanel } from "../../components/server-install/server-config-json-panel";
import { ServerImportDropZone } from "../../components/server-install/server-import-drop-zone";
import { SERVER_INSTALL_FORM_ROW_LABEL_CLASS } from "../../components/server-install/field-list";
import { useServerTypeOptions } from "../../components/server-install/hooks/use-server-type-options";
import type { ManualServerFormValues } from "../../components/server-install/types";
import type { FormViewMode } from "../../components/server-install/view-mode-toggle";
import type {
	OAuthConfigRequest,
	RegistryRepositoryInfo,
	SecretOrigin,
	ServerDetail,
	ServerMetaInfo,
} from "../../lib/types";
import {
	compactKeyValueFields,
	shouldAppendKeyValueRow,
} from "../../lib/key-value-fields";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { inspectorApi, serversApi } from "../../lib/api";
import {
	normalizeIngestPayload,
	type ServerIngestPayload,
} from "../../lib/install-normalizer";
import { startOAuthAccessFlow } from "../../lib/oauth-callback-access";
import {
	canIngestFromDataTransfer,
	extractPayloadFromDataTransfer,
	formatServerUniImportTransferError,
} from "../../lib/server-uni-import-transfer";
import { cn } from "../../lib/utils";

const INSPECTOR_MEDIUM_INPUT_CLASS =
	"[&_input]:h-9 [&_input]:px-3 [&_input]:py-1.5 [&_input]:text-sm [&_input]:focus-visible:ring-2 [&_input]:focus-visible:ring-offset-0 [&_button]:focus-visible:ring-offset-0";
const INSPECTOR_MEDIUM_SEGMENT_LIST_CLASS = "min-h-9 h-9 p-1";
const INSPECTOR_MEDIUM_SEGMENT_TRIGGER_CLASS =
	"h-7 px-2 py-1 text-sm leading-none focus-visible:ring-offset-0";
const INSPECTOR_MEDIUM_DOT_CLASS = "size-2.5 border-[1.5px]";
const INSPECTOR_INGEST_MESSAGE =
	"Drag or paste a server config. Click this area to expand it when collapsed.";

export type InspectorConnectedTargetSnapshot =
	| {
		source: "managed";
		serverId: string;
		name: string;
	}
	| {
		source: "scratch";
		scratchId: string;
		name: string;
		config: Record<string, unknown>;
	};

type InspectorConnectServerFormProps = {
	selectedTargetKey: string | null;
	connectedTargetKey: string | null;
	connectedTargetSnapshot: InspectorConnectedTargetSnapshot | null;
	connected: boolean;
	connecting: boolean;
	onConnect: (candidate: InspectorConnectCandidate) => Promise<void> | void;
	onDisconnect: () => void;
	disabled?: boolean;
	onTransportChange?: (transport: ManualServerFormValues["kind"]) => void;
};

export type InspectorConnectCandidate =
	| {
		source: "managed";
		serverId: string;
		draft: ServerInstallDraft;
	}
	| {
		source: "scratch";
		scratchId?: string;
		draft: ServerInstallDraft;
	};

type InspectorConnectServerOption =
	| {
		source: "managed";
		id: string;
		name: string;
		serverType?: string | null;
	}
	| {
		source: "scratch";
		id: string;
		name: string;
		serverType?: string | null;
		config: Record<string, unknown>;
	};

function toRecord(
	fields: Array<{ key?: string; value?: string }> | undefined,
): Record<string, string> {
	return Object.fromEntries(
		(fields ?? [])
			.filter((item) => item.key || item.value)
			.map((item) => [item.key ?? "", item.value ?? ""]),
	);
}

function trimOptional(value?: string | null): string | undefined {
	const trimmed = value?.trim();
	return trimmed ? trimmed : undefined;
}

function keyValueToRecord(
	items?: Array<{ key?: string; value?: string }>,
): Record<string, string> | undefined {
	const pairs = (items ?? [])
		.map((item) => {
			const key = trimOptional(item.key);
			return key ? { key, value: trimOptional(item.value) ?? "" } : null;
		})
		.filter((item): item is { key: string; value: string } => Boolean(item));

	return pairs.length
		? pairs.reduce<Record<string, string>>((accumulator, item) => {
			accumulator[item.key] = item.value;
			return accumulator;
		}, {})
		: undefined;
}

function recordToKeyValueRows(
	record?: Record<string, string>,
): Array<{ key: string; value: string }> {
	return Object.entries(record ?? {}).map(([key, value]) => ({ key, value }));
}

function draftToFormValues(draft: ServerInstallDraft): ManualServerFormValues {
	return {
		name: draft.name,
		kind: draft.kind,
		command: draft.command ?? "",
		url: draft.url ?? "",
		args: (draft.args ?? []).map((value) => ({ value })),
		env: recordToKeyValueRows(draft.env),
		headers: recordToKeyValueRows(draft.headers),
		urlParams: recordToKeyValueRows(draft.urlParams),
		meta_description: "",
		meta_icon_url: "",
		meta_version: "",
		meta_website_url: "",
		meta_repository_url: "",
		meta_repository_source: "",
		meta_repository_subfolder: "",
		meta_repository_id: "",
	};
}

function serverKindFromDetail(
	server: Pick<ServerDetail, "server_type">,
): ManualServerFormValues["kind"] {
	switch (server.server_type) {
		case "sse":
		case "streamable_http":
			return server.server_type;
		default:
			return "stdio";
	}
}

function serverDetailToFormValues(server: ServerDetail): ManualServerFormValues {
	return {
		name: server.name,
		kind: serverKindFromDetail(server),
		command: server.command ?? server.commandPath ?? "",
		url: server.url ?? "",
		args: (server.args ?? []).map((value) => ({ value })),
		env: recordToKeyValueRows(server.env),
		headers: recordToKeyValueRows(server.headers ?? undefined),
		urlParams: [],
		meta_description: "",
		meta_icon_url: "",
		meta_version: "",
		meta_website_url: "",
		meta_repository_url: "",
		meta_repository_source: "",
		meta_repository_subfolder: "",
		meta_repository_id: "",
	};
}

function toInspectorDraft(
	values: ManualServerFormValues,
	serverId?: string | null,
): ServerInstallDraft {
	const repository: RegistryRepositoryInfo = {};
	const repositoryUrl = trimOptional(values.meta_repository_url);
	const repositorySource = trimOptional(values.meta_repository_source);
	const repositorySubfolder = trimOptional(values.meta_repository_subfolder);
	const repositoryId = trimOptional(values.meta_repository_id);
	if (repositoryUrl) repository.url = repositoryUrl;
	if (repositorySource) repository.source = repositorySource;
	if (repositorySubfolder) repository.subfolder = repositorySubfolder;
	if (repositoryId) repository.id = repositoryId;

	const meta: ServerMetaInfo = {};
	const description = trimOptional(values.meta_description);
	const version = trimOptional(values.meta_version);
	const websiteUrl = trimOptional(values.meta_website_url);
	const iconUrl = trimOptional(values.meta_icon_url);
	if (description) meta.description = description;
	if (version) meta.version = version;
	if (websiteUrl) meta.websiteUrl = websiteUrl;
	if (Object.keys(repository).length) meta.repository = repository;
	if (iconUrl) meta.icons = [{ src: iconUrl }];

	const args = (values.args ?? [])
		.map((item) => trimOptional(item.value))
		.filter((value): value is string => Boolean(value));
	const env = keyValueToRecord(values.env);
	const headers = keyValueToRecord(values.headers);
	const urlParams = keyValueToRecord(values.urlParams);

	return {
		name: values.name.trim(),
		serverId: serverId ?? undefined,
		kind: values.kind,
		command: values.kind === "stdio" ? trimOptional(values.command) : undefined,
		url: values.kind === "stdio" ? undefined : trimOptional(values.url),
		args: values.kind === "stdio" && args.length ? args : undefined,
		env: values.kind === "stdio" ? env : undefined,
		headers: values.kind !== "stdio" ? headers : undefined,
		urlParams: values.kind !== "stdio" ? urlParams : undefined,
		meta: Object.keys(meta).length ? meta : undefined,
	};
}

function isValidScratchServerName(name: string): boolean {
	return /^[A-Za-z][A-Za-z0-9 _-]*$/.test(name.trim());
}

function hasMinimumRuntimeConfig(draft: ServerInstallDraft): boolean {
	return draft.kind === "stdio"
		? Boolean(draft.command?.trim())
		: Boolean(draft.url?.trim());
}

function normalizeScratchKind(value: unknown): ManualServerFormValues["kind"] {
	const token = typeof value === "string" ? value.trim().toLowerCase() : "";
	switch (token) {
		case "sse":
		case "server-sent-events":
			return "sse";
		case "streamable_http":
		case "streamable-http":
		case "http":
		case "http_stream":
			return "streamable_http";
		default:
			return "stdio";
	}
}

function recordFromUnknown(value: unknown): Record<string, string> | undefined {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return undefined;
	}
	const entries = Object.entries(value as Record<string, unknown>)
		.filter(([key]) => Boolean(key))
		.map(([key, raw]) => [key, typeof raw === "string" ? raw : String(raw ?? "")]);
	return entries.length
		? Object.fromEntries(entries)
		: undefined;
}

function scratchConfigToDraft(
	name: string,
	config: Record<string, unknown>,
): ServerInstallDraft {
	const kind = normalizeScratchKind(config.type);
	if (kind === "stdio") {
		const args = Array.isArray(config.args)
			? config.args.filter((entry): entry is string => typeof entry === "string")
			: undefined;
		return {
			name,
			kind,
			command: typeof config.command === "string" ? config.command : "",
			args: args?.length ? args : undefined,
			env: recordFromUnknown(config.env),
		};
	}

	return {
		name,
		kind,
		url: typeof config.url === "string" ? config.url : "",
		headers: recordFromUnknown(config.headers),
	};
}

export function InspectorConnectServerForm({
	connectedTargetKey,
	connectedTargetSnapshot,
	connected,
	connecting,
	onConnect,
	onDisconnect,
	disabled = false,
	onTransportChange,
}: InspectorConnectServerFormProps) {
	const { t } = useTranslation("servers");
	const nameId = useId();
	const kindId = useId();
	const commandId = useId();
	const urlId = useId();
	const manualJsonId = useId();
	const { serverTypeOptions } = useServerTypeOptions();
	const [viewMode, setViewMode] = useState<FormViewMode>("form");
	const [jsonText, setJsonText] = useState("");
	const [jsonError] = useState<string | null>(null);
	const [ingestMessage, setIngestMessage] = useState(INSPECTOR_INGEST_MESSAGE);
	const [ingestError, setIngestError] = useState<string | null>(null);
	const [isIngestSuccess, setIsIngestSuccess] = useState(false);
	const [isDropZoneCollapsed, setIsDropZoneCollapsed] = useState(false);
	const [isDragOver, setIsDragOver] = useState(false);
	const [isIngesting, setIsIngesting] = useState(false);
	const [serverPickerOpen, setServerPickerOpen] = useState(false);
	const [selectedServerId, setSelectedServerId] = useState<string | null>(null);
	const [selectedScratchId, setSelectedScratchId] = useState<string | null>(null);
	const [deleteConfirmStates, setDeleteConfirmStates] = useState<
		Record<string, boolean>
	>({});
	const [pendingImportServerId, setPendingImportServerId] = useState<
		string | null
	>(null);
	const pendingImportServerRef = useRef<string | null>(null);
	const lastBackfilledTargetKeyRef = useRef<string | null>(null);
	const { data: serverList, refetch: refetchServerList } = useQuery({
		queryKey: ["inspector-connect-servers"],
		queryFn: serversApi.getAll,
	});
	const { data: scratchServerList, refetch: refetchScratchServerList } = useQuery({
		queryKey: ["inspector-connect-scratch-servers"],
		queryFn: inspectorApi.scratchServerList,
	});
	const serverOptions = useMemo<InspectorConnectServerOption[]>(() => {
		const managedOptions = (serverList?.servers ?? [])
			.filter((server) => server.id && server.name)
			.map((server) => ({
				source: "managed" as const,
				id: server.id,
				name: server.name,
				serverType: server.server_type,
			}));
		const scratchOptions = (scratchServerList?.data?.records ?? [])
			.filter((record) => record.id && record.name && record.config)
			.map((record) => ({
				source: "scratch" as const,
				id: record.id,
				name: record.name,
				serverType: normalizeScratchKind(record.config.type),
				config: record.config,
			}));
		return [...managedOptions, ...scratchOptions].sort((left, right) =>
			left.name.localeCompare(right.name),
		);
	}, [scratchServerList?.data?.records, serverList?.servers]);

	const {
		control,
		register,
		reset,
		setValue,
		watch,
		getValues,
		formState: { errors },
	} = useForm<ManualServerFormValues>({
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
			meta_icon_url: "",
			meta_version: "",
			meta_website_url: "",
			meta_repository_url: "",
			meta_repository_source: "",
			meta_repository_subfolder: "",
			meta_repository_id: "",
		},
	});

	const {
		fields: argFields,
		append: appendArg,
		remove: removeArg,
	} = useFieldArray({ control, name: "args" });

	const {
		fields: envFields,
		append: appendEnvRaw,
		remove: removeEnvRaw,
		replace: replaceEnv,
	} = useFieldArray({ control, name: "env" });

	const {
		fields: headerFields,
		append: appendHeaderRaw,
		remove: removeHeaderRaw,
		replace: replaceHeaders,
	} = useFieldArray({ control, name: "headers" });

	const {
		fields: urlParamFields,
		append: appendUrlParamRaw,
		remove: removeUrlParamRaw,
		replace: replaceUrlParams,
	} = useFieldArray({ control, name: "urlParams" });

	const kind = watch("kind");
	const values = watch();
	const isStdio = kind === "stdio";
	const watchedName = values.name;
	const currentDraft = useMemo(
		() => toInspectorDraft(values, pendingImportServerRef.current),
		[values],
	);
	const connectCandidate = useMemo<InspectorConnectCandidate | null>(() => {
		if (!hasMinimumRuntimeConfig(currentDraft)) return null;
		if (selectedScratchId) {
			return {
				source: "scratch",
				scratchId: selectedScratchId,
				draft: currentDraft,
			};
		}
		if (selectedServerId && pendingImportServerRef.current === selectedServerId) {
			return {
				source: "managed",
				serverId: selectedServerId,
				draft: currentDraft,
			};
		}
		if (!isValidScratchServerName(currentDraft.name)) return null;
		return {
			source: "scratch",
			draft: currentDraft,
		};
	}, [currentDraft, selectedScratchId, selectedServerId]);

	useEffect(() => {
		if (connected) {
			setServerPickerOpen(false);
		}
	}, [connected]);

	useEffect(() => {
		if (!serverPickerOpen) {
			return;
		}
		void refetchServerList();
		void refetchScratchServerList();
	}, [refetchScratchServerList, refetchServerList, serverPickerOpen]);

	useEffect(() => {
		onTransportChange?.(kind);
	}, [kind, onTransportChange]);

	useEffect(() => {
		if (!connected) {
			lastBackfilledTargetKeyRef.current = null;
			return;
		}
		if (!connectedTargetKey || !connectedTargetSnapshot) {
			return;
		}
		if (lastBackfilledTargetKeyRef.current === connectedTargetKey) {
			return;
		}

		let cancelled = false;
		void (async () => {
			try {
				if (connectedTargetSnapshot.source === "managed") {
					const detail = await serversApi.getServer(connectedTargetSnapshot.serverId);
					if (cancelled) return;
					reset(serverDetailToFormValues(detail));
					setSelectedServerId(connectedTargetSnapshot.serverId);
					setSelectedScratchId(null);
					pendingImportServerRef.current = connectedTargetSnapshot.serverId;
					setPendingImportServerId(connectedTargetSnapshot.serverId);
				} else {
					reset(
						draftToFormValues(
							scratchConfigToDraft(
								connectedTargetSnapshot.name,
								connectedTargetSnapshot.config,
							),
						),
					);
					setSelectedServerId(null);
					setSelectedScratchId(connectedTargetSnapshot.scratchId);
					pendingImportServerRef.current = null;
					setPendingImportServerId(null);
				}
				if (cancelled) return;
				setViewMode("form");
				setIngestMessage(`Loaded ${connectedTargetSnapshot.name}.`);
				setIsIngestSuccess(true);
				setIsDropZoneCollapsed(true);
				setIngestError(null);
				lastBackfilledTargetKeyRef.current = connectedTargetKey;
			} catch (error) {
				if (!cancelled) {
					setIngestError(error instanceof Error ? error.message : String(error));
					setIngestMessage(INSPECTOR_INGEST_MESSAGE);
					setIsIngestSuccess(false);
				}
			}
		})();

		return () => {
			cancelled = true;
		};
	}, [connected, connectedTargetKey, connectedTargetSnapshot, reset]);

	const resetForm = useCallback(() => {
		reset({
			name: "",
			kind: "stdio",
			command: "",
			url: "",
			args: [],
			env: [],
			headers: [],
			urlParams: [],
			meta_description: "",
			meta_icon_url: "",
			meta_version: "",
			meta_website_url: "",
			meta_repository_url: "",
			meta_repository_source: "",
			meta_repository_subfolder: "",
			meta_repository_id: "",
		});
		setViewMode("form");
		setIngestMessage(INSPECTOR_INGEST_MESSAGE);
		setIngestError(null);
		setIsIngestSuccess(false);
		setIsDropZoneCollapsed(false);
		setIsDragOver(false);
		setSelectedServerId(null);
		setSelectedScratchId(null);
		pendingImportServerRef.current = null;
		setPendingImportServerId(null);
		lastBackfilledTargetKeyRef.current = null;
	}, [reset]);

	const applyIngestPayload = useCallback(
		async (payload: ServerIngestPayload) => {
			setIsIngesting(true);
			setIngestError(null);
			try {
				const drafts = await normalizeIngestPayload(payload);
				const [draft] = drafts;
				if (!draft) {
					setIngestError("No server configuration detected.");
					setIngestMessage(INSPECTOR_INGEST_MESSAGE);
					setIsIngestSuccess(false);
					return;
				}
				reset(draftToFormValues(draft));
				setViewMode("form");
				setIngestMessage(`Loaded ${draft.name || "server configuration"}.`);
				setIsIngestSuccess(true);
				setIsDropZoneCollapsed(true);
				setSelectedServerId(null);
				setSelectedScratchId(null);
				pendingImportServerRef.current = null;
				setPendingImportServerId(null);
			} catch (error) {
				setIngestError(error instanceof Error ? error.message : String(error));
				setIngestMessage(INSPECTOR_INGEST_MESSAGE);
				setIsIngestSuccess(false);
			} finally {
				setIsIngesting(false);
			}
		},
		[reset],
	);

	const handleServerSelect = useCallback(
		async (server: InspectorConnectServerOption) => {
			setServerPickerOpen(false);
			setIngestError(null);
			setIsIngestSuccess(false);
			setIngestMessage(`Loaded ${server.name}.`);
			if (server.source === "scratch") {
				reset(
					draftToFormValues(scratchConfigToDraft(server.name, server.config)),
				);
				setSelectedServerId(null);
				setSelectedScratchId(server.id);
				pendingImportServerRef.current = null;
				setPendingImportServerId(null);
				setViewMode("form");
				setIsDropZoneCollapsed(true);
				return;
			}

			setSelectedServerId(server.id);
			setSelectedScratchId(null);
			const detail = await serversApi.getServer(server.id);
			reset(serverDetailToFormValues(detail));
			setViewMode("form");
			setIsDropZoneCollapsed(true);
			pendingImportServerRef.current = server.id;
			setPendingImportServerId(server.id);
		},
		[reset],
	);

	const collapseDropZone = useCallback(() => {
		if (!isDropZoneCollapsed) {
			setIsDropZoneCollapsed(true);
		}
	}, [isDropZoneCollapsed]);

	const handleDropZoneClick = useCallback(
		(event: MouseEvent<HTMLButtonElement>) => {
			event.stopPropagation();
			if (isDropZoneCollapsed) {
				setIsDropZoneCollapsed(false);
				setIngestError(null);
				setIsIngestSuccess(false);
				setIngestMessage(INSPECTOR_INGEST_MESSAGE);
			}
		},
		[isDropZoneCollapsed],
	);

	const handleDropZoneDragOver = useCallback(
		(event: DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			setIsDragOver(true);
		},
		[],
	);

	const handleDropZoneDragEnter = useCallback(
		(event: DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			setIsDragOver(true);
			setIngestError(null);
		},
		[],
	);

	const handleDropZoneDragLeave = useCallback(
		(event: DragEvent<HTMLButtonElement>) => {
			event.preventDefault();
			event.stopPropagation();
			if (!event.currentTarget.contains(event.relatedTarget as Node)) {
				setIsDragOver(false);
			}
		},
		[],
	);

	const handleDropZoneDrop = useCallback(
		async (event: DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			setIsDragOver(false);
			try {
				const payload = await extractPayloadFromDataTransfer(event.dataTransfer);
				if (payload) {
					await applyIngestPayload(payload);
				}
			} catch (error) {
				setIngestError(formatServerUniImportTransferError(error, t));
				setIngestMessage(INSPECTOR_INGEST_MESSAGE);
				setIsIngestSuccess(false);
			}
		},
		[applyIngestPayload, t],
	);

	const handleDropZonePaste = useCallback(
		async (event: ClipboardEvent<HTMLButtonElement>) => {
			const text = event.clipboardData.getData("text/plain");
			if (!text) return;
			event.preventDefault();
			event.stopPropagation();
			await applyIngestPayload({ text });
		},
		[applyIngestPayload],
	);

	const handleDeleteClick = useCallback((id: string, removeFn: () => void) => {
		setDeleteConfirmStates((previous) => {
			if (previous[id]) {
				removeFn();
				const { [id]: _omit, ...rest } = previous;
				return rest;
			}
			return { ...previous, [id]: true };
		});
		setTimeout(() => {
			setDeleteConfirmStates((previous) => {
				const { [id]: _omit, ...rest } = previous;
				return rest;
			});
		}, 2000);
	}, []);

	const handleGhostClick = useCallback((addFn: () => void) => {
		addFn();
	}, []);

	const appendEnv = useCallback(
		(value: { key: string; value: string }) => {
			const current = getValues("env") ?? [];
			if (!shouldAppendKeyValueRow(current)) return;
			appendEnvRaw(value);
		},
		[appendEnvRaw, getValues],
	);

	const removeEnv = useCallback(
		(index: number) => {
			removeEnvRaw(index);
			queueMicrotask(() => {
				const current = getValues("env") ?? [];
				const compacted = compactKeyValueFields(current);
				if (compacted.length !== current.length) {
					replaceEnv(compacted);
				}
			});
		},
		[getValues, removeEnvRaw, replaceEnv],
	);

	const appendHeader = useCallback(
		(value: { key: string; value: string }) => {
			const current = getValues("headers") ?? [];
			if (!shouldAppendKeyValueRow(current)) return;
			appendHeaderRaw(value);
		},
		[appendHeaderRaw, getValues],
	);

	const removeHeader = useCallback(
		(index: number) => {
			removeHeaderRaw(index);
			queueMicrotask(() => {
				const current = getValues("headers") ?? [];
				const compacted = compactKeyValueFields(current);
				if (compacted.length !== current.length) {
					replaceHeaders(compacted);
				}
			});
		},
		[getValues, removeHeaderRaw, replaceHeaders],
	);

	const appendUrlParam = useCallback(
		(value: { key: string; value: string }) => {
			const current = getValues("urlParams") ?? [];
			if (!shouldAppendKeyValueRow(current)) return;
			appendUrlParamRaw(value);
		},
		[appendUrlParamRaw, getValues],
	);

	const removeUrlParam = useCallback(
		(index: number) => {
			removeUrlParamRaw(index);
			queueMicrotask(() => {
				const current = getValues("urlParams") ?? [];
				const compacted = compactKeyValueFields(current);
				if (compacted.length !== current.length) {
					replaceUrlParams(compacted);
				}
			});
		},
		[getValues, removeUrlParamRaw, replaceUrlParams],
	);

	const secretOriginBase = useMemo<SecretOrigin>(
		() => ({
			server_id: null,
			server_name: watchedName?.trim() || null,
			server_kind: kind,
			source: "inspector",
		}),
		[kind, watchedName],
	);

	const handleInitiateOAuth = useCallback(
		async (config: OAuthConfigRequest) => {
			const draft = toInspectorDraft(getValues(), pendingImportServerRef.current);
			if (!draft.name) {
				throw new Error(
					t("manual.errors.nameRequired", {
						defaultValue: "Name is required",
					}),
				);
			}

			let targetServerId = pendingImportServerRef.current;
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

			await startOAuthAccessFlow(targetServerId, config);
		},
		[getValues, t],
	);

	useEffect(() => {
		setJsonText(
			JSON.stringify(
				{
					name: values.name ?? "",
					kind: values.kind ?? "stdio",
					...(values.kind === "stdio"
						? {
							command: values.command ?? "",
							args: (values.args ?? [])
								.map((item) => item.value)
								.filter(Boolean),
							env: toRecord(values.env),
						}
						: {
							url: values.url ?? "",
							urlParams: toRecord(values.urlParams),
							headers: toRecord(values.headers),
						}),
				},
				null,
				2,
			),
		);
	}, [values]);

	const statusDetail = connected
		? "Inspector session is ready."
		: connecting
			? "Opening Inspector session..."
			: connectCandidate
				? "Ready to connect with the current server parameters."
				: currentDraft.kind === "stdio" && currentDraft.command?.trim()
					? "Scratch server name must start with an ASCII letter."
					: currentDraft.kind !== "stdio" && currentDraft.url?.trim()
						? "Scratch server name must start with an ASCII letter."
						: "Configure a server, then connect when the runtime draft is ready.";

	const statusColor = connected
		? "bg-green-500"
		: connecting
			? "bg-blue-500"
			: "bg-muted-foreground/30";

	const formContent = (
		<div className="mx-3 box-border flex flex-col gap-3 overflow-visible pb-px pt-3">
				<div className="flex items-center gap-3">
					<Label htmlFor={nameId} className={SERVER_INSTALL_FORM_ROW_LABEL_CLASS}>
						{t("manual.fields.name.label", { defaultValue: "Name" })}
					</Label>
					<div className="flex min-w-0 flex-1 items-center gap-2">
						<Input
							id={nameId}
							{...register("name")}
							placeholder={t("manual.fields.name.placeholder", {
								defaultValue: "e.g., local-mcp",
							})}
							disabled={disabled}
						/>
						{!connected ? (
							<Popover open={serverPickerOpen} onOpenChange={setServerPickerOpen}>
								<PopoverTrigger asChild>
									<Button
										type="button"
										variant="outline"
										size="icon"
										className="h-9 w-9 shrink-0"
										disabled={disabled}
										aria-label="Choose existing server"
										title="Choose existing server"
									>
										<ChevronsUpDown className="h-4 w-4 opacity-60" />
									</Button>
								</PopoverTrigger>
								<PopoverContent
									align="end"
									className="w-[min(420px,var(--radix-popover-content-available-width))] overflow-hidden p-0"
								>
									<Command className="max-h-full [&_[cmdk-list-sizer]]:w-full">
										<CommandInput placeholder="Search servers..." />
										<CommandList className="max-h-[280px] !overflow-x-visible !overflow-y-visible overscroll-contain p-0">
											<CommandEmpty>No servers found.</CommandEmpty>
											{serverOptions.map((server) => {
												const SourceIcon =
													server.source === "managed" ? Database : Pencil;
												const isSelected =
													(server.source === "managed" &&
														selectedServerId === server.id) ||
													(server.source === "scratch" &&
														selectedScratchId === server.id);
												return (
													<CommandItem
														key={`${server.source}:${server.id}`}
														value={`${server.name} ${server.id} ${server.serverType ?? ""} ${server.source}`}
														onSelect={() => void handleServerSelect(server)}
														className="w-full gap-2 rounded-none px-3 py-2"
													>
														<SourceIcon
															className={cn(
																"h-4 w-4 shrink-0",
																isSelected
																	? "text-slate-950 dark:text-slate-50"
																	: "text-muted-foreground",
															)}
															aria-hidden
														/>
														<span className="sr-only">
															{server.source === "managed" ? "Managed" : "Scratch"}
														</span>
														<span className="min-w-0 flex-1 truncate font-medium">
															{server.name}
														</span>
														<span className="shrink-0 text-xs text-muted-foreground">
															{server.serverType ?? "unknown"}
														</span>
													</CommandItem>
												);
											})}
										</CommandList>
									</Command>
								</PopoverContent>
							</Popover>
						) : null}
					</div>
				</div>
			<div className="flex items-center gap-3">
				<Label htmlFor={kindId} className={SERVER_INSTALL_FORM_ROW_LABEL_CLASS}>
					{t("manual.fields.type.label", { defaultValue: "Type" })}
				</Label>
				<div className="min-w-0 flex-1">
					<Segment
						options={serverTypeOptions}
						value={kind}
						onValueChange={(value) =>
							setValue("kind", value as ManualServerFormValues["kind"], {
								shouldDirty: true,
								shouldTouch: true,
							})
						}
						showDots
						disabled={disabled}
						listClassName={INSPECTOR_MEDIUM_SEGMENT_LIST_CLASS}
						triggerClassName={INSPECTOR_MEDIUM_SEGMENT_TRIGGER_CLASS}
						dotClassName={INSPECTOR_MEDIUM_DOT_CLASS}
					/>
				</div>
			</div>

			<CommandField
				kind={kind}
				control={control}
				errors={errors}
				commandId={commandId}
				urlId={urlId}
				viewMode={viewMode}
				secretOriginBase={secretOriginBase}
			/>

			<ServerAuthSection
				serverId={pendingImportServerId ?? undefined}
				isStdio={isStdio}
				viewMode={viewMode}
				isNewServer
				onInitiateOAuth={handleInitiateOAuth}
				onOAuthConnected={(serverId) => {
					pendingImportServerRef.current = serverId;
					setPendingImportServerId(serverId);
				}}
				className="space-y-3"
				segmentListClassName={INSPECTOR_MEDIUM_SEGMENT_LIST_CLASS}
				segmentTriggerClassName={INSPECTOR_MEDIUM_SEGMENT_TRIGGER_CLASS}
				segmentDotClassName={INSPECTOR_MEDIUM_DOT_CLASS}
				oauthPanelClassName="space-y-3 p-3"
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
				control={control}
				deleteConfirmStates={deleteConfirmStates}
				onDeleteClick={handleDeleteClick}
				onGhostClick={handleGhostClick}
				secretOriginBase={secretOriginBase}
				className="space-y-3"
				getEnvRowKeyAt={(index) =>
					getValues(`env.${index}.key`)?.trim() || undefined
				}
			/>

			<UrlParams
				viewMode={viewMode}
				isStdio={isStdio}
				urlParamFields={urlParamFields}
				removeUrlParam={removeUrlParam}
				appendUrlParam={appendUrlParam}
				register={register}
				control={control}
				deleteConfirmStates={deleteConfirmStates}
				onDeleteClick={handleDeleteClick}
				onGhostClick={handleGhostClick}
				secretOriginBase={secretOriginBase}
				getRowKeyAt={(index) =>
					getValues(`urlParams.${index}.key`)?.trim() || undefined
				}
			/>

			<HttpHeaders
				viewMode={viewMode}
				isStdio={isStdio}
				headerFields={headerFields}
				removeHeader={removeHeader}
				appendHeader={appendHeader}
				register={register}
				control={control}
				deleteConfirmStates={deleteConfirmStates}
				onDeleteClick={handleDeleteClick}
				onGhostClick={handleGhostClick}
				secretOriginBase={secretOriginBase}
				getRowKeyAt={(index) =>
					getValues(`headers.${index}.key`)?.trim() || undefined
				}
			/>
		</div>
	);

	return (
		<div
			className={cn(
				"flex h-full min-h-0 flex-col bg-background",
				INSPECTOR_MEDIUM_INPUT_CLASS,
			)}
		>
			{!connected ? (
				<ServerImportDropZone
					collapsed={isDropZoneCollapsed}
					message={ingestMessage}
					error={ingestError}
					success={isIngestSuccess}
					dragOver={isDragOver}
					ingesting={isIngesting}
					className="mx-3 w-auto shrink-0"
					onClick={handleDropZoneClick}
					onDragOver={handleDropZoneDragOver}
					onDragEnter={handleDropZoneDragEnter}
					onDragLeave={handleDropZoneDragLeave}
					onDrop={handleDropZoneDrop}
					onPaste={handleDropZonePaste}
				/>
			) : null}

				<div
					className="mb-3 flex min-h-0 flex-1 flex-col overflow-visible"
					onFocusCapture={collapseDropZone}
					onPointerDown={collapseDropZone}
				>
					<CoreConfigTabPanel
						viewMode={viewMode}
						onViewModeChange={setViewMode}
						toolbarInsideScroll
						toolbarClassName={cn("mx-3 px-0 pb-0", connected ? "pt-0" : "pt-3")}
						scrollClassName="space-y-0 overflow-y-auto overscroll-contain p-0"
						formContent={formContent}
						jsonContent={
							<ServerConfigJsonPanel
								id={manualJsonId}
								label={t("manual.fields.json.label", { defaultValue: "JSON" })}
								jsonText={jsonText}
								jsonError={jsonError}
								jsonEditingEnabled
								className="pr-3 pt-3"
								onJsonChange={setJsonText}
								copyLabel={t("manual.fields.json.copy", {
									defaultValue: "Copy JSON",
								})}
							/>
						}
					/>
				</div>

				<div className="mx-3 flex shrink-0 flex-col gap-1.5 pt-0 sm:flex-row sm:items-center sm:justify-between">
					<div className="flex min-w-0 items-center gap-2.5">
						<span className={cn("h-2.5 w-2.5 shrink-0 rounded-full", statusColor)} />
						<p className="min-w-0 truncate text-xs text-muted-foreground">
							{statusDetail}
						</p>
					</div>
					<div className="flex shrink-0 items-center justify-end gap-2">
						{!connected ? (
							<Button
								type="button"
								variant="outline"
								onClick={resetForm}
								disabled={connecting || disabled}
								size="sm"
								className="gap-2"
							>
								<RotateCcw className="h-4 w-4" />
								Reset
							</Button>
						) : null}
						<Button
							type="button"
							onClick={() => {
								if (connected) {
									onDisconnect();
									return;
								}
								if (connectCandidate) {
									void onConnect(connectCandidate);
								}
							}}
							disabled={connecting || disabled || (!connected && !connectCandidate)}
							size="sm"
							className="min-w-32 gap-2"
						>
							{connecting ? (
								<Loader2 className="h-4 w-4 animate-spin" />
							) : connected ? (
								<Unplug className="h-4 w-4" />
							) : (
								<Plug className="h-4 w-4" />
							)}
							{connecting ? "Connecting" : connected ? "Disconnect" : "Connect"}
						</Button>
				</div>
			</div>
		</div>
	);
}
