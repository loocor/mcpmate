import {
	AlertCircle,
	AlertTriangle,
	CheckCircle2,
	ChevronsUpDown,
	Copy,
	Eraser,
	ExternalLink,
	Loader2,
	Maximize2,
	Minimize2,
	RefreshCw,
	ShieldAlert,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	inspectorApi,
	isInspectorSessionUnavailableError,
	systemApi,
} from "../lib/api";
import { writeClipboardText } from "../lib/clipboard";
import { smartFormat } from "../lib/format";
import { usePageTranslations } from "../lib/i18n/usePageTranslations";
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Textarea } from "./ui/textarea";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "./ui/tooltip";

type InspectorKind = "tool" | "resource" | "prompt" | "template";
type InspectorMode = "proxy" | "native";
type InspectorProxyMode = "hosted" | "unify";
type InspectorProxyScope = "isolated" | "active_catalog";

export interface InspectorDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	serverId?: string;
	serverName?: string;
	scratchId?: string;
	showStandaloneButton?: boolean;
	kind: InspectorKind;
	item: CapabilityRecord | null;
	capabilityOptions?: CapabilityRecord[];
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

type InspectorCapabilityListData = {
	tools?: unknown[];
	prompts?: unknown[];
	resources?: unknown[];
	templates?: unknown[];
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

type InspectorTargetPayload = {
	server_id?: string;
	server_name?: string;
	scratch_id?: string;
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
	"resource_uri",
	"uri",
	"name",
];

const TEMPLATE_KIND_KEYS: Array<keyof CapabilityRecord> = [
	"uriTemplate",
	"uri_template",
	"name",
];

const INSPECT_SESSION_GRACE_MS = 30_000;
const INSPECT_SESSION_KEEPALIVE_MS = 60_000;

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
		return uniqueName || toolName || rawName || "";
	}
	return toolName || rawName || uniqueName || "";
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
		return uniqueName || promptName || rawName || "";
	}
	return promptName || rawName || uniqueName || "";
}

function pickTemplateName(source: CapabilityRecord | null): string {
	return (
		toStringValue(source?.uriTemplate) ??
		toStringValue(source?.uri_template) ??
		toStringValue(source?.name) ??
		""
	);
}

function pickResourceUri(source: CapabilityRecord | null): string {
	return (
		toStringValue(source?.resource_uri) ??
		toStringValue(source?.uri) ??
		toStringValue(source?.name) ??
		""
	);
}

function normalizeCapabilityOptions(
	resp: InspectorResponse<InspectorCapabilityListData> | undefined,
	kind: InspectorKind,
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
			.filter(Boolean) as CapabilityRecord[]
		: [];
}

export function InspectorDrawer({
	open,
	onOpenChange,
	serverId,
	serverName,
	scratchId,
	showStandaloneButton = true,
	kind,
	item,
	capabilityOptions,
	onLog,
}: InspectorDrawerProps) {
	const { t } = useTranslation("inspector");
	usePageTranslations("inspector");
	const drawerContentRef = useRef<HTMLDivElement | null>(null);
	const [mode, setMode] = useState<InspectorMode>("native");
	const [proxyMode, setProxyMode] = useState<InspectorProxyMode>("hosted");
	const [proxyScope, setProxyScope] =
		useState<InspectorProxyScope>("isolated");
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
	const initializedFormKeyRef = useRef<string>("");
	const [overrideItem, setOverrideItem] = useState<CapabilityRecord | null>(
		null,
	);
	const currentItem = overrideItem ?? item;
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
	const [events, setEvents] = useState<InspectorEventEntry[]>([]);
	const eventsEndRef = useRef<HTMLDivElement | null>(null);
	const wsRef = useRef<WebSocket | null>(null);
	const [activeCallId, setActiveCallId] = useState<string | null>(null);
	const activeCallIdRef = useRef<string | null>(null);
	const [capOptions, setCapOptions] = useState<CapabilityRecord[]>([]);
	const [capOptionsLoading, setCapOptionsLoading] = useState(false);
	const [capOptionsError, setCapOptionsError] = useState<string | null>(null);
	const [listedOptionKeys, setListedOptionKeys] = useState<Set<string>>(
		() => new Set(),
	);
	const [nativeSession, setNativeSession] =
		useState<InspectorSessionOpenData | null>(null);
	const nativeSessionRef = useRef<InspectorSessionOpenData | null>(null);
	const nativeSessionTargetKeyRef = useRef<string | null>(null);
	const pendingNativeSessionRef = useRef<{
		targetKey: string;
		promise: Promise<InspectorSessionOpenData>;
	} | null>(null);
	const nativeSessionCloseTimer = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const mountedRef = useRef(true);
	const [view, setView] = useState<"response" | "events">("response");

	// combobox open/width is handled in CapabilityCombobox
	const [formCollapsed, setFormCollapsed] = useState(false);

	const proxyUsesActiveCatalog =
		mode === "proxy" && proxyScope === "active_catalog";
	const targetPayload = useMemo<InspectorTargetPayload>(
		() => ({
			server_id: scratchId ? undefined : serverId,
			server_name: scratchId ? undefined : serverName,
			scratch_id: scratchId,
		}),
		[scratchId, serverId, serverName],
	);
	const nativeTargetKey = scratchId || serverId || serverName || "missing-target";
	const requestTargetPayload = useMemo<InspectorTargetPayload>(
		() => (proxyUsesActiveCatalog ? {} : targetPayload),
		[proxyUsesActiveCatalog, targetPayload],
	);
	const hasRequestTarget =
		proxyUsesActiveCatalog ||
		Boolean(
			requestTargetPayload.server_id ||
				requestTargetPayload.server_name ||
				requestTargetPayload.scratch_id,
		);
	const optionListKey = `${nativeTargetKey}:${mode}:${proxyMode}:${proxyScope}:${kind}`;
	const hasListedOptions = listedOptionKeys.has(optionListKey);
	const hasProvidedOptions = capabilityOptions !== undefined;
	const propItemKey = useMemo(() => computeRecordKey(item, kind), [item, kind]);
	const currentItemKey = useMemo(
		() => computeRecordKey(currentItem, kind),
		[currentItem, kind],
	);
	const formStateKey = useMemo(
		() => (currentItemKey ? `${kind}:${currentItemKey}` : ""),
		[currentItemKey, kind],
	);
	const lastPropKeyRef = useRef<string>(propItemKey);
	const wasOpenRef = useRef<boolean>(false);

	useEffect(() => {
		mountedRef.current = true;
		return () => {
			mountedRef.current = false;
		};
	}, []);

	const setNativeSessionState = useCallback(
		(session: InspectorSessionOpenData | null, targetKey?: string | null) => {
			nativeSessionRef.current = session;
			nativeSessionTargetKeyRef.current = session
				? (targetKey ?? nativeSessionTargetKeyRef.current)
				: null;
			if (mountedRef.current) {
				setNativeSession(session);
			}
		},
		[],
	);

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
				setNativeSessionState(null);
			}
		},
		[setNativeSessionState],
	);

	const closePendingNativeSession = useCallback(() => {
		const pending = pendingNativeSessionRef.current;
		pendingNativeSessionRef.current = null;
		if (!pending) {
			return;
		}
		void pending.promise
			.then((session) => closeNativeSession(session))
			.catch((error) => {
				console.warn("Pending inspector session did not open", error);
			});
	}, [closeNativeSession]);

	const invalidateNativeSession = useCallback(() => {
		clearNativeSessionCloseTimer();
		closePendingNativeSession();
		const current = nativeSessionRef.current;
		setNativeSessionState(null);
		if (current) {
			void closeNativeSession(current);
		}
	}, [
		clearNativeSessionCloseTimer,
		closeNativeSession,
		closePendingNativeSession,
		setNativeSessionState,
	]);

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
		if (
			!targetPayload.server_id &&
			!targetPayload.server_name &&
			!targetPayload.scratch_id
		) {
			return undefined;
		}
		clearNativeSessionCloseTimer();

		const current = nativeSessionRef.current;
		if (
			current?.mode === "native" &&
			nativeSessionTargetKeyRef.current === nativeTargetKey
		) {
			return current.session_id;
		}

		const pending = pendingNativeSessionRef.current;
		if (pending?.targetKey === nativeTargetKey) {
			const session = await pending.promise;
			return session.session_id;
		}

		if (current) {
			await closeNativeSession(current);
		}

		const pendingPromise = inspectorApi
			.sessionOpen({
				mode: "native",
				...targetPayload,
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
			targetKey: nativeTargetKey,
			promise: pendingPromise,
		};

		try {
			const session = await pendingPromise;
			if (pendingNativeSessionRef.current?.promise !== pendingPromise) {
				void closeNativeSession(session);
				return undefined;
			}
			pendingNativeSessionRef.current = null;
			setNativeSessionState(session, nativeTargetKey);
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
		nativeTargetKey,
		setNativeSessionState,
		targetPayload,
	]);

	const refreshNativeSession = useCallback(
		async (session: InspectorSessionOpenData) => {
			const response = await inspectorApi.sessionRefresh({
				session_id: session.session_id,
			});
			if (!response?.success || !response.data) {
				throw new Error(
					response?.error
						? String(response.error)
						: "Failed to refresh inspector session",
				);
			}
			if (nativeSessionRef.current?.session_id !== session.session_id) {
				return;
			}
			setNativeSessionState(response.data);
		},
		[setNativeSessionState],
	);

	useEffect(() => {
		const current = nativeSessionRef.current;
		if (!current) {
			if (!open || mode !== "native") {
				closePendingNativeSession();
			}
			return;
		}

		if (
			open &&
			mode === "native" &&
			nativeSessionTargetKeyRef.current === nativeTargetKey
		) {
			clearNativeSessionCloseTimer();
			return;
		}

		closePendingNativeSession();
		scheduleNativeSessionClose(current);
	}, [
		clearNativeSessionCloseTimer,
		closePendingNativeSession,
		mode,
		nativeTargetKey,
		open,
		scheduleNativeSessionClose,
	]);

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
	}, [ensureNativeSession, mode, nativeSession?.session_id, open, t]);

	useEffect(() => {
		if (!open || mode !== "native" || !nativeSession?.session_id) {
			return;
		}

		let cancelled = false;
		const refresh = async () => {
			const current = nativeSessionRef.current;
			if (!current) {
				return;
			}
			try {
				await refreshNativeSession(current);
			} catch (error) {
				if (cancelled) {
					return;
				}
				console.warn("Failed to refresh inspector session", error);
				if (
					isInspectorSessionUnavailableError(error) &&
					nativeSessionRef.current?.session_id === current.session_id
				) {
					invalidateNativeSession();
				}
			}
		};

		const interval = window.setInterval(() => {
			void refresh();
		}, INSPECT_SESSION_KEEPALIVE_MS);

		return () => {
			cancelled = true;
			window.clearInterval(interval);
		};
	}, [
		mode,
		nativeSession?.session_id,
		open,
		invalidateNativeSession,
		refreshNativeSession,
	]);

	useEffect(() => {
		return () => {
			closePendingNativeSession();
			const current = nativeSessionRef.current;
			if (current) {
				scheduleNativeSessionClose(current);
			}
		};
	}, [closePendingNativeSession, scheduleNativeSessionClose]);

	useEffect(() => {
		if (propItemKey !== lastPropKeyRef.current) {
			setOverrideItem(null);
			lastPropKeyRef.current = propItemKey;
		}
	}, [propItemKey]);

	useEffect(() => {
		setResult(null);
		setActiveCallId(null);
		activeCallIdRef.current = null;
		setOverrideItem(null);
		if (!hasProvidedOptions) {
			setCapOptions(item ? [item] : []);
			setListedOptionKeys(new Set());
		}
	}, [hasProvidedOptions, item, nativeTargetKey]);

	useEffect(() => {
		if (open && !wasOpenRef.current) {
			setMode("native");
			setResult(null);
			setEvents([]);
			setView("response");
			setOverrideItem(null);
			setListedOptionKeys(new Set());
			setCapOptionsError(null);
			setFormCollapsed(false);
		}
		if (!open && wasOpenRef.current) {
			setOverrideItem(null);
			formSnapshotsRef.current.clear();
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
	}, [open]);

	useEffect(() => {
		if (!open) {
			return;
		}
		setResult(null);
		setView("response");
		setSubmitting(false);
		setCancelling(false);
		setActiveCallId(null);
		activeCallIdRef.current = null;
		if (wsRef.current) {
			wsRef.current.close();
			wsRef.current = null;
		}
		setFormCollapsed(false);
	}, [open, currentItemKey, kind]);

	useEffect(() => {
		eventsEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
	}, [events]);

	useEffect(() => {
		if (!open) {
			return;
		}
		setCapOptions(capabilityOptions ?? (item ? [item] : []));
		setCapOptionsLoading(false);
		setCapOptionsError(null);
	}, [capabilityOptions, item, kind, mode, open, propItemKey]);

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
			if (kind === "tool") {
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
			if (kind === "prompt") {
				return normalizeArguments(sourceItem?.arguments).map((arg) => ({
					name: arg.name ?? "arg",
					type: arg.type ?? "string",
					required: Boolean(arg.required),
					description: arg.description,
					default: arg.default,
				}));
			}
			if (kind === "template") {
				// Parse {placeholder} from uriTemplate
				const uriTemplate =
					toStringValue(sourceItem?.uriTemplate) ??
					toStringValue(sourceItem?.uri_template) ??
					"";
				const placeholderRegex = /\{([^}]+)\}/g;
				const matches = [...uriTemplate.matchAll(placeholderRegex)];
				return matches.map((match) => ({
					name: match[1],
					type: "string",
					required: true,
					description: `Value for {${match[1]}} placeholder`,
				}));
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
		if (kind === "tool") {
			schema = extractToolSchema(source);
			if (!schema) {
				const args = normalizeArguments(source?.arguments);
				if (args.length > 0) {
					schema = buildSchemaFromArguments(args);
				}
			}
		} else if (kind === "prompt") {
			const args = normalizeArguments(source?.arguments);
			if (args.length > 0) {
				schema = buildSchemaFromArguments(args);
			}
		} else if (kind === "template") {
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
	}, [open, currentItem, kind, formStateKey]);

	useEffect(() => {
		if (!open) {
			return;
		}
		const source = currentItem ?? null;
		if (kind === "tool") {
			setName(pickToolNameForMode(source, mode));
		} else if (kind === "prompt") {
			setName(pickPromptNameForMode(source, mode));
		} else if (kind === "resource") {
			setUri(pickResourceUri(source));
		} else if (kind === "template") {
			setName(pickTemplateName(source));
		}
	}, [open, currentItem, kind, mode]);

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

	function readRequestArguments(): JsonObject | undefined {
		if (!expectsArguments) {
			return undefined;
		}
		if (useRaw) {
			return parseArgs();
		}
		return values;
	}

	// Try to extract text from common MCP/LLM response envelopes
	function extractHumanText(value: unknown): string | null {
		if (value && typeof value === "object" && !Array.isArray(value)) {
			const rec = value as Record<string, unknown>;
			if (rec.type === "text" && typeof rec.text === "string") {
				return rec.text as string;
			}
			if (Array.isArray(rec.content)) {
				const segments = rec.content.map((seg) => {
					if (typeof seg === "string") return seg;
					if (
						isRecord(seg) &&
						(seg.type === "text" || seg.type === "input_text") &&
						typeof seg.text === "string"
					) {
						return seg.text;
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
			const key = computeRecordKey(entry, kind) || `index:${index}`;
			map.set(key, entry);
		});
		return map;
	}, [capOptions, kind]);

	const refreshCapabilityOptions = useCallback(async () => {
		if (!hasRequestTarget) {
			setCapOptionsError(t("errors.sessionMissing"));
			return;
		}
		setCapOptionsLoading(true);
		setCapOptionsError(null);
		try {
			const sessionId =
				mode === "native" ? await ensureNativeSession() : undefined;
			if (mode === "native" && !sessionId) {
				throw new Error(t("errors.sessionMissing"));
			}
			const commonPayload = {
				...requestTargetPayload,
				mode,
				session_id: sessionId,
				refresh: true,
				...(mode === "proxy"
					? { proxy_mode: proxyMode, proxy_scope: proxyScope }
					: {}),
			};
			let resp: InspectorResponse<InspectorCapabilityListData> | undefined;
			if (kind === "tool") {
				resp = (await inspectorApi.toolsList(commonPayload)) as
					| InspectorResponse<{ tools?: unknown[] }>
					| undefined;
			} else if (kind === "prompt") {
				resp = (await inspectorApi.promptsList(commonPayload)) as
					| InspectorResponse<{ prompts?: unknown[] }>
					| undefined;
			} else if (kind === "resource") {
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
			setCapOptions(normalizeCapabilityOptions(resp, kind));
			setListedOptionKeys((current) => {
				const next = new Set(current);
				next.add(optionListKey);
				return next;
			});
		} catch (error) {
			if (mode === "native" && isInspectorSessionUnavailableError(error)) {
				invalidateNativeSession();
			}
			setCapOptionsError(
				error instanceof Error ? error.message : String(error ?? ""),
			);
		} finally {
			setCapOptionsLoading(false);
		}
	}, [
		ensureNativeSession,
		invalidateNativeSession,
		kind,
		mode,
		optionListKey,
		proxyMode,
		proxyScope,
		hasRequestTarget,
		requestTargetPayload,
		t,
	]);

	const handleCapabilitySelect = useCallback(
		(value: string) => {
			setResult(null);
			setView("response");
			setActiveCallId(null);
			activeCallIdRef.current = null;
			setUseRaw(false);
			const match = optionsMap.get(value);
			if (match) {
				setOverrideItem(match);
				if (kind === "tool") setName(pickToolNameForMode(match, mode));
				else if (kind === "prompt") {
					setName(pickPromptNameForMode(match, mode));
				} else if (kind === "resource") {
					setUri(pickResourceUri(match));
				} else if (kind === "template") {
					setName(pickTemplateName(match));
				}
			} else {
				setOverrideItem(null);
				if (kind === "resource") setUri(value);
				else setName(value);
			}
		},
		[optionsMap, kind, mode],
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
		setView("response");
	}, []);

	const hasSchemaInputs =
		schemaObj &&
		schemaObj.properties &&
		Object.keys(schemaObj.properties).length > 0;
	const hasFieldInputs = fields.length > 0;
	const expectsArguments =
		kind !== "resource" && (hasSchemaInputs || hasFieldInputs);
	const sessionExpiry = (() => {
		const ms = Number(nativeSession?.expires_at_epoch_ms ?? NaN);
		if (!Number.isFinite(ms)) return null;
		return new Date(ms).toLocaleTimeString([], {
			hour: "2-digit",
			minute: "2-digit",
		});
	})();
	const displayTargetLabel = scratchId
		? `Scratch: ${serverName ?? scratchId}`
		: serverName || serverId || "-";
	const serverInputValue = scratchId
		? (serverName ?? scratchId)
		: serverName || serverId || "-";
	const standaloneUrl = useMemo(() => {
		if (!showStandaloneButton || scratchId || (!serverId && !serverName)) {
			return null;
		}
		const params = new URLSearchParams();
		if (serverId) {
			params.set("server_id", serverId);
		} else if (serverName) {
			params.set("server_name", serverName);
		}
		params.set("kind", kind);
		const capabilityKey =
			currentItemKey || (kind === "resource" ? uri : name).trim();
		if (capabilityKey) {
			params.set("capability_key", capabilityKey);
		}
		return `/inspector?${params.toString()}`;
	}, [
		currentItemKey,
		kind,
		name,
		scratchId,
		serverId,
		serverName,
		showStandaloneButton,
		uri,
	]);
	const handleOpenStandalone = useCallback(() => {
		if (!standaloneUrl) return;
		window.open(standaloneUrl, "_blank", "noopener,noreferrer");
	}, [standaloneUrl]);

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

	async function onSubmit() {
		try {
			setSubmitting(true);
			setResult(null);
			if (kind === "tool") {
				setCancelling(false);
			}
			let resp: InspectorResponse<Record<string, unknown>> | null = null;
			const baseLog = {
				id: newLogId(),
				timestamp: Date.now(),
				channel: "inspector" as const,
				mode,
			};
			const proxyPayload =
				mode === "proxy"
					? { proxy_mode: proxyMode, proxy_scope: proxyScope }
					: {};
			const effectiveSessionId =
				mode === "native" ? await ensureNativeSession() : undefined;
			if (mode === "native" && !effectiveSessionId) {
				throw new Error(t("errors.sessionMissing"));
			}
			if (kind === "tool") {
				const args = readRequestArguments();
				if (expectsArguments && args === undefined) {
					setSubmitting(false);
					return;
				}

				if (!hasRequestTarget) {
					throw new Error(t("errors.sessionMissing"));
				}

				onLog?.({
					...baseLog,
					event: "request",
					method: "tools/call",
					payload: {
						tool: name,
						...requestTargetPayload,
						arguments: args,
						timeout_ms: timeoutMs,
						session_id: effectiveSessionId,
						...proxyPayload,
					},
				});

				const response = await inspectorApi.toolCallStart({
					tool: name,
					...requestTargetPayload,
					mode,
					arguments: args,
					timeout_ms: timeoutMs,
					session_id: effectiveSessionId,
					...proxyPayload,
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
			} else if (kind === "prompt") {
				const args = readRequestArguments();
				if (expectsArguments && args === undefined) return;
				onLog?.({
					...baseLog,
					event: "request",
					method: "prompts/get",
					payload: {
						name,
						...requestTargetPayload,
						arguments: args,
						session_id: effectiveSessionId,
						...proxyPayload,
					},
				});
				resp = (await inspectorApi.promptGet({
					name,
					...requestTargetPayload,
					mode,
					arguments: args,
					session_id: effectiveSessionId,
					...proxyPayload,
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
			} else if (kind === "template") {
				const args = readRequestArguments();
				if (expectsArguments && args === undefined) return;

				// Generate URI from template by replacing {arg} placeholders
				let generatedUri = name;
				if (args) {
					Object.entries(args).forEach(([key, value]) => {
						generatedUri = generatedUri.replace(`{${key}}`, String(value));
					});
				}

				onLog?.({
					...baseLog,
					event: "request",
					method: "resources/read",
					payload: {
						uri: generatedUri,
						template: name,
						arguments: args,
						...requestTargetPayload,
						session_id: effectiveSessionId,
						...proxyPayload,
					},
				});
				resp = (await inspectorApi.resourceRead({
					uri: generatedUri,
					...requestTargetPayload,
					session_id: effectiveSessionId,
					mode,
					...proxyPayload,
				})) as InspectorResponse<Record<string, unknown>>;
				if (!resp?.success) {
					throw new Error(
						resp?.error ? String(resp.error) : "Template read failed",
					);
				}
				const data = (resp.data ?? {}) as Record<string, unknown>;
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
			} else {
				onLog?.({
					...baseLog,
					event: "request",
					method: "resources/read",
					payload: {
						uri,
						...requestTargetPayload,
						session_id: effectiveSessionId,
						...proxyPayload,
					},
				});
				resp = (await inspectorApi.resourceRead({
					uri,
					...requestTargetPayload,
					session_id: effectiveSessionId,
					mode,
					...proxyPayload,
				})) as InspectorResponse<Record<string, unknown>>;
				if (!resp?.success) {
					throw new Error(
						resp?.error ? String(resp.error) : "Resource read failed",
					);
				}
				const data = (resp.data ?? {}) as Record<string, unknown>;
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
					kind === "tool"
						? "tools/call"
						: kind === "prompt"
							? "prompts/get"
							: kind === "template"
								? "templates/read"
								: "resources/read",
				mode,
				message: e instanceof Error ? e.message : String(e),
				payload: e,
			});
			notifyError("Inspector request failed", String(e));
			setSubmitting(false);
		} finally {
			if (kind !== "tool") {
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
		} catch (err) {
			notifyError(
				t("notifications.copyFailed"),
				err instanceof Error ? err.message : String(err),
			);
		}
	}, [result, t]);

	const displaySessionId =
		mode === "native" ? nativeSession?.session_id : undefined;
	const sessionActive = Boolean(displaySessionId);
	const sessionIndicator =
		kind === "tool" && mode === "native" ? (
			<TooltipProvider delayDuration={150}>
				<Tooltip>
					<TooltipTrigger asChild>
						<button
							type="button"
							className="inline-flex h-7 w-7 items-center justify-center text-slate-500 transition hover:text-slate-700 dark:text-slate-300 dark:hover:text-slate-100"
							aria-label={
								sessionActive ? t("session.active") : t("session.pending")
							}
						>
							{sessionActive ? (
								<CheckCircle2 className="h-4 w-4 text-emerald-500" />
							) : (
								<AlertCircle className="h-4 w-4 text-amber-500" />
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
									serverName: displayTargetLabel,
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

	const listButton = (
		<Button
			type="button"
			variant="default"
			onClick={() => void refreshCapabilityOptions()}
			disabled={capOptionsLoading || submitting}
			className="h-9 shrink-0 gap-2 px-3"
		>
			{capOptionsLoading ? (
				<Loader2 className="h-4 w-4 animate-spin" />
			) : (
				<RefreshCw className="h-4 w-4" />
			)}
			{hasProvidedOptions || hasListedOptions
				? t("actions.refresh", { defaultValue: "Refresh" })
				: t("actions.list", { defaultValue: "List" })}
		</Button>
	);
	const outputToggleLabel = formCollapsed
		? t("actions.restoreInput", { defaultValue: "Restore input" })
		: t("actions.maximizeOutput", { defaultValue: "Maximize output" });

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent
				ref={drawerContentRef}
				className="flex h-full flex-col overflow-hidden"
			>
				<DrawerHeader className="shrink-0">
					<div className="flex items-start justify-between gap-3">
						<div>
							<DrawerTitle>
								{t("title")} ·{" "}
								{kind === "tool"
									? t("modes.toolCall")
									: kind === "resource"
										? t("modes.readResource")
										: kind === "template"
											? t("modes.getTemplate")
											: t("modes.getPrompt")}
							</DrawerTitle>
							<DrawerDescription>{t("subtitle")}</DrawerDescription>
						</div>
						<div className="flex shrink-0 items-center gap-1">
							{standaloneUrl ? (
								<TooltipProvider delayDuration={150}>
									<Tooltip>
										<TooltipTrigger asChild>
											<Button
												type="button"
												variant="ghost"
												size="icon"
												className="h-8 w-8"
												onClick={handleOpenStandalone}
												aria-label={t("standalone.open", {
													defaultValue: "Open standalone Inspector",
												})}
											>
												<ExternalLink className="h-4 w-4" />
											</Button>
										</TooltipTrigger>
										<TooltipContent side="left" align="end">
											{t("standalone.open", {
												defaultValue: "Open standalone Inspector",
											})}
										</TooltipContent>
									</Tooltip>
								</TooltipProvider>
							) : null}
						</div>
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
												onClick={() => setMode("native")}
											>
												<AlertTriangle className="h-4 w-4" />
												{t("form.native", { defaultValue: "Native" })}
											</Button>
											<Button
												type="button"
												variant={mode === "proxy" ? "default" : "outline"}
												className="h-9 flex-1 rounded-l-none gap-2 px-3"
												disabled={Boolean(scratchId)}
												onClick={() => setMode("proxy")}
											>
												<ShieldAlert className="h-4 w-4" />
												{t("form.proxy", { defaultValue: "Proxy" })}
											</Button>
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
										<div className="relative">
											<TooltipProvider delayDuration={200}>
												<Tooltip>
													<TooltipTrigger asChild>
														<Input
															value={serverInputValue}
															readOnly
															aria-readonly="true"
															className="h-9 cursor-default pr-10 text-slate-600 dark:text-slate-300"
														/>
													</TooltipTrigger>
													{!scratchId && serverName && serverId ? (
														<TooltipContent side="top" align="start">
															<p className="text-xs">ID: {serverId}</p>
														</TooltipContent>
													) : null}
												</Tooltip>
											</TooltipProvider>
											{sessionIndicator ? (
												<div
													className="absolute inset-y-0 right-1 flex items-center"
													data-prevent-collapse="true"
												>
													{sessionIndicator}
												</div>
											) : null}
										</div>
									</div>
								</div>
								{mode === "proxy" ? (
									<div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
										<div className="space-y-1">
											<Label>
												{t("form.proxyMode", { defaultValue: "Proxy mode" })}
											</Label>
											<ButtonGroup className="flex w-full">
												<Button
													type="button"
													variant={proxyMode === "hosted" ? "default" : "outline"}
													className="h-9 flex-1 rounded-r-none px-3"
													onClick={() => setProxyMode("hosted")}
												>
													{t("form.hosted", { defaultValue: "Hosted" })}
												</Button>
												<Button
													type="button"
													variant={proxyMode === "unify" ? "default" : "outline"}
													className="h-9 flex-1 rounded-l-none px-3"
													onClick={() => setProxyMode("unify")}
												>
													{t("form.unify", { defaultValue: "Unify" })}
												</Button>
											</ButtonGroup>
										</div>
										<div className="space-y-1">
											<Label>
												{t("form.proxyScope", { defaultValue: "Surface" })}
											</Label>
											<ButtonGroup className="flex w-full">
												<Button
													type="button"
													variant={proxyScope === "isolated" ? "default" : "outline"}
													className="h-9 flex-1 rounded-r-none px-3"
													onClick={() => setProxyScope("isolated")}
												>
													{t("form.isolated", { defaultValue: "Isolated" })}
												</Button>
												<Button
													type="button"
													variant={
														proxyScope === "active_catalog" ? "default" : "outline"
													}
													className="h-9 flex-1 rounded-l-none px-3"
													onClick={() => setProxyScope("active_catalog")}
												>
													{t("form.activeCatalog", {
														defaultValue: "Active catalog",
													})}
												</Button>
											</ButtonGroup>
										</div>
									</div>
								) : null}

								{kind === "resource" || kind === "template" ? (
									<div className="space-y-1">
										<Label>
											{kind === "resource"
												? t("form.resourceUri")
												: t("form.template")}
										</Label>
										<div className="flex min-w-0 gap-2">
											<div className="min-w-0 flex-1">
												<CapabilityCombobox
													kind={kind}
													items={capOptions}
													value={currentItemKey || undefined}
													onChange={(key) => handleCapabilitySelect(key)}
													loading={capOptionsLoading}
													error={capOptionsError}
													container={drawerContentRef.current}
													triggerClassName="h-9"
													placeholder={
														kind === "resource"
															? (t("form.selectResource", {
																defaultValue: "Select resource",
															}) as string)
															: (t("form.selectTemplate", {
																defaultValue: "Select template",
															}) as string)
													}
													getKey={(it) =>
														computeRecordKey(it as CapabilityRecord, kind)
													}
													getLabel={(it) => {
														const entry = it as CapabilityRecord;
														if (kind === "template") {
															return (toStringValue(entry.uriTemplate) ||
																toStringValue(entry.uri_template) ||
																toStringValue(entry.name) ||
																computeRecordKey(entry, kind)) as string;
														}
														return (toStringValue(entry.resource_uri) ||
															toStringValue(entry.uri) ||
															toStringValue(entry.name) ||
															computeRecordKey(entry, kind)) as string;
													}}
													getDescription={(it) => {
														const entry = it as CapabilityRecord;
														return (
															toStringValue(entry.description) || undefined
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
												{kind === "tool" ? t("form.tool") : t("form.prompt")}
											</Label>
											<div className="flex min-w-0 gap-2">
												<div className="min-w-0 flex-1">
													<CapabilityCombobox
														kind={kind}
														items={capOptions}
														value={currentItemKey || undefined}
														onChange={(key) => handleCapabilitySelect(key)}
														loading={capOptionsLoading}
														error={capOptionsError}
														container={drawerContentRef.current}
														triggerClassName="h-9"
														placeholder={
															kind === "tool"
																? (t("form.selectTool", {
																	defaultValue: "Select tool",
																}) as string)
																: (t("form.selectPrompt", {
																	defaultValue: "Select prompt",
																}) as string)
														}
														getKey={(it) =>
															computeRecordKey(it as CapabilityRecord, kind)
														}
														getLabel={(it) => {
															const entry = it as CapabilityRecord;
															if (kind === "tool") {
																return (
																	pickToolNameForMode(entry, mode) ||
																	computeRecordKey(entry, kind)
																);
															} else {
																const uniqueName = toStringValue(
																	entry.unique_name,
																);
																const promptName = toStringValue(
																	entry.prompt_name,
																);
																const rawName = toStringValue(
																	entry.name,
																);
																return (
																	(mode === "proxy"
																		? uniqueName || promptName || rawName
																		: promptName || rawName || uniqueName) ||
																	computeRecordKey(entry, kind)
																);
															}
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

								{kind !== "resource" ? (
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
						>
							<CardListScrollBody>
								<div className="group relative min-h-full text-xs text-slate-700 dark:text-slate-200">
									<div className="pointer-events-none absolute top-0 right-0 z-10 flex w-full justify-end p-2">
										<ButtonGroup className="pointer-events-none bg-white/95 opacity-0 shadow-sm backdrop-blur-sm transition-opacity group-hover:pointer-events-auto group-hover:opacity-50 dark:bg-slate-900/95">
											{result ? (
												<>
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
															clearOutput();
														}}
														data-prevent-collapse="true"
														title={t("actions.clear")}
													>
														<Eraser className="h-3.5 w-3.5" />
													</Button>
												</>
											) : null}
											<Button
												type="button"
												variant="outline"
												size="sm"
												className="h-7 w-7 p-0"
												onClick={(event) => {
													event.stopPropagation();
													setFormCollapsed((collapsed) => !collapsed);
												}}
												data-prevent-collapse="true"
												title={outputToggleLabel}
												aria-label={outputToggleLabel}
											>
												{formCollapsed ? (
													<Minimize2 className="h-3.5 w-3.5" />
												) : (
													<Maximize2 className="h-3.5 w-3.5" />
												)}
											</Button>
										</ButtonGroup>
									</div>
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
						>
							<CardListScrollBody>
								<div className="group relative min-h-full">
									<div className="pointer-events-none absolute top-0 right-0 z-10 flex w-full justify-end p-2">
										<ButtonGroup className="pointer-events-none bg-white/95 opacity-0 shadow-sm backdrop-blur-sm transition-opacity group-hover:pointer-events-auto group-hover:opacity-50 dark:bg-slate-900/95">
											<Button
												type="button"
												variant="outline"
												size="sm"
												className="h-7 w-7 p-0"
												onClick={(event) => {
													event.stopPropagation();
													setFormCollapsed((collapsed) => !collapsed);
												}}
												data-prevent-collapse="true"
												title={outputToggleLabel}
												aria-label={outputToggleLabel}
											>
												{formCollapsed ? (
													<Minimize2 className="h-3.5 w-3.5" />
												) : (
													<Maximize2 className="h-3.5 w-3.5" />
												)}
											</Button>
										</ButtonGroup>
									</div>
									{events.length === 0 ? (
										<div className="min-h-full p-3 text-xs text-slate-500 dark:text-slate-300">
											{t("events.placeholder")}
										</div>
									) : (
										<>
											<ul className="text-xs">
												{events.map((entry, index) => {
													const label = formatEventLabel(entry, t);
													const detail = formatEventDetails(entry, t);
													const key = `${entry.data.event}-${entry.timestamp}-${index}`;
													const inlineDetail =
														entry.data.event === "result" ? detail : null;
													const blockDetail =
														detail && !inlineDetail ? detail : null;
													return (
														<li
															key={key}
															className="flex min-h-14 items-center px-3 py-1.5 even:bg-white odd:bg-slate-50 dark:even:bg-slate-900 dark:odd:bg-slate-800/70"
														>
															<div className="min-w-0 flex-1">
																<div className="flex min-w-0 items-center gap-2">
																	<Badge
																		variant={badgeVariantForEvent(entry.data.event)}
																		className="uppercase"
																	>
																		{entry.data.event}
																	</Badge>
																	<span className="min-w-0 truncate font-medium text-slate-700 dark:text-slate-100">
																		{label}
																	</span>
																</div>
																<div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 font-mono text-[11px] leading-relaxed text-slate-500 dark:text-slate-300">
																	<span>{formatTimestamp(entry.timestamp)}</span>
																	{inlineDetail ? <span>{inlineDetail}</span> : null}
																</div>
																{blockDetail ? (
																	<pre className="mt-1 whitespace-pre-wrap break-words text-[11px] leading-relaxed text-slate-600 dark:text-slate-300">
																		{blockDetail}
																	</pre>
																) : null}
															</div>
														</li>
													);
												})}
											</ul>
											<div ref={eventsEndRef} />
										</>
									)}
								</div>
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
							{kind === "tool" && activeCallId && submitting ? (
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
								disabled={submitting}
								className="w-full sm:w-auto"
							>
								{submitting ? t("actions.running") : t("actions.run")}
							</Button>
						</div>
					</div>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}

export default InspectorDrawer;
