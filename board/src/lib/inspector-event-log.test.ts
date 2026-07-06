import { describe, expect, it } from "bun:test";
import {
	createInspectorLogEntry,
	filterInspectorLogEventsForServer,
	filterInspectorStandaloneActivityLogEvents,
	formatInspectorEventAction,
	inspectorEventBelongsToServer,
	inspectorEventCategory,
	inspectorEventHasPayload,
	inspectorEventMatchesContextFilter,
	inspectorEventRowKey,
	resolveInspectorEventCategoryKind,
	resolveInspectorEventServerId,
	resolveInspectorEventSessionId,
} from "./inspector-event-log";
import { INSPECTOR_EPHEMERAL_SERVER_ID } from "./inspector-ephemeral";

describe("createInspectorLogEntry", () => {
	it("stores request and response payloads", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "session_open",
				server_id: "srv-1",
				mode: "native",
			},
			request: { mode: "native", server_id: "srv-1" },
			response: { session_id: "sess-1" },
		});
		expect(entry.id).toBeTruthy();
		expect(entry.request).toEqual({ mode: "native", server_id: "srv-1" });
		expect(entry.response).toEqual({ session_id: "sess-1" });
		expect(inspectorEventHasPayload(entry)).toBe(true);
	});
});

describe("inspectorEventRowKey", () => {
	it("prefers stable entry ids", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "mcp_list",
				list_kind: "tools",
				server_id: "srv-1",
				refresh: "cache",
				mode: "native",
			},
		});
		expect(inspectorEventRowKey(entry, 0)).toBe(entry.id);
	});
});

describe("formatInspectorEventAction", () => {
	it("labels protocol events", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "tool_call_start",
				call_id: "call-1",
				tool: "echo",
				server_id: "srv-1",
				mode: "native",
			},
		});
		expect(
			formatInspectorEventAction(entry, (_key, options) => String(options?.defaultValue)),
		).toBe("Tool call start");
	});
});

describe("inspector event server scoping", () => {
	it("resolves server_id from event data", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "mcp_list",
				list_kind: "tools",
				server_id: "server-a",
				refresh: "cache",
				mode: "native",
			},
		});
		expect(resolveInspectorEventServerId(entry)).toBe("server-a");
		expect(inspectorEventBelongsToServer(entry, "server-a")).toBe(true);
		expect(inspectorEventBelongsToServer(entry, "server-b")).toBe(false);
	});

	it("resolves server_id from request when data omits it", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "ai_started",
				provider_name: "openai",
				model_id: "gpt-4",
				tool_name: "search",
			},
			request: {
				server_id: "server-a",
				tool_name: "search",
			},
		});
		expect(inspectorEventBelongsToServer(entry, "server-a")).toBe(true);
		expect(inspectorEventBelongsToServer(entry, "server-b")).toBe(false);
	});

	it("maps ephemeral invoke events to the probe server id", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "ephemeral_invoke",
				server_id: INSPECTOR_EPHEMERAL_SERVER_ID,
				operation: "tool",
				name: "search",
			},
		});
		expect(inspectorEventBelongsToServer(entry, INSPECTOR_EPHEMERAL_SERVER_ID)).toBe(true);
		expect(inspectorEventBelongsToServer(entry, "server-a")).toBe(false);
	});

	it("filters mixed event lists by server", () => {
		const serverA = createInspectorLogEntry({
			data: {
				event: "session_open",
				server_id: "server-a",
				mode: "native",
			},
		});
		const serverB = createInspectorLogEntry({
			data: {
				event: "session_open",
				server_id: "server-b",
				mode: "native",
			},
		});
		expect(filterInspectorLogEventsForServer([serverA, serverB], "server-a")).toEqual([serverA]);
	});
});

describe("inspector event categories", () => {
	const t = (_key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? "";

	it("marks platform housekeeping separately from mcp protocol traffic", () => {
		const platformList = createInspectorLogEntry({
			data: {
				event: "mcp_list",
				list_kind: "tools",
				server_id: "srv-1",
				refresh: "cache",
				mode: "native",
			},
		});
		const mcpToolCall = createInspectorLogEntry({
			data: {
				event: "tool_call_start",
				call_id: "call-1",
				tool: "echo",
				server_id: "srv-1",
				mode: "native",
			},
		});
		const mcpResult = createInspectorLogEntry({
			data: {
				event: "result",
				call_id: "call-1",
				result: { ok: true },
			},
		});

		expect(resolveInspectorEventCategoryKind(platformList)).toBe("platform");
		expect(resolveInspectorEventCategoryKind(mcpToolCall)).toBe("mcp");
		expect(resolveInspectorEventCategoryKind(mcpResult)).toBe("mcp");
		expect(inspectorEventCategory(platformList, t)).toBe("Platform");
		expect(inspectorEventCategory(mcpToolCall, t)).toBe("MCP");
	});
});

describe("filterInspectorStandaloneActivityLogEvents", () => {
	it("keeps session lifecycle entries and MCP protocol traffic", () => {
		const sessionOpen = createInspectorLogEntry({
			data: {
				event: "session_open",
				server_id: "srv-1",
				mode: "native",
			},
		});
		const uiSelection = createInspectorLogEntry({
			data: {
				event: "capability_detail",
				kind: "tools",
				key: "echo",
				server_id: "srv-1",
			},
		});
		const sessionClose = createInspectorLogEntry({
			data: {
				event: "session_close",
				session_id: "sess-1",
				server_id: "srv-1",
			},
		});
		const mcpRequest = createInspectorLogEntry({
			data: {
				event: "mcp_exchange",
				direction: "outbound",
				method: "tools/list",
				server_id: "srv-1",
				mode: "native",
			},
			request: { jsonrpc: "2.0", method: "tools/list" },
		});
		const mcpResult = createInspectorLogEntry({
			data: {
				event: "result",
				call_id: "call-1",
				server_id: "srv-1",
				elapsed_ms: 12,
				result: { tools: [] },
			},
		});
		const aiEvent = createInspectorLogEntry({
			data: {
				event: "ai_started",
				provider_name: "OpenAI",
				model_id: "gpt-5",
				tool_name: "echo",
			},
		});

		expect(
			filterInspectorStandaloneActivityLogEvents([
				sessionOpen,
				uiSelection,
				sessionClose,
				mcpRequest,
				mcpResult,
				aiEvent,
			]),
		).toEqual([sessionOpen, sessionClose, mcpRequest, mcpResult]);
	});
});

describe("inspector activity context filters", () => {
	it("resolves session ids from event data and payloads", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "mcp_exchange",
				direction: "outbound",
				method: "initialize",
				server_id: "srv-1",
				mode: "native",
				session_id: "sess-abc",
			},
		});
		expect(resolveInspectorEventSessionId(entry)).toBe("sess-abc");
	});

	it("filters activity rows by server id and session id", () => {
		const matching = createInspectorLogEntry({
			data: {
				event: "mcp_exchange",
				direction: "outbound",
				method: "tools/list",
				server_id: "scratch:everything",
				mode: "native",
				session_id: "INSPSESdVcDZK4iYjZi",
			},
		});
		const otherSession = createInspectorLogEntry({
			data: {
				event: "mcp_exchange",
				direction: "outbound",
				method: "tools/list",
				server_id: "scratch:everything",
				mode: "native",
				session_id: "OTHER",
			},
		});

		expect(
			inspectorEventMatchesContextFilter(matching, {
				field: "server_id",
				value: "scratch:everything",
			}),
		).toBe(true);
		expect(
			inspectorEventMatchesContextFilter(otherSession, {
				field: "session_id",
				value: "INSPSESdVcDZK4iYjZi",
			}),
		).toBe(false);
		expect(
			inspectorEventMatchesContextFilter(matching, {
				field: "session_id",
				value: "INSPSESdVcDZK4iYjZi",
			}),
		).toBe(true);
	});
});
