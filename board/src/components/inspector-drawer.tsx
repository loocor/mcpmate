import { EventSourcePolyfill, NativeEventSource } from "event-source-polyfill";
import {
	AlertCircle,
	CheckCircle2,
	ChevronsUpDown,
	Copy,
	Eraser,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { inspectorApi } from "../lib/api";
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
import { SchemaForm } from "./schema-form";
import { defaultFromSchema } from "./schema-form-utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ButtonGroup } from "./ui/button-group";
import { Card, CardContent } from "./ui/card";
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
import { ScrollArea } from "./ui/scroll-area";
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

type InspectorKind = "tool" | "resource" | "prompt" | "template";

export interface InspectorDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	serverId?: string;
	serverName?: string;
	kind: InspectorKind;
	item: CapabilityRecord | null;
	mode: "proxy" | "native";
	onLog?: (entry: InspectorLogEntry) => void;
}

type Field = {
	name: string;
	type: string;
	required?: boolean;
	description?: string;
	enum?: string[];
};

export interface InspectorLogEntry {
	id: string;
	timestamp: number;
	channel: "inspector";
	event: "request" | "success" | "error" | "progress" | "log" | "cancelled";
	method: string;
	mode: "proxy" | "native";
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
	t: (key: string) => string,
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
			return entry.data.event;
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
	mode: "proxy" | "native",
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

export function InspectorDrawer({
	open,
	onOpenChange,
	serverId,
	serverName,
	kind,
	item,
	mode,
	onLog,
}: InspectorDrawerProps) {
	const { t } = useTranslation("inspector");
	usePageTranslations("inspector");
	const drawerContentRef = useRef<HTMLDivElement | null>(null);
	const [timeoutMs, setTimeoutMs] = useState<number>(8000);
	const [argsJson, setArgsJson] = useState<string>("{}");
	const [useRaw, setUseRaw] = useState(false);
	const [fields, setFields] = useState<Field[]>([]);
	const [values, setValues] = useState<JsonObject>({});
	const [schemaObj, setSchemaObj] = useState<JsonSchema | null>(null);
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
	const [session, setSession] = useState<InspectorSessionOpenData | null>(null);
	const eventSourceRef = useRef<EventSource | null>(null);
	const [activeCallId, setActiveCallId] = useState<string | null>(null);
	const activeCallIdRef = useRef<string | null>(null);
	const lastSessionParams = useRef<{
		mode: "proxy" | "native";
		serverId?: string;
		serverName?: string;
	} | null>(null);
	const [capOptions, setCapOptions] = useState<CapabilityRecord[]>([]);
	const [capOptionsLoading, setCapOptionsLoading] = useState(false);
	const [capOptionsError, setCapOptionsError] = useState<string | null>(null);
	const [view, setView] = useState<"response" | "events">("response");

	// combobox open/width is handled in CapabilityCombobox
	const [formCollapsed, setFormCollapsed] = useState(false);

	// Combobox state is managed directly by Popover component
	const propItemKey = useMemo(() => computeRecordKey(item, kind), [item, kind]);
	const currentItemKey = useMemo(
		() => computeRecordKey(currentItem, kind),
		[currentItem, kind],
	);
	const lastPropKeyRef = useRef<string>(propItemKey);
	const wasOpenRef = useRef<boolean>(false);

	useEffect(() => {
		if (propItemKey !== lastPropKeyRef.current) {
			setOverrideItem(null);
			lastPropKeyRef.current = propItemKey;
		}
	}, [propItemKey]);

	useEffect(() => {
		if (open && !wasOpenRef.current) {
			setResult(null);
			setEvents([]);
			setView("response");
			setOverrideItem(null);
			setFormCollapsed(false);
		}
		if (!open && wasOpenRef.current) {
			setOverrideItem(null);
			setEvents([]);
			setActiveCallId(null);
			activeCallIdRef.current = null;
			setSubmitting(false);
			setCancelling(false);
			if (eventSourceRef.current) {
				eventSourceRef.current.close();
				eventSourceRef.current = null;
			}
		}
		wasOpenRef.current = open;
	}, [open]);

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
		if (eventSourceRef.current) {
			eventSourceRef.current.close();
			eventSourceRef.current = null;
		}
		setFormCollapsed(false);
	}, [open, currentItemKey, mode, kind]);

	useEffect(() => {
		eventsEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
	}, [events]);

	useEffect(() => {
		if (!open) {
			return;
		}
		// Reset when missing server context
		if (!serverId && !serverName) {
			setCapOptions([]);
			setCapOptionsLoading(false);
			setCapOptionsError(null);
			return;
		}
		let cancelled = false;
		setCapOptionsLoading(true);
		setCapOptionsError(null);
		(async () => {
			try {
				let resp: InspectorResponse<any> | undefined;
				if (kind === "tool") {
					resp = (await inspectorApi.toolsList({
						server_id: serverId,
						server_name: serverName,
						mode,
					})) as InspectorResponse<{ tools?: unknown[] }> | undefined;
					const rawList = Array.isArray(resp?.data?.tools)
						? resp?.data?.tools
						: [];
					const normalized = rawList
						.map((entry: unknown) => toCapabilityRecord(entry))
						.filter(Boolean) as CapabilityRecord[];
					if (!cancelled) setCapOptions(normalized);
				} else if (kind === "prompt") {
					resp = (await inspectorApi.promptsList({
						server_id: serverId,
						server_name: serverName,
						mode,
					})) as InspectorResponse<{ prompts?: unknown[] }> | undefined;
					const rawList = Array.isArray(resp?.data?.prompts)
						? resp?.data?.prompts
						: [];
					const normalized = rawList
						.map((entry: unknown) => toCapabilityRecord(entry))
						.filter(Boolean) as CapabilityRecord[];
					if (!cancelled) setCapOptions(normalized);
				} else if (kind === "resource") {
					resp = (await inspectorApi.resourcesList({
						server_id: serverId,
						server_name: serverName,
						mode,
					})) as InspectorResponse<{ resources?: unknown[] }> | undefined;
					const rawList = Array.isArray(resp?.data?.resources)
						? resp?.data?.resources
						: [];
					const normalized = rawList
						.map((entry: unknown) => toCapabilityRecord(entry))
						.filter(Boolean) as CapabilityRecord[];
					if (!cancelled) setCapOptions(normalized);
				} else {
					resp = (await inspectorApi.templatesList({
						server_id: serverId,
						server_name: serverName,
						mode,
					})) as InspectorResponse<{ templates?: unknown[] }> | undefined;
					const rawList = Array.isArray(resp?.data?.templates)
						? resp?.data?.templates
						: [];
					const normalized = rawList
						.map((entry: unknown) => toCapabilityRecord(entry))
						.filter(Boolean) as CapabilityRecord[];
					if (!cancelled) setCapOptions(normalized);
				}
			} catch (error) {
				if (!cancelled) {
					setCapOptionsError(
						error instanceof Error ? error.message : String(error ?? ""),
					);
				}
			} finally {
				if (!cancelled) {
					setCapOptionsLoading(false);
				}
			}
		})();
		return () => {
			cancelled = true;
		};
	}, [open, kind, serverId, serverName, mode, propItemKey]);

	useEffect(() => {
		activeCallIdRef.current = activeCallId;
	}, [activeCallId]);

	useEffect(() => {
		let cancelled = false;

		const ensureSession = async () => {
			if (!open || kind !== "tool") {
				if (session) {
					try {
						await inspectorApi.sessionClose({ session_id: session.session_id });
					} catch (error) {
						console.warn("Failed to close inspector session", error);
					}
					if (!cancelled) {
						setSession(null);
					}
				}
				lastSessionParams.current = null;
				return;
			}

			const params = { mode, serverId, serverName };

			if (
				session &&
				lastSessionParams.current &&
				lastSessionParams.current.mode === mode &&
				lastSessionParams.current.serverId === serverId &&
				lastSessionParams.current.serverName === serverName
			) {
				return;
			}

			if (session) {
				try {
					await inspectorApi.sessionClose({ session_id: session.session_id });
				} catch (error) {
					console.warn("Failed to close inspector session", error);
				}
				if (cancelled) return;
				setSession(null);
			}

			try {
				const response = await inspectorApi.sessionOpen({
					mode,
					server_id: serverId,
					server_name: serverName,
				});
				if (!cancelled && response?.success && response.data) {
					setSession(response.data);
					lastSessionParams.current = params;
				}
			} catch (error) {
				if (!cancelled) {
					notifyError("Failed to open inspector session", String(error));
					lastSessionParams.current = null;
				}
			}
		};

		void ensureSession();

		return () => {
			cancelled = true;
		};
	}, [open, kind, mode, serverId, serverName, session]);

	useEffect(() => {
		return () => {
			if (eventSourceRef.current) {
				eventSourceRef.current.close();
				eventSourceRef.current = null;
			}
			if (session) {
				void inspectorApi
					.sessionClose({ session_id: session.session_id })
					.catch((error) =>
						console.warn("Failed to close inspector session", error),
					);
			}
		};
	}, [session]);

	useEffect(() => {
		if (!open && eventSourceRef.current) {
			eventSourceRef.current.close();
			eventSourceRef.current = null;
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
			if (f.enum && f.enum.length) acc[f.name] = f.enum[0];
			else acc[f.name] = mockOfType(f.type);
		});
		return acc;
	}

	useEffect(() => {
		if (!open) {
			return;
		}
		const source = currentItem ?? null;
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
			// For templates, derive fields from uriTemplate placeholders
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
			setValues(mock);
			setArgsJson(JSON.stringify(mock, null, 2));
			setFields([]);
		} else {
			const fs = deriveFields(source);
			setFields(fs);
			if (fs.length > 0) {
				const generatedSchema = buildSchemaFromFields(fs);
				setSchemaObj(generatedSchema);
				const mock = toJsonObject(defaultFromSchema(generatedSchema));
				setValues(mock);
				setArgsJson(JSON.stringify(mock, null, 2));
			} else {
				setSchemaObj(null);
				const mock = fillMock(fs);
				setValues(mock);
				setArgsJson(JSON.stringify(mock, null, 2));
				setUseRaw(false);
			}
		}

		if (kind === "tool") {
			setName(pickToolNameForMode(source, mode));
		} else if (kind === "prompt") {
			const promptName =
				(mode === "proxy"
					? (toStringValue(source?.unique_name) ??
						toStringValue(source?.prompt_name) ??
						toStringValue(source?.name))
					: (toStringValue(source?.prompt_name) ??
						toStringValue(source?.name) ??
						toStringValue(source?.unique_name))) ?? "";
			setName(promptName);
		} else if (kind === "resource") {
			const resourceUri =
				toStringValue(source?.resource_uri) ??
				toStringValue(source?.uri) ??
				toStringValue(source?.name) ??
				"";
			setUri(resourceUri);
		} else if (kind === "template") {
			const templateName =
				toStringValue(source?.uriTemplate) ??
				toStringValue(source?.uri_template) ??
				toStringValue(source?.name) ??
				"";
			setName(templateName);
		}
		// Maintaining manual dependency list—internal helpers are stable within this component lifecycle.
		// eslint-disable-next-line react-hooks/exhaustive-deps
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
			const key = computeRecordKey(entry, kind) || `index:${index}`;
			map.set(key, entry);
		});
		return map;
	}, [capOptions, kind]);

	const handleCapabilitySelect = useCallback(
		(value: string) => {
			setResult(null);
			setEvents([]);
			setView("response");
			setActiveCallId(null);
			activeCallIdRef.current = null;
			setUseRaw(false);
			const match = optionsMap.get(value);
			if (match) {
				setOverrideItem(match);
				if (kind === "tool") setName(pickToolNameForMode(match, mode));
				else if (kind === "prompt") {
					const promptName =
						mode === "proxy"
							? toStringValue((match as any).unique_name) ||
								toStringValue((match as any).prompt_name) ||
								toStringValue((match as any).name)
							: toStringValue((match as any).prompt_name) ||
								toStringValue((match as any).name) ||
								toStringValue((match as any).unique_name);
					setName(promptName ?? "");
				} else if (kind === "resource") {
					const resourceUri =
						toStringValue((match as any).resource_uri) ||
						toStringValue((match as any).uri) ||
						toStringValue((match as any).name) ||
						"";
					setUri(resourceUri);
				} else if (kind === "template") {
					const templateName =
						toStringValue((match as any).uriTemplate) ||
						toStringValue((match as any).uri_template) ||
						toStringValue((match as any).name) ||
						"";
					setName(templateName);
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
		setEvents([]);
		setView("response");
	}, []);

	const hasSchemaInputs =
		schemaObj &&
		schemaObj.properties &&
		Object.keys(schemaObj.properties).length > 0;
	const hasFieldInputs = fields.length > 0;
	const expectsArguments =
		kind !== "resource" && (hasSchemaInputs || hasFieldInputs);
	const sessionExpiry = useMemo(() => {
		if (!session?.expires_at_epoch_ms) return null;
		const ms = Number(session.expires_at_epoch_ms);
		if (!Number.isFinite(ms)) return null;
		return new Date(ms).toLocaleTimeString([], {
			hour: "2-digit",
			minute: "2-digit",
		});
	}, [session]);

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
					if (eventSourceRef.current) {
						eventSourceRef.current.close();
						eventSourceRef.current = null;
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
					if (eventSourceRef.current) {
						eventSourceRef.current.close();
						eventSourceRef.current = null;
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
					if (eventSourceRef.current) {
						eventSourceRef.current.close();
						eventSourceRef.current = null;
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
			if (eventSourceRef.current) {
				eventSourceRef.current.close();
				eventSourceRef.current = null;
			}

			try {
				const url = inspectorApi.toolCallEventsUrl(callId);
					// Force polyfill for Safari/WKWebView (Tauri), else prefer native.
					const shouldForcePolyfill = (() => {
						try {
							const isTauri =
								typeof window !== "undefined" && (window as any).__MCPMATE_IS_TAURI__ === true;
							const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
							const isSafari = /Safari\//.test(ua) && !/Chrome\//.test(ua) && !/Chromium\//.test(ua) && !/Edg\//.test(ua);
							return isTauri || isSafari;
						} catch {
							return false;
						}
					})();
					const ES = (shouldForcePolyfill
						? EventSourcePolyfill
						: (NativeEventSource || EventSourcePolyfill)) as typeof EventSource;
					const source = new ES(url, { withCredentials: true });
				eventSourceRef.current = source;

				source.onmessage = (event) => {
					try {
						const data: InspectorSseEvent = JSON.parse(event.data);
						handleInspectorEvent(data);
					} catch (error) {
						console.warn("Failed to parse inspector event", error);
					}
				};

				source.onerror = () => {
					source.close();
					eventSourceRef.current = null;
					setSubmitting(false);
				};
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
			if (kind === "tool") {
				const args = expectsArguments
					? useRaw
						? parseArgs()
						: values
					: undefined;
				if (expectsArguments && args === undefined) return;

				const effectiveServerId = session?.server_id ?? serverId;
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
						session_id: session?.session_id,
					},
				});

				const response = await inspectorApi.toolCallStart({
					tool: name,
					server_id: effectiveServerId,
					server_name: serverName,
					mode,
					arguments: args,
					timeout_ms: timeoutMs,
					session_id: session?.session_id,
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
					},
				});
				resp = (await inspectorApi.promptGet({
					name,
					server_id: serverId,
					server_name: serverName,
					mode,
					arguments: args,
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
				const args = expectsArguments
					? useRaw
						? parseArgs()
						: values
					: undefined;
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
						server_id: serverId,
						server_name: serverName,
					},
				});
				resp = (await inspectorApi.resourceRead({
					uri: generatedUri,
					server_id: serverId,
					server_name: serverName,
					mode,
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
					payload: { uri, server_id: serverId, server_name: serverName },
				});
				resp = (await inspectorApi.resourceRead({
					uri,
					server_id: serverId,
					server_name: serverName,
					mode,
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
			if (kind !== "tool") {
				setSubmitting(false);
			}
		} catch (e) {
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

	const sessionActive = Boolean(session);
	const sessionIndicator =
		kind === "tool" ? (
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
									serverName: serverName || session?.server_id,
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

	const handleCollapseFormClick = useCallback(
		(event: React.MouseEvent<HTMLDivElement>) => {
			if (formCollapsed) return;
			const target = event.target as HTMLElement;
			// 排除交互控件（按钮、链接、弹层等）点击，避免误触发折叠
			if (
				target.closest("button") ||
				target.closest("a") ||
				target.closest('[data-prevent-collapse="true"]') ||
				target.closest("[data-radix-popper-content-wrapper]")
			) {
				return;
			}
			setFormCollapsed(true);
		},
		[formCollapsed],
	);

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
						{sessionIndicator}
					</div>
				</DrawerHeader>

				<div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
					<div
						className={`transition-all duration-300 ease-in-out ${
							formCollapsed ? "max-h-12 overflow-hidden" : "max-h-[800px]"
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
								className="flex h-10 cursor-pointer items-center justify-between rounded-md border border-dashed border-slate-200 px-3 text-sm text-slate-600 transition hover:border-slate-300 hover:bg-slate-100 dark:border-slate-800 dark:text-slate-300 dark:hover:border-slate-700 dark:hover:bg-slate-900/40"
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
							<div className="space-y-4">
								<div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
									<div className="space-y-1">
										<Label>{t("form.mode")}</Label>
										<Input value={mode} disabled className="font-mono" />
									</div>
									{kind === "tool" ? (
										<div className="space-y-1">
											<Label>{t("form.timeout")}</Label>
											<Input
												type="number"
												min={1000}
												step={500}
												value={timeoutMs}
												onChange={(e) =>
													setTimeoutMs(parseInt(e.target.value, 10) || 8000)
												}
											/>
										</div>
									) : null}
									<div className="space-y-1">
										<Label>{t("form.server")}</Label>
										<TooltipProvider delayDuration={200}>
											<Tooltip>
												<TooltipTrigger asChild>
													<Input
														value={serverName || serverId || "-"}
														disabled
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

								{kind === "resource" || kind === "template" ? (
									<div className="space-y-1">
										<Label>
											{kind === "resource"
												? t("form.resourceUri")
												: t("form.template")}
										</Label>
										<CapabilityCombobox
											kind={kind as any}
											items={capOptions}
											value={currentItemKey || undefined}
											onChange={(key) => handleCapabilitySelect(key)}
											loading={capOptionsLoading}
											error={capOptionsError}
											container={drawerContentRef.current}
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
													return (toStringValue((entry as any).uriTemplate) ||
														toStringValue((entry as any).uri_template) ||
														toStringValue((entry as any).name) ||
														computeRecordKey(entry, kind)) as string;
												}
												return (toStringValue((entry as any).resource_uri) ||
													toStringValue((entry as any).uri) ||
													toStringValue((entry as any).name) ||
													computeRecordKey(entry, kind)) as string;
											}}
											getDescription={(it) => {
												const entry = it as CapabilityRecord;
												return (
													toStringValue((entry as any).description) || undefined
												);
											}}
										/>
									</div>
								) : (
									<div className="space-y-2">
										<div className="space-y-1">
											<Label>
												{kind === "tool" ? t("form.tool") : t("form.prompt")}
											</Label>
											<CapabilityCombobox
												kind={kind as any}
												items={capOptions}
												value={currentItemKey || undefined}
												onChange={(key) => handleCapabilitySelect(key)}
												loading={capOptionsLoading}
												error={capOptionsError}
												container={drawerContentRef.current}
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
															(entry as any).unique_name,
														);
														const promptName = toStringValue(
															(entry as any).prompt_name,
														);
														const rawName = toStringValue((entry as any).name);
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
														toStringValue((entry as any).description) ||
														undefined
													);
												}}
											/>
										</div>
									</div>
								)}

								{kind !== "resource" ? (
									expectsArguments ? (
										<div className="space-y-4">
											<div className="flex items-center justify-between">
												<Label>{t("form.parameters")}</Label>
												<ButtonGroup>
													<Button
														size="sm"
														variant="outline"
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
														variant="outline"
														onClick={() => {
															setValues({});
															setArgsJson("{}");
														}}
													>
														{t("actions.clean")}
													</Button>
													<Button
														size="sm"
														variant={useRaw ? "default" : "outline"}
														onClick={() => setUseRaw((v) => !v)}
													>
														{useRaw ? t("actions.form") : t("actions.json")}
													</Button>
												</ButtonGroup>
											</div>
											{useRaw ? (
												<Textarea
													rows={8}
													className="font-mono text-xs"
													value={argsJson}
													onChange={(e) => setArgsJson(e.target.value)}
												/>
											) : schemaObj ? (
												<Card>
													<CardContent className="p-4">
														<SchemaForm
															schema={schemaObj}
															value={values}
															onChange={(v) => {
																const next = toJsonObject(v);
																setValues(next);
																setArgsJson(JSON.stringify(next, null, 2));
															}}
														/>
													</CardContent>
												</Card>
											) : (
												<Textarea
													rows={6}
													className="font-mono text-xs"
													value={argsJson}
													onChange={(e) => setArgsJson(e.target.value)}
												/>
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
						className="space-y-3"
					>
						<TabsList className="grid w-full grid-cols-2 text-sm">
							<TabsTrigger value="response">{t("tabs.response")}</TabsTrigger>
							<TabsTrigger value="events">{t("tabs.events")}</TabsTrigger>
						</TabsList>
						<TabsContent
							value="response"
							className="space-y-2"
							onClick={handleCollapseFormClick}
						>
							<div className="flex items-center justify-between gap-2 text-sm mb-2">
								<Label>{t("tabs.response")}</Label>
							</div>
							<div className="group relative max-h-[40vh] overflow-auto rounded border border-slate-200 bg-white font-mono text-xs text-slate-700 dark:border-slate-800 dark:bg-slate-900/50 dark:text-slate-200">
								{result ? (
									<div className="pointer-events-none absolute top-0 right-0 z-10 flex w-full justify-end p-2">
										<ButtonGroup className="pointer-events-auto bg-white/95 backdrop-blur-sm opacity-0 shadow-sm transition-opacity group-hover:opacity-100 dark:bg-slate-900/95">
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
										</ButtonGroup>
									</div>
								) : null}
								<div className="p-3 whitespace-pre-wrap break-words">
									{result
										? (extractHumanText(result) ?? pretty(result))
										: t("response.placeholder")}
								</div>
							</div>
						</TabsContent>
						<TabsContent
							value="events"
							className="space-y-2"
							onClick={handleCollapseFormClick}
						>
							<div className="flex items-center justify-between">
								<Label>{t("events.title")}</Label>
							</div>
							{events.length === 0 ? (
								<div className="rounded border border-dashed border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-500 dark:border-slate-800 dark:bg-slate-900/40 dark:text-slate-300">
									{t("events.placeholder")}
								</div>
							) : (
								<ScrollArea className="max-h-[32vh]">
									<ul className="space-y-2">
										{events.map((entry, index) => {
											const label = formatEventLabel(entry, t);
											const detail = formatEventDetails(entry, t);
											const key = `${entry.data.event}-${entry.timestamp}-${index}`;
											return (
												<li
													key={key}
													className="rounded border border-slate-200 bg-white p-3 text-xs shadow-sm dark:border-slate-800 dark:bg-slate-900/50"
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
								</ScrollArea>
							)}
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
