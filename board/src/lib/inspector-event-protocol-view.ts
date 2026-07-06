import {
	resolveInspectorEventCategoryKind,
	type InspectorLogEvent,
	type InspectorLogEventEntry,
} from "./inspector-event-log";

export type InspectorEventProtocolView = {
	request?: unknown;
	response?: unknown;
	notification?: unknown;
	context: Record<string, unknown>;
};

export const INSPECTOR_CONTEXT_KEYS = [
	"server_id",
	"server_name",
	"mode",
	"session_id",
	"timeout_ms",
	"call_id",
	"elapsed_ms",
] as const;

export type InspectorContextKey = (typeof INSPECTOR_CONTEXT_KEYS)[number];

function asRecord(value: unknown): Record<string, unknown> | null {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return null;
	}
	return value as Record<string, unknown>;
}

function pickContextFields(
	...sources: Array<Record<string, unknown> | null | undefined>
): Record<string, unknown> {
	const context: Record<string, unknown> = {};
	for (const source of sources) {
		if (!source) {
			continue;
		}
		for (const key of INSPECTOR_CONTEXT_KEYS) {
			if (context[key] !== undefined) {
				continue;
			}
			const value = source[key];
			if (value !== undefined && value !== null && value !== "") {
				context[key] = value;
			}
		}
	}
	return context;
}

function inspectorApiRequestToMcpRequest(request: unknown): unknown | undefined {
	const record = asRecord(request);
	if (!record) {
		return undefined;
	}

	if (typeof record.tool === "string") {
		return {
			jsonrpc: "2.0",
			method: "tools/call",
			params: {
				name: record.tool,
				arguments: record.arguments ?? {},
			},
		};
	}

	if (typeof record.name === "string" && record.uri === undefined) {
		return {
			jsonrpc: "2.0",
			method: "prompts/get",
			params: {
				name: record.name,
				arguments: record.arguments ?? {},
			},
		};
	}

	if (typeof record.uri === "string") {
		return {
			jsonrpc: "2.0",
			method: "resources/read",
			params: {
				uri: record.uri,
			},
		};
	}

	const operation = record.operation;
	const name = record.name;
	if (operation === "tool" && typeof name === "string") {
		return {
			jsonrpc: "2.0",
			method: "tools/call",
			params: {
				name,
				arguments: record.arguments ?? {},
			},
		};
	}
	if (operation === "prompt" && typeof name === "string") {
		return {
			jsonrpc: "2.0",
			method: "prompts/get",
			params: {
				name,
				arguments: record.arguments ?? {},
			},
		};
	}
	if (operation === "resource" && typeof name === "string") {
		return {
			jsonrpc: "2.0",
			method: "resources/read",
			params: {
				uri: name,
			},
		};
	}

	return undefined;
}

function unwrapApiPayload(response: unknown): unknown | undefined {
	const record = asRecord(response);
	if (!record) {
		return undefined;
	}
	const data = asRecord(record.data);
	if (data && "result" in data) {
		return data.result;
	}
	if ("result" in record) {
		return record.result;
	}
	return undefined;
}

function terminalEventToMcpResponse(data: InspectorLogEvent): unknown | undefined {
	switch (data.event) {
		case "result":
			return {
				jsonrpc: "2.0",
				result: data.result,
			};
		case "error":
			return {
				jsonrpc: "2.0",
				error: {
					message: data.message,
				},
			};
		case "cancelled":
			return {
				jsonrpc: "2.0",
				error: {
					code: -32800,
					message: data.reason ?? "Cancelled",
				},
			};
		default:
			return undefined;
	}
}

function sseEventToMcpNotification(data: InspectorLogEvent): unknown | undefined {
	switch (data.event) {
		case "progress":
			return {
				jsonrpc: "2.0",
				method: "notifications/progress",
				params: {
					progressToken: data.call_id,
					progress: data.progress,
					total: data.total,
					message: data.message,
				},
			};
		case "log":
			return {
				jsonrpc: "2.0",
				method: "notifications/message",
				params: {
					level: data.level,
					logger: data.logger,
					data: data.data,
				},
			};
		default:
			return undefined;
	}
}

function protocolEventToMcpRequest(data: InspectorLogEvent): unknown | undefined {
	switch (data.event) {
		case "tool_call_start":
			return {
				jsonrpc: "2.0",
				method: "tools/call",
				params: {
					name: data.tool,
				},
			};
		case "prompt_get":
			return {
				jsonrpc: "2.0",
				method: "prompts/get",
				params: {
					name: data.name,
				},
			};
		case "resource_read":
			return {
				jsonrpc: "2.0",
				method: "resources/read",
				params: {
					uri: data.uri,
				},
			};
		case "ephemeral_invoke":
			return {
				jsonrpc: "2.0",
				method:
					data.operation === "prompt"
						? "prompts/get"
						: data.operation === "resource"
							? "resources/read"
							: "tools/call",
				params: {
					name: data.name,
				},
			};
		default:
			return undefined;
	}
}

function protocolEventToMcpResponse(
	entry: InspectorLogEventEntry,
	data: InspectorLogEvent,
): unknown | undefined {
	const terminal = terminalEventToMcpResponse(data);
	if (terminal) {
		return terminal;
	}
	const fromApi = unwrapApiPayload(entry.response);
	if (fromApi !== undefined) {
		return {
			jsonrpc: "2.0",
			result: fromApi,
		};
	}
	return undefined;
}

function mcpExchangeToProtocolView(
	entry: InspectorLogEventEntry,
	data: Extract<InspectorLogEvent, { event: "mcp_exchange" }>,
): Pick<InspectorEventProtocolView, "request" | "response" | "notification"> {
	if (data.direction === "outbound") {
		if (data.method.startsWith("notifications/")) {
			return { notification: entry.request };
		}
		return { request: entry.request };
	}
	return { response: entry.response };
}

export function buildInspectorEventProtocolView(
	entry: InspectorLogEventEntry,
): InspectorEventProtocolView | null {
	if (resolveInspectorEventCategoryKind(entry) !== "mcp") {
		return null;
	}

	const data = entry.data;
	if (data.event === "mcp_exchange") {
		const dataRecord = asRecord(data);
		const context = pickContextFields(dataRecord, {
			server_id: data.server_id,
			mode: data.mode,
			session_id: data.session_id,
		});
		return {
			...mcpExchangeToProtocolView(entry, data),
			context,
		};
	}

	const dataRecord = asRecord(data);
	const requestRecord = asRecord(entry.request);
	const responseRecord = asRecord(entry.response);

	const context = pickContextFields(
		dataRecord,
		requestRecord,
		responseRecord,
		entry.durationMs != null ? { elapsed_ms: entry.durationMs } : null,
	);

	const request =
		inspectorApiRequestToMcpRequest(entry.request) ?? protocolEventToMcpRequest(data);
	const response = protocolEventToMcpResponse(entry, data);
	const notification = sseEventToMcpNotification(data);

	return {
		request,
		response,
		notification,
		context,
	};
}

export function serializeInspectorEventEntryForDisplay(
	entry: InspectorLogEventEntry,
): string {
	const protocolView = buildInspectorEventProtocolView(entry);
	if (!protocolView) {
		return JSON.stringify(entry, null, 2);
	}
	return JSON.stringify(
		{
			mcp: {
				request: protocolView.request,
				response: protocolView.response,
				notification: protocolView.notification,
			},
			inspector_context: protocolView.context,
		},
		null,
		2,
	);
}
