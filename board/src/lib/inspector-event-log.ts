import type { InspectorMode } from "./inspector-capability";
import { INSPECTOR_EPHEMERAL_SERVER_ID } from "./inspector-ephemeral";
import type { InspectorSseEvent } from "./types";
import { smartFormat } from "./format";
import { formatTokenCount } from "./token-format";

export type InspectorLogTranslate = (
	key: string,
	options?: Record<string, unknown>,
) => string;

export type InspectorMcpListKind =
	| "tools"
	| "resources"
	| "prompts"
	| "templates"
	| "tasks";

export type InspectorLocalAiEvent =
	| {
			event: "ai_started";
			provider_name: string;
			model_id: string;
			tool_name: string;
	  }
	| {
			event: "ai_result";
			provider_name: string;
			model_id: string;
			tool_name: string;
			case_count: number;
			usage?: { total_tokens?: number | null } | null;
	  }
	| {
			event: "ai_error";
			tool_name: string;
			message: string;
	  };

export type InspectorProtocolEvent =
	| {
			event: "session_open";
			server_id: string;
			mode: InspectorMode;
	  }
	| {
			event: "session_close";
			session_id: string;
			server_id?: string;
	  }
	| {
			event: "mcp_list";
			list_kind: InspectorMcpListKind;
			server_id: string;
			refresh: string;
			mode: InspectorMode;
	  }
	| {
			event: "capability_detail";
			kind: InspectorMcpListKind;
			key: string;
			server_id: string;
	  }
	| {
			event: "tool_call_start";
			call_id: string;
			tool: string;
			server_id: string;
			mode: InspectorMode;
	  }
	| {
			event: "tool_call_cancel";
			call_id: string;
			server_id?: string;
			reason?: string;
	  }
	| {
			event: "prompt_get";
			name: string;
			server_id: string;
			mode: InspectorMode;
	  }
	| {
			event: "resource_read";
			uri: string;
			server_id: string;
			mode: InspectorMode;
	  }
	| {
			event: "ephemeral_invoke";
			server_id: string;
			operation: string;
			name: string;
			server_name?: string;
	  }
	| {
			event: "mcp_exchange";
			direction: "outbound" | "inbound";
			method: string;
			server_id: string;
			mode: InspectorMode;
			session_id?: string;
	  };

export type InspectorLogEvent =
	| InspectorSseEvent
	| InspectorLocalAiEvent
	| InspectorProtocolEvent;

export type InspectorLogEventEntry = {
	id: string;
	data: InspectorLogEvent;
	timestamp: number;
	request?: unknown;
	response?: unknown;
	durationMs?: number | null;
};

export type CreateInspectorLogEntryInput = {
	data: InspectorLogEvent;
	request?: unknown;
	response?: unknown;
	durationMs?: number | null;
	timestamp?: number;
	id?: string;
};

function createLogEntryId(): string {
	if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
		return crypto.randomUUID();
	}
	return `inspector-log-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export const MAX_INSPECTOR_STORED_EVENTS = 500;

function normalizeStoredEvent(entry: InspectorLogEventEntry): InspectorLogEventEntry {
	if (entry.id) {
		return entry;
	}
	return {
		...entry,
		id: createLogEntryId(),
	};
}

export function trimInspectorLogEvents(
	events: InspectorLogEventEntry[],
): InspectorLogEventEntry[] {
	if (events.length <= MAX_INSPECTOR_STORED_EVENTS) {
		return events;
	}
	return events.slice(events.length - MAX_INSPECTOR_STORED_EVENTS);
}

export function normalizeInspectorLogEvents(
	events: InspectorLogEventEntry[],
): InspectorLogEventEntry[] {
	return trimInspectorLogEvents(events.map(normalizeStoredEvent));
}

function readServerIdFromRequest(request: unknown): string | null {
	if (!request || typeof request !== "object") {
		return null;
	}
	const serverId = (request as { server_id?: unknown }).server_id;
	return typeof serverId === "string" ? serverId : null;
}

export function resolveInspectorEventServerId(
	entry: InspectorLogEventEntry,
): string | null {
	const { data } = entry;
	if ("server_id" in data && typeof data.server_id === "string") {
		return data.server_id;
	}
	if (data.event === "ephemeral_invoke") {
		return INSPECTOR_EPHEMERAL_SERVER_ID;
	}
	return readServerIdFromRequest(entry.request);
}

export function inspectorEventBelongsToServer(
	entry: InspectorLogEventEntry,
	serverId: string,
): boolean {
	const eventServerId = resolveInspectorEventServerId(entry);
	if (!eventServerId) {
		return false;
	}
	return eventServerId === serverId;
}

function readSessionIdFromRecord(record: unknown): string | null {
	if (!record || typeof record !== "object") {
		return null;
	}
	const sessionId = (record as { session_id?: unknown }).session_id;
	return typeof sessionId === "string" ? sessionId : null;
}

export function resolveInspectorEventSessionId(
	entry: InspectorLogEventEntry,
): string | null {
	const { data } = entry;
	if ("session_id" in data && typeof data.session_id === "string") {
		return data.session_id;
	}
	return readSessionIdFromRecord(entry.request) ?? readSessionIdFromRecord(entry.response);
}

export function inspectorEventBelongsToSession(
	entry: InspectorLogEventEntry,
	sessionId: string,
): boolean {
	const eventSessionId = resolveInspectorEventSessionId(entry);
	if (!eventSessionId) {
		return false;
	}
	return eventSessionId === sessionId;
}

export type InspectorActivityContextFilter =
	| { field: "server_id"; value: string }
	| { field: "session_id"; value: string };

export function inspectorEventMatchesContextFilter(
	entry: InspectorLogEventEntry,
	filter: InspectorActivityContextFilter | null,
): boolean {
	if (!filter) {
		return true;
	}
	if (filter.field === "server_id") {
		return inspectorEventBelongsToServer(entry, filter.value);
	}
	return inspectorEventBelongsToSession(entry, filter.value);
}

export function filterInspectorLogEventsForServer(
	events: InspectorLogEventEntry[],
	serverId: string,
): InspectorLogEventEntry[] {
	if (!serverId) {
		return [];
	}
	return events.filter((entry) => inspectorEventBelongsToServer(entry, serverId));
}

const STANDALONE_ACTIVITY_EVENTS = new Set<InspectorLogEvent["event"]>([
	"session_open",
	"session_close",
	"mcp_exchange",
	"tool_call_start",
	"prompt_get",
	"resource_read",
	"progress",
	"log",
	"result",
	"error",
	"cancelled",
]);

export function isInspectorStandaloneActivityLogEntry(
	entry: InspectorLogEventEntry,
): boolean {
	return STANDALONE_ACTIVITY_EVENTS.has(entry.data.event);
}

export function filterInspectorStandaloneActivityLogEvents(
	events: InspectorLogEventEntry[],
): InspectorLogEventEntry[] {
	return events.filter(isInspectorStandaloneActivityLogEntry);
}

export function createInspectorLogEntry(
	input: CreateInspectorLogEntryInput,
): InspectorLogEventEntry {
	return {
		id: input.id ?? createLogEntryId(),
		timestamp: input.timestamp ?? Date.now(),
		data: input.data,
		request: input.request,
		response: input.response,
		durationMs: input.durationMs ?? null,
	};
}

export function inspectorEventHasPayload(entry: InspectorLogEventEntry): boolean {
	return entry.request !== undefined || entry.response !== undefined;
}

export function formatInspectorEventAction(
	entry: InspectorLogEventEntry,
	t: InspectorLogTranslate,
): string {
	const { data } = entry;
	switch (data.event) {
		case "session_open":
			return t("inspector:eventLabels.sessionOpen", { defaultValue: "Session open" });
		case "session_close":
			return t("inspector:eventLabels.sessionClose", { defaultValue: "Session close" });
		case "mcp_list":
			return t("inspector:eventLabels.mcpList", {
				defaultValue: "{{kind}} list",
				kind: data.list_kind,
			});
		case "capability_detail":
			return t("inspector:eventLabels.capabilityDetail", {
				defaultValue: "Capability detail",
			});
		case "tool_call_start":
			return t("inspector:eventLabels.toolCallStart", { defaultValue: "Tool call start" });
		case "tool_call_cancel":
			return t("inspector:eventLabels.toolCallCancel", { defaultValue: "Tool call cancel" });
		case "prompt_get":
			return t("inspector:eventLabels.promptGet", { defaultValue: "Prompt get" });
		case "resource_read":
			return t("inspector:eventLabels.resourceRead", { defaultValue: "Resource read" });
		case "ephemeral_invoke":
			return t("inspector:eventLabels.ephemeralInvoke", { defaultValue: "Ephemeral invoke" });
		case "mcp_exchange": {
			const arrow = data.direction === "outbound" ? "→" : "←";
			return `${arrow} ${data.method}`;
		}
		case "started":
			return t("inspector:eventLabels.started", { defaultValue: "Started" });
		case "progress":
			return data.total
				? `${t("inspector:eventLabels.progress", { defaultValue: "Progress" })} ${data.progress}/${data.total}`
				: `${t("inspector:eventLabels.progress", { defaultValue: "Progress" })} ${data.progress}`;
		case "log":
			return data.logger || data.level || t("inspector:eventLabels.log", { defaultValue: "Log" });
		case "result":
			return t("inspector:eventLabels.result", { defaultValue: "Result" });
		case "error":
			return t("inspector:eventLabels.error", { defaultValue: "Error" });
		case "cancelled":
			return t("inspector:eventLabels.cancelled", { defaultValue: "Cancelled" });
		case "ai_started":
			return t("inspector:eventLabels.aiStarted", { defaultValue: "AI started" });
		case "ai_result":
			return t("inspector:eventLabels.aiResult", { defaultValue: "AI filled parameters" });
		case "ai_error":
			return t("inspector:eventLabels.aiError", { defaultValue: "AI failed" });
		default:
			return t("inspector:eventLabels.unknown", { defaultValue: "Unknown" });
	}
}

export function formatInspectorEventDetails(
	entry: InspectorLogEventEntry,
	t: InspectorLogTranslate,
): string | null {
	const { data } = entry;
	switch (data.event) {
		case "session_open":
			return t("inspector:eventDetails.sessionOpen", {
				defaultValue: "Server: {{serverId}} · Mode: {{mode}}",
				serverId: data.server_id,
				mode: data.mode,
			});
		case "session_close":
			return t("inspector:eventDetails.sessionClose", {
				defaultValue: "Session: {{sessionId}}",
				sessionId: data.session_id,
			});
		case "mcp_list":
			return t("inspector:eventDetails.mcpList", {
				defaultValue: "{{kind}} · refresh={{refresh}}",
				kind: data.list_kind,
				refresh: data.refresh,
			});
		case "capability_detail":
			return t("inspector:eventDetails.capabilityDetail", {
				defaultValue: "{{kind}} · {{key}}",
				kind: data.kind,
				key: data.key,
			});
		case "tool_call_start":
			return data.tool;
		case "tool_call_cancel":
			return data.reason ?? data.call_id;
		case "prompt_get":
			return data.name;
		case "resource_read":
			return data.uri;
		case "ephemeral_invoke":
			return `${data.operation} · ${data.name}`;
		case "mcp_exchange":
			return data.session_id
				? t("inspector:eventDetails.mcpExchange", {
						defaultValue: "Session: {{sessionId}}",
						sessionId: data.session_id,
					})
				: null;
		case "started":
			return t("inspector:eventDetails.session", {
				defaultValue: "Session: {{sessionId}}",
				sessionId: data.session_id ?? "n/a",
			});
		case "progress":
			return data.message ?? null;
		case "log":
			return smartFormat(data.data);
		case "result":
			return null;
		case "error":
			return data.message;
		case "cancelled":
			return data.reason ?? null;
		case "ai_started":
			return t("inspector:eventDetails.aiStarted", {
				defaultValue: "Provider: {{provider}}\nModel: {{model}}\nTool: {{tool}}",
				provider: data.provider_name,
				model: data.model_id,
				tool: data.tool_name,
			});
		case "ai_result":
			return t("inspector:eventDetails.aiResult", {
				defaultValue: "Generated {{count}} case for {{tool}}.",
				count: data.case_count,
				tool: data.tool_name,
			});
		case "ai_error":
			return data.message;
		default:
			return null;
	}
}

export function inspectorEventDurationMs(entry: InspectorLogEventEntry): number | null {
	if (entry.durationMs != null) {
		return entry.durationMs;
	}
	if (entry.data.event === "result") {
		return entry.data.elapsed_ms;
	}
	return null;
}

export function inspectorEventTarget(entry: InspectorLogEventEntry): string {
	const { data } = entry;
	switch (data.event) {
		case "session_open":
			return data.server_id;
		case "session_close":
			return data.session_id;
		case "mcp_list":
			return data.list_kind;
		case "capability_detail":
			return data.key;
		case "tool_call_start":
			return data.tool;
		case "tool_call_cancel":
			return data.call_id;
		case "prompt_get":
			return data.name;
		case "resource_read":
			return data.uri;
		case "ephemeral_invoke":
			return data.name;
		case "mcp_exchange":
			return data.method;
		case "ai_started":
		case "ai_result":
		case "ai_error":
			return data.tool_name;
		case "progress":
			return data.message ?? data.call_id;
		case "log":
			return data.logger ?? data.level ?? data.call_id;
		case "error":
			return data.message;
		case "cancelled":
			return data.reason ?? data.call_id;
		case "started":
		case "result":
			return data.call_id;
		default:
			return "—";
	}
}

export type InspectorEventCategoryKind = "platform" | "mcp" | "ai";

const PLATFORM_EVENTS = new Set<InspectorLogEvent["event"]>([
	"session_open",
	"session_close",
	"mcp_list",
	"capability_detail",
]);

const MCP_EVENTS = new Set<InspectorLogEvent["event"]>([
	"mcp_exchange",
	"tool_call_start",
	"tool_call_cancel",
	"prompt_get",
	"resource_read",
	"ephemeral_invoke",
	"started",
	"progress",
	"log",
	"result",
	"error",
	"cancelled",
]);

export function resolveInspectorEventCategoryKind(
	entry: InspectorLogEventEntry,
): InspectorEventCategoryKind {
	const { event } = entry.data;
	if (event.startsWith("ai_")) {
		return "ai";
	}
	if (PLATFORM_EVENTS.has(event)) {
		return "platform";
	}
	if (MCP_EVENTS.has(event)) {
		return "mcp";
	}
	return "mcp";
}

export function inspectorEventCategory(
	entry: InspectorLogEventEntry,
	t: InspectorLogTranslate,
): string {
	const kind = resolveInspectorEventCategoryKind(entry);
	switch (kind) {
		case "ai":
			return t("inspector:activity.category.ai", { defaultValue: "AI" });
		case "platform":
			return t("inspector:activity.category.platform", { defaultValue: "Platform" });
		case "mcp":
			return t("inspector:activity.category.mcp", { defaultValue: "MCP" });
	}
}

export type InspectorEventStatus = "success" | "failed" | "cancelled" | "active";

export function inspectorEventStatus(entry: InspectorLogEventEntry): InspectorEventStatus {
	switch (entry.data.event) {
		case "error":
		case "ai_error":
			return "failed";
		case "cancelled":
		case "tool_call_cancel":
			return "cancelled";
		case "result":
		case "ai_result":
		case "session_open":
		case "session_close":
		case "mcp_list":
		case "capability_detail":
		case "tool_call_start":
		case "prompt_get":
		case "resource_read":
		case "ephemeral_invoke":
			return "success";
		default:
			return "active";
	}
}

export function formatInspectorEventMeta(
	entry: InspectorLogEventEntry,
	_t: InspectorLogTranslate,
): string[] {
	const parts: string[] = [];
	if (entry.data.event === "ai_result" && entry.data.usage?.total_tokens != null) {
		parts.push(`${formatTokenCount(entry.data.usage.total_tokens)} tokens`);
	}
	return parts;
}

export function inspectorEventRowKey(entry: InspectorLogEventEntry, index: number): string {
	return entry.id || `${entry.data.event}-${entry.timestamp}-${index}`;
}

export function inspectorEventMatchesSearch(
	entry: InspectorLogEventEntry,
	keyword: string,
	t: InspectorLogTranslate,
): boolean {
	const normalized = keyword.trim().toLowerCase();
	if (!normalized) {
		return true;
	}

	const status = inspectorEventStatus(entry);
	const statusLabel =
		status === "active"
			? t("inspector:activity.status.active", { defaultValue: "Active" })
			: t(`audit:statusValues.${status}`, { defaultValue: status });

	const haystacks = [
		formatInspectorEventAction(entry, t),
		inspectorEventCategory(entry, t),
		statusLabel,
		inspectorEventTarget(entry),
		formatInspectorEventDetails(entry, t),
		entry.data.event,
		entry.id,
		entry.timestamp ? new Date(entry.timestamp).toLocaleString() : null,
		entry.durationMs != null ? String(entry.durationMs) : null,
		entry.data.event === "result" ? String(entry.data.elapsed_ms) : null,
		entry.request != null ? JSON.stringify(entry.request) : null,
		entry.response != null ? JSON.stringify(entry.response) : null,
	]
		.filter((value): value is string => Boolean(value))
		.map((value) => value.toLowerCase());

	return haystacks.some((value) => value.includes(normalized));
}
