import {
	AlertCircle,
	AlertTriangle,
	CheckCircle2,
	ChevronDown,
	ChevronsUpDown,
	Copy,
	Eraser,
	Loader2,
	RefreshCw,
	ShieldAlert,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	configSuitsApi,
	inspectorApi,
	isInspectorSessionUnavailableError,
	systemApi,
} from "../lib/api";
import { writeClipboardText } from "../lib/clipboard";
import { smartFormat } from "../lib/format";
import { usePageTranslations } from "../lib/i18n/usePageTranslations";
import {
	extractInspectorResourceTemplateParameters,
	pickInspectorResourceTemplateForMode,
} from "../lib/inspector-resource-template";
import {
	getInspectorModeIdentity,
	getInspectorOperationLabelKey,
	getInspectorPrimaryActionKey,
	normalizeInspectorCapabilityOption,
	resolveInspectorCounterpartIdentity,
	shouldAutoLoadInspectorOptions,
	switchInspectorOperationSnapshot,
	type InspectorOperationKind,
} from "../lib/inspector-operation";
import { notifyError, notifySuccess } from "../lib/notify";
import type { InspectorSessionOpenData, InspectorSseEvent } from "../lib/types";
import type {
	CapabilityArgument,
	CapabilityRecord,
} from "../types/capabilities";
import type { JsonObject, JsonSchema, JsonValue } from "../types/json";
import CapabilityCombobox from "./capability-combobox";
import { CardListScrollBody } from "./card-list-scroll-body";
import { SchemaForm } from "./schema-form";
import { defaultFromSchema } from "./schema-form-utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ButtonGroup } from "./ui/button-group";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Textarea } from "./ui/textarea";
import {
	Tooltip,
	TooltipArrow,
	TooltipContent,
	TooltipPortal,
	TooltipProvider,
	TooltipTrigger,
} from "./ui/tooltip";

type InspectorKind = InspectorOperationKind;
type InspectorMode = "proxy" | "native";
type InspectorCapabilityOptionsByKind = Partial<
	Record<InspectorKind, CapabilityRecord[]>
>;

export interface InspectorDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	serverId?: string;
	serverName?: string;
	kind: InspectorKind;
	item: CapabilityRecord | null;
	capabilityOptionsByKind?: InspectorCapabilityOptionsByKind;
	onLog?: (entry: InspectorLogEntry) => void;
}

type Field = {
	name: string;
	type: string;
	required?: boolean;
	description?: string;
	enum?: string[];
	default?: JsonValue;
};

export interface InspectorLogEntry {
	id: string;
	timestamp: number;
	channel: "inspector";
	event: "request" | "success" | "error" | "progress" | "log" | "cancelled";
	method: string;
	mode: InspectorMode;
	payload?: unknown;
	message?: string;
}

function newLogId() {
	if (typeof crypto !== "undefined" && crypto.randomUUID)
		return crypto.randomUUID();
	return `log_${Date.now()}_${Math.random().toString(16).slice(2, 8)}`;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

const toCapabilityRecord = (value: unknown): CapabilityRecord | null =>
	isRecord(value) ? (value as CapabilityRecord) : null;

const toStringValue = (value: unknown): string | undefined =>
	typeof value === "string" ? value : undefined;

const isJsonObjectValue = (value: unknown): value is JsonObject =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

const toJsonObject = (value: JsonValue | undefined): JsonObject =>
	isJsonObjectValue(value) ? value : {};

const getDefaultValue = (record: CapabilityRecord): JsonValue | undefined =>
	Object.prototype.hasOwnProperty.call(record, "default")
		? (record.default as JsonValue)
		: undefined;

const toSchema = (value: unknown): JsonSchema | null => {
	const record = toCapabilityRecord(value);
	if (!record) return null;
	const nested = toCapabilityRecord(record.schema);
	if (nested) return nested as JsonSchema;
	return record as JsonSchema;
};

const normalizeArguments = (value: unknown): CapabilityArgument[] => {
	if (!Array.isArray(value)) return [];
	return value.map((entry, index) => {
		const record = toCapabilityRecord(entry);
		if (!record) {
			return { name: `arg_${index}` };
		}
		return {
			name: toStringValue(record.name) ?? `arg_${index}`,
			type: toStringValue(record.type) ?? "string",
			description: toStringValue(record.description),
			default: getDefaultValue(record),
			required:
				typeof record.required === "boolean" ? record.required : undefined,
		};
	});
};

const buildSchemaFromArguments = (args: CapabilityArgument[]): JsonSchema => {
	const properties: Record<string, JsonSchema> = {};
	const required: string[] = [];
	args.forEach((arg, index) => {
		const name = arg.name ?? `arg_${index}`;
		properties[name] = {
			type: arg.type ?? "string",
			description: arg.description,
		};
		if (arg.default !== undefined) {
			properties[name].default = arg.default;
		}
		if (arg.required) {
			required.push(name);
		}
	});
	return {
		type: "object",
		properties,
		required: required.length ? required : undefined,
	};
};

const buildSchemaFromFields = (fields: Field[]): JsonSchema => {
	const properties: Record<string, JsonSchema> = {};
	const required: string[] = [];
	fields.forEach((field) => {
		properties[field.name] = {
			type: field.type || "string",
			description: field.description,
			enum: field.enum,
		};
		if (field.default !== undefined) {
			properties[field.name].default = field.default;
		}
		if (field.required) {
			required.push(field.name);
		}
	});
	return {
		type: "object",
		properties,
		required: required.length ? required : undefined,
	};
};

type InspectorResponse<T = unknown> = {
	success?: boolean;
	data?: T | null;
	error?: unknown;
};

type InspectorEventEntry = {
	data: InspectorSseEvent;
	timestamp: number;
};

type InspectorFormSnapshot = {
	argsJson: string;
	useRaw: boolean;
	values: JsonObject;
};

type InspectorOperationSnapshot = {
	overrideItem: CapabilityRecord | null;
	ignorePropItem: boolean;
	name: string;
	uri: string;
	formCollapsed: boolean;
};

const TOOL_KIND_KEYS: Array<keyof CapabilityRecord> = [
	"unique_name",
	"tool_name",
	"name",
];

const PROMPT_KIND_KEYS: Array<keyof CapabilityRecord> = [
	"unique_name",
	"prompt_name",
	"name",
];

const RESOURCE_KIND_KEYS: Array<keyof CapabilityRecord> = [
	"unique_uri",
	"resource_uri",
	"uri",
	"name",
];

const TEMPLATE_KIND_KEYS: Array<keyof CapabilityRecord> = [
	"unique_uri_template",
	"unique_name",
	"uriTemplate",
	"uri_template",
	"name",
];

const INSPECT_SESSION_GRACE_MS = 30_000;
const INSPECTOR_OPERATIONS: InspectorKind[] = [
	"tool",
	"prompt",
	"resource",
	"template",
];

function computeRecordKey(
	record: CapabilityRecord | null,
	kind: InspectorKind,
): string {
	if (!record) return "";
	const sources =
		kind === "tool"
			? TOOL_KIND_KEYS
			: kind === "prompt"
				? PROMPT_KIND_KEYS
				: kind === "template"
					? TEMPLATE_KIND_KEYS
					: RESOURCE_KIND_KEYS;
	for (const key of sources) {
		const value = toStringValue(record[key]);
		if (value) return value;
	}
	return "";
}

function formatEventLabel(
	entry: InspectorEventEntry,
	t: (key: string, options?: Record<string, unknown>) => string,
): string {
	switch (entry.data.event) {
		case "started":
			return t("eventLabels.started");
		case "progress":
			return entry.data.total
				? `${t("eventLabels.progress")} ${entry.data.progress}/${entry.data.total}`
				: `${t("eventLabels.progress")} ${entry.data.progress}`;
		case "log":
			return entry.data.logger || entry.data.level || t("eventLabels.log");
		case "result":
			return t("eventLabels.result");
		case "error":
			return t("eventLabels.error");
		case "cancelled":
			return t("eventLabels.cancelled");
		default:
			return t("eventLabels.unknown", { defaultValue: "Unknown" });
	}
}

function formatEventDetails(
	entry: InspectorEventEntry,
	t: (key: string, options?: Record<string, unknown>) => string,
): string | null {
	const { data } = entry;
	switch (data.event) {
		case "started":
			return t("eventDetails.session", { sessionId: data.session_id ?? "n/a" });
		case "progress":
			return data.message ?? null;
		case "log":
			return smartFormat(data.data);
		case "result":
			return t("eventDetails.elapsed", { elapsedMs: data.elapsed_ms });
		case "error":
			return data.message;
		case "cancelled":
			return data.reason ?? null;
		default:
			return null;
	}
}

// Note: smartFormat centralizes pretty rendering for logs and results

function formatTimestamp(ts: number): string {
	return new Date(ts).toLocaleTimeString([], {
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
	});
}

function badgeVariantForEvent(
	event: InspectorSseEvent["event"],
): "default" | "secondary" | "destructive" | "outline" {
	switch (event) {
		case "started":
			return "secondary";
		case "progress":
			return "default";
		case "log":
			return "outline";
		case "result":
			return "default";
		case "error":
		case "cancelled":
			return "destructive";
		default:
			return "outline";
	}
}

function pickToolNameForMode(
	source: CapabilityRecord | null,
	mode: InspectorMode,
): string {
	if (!source) return "";
	const uniqueName = toStringValue(source.unique_name);
	const toolName = toStringValue(source.tool_name);
	const rawName = toStringValue(source.name);
	if (mode === "proxy") {
		return uniqueName || "";
	}
	return toolName || rawName || "";
}

function pickPromptNameForMode(
	source: CapabilityRecord | null,
	mode: InspectorMode,
): string {
	if (!source) return "";
	const uniqueName = toStringValue(source.unique_name);
	const promptName = toStringValue(source.prompt_name);
	const rawName = toStringValue(source.name);
	if (mode === "proxy") {
		return uniqueName || "";
	}
	return promptName || rawName || "";
}

function pickTemplateName(
	source: CapabilityRecord | null,
	mode: InspectorMode,
): string {
	return pickInspectorResourceTemplateForMode(source, mode);
}

function pickResourceUriForMode(
	source: CapabilityRecord | null,
	mode: InspectorMode,
): string {
	if (!source) return "";
	const uniqueUri = toStringValue(source.unique_uri);
	const resourceUri = toStringValue(source.resource_uri);
	const rawUri = toStringValue(source.uri);
	if (mode === "proxy") {
		return uniqueUri || "";
	}
	return resourceUri || rawUri || "";
}

function normalizeCapabilityOptions(
	resp: InspectorResponse<any> | undefined,
	kind: InspectorKind,
	mode: InspectorMode,
): CapabilityRecord[] {
	const data = resp?.data;
	const rawList =
		kind === "tool"
			? data?.tools
			: kind === "prompt"
				? data?.prompts
				: kind === "resource"
					? data?.resources
					: data?.templates;

	return Array.isArray(rawList)
		? rawList
			.map((entry: unknown) => toCapabilityRecord(entry))
			.map((entry) =>
				entry
					? (normalizeInspectorCapabilityOption(
							kind,
							mode,
							entry,
						) as CapabilityRecord)
					: null,
			)
			.filter(Boolean) as CapabilityRecord[]
		: [];
}

export function InspectorDrawer({
	open,
	onOpenChange,
	serverId,
	serverName,
	kind,
	item,
	capabilityOptionsByKind,
	onLog,
}: InspectorDrawerProps) {
	const { t } = useTranslation("inspector");
	usePageTranslations("inspector");
	const drawerContentRef = useRef<HTMLDivElement | null>(null);
	const [activeKind, setActiveKind] = useState<InspectorKind>(kind);
	const [mode, setMode] = useState<InspectorMode>("native");
	const [timeoutMs, setTimeoutMs] = useState<number>(8000);
	const [timeoutInitialized, setTimeoutInitialized] = useState(false);

	useEffect(() => {
		if (open && !timeoutInitialized) {
			systemApi
				.getSettings()
				.then((settings) => {
					setTimeoutMs(settings.inspector_timeout_ms);
					setTimeoutInitialized(true);
				})
				.catch(() => {
					setTimeoutInitialized(true);
				});
		}
	}, [open, timeoutInitialized]);

	const [argsJson, setArgsJson] = useState<string>("{}");
	const [useRaw, setUseRaw] = useState(false);
	const rawArgumentRows = useMemo(() => {
		const lineCount = argsJson.split(/\r\n|\r|\n/).length;
		return Math.max(3, lineCount);
	}, [argsJson]);
	const [fields, setFields] = useState<Field[]>([]);
	const [values, setValues] = useState<JsonObject>({});
	const [schemaObj, setSchemaObj] = useState<JsonSchema | null>(null);
	const formSnapshotsRef = useRef<Map<string, InspectorFormSnapshot>>(
		new Map(),
	);
	const operationSnapshotsRef = useRef<
		Map<string, InspectorOperationSnapshot>
	>(new Map());
	const initializedFormKeyRef = useRef<string>("");
	const [overrideItem, setOverrideItem] = useState<CapabilityRecord | null>(
		null,
	);
	const [ignorePropItem, setIgnorePropItem] = useState(false);
	const currentItem =
		overrideItem ??
		(!ignorePropItem && activeKind === kind ? item : null);
	const [uri, setUri] = useState<string>(
		String(currentItem?.resource_uri || currentItem?.uri || ""),
	);
	const [name, setName] = useState<string>(
		String(
			currentItem?.unique_name ||
			currentItem?.tool_name ||
			currentItem?.prompt_name ||
			currentItem?.name ||
			"",
		),
	);
	const [submitting, setSubmitting] = useState(false);
	const [cancelling, setCancelling] = useState(false);
	const [result, setResult] = useState<unknown>(null);
	const [responseActionsHidden, setResponseActionsHidden] = useState(false);
	const responseActionsHideTimer = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const [events, setEvents] = useState<InspectorEventEntry[]>([]);
	const eventsEndRef = useRef<HTMLDivElement | null>(null);
	const wsRef = useRef<WebSocket | null>(null);
	const [activeCallId, setActiveCallId] = useState<string | null>(null);
	const activeCallIdRef = useRef<string | null>(null);
	const [capOptions, setCapOptions] = useState<CapabilityRecord[]>([]);
	const capOptionsCacheRef = useRef<Map<string, CapabilityRecord[]>>(new Map());
	const autoAttemptedOptionKeysRef = useRef<Set<string>>(new Set());
	const pendingOptionListKeysRef = useRef<Set<string>>(new Set());
	const pendingCounterpartSelectionRef = useRef<{
		kind: InspectorKind;
		mode: InspectorMode;
		identity: string;
	} | null>(null);
	const activeOptionListKeyRef = useRef("");
	const [capOptionsLoading, setCapOptionsLoading] = useState(false);
	const [capOptionsError, setCapOptionsError] = useState<string | null>(null);
	const [listedOptionKeys, setListedOptionKeys] = useState<Set<string>>(
		() => new Set(),
	);
	const [nativeSession, setNativeSession] =
		useState<InspectorSessionOpenData | null>(null);
	const nativeSessionRef = useRef<InspectorSessionOpenData | null>(null);
	const pendingNativeSessionRef = useRef<{
		serverId: string;
		promise: Promise<InspectorSessionOpenData>;
	} | null>(null);
	const nativeSessionCloseTimer = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const mountedRef = useRef(true);
	const [view, setView] = useState<"response" | "events">("response");
	const [operationMenuOpen, setOperationMenuOpen] = useState(false);
	const [drawerInitialized, setDrawerInitialized] = useState(false);

	// combobox open/width is handled in CapabilityCombobox
	const [formCollapsed, setFormCollapsed] = useState(false);

	// Combobox state is managed directly by Popover component
	const activeProfilesQ = useQuery({
		queryKey: ["inspector-proxy-profiles", serverId],
		enabled: open && mode === "proxy" && Boolean(serverId),
		queryFn: async () => {
			const suitsResp = await configSuitsApi.getAll();
			const active = suitsResp.suits.filter((suit) => suit.is_active);
			const enabled: string[] = [];
			await Promise.all(
				active.map(async (suit) => {
					try {
						const res = await configSuitsApi.getServers(suit.id);
						const match = (res.servers || []).find(
							(server) => server.id === serverId && server.enabled,
						);
						if (match) {
							enabled.push(suit.name || suit.id);
						}
					} catch (error) {
						console.error("Failed to load servers for suit", suit.id, error);
					}
				}),
			);
			return enabled;
		},
	});
	const proxyAvailable = (activeProfilesQ.data?.length ?? 0) > 0;
	const isProxyChecking =
		mode === "proxy" && activeProfilesQ.isFetching && !activeProfilesQ.isFetched;
	const canUseCurrentMode = mode === "native" || proxyAvailable;
	const proxyUnavailable = mode === "proxy" && !isProxyChecking && !proxyAvailable;
	const optionListKey = `${serverId || serverName || "unknown"}:${mode}:${activeKind}`;
	activeOptionListKeyRef.current = open ? optionListKey : "";
	const hasListedOptions = listedOptionKeys.has(optionListKey);
	const capabilityMappings = useMemo(
		() => capabilityOptionsByKind?.[activeKind] ?? [],
		[activeKind, capabilityOptionsByKind],
	);
	const hasProvidedOptions =
		capabilityOptionsByKind?.[activeKind] !== undefined;
	const propItemKey = useMemo(() => computeRecordKey(item, kind), [item, kind]);
	const currentItemKey = useMemo(
		() => computeRecordKey(currentItem, activeKind),
		[activeKind, currentItem],
	);
	const currentModeIdentity = useMemo(
		() =>
			currentItem
				? getInspectorModeIdentity(activeKind, mode, currentItem)
				: "",
		[activeKind, currentItem, mode],
	);
	const formStateKey = useMemo(
		() => (currentItemKey ? `${mode}:${activeKind}:${currentItemKey}` : ""),
		[activeKind, currentItemKey, mode],
	);
	const lastPropKeyRef = useRef<string>(propItemKey);
	const wasOpenRef = useRef<boolean>(false);

	useEffect(() => {
		mountedRef.current = true;
		return () => {
			mountedRef.current = false;
		};
	}, []);

	useEffect(() => {
		nativeSessionRef.current = nativeSession;
	}, [nativeSession]);

	const clearNativeSessionCloseTimer = useCallback(() => {
		if (nativeSessionCloseTimer.current) {
			clearTimeout(nativeSessionCloseTimer.current);
			nativeSessionCloseTimer.current = null;
		}
	}, []);

	const closeNativeSession = useCallback(
		async (session: InspectorSessionOpenData) => {
			try {
				await inspectorApi.sessionClose({ session_id: session.session_id });
			} catch (error) {
				console.warn("Failed to close inspector session", error);
			}
			if (
				mountedRef.current &&
				nativeSessionRef.current?.session_id === session.session_id
			) {
				setNativeSession(null);
				nativeSessionRef.current = null;
			}
		},
		[],
	);

	const invalidateNativeSession = useCallback(() => {
		clearNativeSessionCloseTimer();
		pendingNativeSessionRef.current = null;
		const current = nativeSessionRef.current;
		nativeSessionRef.current = null;
		if (mountedRef.current) {
			setNativeSession(null);
		}
		if (current) {
			void closeNativeSession(current);
		}
	}, [clearNativeSessionCloseTimer, closeNativeSession]);

	const scheduleNativeSessionClose = useCallback(
		(session: InspectorSessionOpenData) => {
			clearNativeSessionCloseTimer();
			nativeSessionCloseTimer.current = setTimeout(() => {
				void closeNativeSession(session);
			}, INSPECT_SESSION_GRACE_MS);
		},
		[clearNativeSessionCloseTimer, closeNativeSession],
	);

	const ensureNativeSession = useCallback(async (): Promise<
		string | undefined
	> => {
		if (!serverId) {
			return undefined;
		}
		clearNativeSessionCloseTimer();

		const current = nativeSessionRef.current;
		if (current?.mode === "native" && current.server_id === serverId) {
			return current.session_id;
		}

		const pending = pendingNativeSessionRef.current;
		if (pending?.serverId === serverId) {
			const session = await pending.promise;
			return session.session_id;
		}

		if (current) {
			await closeNativeSession(current);
		}

		const pendingPromise = inspectorApi
			.sessionOpen({
				mode: "native",
				server_id: serverId,
				server_name: serverName,
				timeout_ms: timeoutMs,
			})
			.then((response) => {
				if (!response?.success || !response.data) {
					throw new Error(
						response?.error
							? String(response.error)
							: "Failed to open inspector session",
					);
				}
				return response.data;
			});
		pendingNativeSessionRef.current = {
			serverId,
			promise: pendingPromise,
		};

		try {
			const session = await pendingPromise;
			if (pendingNativeSessionRef.current?.promise !== pendingPromise) {
				void closeNativeSession(session);
				return undefined;
			}
			pendingNativeSessionRef.current = null;
			if (mountedRef.current) {
				setNativeSession(session);
			}
			nativeSessionRef.current = session;
			return session.session_id;
		} catch (error) {
			if (pendingNativeSessionRef.current?.promise === pendingPromise) {
				pendingNativeSessionRef.current = null;
			}
			throw error;
		}
	}, [
		clearNativeSessionCloseTimer,
		closeNativeSession,
		serverId,
		serverName,
		timeoutMs,
	]);

	useEffect(() => {
		const current = nativeSessionRef.current;
		if (!current) {
			return;
		}

		if (open && mode === "native" && current.server_id === serverId) {
			clearNativeSessionCloseTimer();
			return;
		}

		scheduleNativeSessionClose(current);
	}, [clearNativeSessionCloseTimer, mode, open, scheduleNativeSessionClose, serverId]);

	useEffect(() => {
		if (!open || mode !== "native") {
			return;
		}
		void ensureNativeSession().catch((error) => {
			notifyError(
				t("notifications.failed"),
				error instanceof Error ? error.message : String(error ?? ""),
			);
		});
	}, [ensureNativeSession, mode, open, t]);

	useEffect(() => {
		return () => {
			const current = nativeSessionRef.current;
			if (current) {
				scheduleNativeSessionClose(current);
			}
		};
	}, [scheduleNativeSessionClose]);

	useEffect(() => {
		if (propItemKey !== lastPropKeyRef.current) {
			setOverrideItem(null);
			setIgnorePropItem(false);
			lastPropKeyRef.current = propItemKey;
		}
	}, [propItemKey]);

	useEffect(() => {
		if (open && !wasOpenRef.current) {
			setDrawerInitialized(false);
			setActiveKind(kind);
			setMode("native");
			setResult(null);
			setEvents([]);
			setView("response");
			setOverrideItem(null);
			setIgnorePropItem(false);
			capOptionsCacheRef.current.clear();
			autoAttemptedOptionKeysRef.current.clear();
			pendingCounterpartSelectionRef.current = null;
			operationSnapshotsRef.current.clear();
			setListedOptionKeys(new Set());
			setCapOptionsError(null);
			setFormCollapsed(false);
			setDrawerInitialized(true);
		}
		if (!open && wasOpenRef.current) {
			setDrawerInitialized(false);
			setOverrideItem(null);
			setIgnorePropItem(false);
			formSnapshotsRef.current.clear();
			operationSnapshotsRef.current.clear();
			pendingCounterpartSelectionRef.current = null;
			initializedFormKeyRef.current = "";
			setEvents([]);
			setActiveCallId(null);
			activeCallIdRef.current = null;
			setSubmitting(false);
			setCancelling(false);
			if (wsRef.current) {
				wsRef.current.close();
				wsRef.current = null;
			}
		}
		wasOpenRef.current = open;
	}, [kind, open]);

	useEffect(() => {
		if (!open) {
			return;
		}
		setResult(null);
		setEvents([]);
		setView("response");
		setSubmitting(false);
		setCancelling(false);
		setActiveCallId(null);
		activeCallIdRef.current = null;
		if (wsRef.current) {
			wsRef.current.close();
			wsRef.current = null;
		}
	}, [activeKind, currentItemKey, open]);

	useEffect(() => {
		eventsEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
	}, [events]);

	useEffect(() => {
		if (!open) {
			return;
		}
		const cacheKey = `${mode}:${activeKind}`;
		if (hasListedOptions) {
			setCapOptions(capOptionsCacheRef.current.get(cacheKey) ?? []);
			setCapOptionsLoading(false);
			setCapOptionsError(null);
			return;
		}
		const providedForKind = capabilityOptionsByKind?.[activeKind];
		if (providedForKind !== undefined || activeKind === kind) {
			const provided = providedForKind ?? (item ? [item] : []);
			capOptionsCacheRef.current.set(cacheKey, provided);
			setCapOptions(provided);
		} else {
			setCapOptions(capOptionsCacheRef.current.get(cacheKey) ?? []);
		}
		setCapOptionsLoading(false);
		setCapOptionsError(null);
	}, [
		activeKind,
		capabilityOptionsByKind,
		hasListedOptions,
		item,
		kind,
		mode,
		open,
		propItemKey,
	]);

	useEffect(() => {
		activeCallIdRef.current = activeCallId;
	}, [activeCallId]);

	useEffect(() => {
		if (!open || !formStateKey) {
			return;
		}
		if (initializedFormKeyRef.current !== formStateKey) {
			return;
		}
		formSnapshotsRef.current.set(formStateKey, {
			argsJson,
			useRaw,
			values,
		});
	}, [argsJson, formStateKey, open, useRaw, values]);

	useEffect(() => {
		if (!open && wsRef.current) {
			wsRef.current.close();
			wsRef.current = null;
		}
	}, [open]);

	// Build mock from JSON Schema types
	function mockOfType(t?: string): JsonValue {
		switch ((t || "string").toLowerCase()) {
			case "integer":
				return 1;
			case "number":
				return 1.0;
			case "boolean":
				return true;
			case "array":
				return ["example"];
			case "object":
				return { key: "value" };
			default:
				return "example";
		}
	}

	function extractToolSchema(raw: CapabilityRecord | null): JsonSchema | null {
		// Support multiple shapes: input_schema.schema, inputSchema.schema, input_schema, inputSchema, schema
		if (!raw) return null;
		const candidates = [raw.input_schema, raw.inputSchema, raw.schema];
		for (const candidate of candidates) {
			const schema = toSchema(candidate);
			if (schema) {
				if (!schema.type && schema.properties) {
					schema.type = "object";
				}
				return schema;
			}
		}
		// Ensure object type when properties exist
		return null;
	}

	function deriveFields(sourceItem: CapabilityRecord | null): Field[] {
		// Tools: item.input_schema?.properties; Prompts: item.arguments (array); Templates: parse {placeholder} from uriTemplate
		try {
			if (activeKind === "tool") {
				const schema = extractToolSchema(sourceItem);
				const props = schema?.properties ?? {};
				let list: Field[] = [];
				if (props && Object.keys(props).length > 0) {
					const required: string[] = Array.isArray(schema?.required)
						? schema.required
						: [];
					list = Object.keys(props).map((k) => {
						const p = props[k] || {};
						const type = Array.isArray(p.type)
							? String(p.type[0])
							: String(p.type || "string");
						const en = Array.isArray(p.enum) ? (p.enum as string[]) : undefined;
						return {
							name: k,
							type,
							required: required.includes(k),
							description: p.description,
							enum: en,
							default: p.default,
						};
					});
				}
				// Fallback to arguments array if schema had no properties
				if (list.length === 0) {
					list = normalizeArguments(sourceItem?.arguments).map((arg) => ({
						name: arg.name ?? "arg",
						type: arg.type ?? "string",
						required: Boolean(arg.required),
						description: arg.description,
						default: arg.default,
					}));
				}
				return list;
			}
			if (activeKind === "prompt") {
				return normalizeArguments(sourceItem?.arguments).map((arg) => ({
					name: arg.name ?? "arg",
					type: arg.type ?? "string",
					required: Boolean(arg.required),
					description: arg.description,
					default: arg.default,
				}));
			}
			if (activeKind === "template") {
				const uriTemplate = pickTemplateName(sourceItem, mode);
				return extractInspectorResourceTemplateParameters(uriTemplate).map(
					(parameter) => ({
						name: parameter,
						type: "string",
						required: false,
						description: `Value for '${parameter}' template variable`,
					}),
				);
			}
			return [];
		} catch {
			return [];
		}
	}

	function fillMock(fs: Field[]): JsonObject {
		const acc: JsonObject = {};
		fs.forEach((f) => {
			if (f.default !== undefined) acc[f.name] = f.default;
			else if (f.enum && f.enum.length) acc[f.name] = f.enum[0];
			else acc[f.name] = mockOfType(f.type);
		});
		return acc;
	}

	useEffect(() => {
		if (!open) {
			return;
		}
		const source = currentItem ?? null;
		const savedState = formStateKey
			? formSnapshotsRef.current.get(formStateKey)
			: undefined;
		initializedFormKeyRef.current = formStateKey;
		const applyArguments = (nextValues: JsonObject) => {
			if (savedState) {
				setValues(savedState.values);
				setArgsJson(savedState.argsJson);
				setUseRaw(savedState.useRaw);
				return;
			}
			setValues(nextValues);
			setArgsJson(JSON.stringify(nextValues, null, 2));
			setUseRaw(false);
		};
		let schema: JsonSchema | null = null;
		if (activeKind === "tool") {
			schema = extractToolSchema(source);
			if (!schema) {
				const args = normalizeArguments(source?.arguments);
				if (args.length > 0) {
					schema = buildSchemaFromArguments(args);
				}
			}
		} else if (activeKind === "prompt") {
			const args = normalizeArguments(source?.arguments);
			if (args.length > 0) {
				schema = buildSchemaFromArguments(args);
			}
		} else if (activeKind === "template") {
			const fs = deriveFields(source);
			if (fs.length > 0) {
				schema = buildSchemaFromFields(fs);
			}
		}

		if (
			schema &&
			schema.type === "object" &&
			schema.properties &&
			Object.keys(schema.properties).length > 0
		) {
			setSchemaObj(schema);
			const mock = toJsonObject(defaultFromSchema(schema));
			applyArguments(mock);
			setFields([]);
		} else {
			const fs = deriveFields(source);
			setFields(fs);
			if (fs.length > 0) {
				const generatedSchema = buildSchemaFromFields(fs);
				setSchemaObj(generatedSchema);
				const mock = toJsonObject(defaultFromSchema(generatedSchema));
				applyArguments(mock);
			} else {
				setSchemaObj(null);
				const mock = fillMock(fs);
				applyArguments(mock);
			}
		}
		// Maintaining manual dependency list because schema helpers are stable
		// within this component lifecycle.
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [activeKind, open, currentItem, formStateKey, mode]);

	useEffect(() => {
		if (!open) {
			return;
		}
		const source = currentItem ?? null;
		if (!source) {
			return;
		}
		if (activeKind === "tool") {
			setName(pickToolNameForMode(source, mode));
		} else if (activeKind === "prompt") {
			setName(pickPromptNameForMode(source, mode));
		} else if (activeKind === "resource") {
			setUri(pickResourceUriForMode(source, mode));
		} else if (activeKind === "template") {
			setName(pickTemplateName(source, mode));
		}
	}, [activeKind, open, currentItem, mode]);

	const changeInspectorMode = useCallback(
		(nextMode: InspectorMode) => {
			if (nextMode === mode) {
				return;
			}
			if (formStateKey) {
				formSnapshotsRef.current.set(formStateKey, {
					argsJson,
					useRaw,
					values,
				});
			}
			operationSnapshotsRef.current.set(`${mode}:${activeKind}`, {
				overrideItem,
				ignorePropItem,
				name,
				uri,
				formCollapsed,
			});
			const counterpartIdentity = resolveInspectorCounterpartIdentity(
				activeKind,
				mode,
				nextMode,
				currentModeIdentity,
				capabilityMappings,
			);
			pendingCounterpartSelectionRef.current = counterpartIdentity
				? {
						kind: activeKind,
						mode: nextMode,
						identity: counterpartIdentity,
					}
				: null;
			const saved = operationSnapshotsRef.current.get(
				`${nextMode}:${activeKind}`,
			);
			setOverrideItem(counterpartIdentity ? null : (saved?.overrideItem ?? null));
			setIgnorePropItem(
				counterpartIdentity
					? true
					: (saved?.ignorePropItem ?? activeKind !== kind),
			);
			setName(counterpartIdentity ? "" : (saved?.name ?? ""));
			setUri(counterpartIdentity ? "" : (saved?.uri ?? ""));
			setFormCollapsed(saved?.formCollapsed ?? false);
			setResult(null);
			setEvents([]);
			setView("response");
			setMode(nextMode);
		},
		[
			activeKind,
			argsJson,
			capabilityMappings,
			currentModeIdentity,
			formCollapsed,
			formStateKey,
			ignorePropItem,
			kind,
			mode,
			name,
			overrideItem,
			uri,
			useRaw,
			values,
		],
	);

	function parseArgs(): JsonObject | undefined {
		try {
			const obj = JSON.parse(argsJson || "{}");
			if (obj && typeof obj === "object" && !Array.isArray(obj)) {
				return obj as JsonObject;
			}
			return undefined;
		} catch {
			notifyError(
				t("notifications.invalidArgs"),
				t("notifications.invalidArgsMessage"),
			);
			return undefined;
		}
	}

	// Try to extract text from common MCP/LLM response envelopes
	function extractHumanText(value: unknown): string | null {
		if (value && typeof value === "object" && !Array.isArray(value)) {
			const rec = value as Record<string, unknown>;
			if (rec.type === "text" && typeof rec.text === "string") {
				return rec.text as string;
			}
			if (Array.isArray((rec as any).content)) {
				const segments = ((rec as any).content as unknown[]).map((seg) => {
					if (typeof seg === "string") return seg;
					if (
						seg &&
						typeof seg === "object" &&
						!Array.isArray(seg) &&
						((seg as any).type === "text" ||
							(seg as any).type === "input_text") &&
						typeof (seg as any).text === "string"
					) {
						return String((seg as any).text);
					}
					return null;
				});
				const texts = segments.filter((s): s is string => Boolean(s));
				if (texts.length) return texts.join("\n\n");
			}
		}
		return null;
	}

	const optionsMap = useMemo(() => {
		const map = new Map<string, CapabilityRecord>();
		capOptions.forEach((entry, index) => {
			const key =
				getInspectorModeIdentity(activeKind, mode, entry) || `index:${index}`;
			map.set(key, entry);
		});
		return map;
	}, [activeKind, capOptions, mode]);

	useEffect(() => {
		const pending = pendingCounterpartSelectionRef.current;
		if (
			!pending ||
			pending.kind !== activeKind ||
			pending.mode !== mode ||
			!hasListedOptions
		) {
			return;
		}

		pendingCounterpartSelectionRef.current = null;
		const targetOptions =
			capOptionsCacheRef.current.get(`${mode}:${activeKind}`) ?? [];
		const match = targetOptions.find(
			(option) =>
				getInspectorModeIdentity(activeKind, mode, option) === pending.identity,
		);
		if (!match) {
			return;
		}

		setOverrideItem(match);
		setIgnorePropItem(true);
		if (activeKind === "tool") {
			setName(pickToolNameForMode(match, mode));
		} else if (activeKind === "prompt") {
			setName(pickPromptNameForMode(match, mode));
		} else if (activeKind === "resource") {
			setUri(pickResourceUriForMode(match, mode));
		} else {
			setName(pickTemplateName(match, mode));
		}
	}, [activeKind, hasListedOptions, mode]);

	const refreshCapabilityOptions = useCallback(async (forceRefresh = true) => {
		const requestedKind = activeKind;
		const requestedMode = mode;
		const requestedListKey = optionListKey;
		const requestedCacheKey = `${requestedMode}:${requestedKind}`;
		if (pendingOptionListKeysRef.current.has(requestedListKey)) {
			return;
		}
		if (!serverId && !serverName) {
			setCapOptionsError(t("errors.sessionMissing"));
			return;
		}
		if (mode === "proxy" && !proxyAvailable) {
			setCapOptionsError(
				t("proxy.unavailable", {
					defaultValue:
						"Proxy mode is unavailable because this server is not enabled in any active profile.",
				}),
			);
			return;
		}

		pendingOptionListKeysRef.current.add(requestedListKey);
		if (activeOptionListKeyRef.current === requestedListKey) {
			setCapOptionsLoading(true);
			setCapOptionsError(null);
		}
		try {
			const sessionId =
				requestedMode === "native" ? await ensureNativeSession() : undefined;
			if (requestedMode === "native" && !sessionId) {
				throw new Error(t("errors.sessionMissing"));
			}
			const commonPayload = {
				server_id: serverId,
				server_name: serverName,
				mode: requestedMode,
				session_id: sessionId,
				refresh: forceRefresh,
				timeout_ms: timeoutMs,
			};
			let resp: InspectorResponse<any> | undefined;
			if (requestedKind === "tool") {
				resp = (await inspectorApi.toolsList(commonPayload)) as
					| InspectorResponse<{ tools?: unknown[] }>
					| undefined;
			} else if (requestedKind === "prompt") {
				resp = (await inspectorApi.promptsList(commonPayload)) as
					| InspectorResponse<{ prompts?: unknown[] }>
					| undefined;
			} else if (requestedKind === "resource") {
				resp = (await inspectorApi.resourcesList(commonPayload)) as
					| InspectorResponse<{ resources?: unknown[] }>
					| undefined;
			} else {
				resp = (await inspectorApi.templatesList(commonPayload)) as
					| InspectorResponse<{ templates?: unknown[] }>
					| undefined;
			}
			if (!resp?.success) {
				throw new Error(resp?.error ? String(resp.error) : "Inspector list failed");
			}
			const options = normalizeCapabilityOptions(
				resp,
				requestedKind,
				requestedMode,
			);
			capOptionsCacheRef.current.set(requestedCacheKey, options);
			if (activeOptionListKeyRef.current === requestedListKey) {
				setCapOptions(options);
			}
			setListedOptionKeys((current) => {
				const next = new Set(current);
				next.add(requestedListKey);
				return next;
			});
		} catch (error) {
			if (
				requestedMode === "native" &&
				isInspectorSessionUnavailableError(error)
			) {
				invalidateNativeSession();
			}
			if (activeOptionListKeyRef.current === requestedListKey) {
				setCapOptionsError(
					error instanceof Error ? error.message : String(error ?? ""),
				);
			}
		} finally {
			pendingOptionListKeysRef.current.delete(requestedListKey);
			if (activeOptionListKeyRef.current === requestedListKey) {
				setCapOptionsLoading(false);
			}
		}
	}, [
		ensureNativeSession,
		invalidateNativeSession,
		activeKind,
		mode,
		optionListKey,
		proxyAvailable,
		serverId,
		serverName,
		t,
		timeoutMs,
	]);

	useEffect(() => {
		const hasAttemptedAutoLoad =
			autoAttemptedOptionKeysRef.current.has(optionListKey);
		if (
			!shouldAutoLoadInspectorOptions({
				canUseCurrentMode,
				hasAttemptedAutoLoad,
				hasListedOptions,
				isDrawerInitialized: drawerInitialized,
				isProxyChecking,
				open,
			})
		) {
			return;
		}
		autoAttemptedOptionKeysRef.current.add(optionListKey);
		void refreshCapabilityOptions(false);
	}, [
		canUseCurrentMode,
		drawerInitialized,
		hasListedOptions,
		isProxyChecking,
		open,
		optionListKey,
		refreshCapabilityOptions,
	]);

	const handleCapabilitySelect = useCallback(
		(value: string) => {
			pendingCounterpartSelectionRef.current = null;
			setResult(null);
			setEvents([]);
			setView("response");
			setActiveCallId(null);
			activeCallIdRef.current = null;
			setUseRaw(false);
			const match = optionsMap.get(value);
			if (match) {
				setOverrideItem(match);
				setIgnorePropItem(true);
				if (activeKind === "tool") setName(pickToolNameForMode(match, mode));
				else if (activeKind === "prompt") {
					setName(pickPromptNameForMode(match, mode));
				} else if (activeKind === "resource") {
					setUri(pickResourceUriForMode(match, mode));
				} else if (activeKind === "template") {
					setName(pickTemplateName(match, mode));
				}
			} else {
				setOverrideItem(null);
				setIgnorePropItem(true);
				if (activeKind === "resource") setUri(value.trim());
				else setName(value);
			}
		},
		[activeKind, mode, optionsMap],
	);

	const switchInspectorOperation = useCallback(
		(nextKind: InspectorKind) => {
			if (nextKind === activeKind || submitting) {
				return;
			}
			pendingCounterpartSelectionRef.current = null;
			if (formStateKey) {
				formSnapshotsRef.current.set(formStateKey, {
					argsJson,
					useRaw,
					values,
				});
			}
			const saved = switchInspectorOperationSnapshot(
				operationSnapshotsRef.current,
				mode,
				activeKind,
				nextKind,
				{
					overrideItem,
					ignorePropItem,
					name,
					uri,
					formCollapsed,
				},
			);
			setActiveKind(nextKind);
			setOverrideItem(saved?.overrideItem ?? null);
			setIgnorePropItem(saved?.ignorePropItem ?? nextKind !== kind);
			setName(saved?.name ?? "");
			setUri(saved?.uri ?? "");
			setResult(null);
			setEvents([]);
			setView("response");
			setFormCollapsed(saved?.formCollapsed ?? false);
			setCapOptions(
				capOptionsCacheRef.current.get(`${mode}:${nextKind}`) ?? [],
			);
			setCapOptionsError(null);
			setOperationMenuOpen(false);
		},
		[
			activeKind,
			argsJson,
			formCollapsed,
			formStateKey,
			ignorePropItem,
			kind,
			mode,
			name,
			overrideItem,
			submitting,
			uri,
			useRaw,
			values,
		],
	);

	const handleCancel = useCallback(async () => {
		if (!activeCallId) {
			return;
		}
		try {
			setCancelling(true);
			setView("events");
			await inspectorApi.toolCallCancel({
				call_id: activeCallId,
				reason: "cancelled_by_user",
			});
		} catch (error) {
			notifyError(
				"Cancel failed",
				error instanceof Error ? error.message : String(error ?? ""),
			);
		} finally {
			setCancelling(false);
		}
	}, [activeCallId]);

	const clearOutput = useCallback(() => {
		setResult(null);
		setEvents([]);
		setView("response");
	}, []);

	const clearResponseActionsHideTimer = useCallback(() => {
		if (responseActionsHideTimer.current != null) {
			clearTimeout(responseActionsHideTimer.current);
			responseActionsHideTimer.current = null;
		}
	}, []);

	const scheduleResponseActionsHide = useCallback(() => {
		clearResponseActionsHideTimer();
		responseActionsHideTimer.current = setTimeout(() => {
			setResponseActionsHidden(true);
			responseActionsHideTimer.current = null;
		}, 450);
	}, [clearResponseActionsHideTimer]);

	const resetResponseActions = useCallback(() => {
		clearResponseActionsHideTimer();
		setResponseActionsHidden(false);
	}, [clearResponseActionsHideTimer]);

	useEffect(() => {
		clearResponseActionsHideTimer();
		setResponseActionsHidden(false);
	}, [clearResponseActionsHideTimer, result]);

	useEffect(() => () => clearResponseActionsHideTimer(), [
		clearResponseActionsHideTimer,
	]);

	const hasSchemaInputs =
		schemaObj &&
		schemaObj.properties &&
		Object.keys(schemaObj.properties).length > 0;
	const hasFieldInputs = fields.length > 0;
	const expectsArguments =
		activeKind !== "resource" && (hasSchemaInputs || hasFieldInputs);
	const sessionExpiry = (() => {
		const ms = Number(nativeSession?.expires_at_epoch_ms ?? NaN);
		if (!Number.isFinite(ms)) return null;
		return new Date(ms).toLocaleTimeString([], {
			hour: "2-digit",
			minute: "2-digit",
		});
	})();

	const handleInspectorEvent = useCallback(
		(payload: InspectorSseEvent) => {
			if (
				activeCallIdRef.current &&
				payload.call_id !== activeCallIdRef.current
			) {
				return;
			}

			setEvents((prev) => {
				const next = [...prev, { data: payload, timestamp: Date.now() }];
				return next.length > 200 ? next.slice(next.length - 200) : next;
			});

			switch (payload.event) {
				case "started":
					onLog?.({
						id: newLogId(),
						timestamp: Date.now(),
						channel: "inspector",
						event: "request",
						method: "tools/call",
						mode,
						payload,
					});
					break;
				case "progress":
					setView("events");
					onLog?.({
						id: newLogId(),
						timestamp: Date.now(),
						channel: "inspector",
						event: "progress",
						method: "tools/call",
						mode,
						message: payload.message ?? undefined,
						payload,
					});
					break;
				case "log":
					setView("events");
					onLog?.({
						id: newLogId(),
						timestamp: Date.now(),
						channel: "inspector",
						event: "log",
						method: "tools/call",
						mode,
						message: payload.logger ?? payload.level ?? undefined,
						payload: payload.data,
					});
					break;
				case "result":
					setSubmitting(false);
					setResult(payload.result);
					setCancelling(false);
					setView("response");
					onLog?.({
						id: newLogId(),
						timestamp: Date.now(),
						channel: "inspector",
						event: "success",
						method: "tools/call",
						mode,
						payload,
					});
					notifySuccess(
						t("notifications.executed"),
						t("notifications.executedMessage"),
					);
					if (wsRef.current) {
						wsRef.current.close();
						wsRef.current = null;
					}
					setActiveCallId(null);
					activeCallIdRef.current = null;
					break;
				case "error":
					setSubmitting(false);
					setCancelling(false);
					setView("events");
					onLog?.({
						id: newLogId(),
						timestamp: Date.now(),
						channel: "inspector",
						event: "error",
						method: "tools/call",
						mode,
						message: payload.message,
						payload,
					});
					notifyError(t("notifications.failed"), payload.message);
					if (wsRef.current) {
						wsRef.current.close();
						wsRef.current = null;
					}
					setActiveCallId(null);
					activeCallIdRef.current = null;
					break;
				case "cancelled":
					setSubmitting(false);
					setCancelling(false);
					setView("events");
					onLog?.({
						id: newLogId(),
						timestamp: Date.now(),
						channel: "inspector",
						event: "cancelled",
						method: "tools/call",
						mode,
						message: payload.reason ?? undefined,
						payload,
					});
					notifyError(
						t("notifications.cancelled"),
						payload.reason ?? t("notifications.cancelledMessage"),
					);
					if (wsRef.current) {
						wsRef.current.close();
						wsRef.current = null;
					}
					setActiveCallId(null);
					activeCallIdRef.current = null;
					break;
			}
		},
		[mode, onLog, t],
	);

	const subscribeToCall = useCallback(
		(callId: string) => {
			// Close existing connections
			if (wsRef.current) {
				wsRef.current.close();
				wsRef.current = null;
			}

			try {
				const url = inspectorApi.toolCallEventsWsUrl(callId);
				const ws = new WebSocket(url);

				ws.onopen = () => {
					console.log("WebSocket connected for inspector events");
				};

				ws.onmessage = (event) => {
					try {
						const data: InspectorSseEvent = JSON.parse(event.data);
						handleInspectorEvent(data);
					} catch (error) {
						console.warn("Failed to parse inspector event", error);
					}
				};

				ws.onerror = (error) => {
					console.warn("WebSocket error for inspector events:", error);
				};

				ws.onclose = (event) => {
					console.log(`WebSocket closed: ${event.code} ${event.reason}`);
					wsRef.current = null;
					setSubmitting(false);
				};

				wsRef.current = ws;
			} catch (error) {
				console.warn("Failed to subscribe to inspector events", error);
				setSubmitting(false);
			}
		},
		[handleInspectorEvent],
	);

	const executeResourceRead = useCallback(
		async (targetUri: string) => {
			try {
				setSubmitting(true);
				setResult(null);
				if (mode === "proxy" && !proxyAvailable) {
					throw new Error(
						t("proxy.unavailable", {
							defaultValue:
								"Proxy mode is unavailable because this server is not enabled in any active profile.",
						}),
					);
				}
				const effectiveSessionId =
					mode === "native" ? await ensureNativeSession() : undefined;
				if (mode === "native" && !effectiveSessionId) {
					throw new Error(t("errors.sessionMissing"));
				}
				const baseLog = {
					id: newLogId(),
					timestamp: Date.now(),
					channel: "inspector" as const,
					mode,
				};
				onLog?.({
					...baseLog,
					event: "request",
					method: "resources/read",
					payload: {
						uri: targetUri,
						server_id: serverId,
						server_name: serverName,
						session_id: effectiveSessionId,
					},
				});
				const response = (await inspectorApi.resourceRead({
					uri: targetUri,
					server_id: serverId,
					server_name: serverName,
					session_id: effectiveSessionId,
					mode,
					timeout_ms: timeoutMs,
				})) as InspectorResponse<Record<string, unknown>>;
				if (!response?.success) {
					throw new Error(
						response?.error ? String(response.error) : "Resource read failed",
					);
				}
				const data = (response.data ?? {}) as Record<string, unknown>;
				setResult((data.result as unknown) ?? data);
				onLog?.({
					...baseLog,
					event: "success",
					method: "resources/read",
					payload: data,
				});
				notifySuccess(
					t("notifications.executed"),
					t("notifications.executedMessage"),
				);
			} catch (error) {
				if (mode === "native" && isInspectorSessionUnavailableError(error)) {
					invalidateNativeSession();
				}
				onLog?.({
					id: newLogId(),
					timestamp: Date.now(),
					channel: "inspector",
					event: "error",
					method: "resources/read",
					mode,
					message: error instanceof Error ? error.message : String(error),
					payload: error,
				});
				notifyError("Inspector request failed", String(error));
			} finally {
				setSubmitting(false);
			}
		},
		[
			ensureNativeSession,
			invalidateNativeSession,
			mode,
			onLog,
			proxyAvailable,
			serverId,
			serverName,
			t,
			timeoutMs,
		],
	);

	async function onSubmit() {
		if (activeKind === "resource") {
			await executeResourceRead(uri);
			return;
		}
		try {
			setSubmitting(true);
			setResult(null);
			if (activeKind === "tool") {
				setEvents([]);
				setCancelling(false);
			}
			let resp: InspectorResponse<Record<string, unknown>> | null = null;
			const baseLog = {
				id: newLogId(),
				timestamp: Date.now(),
				channel: "inspector" as const,
				mode,
			};
			if (mode === "proxy" && !proxyAvailable) {
				throw new Error(
					t("proxy.unavailable", {
						defaultValue:
							"Proxy mode is unavailable because this server is not enabled in any active profile.",
					}),
				);
			}
			const effectiveSessionId =
				mode === "native" ? await ensureNativeSession() : undefined;
			if (mode === "native" && !effectiveSessionId) {
				throw new Error(t("errors.sessionMissing"));
			}
			if (activeKind === "tool") {
				const args = expectsArguments
					? useRaw
						? parseArgs()
						: values
					: undefined;
				if (expectsArguments && args === undefined) {
					setSubmitting(false);
					return;
				}

				const effectiveServerId = serverId;
				if (!effectiveServerId && !serverName) {
					throw new Error(t("errors.sessionMissing"));
				}

				onLog?.({
					...baseLog,
					event: "request",
					method: "tools/call",
					payload: {
						tool: name,
						server_id: effectiveServerId,
						server_name: serverName,
						arguments: args,
						timeout_ms: timeoutMs,
						session_id: effectiveSessionId,
					},
				});

				const response = await inspectorApi.toolCallStart({
					tool: name,
					server_id: effectiveServerId,
					server_name: serverName,
					mode,
					arguments: args,
					timeout_ms: timeoutMs,
					session_id: effectiveSessionId,
				});

				const data = response?.data ?? null;
				if (!response?.success || !data) {
					throw new Error(
						response?.error ? String(response.error) : "Tool call failed",
					);
				}

				setActiveCallId(data.call_id);
				activeCallIdRef.current = data.call_id;
				subscribeToCall(data.call_id);
				return;
			} else if (activeKind === "prompt") {
				const args = expectsArguments
					? useRaw
						? parseArgs()
						: values
					: undefined;
				if (expectsArguments && args === undefined) return;
				onLog?.({
					...baseLog,
					event: "request",
					method: "prompts/get",
					payload: {
						name,
						server_id: serverId,
						server_name: serverName,
						arguments: args,
						session_id: effectiveSessionId,
					},
				});
				resp = (await inspectorApi.promptGet({
					name,
					server_id: serverId,
					server_name: serverName,
					mode,
					arguments: args,
					session_id: effectiveSessionId,
					timeout_ms: timeoutMs,
				})) as InspectorResponse<Record<string, unknown>>;
				if (!resp?.success) {
					throw new Error(
						resp?.error ? String(resp.error) : "Prompt get failed",
					);
				}
				const data = (resp.data ?? {}) as Record<string, unknown>;
				setResult((data.result as unknown) ?? data);
				onLog?.({
					...baseLog,
					event: "success",
					method: "prompts/get",
					payload: data,
				});
				notifySuccess(
					t("notifications.executed"),
					t("notifications.executedMessage"),
				);
			} else if (activeKind === "template") {
				const args = expectsArguments
					? useRaw
						? parseArgs()
						: values
					: undefined;
				if (expectsArguments && args === undefined) return;

				onLog?.({
					...baseLog,
					event: "request",
					method: "resources/read",
					payload: {
						uri_template: name,
						arguments: args,
						server_id: serverId,
						server_name: serverName,
						session_id: effectiveSessionId,
					},
				});
				resp = (await inspectorApi.templateRead({
					uri_template: name,
					arguments: args,
					server_id: serverId,
					server_name: serverName,
					session_id: effectiveSessionId,
					mode,
					timeout_ms: timeoutMs,
				})) as InspectorResponse<Record<string, unknown>>;
				if (!resp?.success) {
					throw new Error(
						resp?.error ? String(resp.error) : "Template read failed",
					);
				}
				const data = (resp.data ?? {}) as Record<string, unknown>;
				const expandedUri = data.expanded_uri;
				if (typeof expandedUri !== "string") {
					throw new Error("Template read response did not include expanded_uri");
				}
				setResult(data.result);
				onLog?.({
					...baseLog,
					event: "success",
					method: "resources/read",
					payload: { ...data, uri: expandedUri },
				});
				notifySuccess(
					t("notifications.executed"),
					t("notifications.executedMessage"),
				);
			}
		} catch (e) {
			if (mode === "native" && isInspectorSessionUnavailableError(e)) {
				invalidateNativeSession();
			}
			onLog?.({
				id: newLogId(),
				timestamp: Date.now(),
				channel: "inspector",
				event: "error",
				method:
					activeKind === "tool"
						? "tools/call"
						: activeKind === "prompt"
							? "prompts/get"
							: activeKind === "template"
								? "resources/read"
								: "resources/read",
				mode,
				message: e instanceof Error ? e.message : String(e),
				payload: e,
			});
			notifyError("Inspector request failed", String(e));
			setSubmitting(false);
		} finally {
			if (activeKind !== "tool") {
				setSubmitting(false);
			}
		}
	}

	function pretty(value: unknown) {
		return smartFormat(value);
	}

	const handleCopy = useCallback(async () => {
		if (result == null) return;
		try {
			const extracted = extractHumanText(result);
			const text = extracted ?? pretty(result);
			await writeClipboardText(text);
			notifySuccess(
				t("notifications.copySuccess"),
				t("notifications.copySuccessMessage"),
			);
			scheduleResponseActionsHide();
		} catch (err) {
			notifyError(
				t("notifications.copyFailed"),
				err instanceof Error ? err.message : String(err),
			);
		}
	}, [result, scheduleResponseActionsHide, t]);

	const displaySessionId =
		mode === "native" ? nativeSession?.session_id : undefined;
	const sessionActive = Boolean(displaySessionId);
	const sessionIndicator =
		activeKind === "tool" && mode === "native" ? (
			<TooltipProvider delayDuration={150}>
				<Tooltip>
					<TooltipTrigger asChild>
						<button
							type="button"
							className="inline-flex h-8 w-8 items-center justify-center text-slate-500 transition hover:text-slate-700 dark:text-slate-300 dark:hover:text-slate-100"
							aria-label={
								sessionActive ? t("session.active") : t("session.pending")
							}
						>
							{sessionActive ? (
								<CheckCircle2 className="h-5 w-5 text-emerald-500" />
							) : (
								<AlertCircle className="h-5 w-5 text-amber-500" />
							)}
						</button>
					</TooltipTrigger>
					<TooltipContent
						side="left"
						align="end"
						className="max-w-xs text-xs leading-relaxed"
					>
						{sessionActive ? (
							<p>
								{t("session.connected", {
									serverName: serverName || serverId,
									expiry: sessionExpiry ?? "soon",
								})}
							</p>
						) : (
							<p>{t("session.notConnected")}</p>
						)}
					</TooltipContent>
				</Tooltip>
			</TooltipProvider>
		) : null;

	const handleToggleFormClick = useCallback(
		(event: React.MouseEvent<HTMLDivElement>) => {
			const target = event.target as HTMLElement;
			// Ignore interactive controls so output clicks are the only input toggle target.
			if (
				target.closest("button") ||
				target.closest("a") ||
				target.closest('[data-prevent-collapse="true"]') ||
				target.closest("[data-radix-popper-content-wrapper]")
			) {
				return;
			}
			setFormCollapsed((collapsed) => !collapsed);
		},
		[],
	);

	const listButton = (
		<Button
			type="button"
			variant="default"
			onClick={() => void refreshCapabilityOptions()}
			disabled={
				capOptionsLoading || submitting || isProxyChecking || !canUseCurrentMode
			}
			className="h-9 shrink-0 gap-2 px-3"
		>
			{capOptionsLoading || isProxyChecking ? (
				<Loader2 className="h-4 w-4 animate-spin" />
			) : (
				<RefreshCw className="h-4 w-4" />
			)}
			{hasProvidedOptions || hasListedOptions
				? t("actions.refresh", { defaultValue: "Refresh" })
				: t("actions.list", { defaultValue: "List" })}
		</Button>
	);
	const proxyUnavailableText = t("proxy.unavailable", {
		defaultValue:
			"Proxy mode is unavailable because this server is not enabled in any active profile.",
	});

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent
				ref={drawerContentRef}
				className="flex h-full flex-col overflow-hidden"
			>
				<DrawerHeader className="shrink-0">
					<div className="flex items-start justify-between gap-3">
						<div>
							<DrawerTitle className="flex items-center gap-1">
								<span>{t("title")} ·</span>
								<Popover
									open={operationMenuOpen}
									onOpenChange={setOperationMenuOpen}
								>
									<PopoverTrigger asChild>
										<button
											type="button"
											disabled={submitting}
											className="inline-flex items-center gap-1 rounded-sm text-left transition hover:text-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60 dark:hover:text-slate-300"
											title={t("actions.changeOperation")}
										>
											<span>{t(getInspectorOperationLabelKey(activeKind))}</span>
											<ChevronDown className="h-4 w-4" aria-hidden="true" />
										</button>
									</PopoverTrigger>
									<PopoverContent
										align="start"
										className="w-56 p-1"
										container={drawerContentRef.current}
									>
										{INSPECTOR_OPERATIONS.map((operation) => (
											<Button
												key={operation}
												type="button"
												variant="ghost"
												className="w-full justify-start"
												disabled={operation === activeKind}
												onClick={() => switchInspectorOperation(operation)}
											>
												{t(getInspectorOperationLabelKey(operation))}
											</Button>
										))}
									</PopoverContent>
								</Popover>
							</DrawerTitle>
							<DrawerDescription>{t("subtitle")}</DrawerDescription>
						</div>
						{sessionIndicator}
					</div>
				</DrawerHeader>

				<div className="flex min-h-0 flex-1 flex-col space-y-3 overflow-y-auto px-4 py-3">
					<div
						className={`transition-all duration-300 ease-in-out ${formCollapsed ? "max-h-12 overflow-hidden" : "max-h-[800px]"
							}`}
					>
						{formCollapsed ? (
							<div
								role="button"
								onClick={() => setFormCollapsed(false)}
								onKeyDown={(event) => {
									if (event.key === "Enter" || event.key === " ") {
										event.preventDefault();
										setFormCollapsed(false);
									}
								}}
								tabIndex={0}
								className="flex h-10 cursor-pointer items-center justify-between rounded-md border border-dashed border-slate-200 px-3 text-sm text-slate-600 transition hover:border-slate-300 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-300 dark:hover:border-slate-600 dark:hover:bg-slate-900/40"
							>
								<span>
									{t("form.parametersCollapsedHint", {
										defaultValue: "click to expand tool input",
									})}
								</span>
								<ChevronsUpDown
									className="h-4 w-4 opacity-70"
									aria-hidden="true"
								/>
							</div>
						) : (
							<div className="space-y-3">
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
									<div className="space-y-1">
										<Label>{t("form.mode")}</Label>
										<ButtonGroup className="flex w-full">
											<Button
												type="button"
												variant={mode === "native" ? "default" : "outline"}
												className="h-9 flex-1 rounded-r-none gap-2 px-3"
												onClick={() => changeInspectorMode("native")}
											>
												<AlertTriangle className="h-4 w-4" />
												{t("form.native", { defaultValue: "Native" })}
											</Button>
											<TooltipProvider delayDuration={200}>
												<Tooltip>
													<TooltipTrigger asChild>
														<Button
															type="button"
															variant={mode === "proxy" ? "default" : "outline"}
															className="h-9 flex-1 rounded-l-none gap-2 px-3"
															onClick={() => changeInspectorMode("proxy")}
														>
															{isProxyChecking ? (
																<Loader2 className="h-4 w-4 animate-spin" />
															) : (
																<ShieldAlert
																	className={`h-4 w-4 ${proxyUnavailable ? "text-amber-300" : ""}`}
																/>
															)}
															<span
																className={
																	proxyUnavailable ? "text-amber-300" : undefined
																}
															>
																{t("form.proxy", { defaultValue: "Proxy" })}
															</span>
														</Button>
													</TooltipTrigger>
													{proxyUnavailable ? (
														<TooltipPortal>
															<TooltipContent side="top" align="start">
																<p className="max-w-xs text-xs leading-relaxed">
																	{proxyUnavailableText}
																</p>
																<TooltipArrow />
															</TooltipContent>
														</TooltipPortal>
													) : null}
												</Tooltip>
											</TooltipProvider>
										</ButtonGroup>
									</div>
									<div className="space-y-1">
										<Label>{t("form.timeout")}</Label>
										<Input
											type="number"
											min={1000}
											step={1000}
											className="h-9"
											value={timeoutMs}
											onChange={(e) =>
												setTimeoutMs(parseInt(e.target.value, 10) || 8000)
											}
										/>
									</div>
									<div className="space-y-1">
										<Label>{t("form.server")}</Label>
										<TooltipProvider delayDuration={200}>
											<Tooltip>
												<TooltipTrigger asChild>
													<Input
														value={serverName || serverId || "-"}
														disabled
														className="h-9"
													/>
												</TooltipTrigger>
												{serverName && serverId ? (
													<TooltipPortal>
														<TooltipContent side="top" align="start">
															<p className="text-xs">ID: {serverId}</p>
															<TooltipArrow />
														</TooltipContent>
													</TooltipPortal>
												) : null}
											</Tooltip>
										</TooltipProvider>
									</div>
								</div>

								{activeKind === "resource" || activeKind === "template" ? (
									<div className="space-y-1">
										<Label>
											{activeKind === "resource"
												? t("form.resourceUri")
												: t("form.template")}
										</Label>
										<div className="flex min-w-0 gap-2">
											<div className="min-w-0 flex-1">
												<CapabilityCombobox
													kind={activeKind}
													items={capOptions}
													value={
														currentItem
															? currentModeIdentity || undefined
															: activeKind === "resource"
																? uri || undefined
																: undefined
													}
													onChange={(key) => handleCapabilitySelect(key)}
													loading={capOptionsLoading}
													error={capOptionsError}
													container={drawerContentRef.current}
													triggerClassName="h-9"
													allowCustomValue={activeKind === "resource"}
													getCustomValueLabel={(value) =>
														t("form.useResourceUri", { uri: value })
													}
													placeholder={
														activeKind === "resource"
															? (t("form.selectResource", {
																defaultValue: "Select resource",
															}) as string)
															: (t("form.selectTemplate", {
																defaultValue: "Select template",
															}) as string)
													}
													getKey={(it) =>
														getInspectorModeIdentity(activeKind, mode, it as CapabilityRecord)
													}
													getLabel={(it) => {
														const entry = it as CapabilityRecord;
														if (activeKind === "template") {
															return (pickTemplateName(entry, mode) ||
																computeRecordKey(entry, activeKind)) as string;
														}
														return (pickResourceUriForMode(entry, mode) ||
															computeRecordKey(entry, activeKind)) as string;
													}}
													getDescription={(it) => {
														const entry = it as CapabilityRecord;
														return (
															toStringValue((entry as any).description) ||
															undefined
														);
													}}
												/>
											</div>
											{listButton}
										</div>
									</div>
								) : (
									<div className="space-y-2">
										<div className="space-y-1">
											<Label>
												{activeKind === "tool" ? t("form.tool") : t("form.prompt")}
											</Label>
											<div className="flex min-w-0 gap-2">
												<div className="min-w-0 flex-1">
													<CapabilityCombobox
														kind={activeKind}
														items={capOptions}
														value={currentModeIdentity || undefined}
														onChange={(key) => handleCapabilitySelect(key)}
														loading={capOptionsLoading}
														error={capOptionsError}
														container={drawerContentRef.current}
														triggerClassName="h-9"
														placeholder={
															activeKind === "tool"
																? (t("form.selectTool", {
																	defaultValue: "Select tool",
																}) as string)
																: (t("form.selectPrompt", {
																	defaultValue: "Select prompt",
																}) as string)
														}
														getKey={(it) =>
															getInspectorModeIdentity(activeKind, mode, it as CapabilityRecord)
														}
														getLabel={(it) => {
															const entry = it as CapabilityRecord;
															if (activeKind === "tool") {
																return (
																	pickToolNameForMode(entry, mode) ||
																	computeRecordKey(entry, activeKind)
																);
															}
															return (
																pickPromptNameForMode(entry, mode) ||
																computeRecordKey(entry, activeKind)
															);
														}}
														getDescription={(it) => {
															const entry = it as CapabilityRecord;
															return (
																toStringValue(entry.description) ||
																undefined
															);
														}}
													/>
												</div>
												{listButton}
											</div>
										</div>
									</div>
								)}

								{activeKind !== "resource" ? (
									expectsArguments ? (
										<div className="space-y-3">
											<div className="flex items-end justify-between">
												<Label className="pb-1">{t("form.parameters")}</Label>
												<ButtonGroup className="overflow-hidden rounded-md border border-input text-xs divide-x divide-input">
													<Button
														size="sm"
														variant="ghost"
														className="h-auto px-2 py-1 text-xs font-medium"
														onClick={() => {
															if (schemaObj) {
																const mock = toJsonObject(
																	defaultFromSchema(schemaObj),
																);
																setValues(mock);
																setArgsJson(JSON.stringify(mock, null, 2));
															} else {
																const mock = fillMock(fields);
																setValues(mock);
																setArgsJson(JSON.stringify(mock, null, 2));
															}
														}}
													>
														{t("actions.fillMock")}
													</Button>
													<Button
														size="sm"
														variant="ghost"
														className="h-auto px-2 py-1 text-xs font-medium"
														onClick={() => {
															if (formStateKey) {
																formSnapshotsRef.current.delete(formStateKey);
															}
															setValues({});
															setArgsJson("{}");
														}}
													>
														{t("actions.clean")}
													</Button>
													<Button
														size="sm"
														variant={useRaw ? "default" : "ghost"}
														className="h-auto px-2 py-1 text-xs font-medium"
														onClick={() => setUseRaw((v) => !v)}
													>
														{useRaw ? t("actions.form") : t("actions.json")}
													</Button>
												</ButtonGroup>
											</div>
											{useRaw ? (
												<CardListScrollBody className="max-h-[230px] flex-none">
													<div className="p-3">
														<Textarea
															rows={rawArgumentRows}
															className="min-h-0 resize-none border-0 bg-transparent p-0 font-mono text-xs shadow-none focus-visible:ring-0 focus-visible:ring-offset-0"
															value={argsJson}
															onChange={(e) => setArgsJson(e.target.value)}
														/>
													</div>
												</CardListScrollBody>
											) : schemaObj ? (
												<CardListScrollBody className="max-h-[230px] flex-none">
													<div className="p-3">
														<SchemaForm
															schema={schemaObj}
															value={values}
															compact
															onChange={(v) => {
																const next = toJsonObject(v);
																setValues(next);
																setArgsJson(JSON.stringify(next, null, 2));
															}}
														/>
													</div>
												</CardListScrollBody>
											) : (
												<CardListScrollBody className="max-h-[260px] flex-none">
													<div className="p-3">
														<Textarea
															rows={5}
															className="min-h-[220px] resize-none font-mono text-xs"
															value={argsJson}
															onChange={(e) => setArgsJson(e.target.value)}
														/>
													</div>
												</CardListScrollBody>
											)}
										</div>
									) : (
										<div className="rounded-md border border-dashed border-slate-200 bg-slate-50 px-2 py-1.5 text-xs leading-relaxed text-slate-500 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
											{t("errors.noArguments")}
										</div>
									)
								) : null}
							</div>
						)}
					</div>

					<Tabs
						value={view}
						onValueChange={(val) => setView(val as "response" | "events")}
						className="flex min-h-[220px] flex-1 flex-col space-y-3"
					>
						<TabsList className="grid w-full grid-cols-2 text-sm">
							<TabsTrigger value="response">{t("tabs.response")}</TabsTrigger>
							<TabsTrigger value="events">{t("tabs.events")}</TabsTrigger>
						</TabsList>
						<TabsContent
							value="response"
							className="min-h-0 flex-1 flex-col data-[state=active]:flex"
							onClick={handleToggleFormClick}
						>
							<CardListScrollBody>
								<div
									className="group relative min-h-full text-xs text-slate-700 dark:text-slate-200"
									onMouseLeave={resetResponseActions}
								>
									{result ? (
										<div className="pointer-events-none absolute top-0 right-0 z-10 flex w-full justify-end p-2">
											<ButtonGroup
												className={`pointer-events-auto bg-white/95 opacity-0 backdrop-blur-sm shadow-sm transition-opacity dark:bg-slate-900/95 ${
													responseActionsHidden
														? ""
														: "group-hover:opacity-100"
												}`}
											>
												<Button
													type="button"
													variant="outline"
													size="sm"
													className="h-7 w-7 p-0"
													onClick={(event) => {
														event.stopPropagation();
														handleCopy();
													}}
													data-prevent-collapse="true"
													title={t("actions.copy")}
												>
													<Copy className="h-3.5 w-3.5" />
												</Button>
												<Button
													type="button"
													variant="outline"
													size="sm"
													className="h-7 w-7 p-0"
													onClick={(event) => {
														event.stopPropagation();
														scheduleResponseActionsHide();
														clearOutput();
													}}
													data-prevent-collapse="true"
													title={t("actions.clear")}
												>
													<Eraser className="h-3.5 w-3.5" />
												</Button>
											</ButtonGroup>
										</div>
									) : null}
									<div
										className={
											result
												? "whitespace-pre-wrap break-words p-3 font-mono"
												: "whitespace-pre-wrap break-words p-3 text-xs text-slate-500 dark:text-slate-300"
										}
									>
										{result
											? (extractHumanText(result) ?? pretty(result))
											: t("response.placeholder")}
									</div>
								</div>
							</CardListScrollBody>
						</TabsContent>
						<TabsContent
							value="events"
							className="min-h-0 flex-1 flex-col data-[state=active]:flex"
							onClick={handleToggleFormClick}
						>
							<CardListScrollBody>
								{events.length === 0 ? (
									<div className="min-h-full p-3 text-xs text-slate-500 dark:text-slate-300">
										{t("events.placeholder")}
									</div>
								) : (
									<>
										<ul className="space-y-2 p-3">
											{events.map((entry, index) => {
												const label = formatEventLabel(entry, t);
												const detail = formatEventDetails(entry, t);
												const key = `${entry.data.event}-${entry.timestamp}-${index}`;
												return (
													<li
														key={key}
														className="rounded border border-slate-200 bg-white p-3 text-xs shadow-sm dark:border-slate-700 dark:bg-slate-900/50"
													>
														<div className="flex items-center justify-between gap-2">
															<div className="flex items-center gap-2">
																<Badge
																	variant={badgeVariantForEvent(entry.data.event)}
																	className="uppercase"
																>
																	{entry.data.event}
																</Badge>
																<span className="font-medium text-slate-700 dark:text-slate-100">
																	{label}
																</span>
															</div>
															<span className="text-[11px] text-slate-500 dark:text-slate-300">
																{formatTimestamp(entry.timestamp)}
															</span>
														</div>
														{detail ? (
															<pre className="mt-2 whitespace-pre-wrap break-words text-[11px] text-slate-600 dark:text-slate-300">
																{detail}
															</pre>
														) : null}
													</li>
												);
											})}
										</ul>
										<div ref={eventsEndRef} />
									</>
								)}
							</CardListScrollBody>
						</TabsContent>
					</Tabs>
				</div>

				<DrawerFooter className="shrink-0 border-t px-6 py-4">
					<div className="flex w-full flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
						<Button
							variant="outline"
							onClick={() => onOpenChange(false)}
							className="w-full sm:w-auto"
						>
							{t("actions.close")}
						</Button>
						<div className="flex w-full flex-col gap-2 sm:w-auto sm:flex-row">
							{activeKind === "tool" && activeCallId && submitting ? (
								<Button
									variant="destructive"
									onClick={handleCancel}
									disabled={cancelling}
									className="w-full sm:w-auto"
								>
									{cancelling ? t("actions.cancelling") : t("actions.cancel")}
								</Button>
							) : null}
							<Button
								onClick={onSubmit}
								disabled={submitting || isProxyChecking || !canUseCurrentMode}
								className="w-full sm:w-auto"
							>
								{submitting
									? t("actions.running")
									: t(getInspectorPrimaryActionKey(activeKind))}
							</Button>
						</div>
					</div>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}

export default InspectorDrawer;
