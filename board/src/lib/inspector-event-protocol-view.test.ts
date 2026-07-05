import { describe, expect, it } from "bun:test";
import { createInspectorLogEntry } from "./inspector-event-log";
import { buildInspectorEventProtocolView } from "./inspector-event-protocol-view";

describe("inspector event protocol view", () => {
	it("maps tool call traffic to MCP JSON-RPC and isolates inspector context", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "result",
				call_id: "INSPCALLoUei9eTadTUk",
				server_id: "SERV9THgrQkrQGMO",
				elapsed_ms: 1459,
				result: {
					content: [{ type: "text", text: "ok" }],
				},
			},
			request: {
				tool: "context7_resolve-library-id",
				server_id: "SERV9THgrQkrQGMO",
				server_name: "Context7",
				mode: "native",
				arguments: { libraryName: "example", query: "example" },
				timeout_ms: 15000,
				session_id: "INSPSESIDsMsfVDAL13",
			},
			response: {
				event: "result",
				call_id: "INSPCALLoUei9eTadTUk",
				server_id: "SERV9THgrQkrQGMO",
				elapsed_ms: 1459,
				result: {
					content: [{ type: "text", text: "ok" }],
				},
			},
			durationMs: 1459,
		});

		const view = buildInspectorEventProtocolView(entry);
		expect(view).not.toBeNull();
		expect(view?.request).toEqual({
			jsonrpc: "2.0",
			method: "tools/call",
			params: {
				name: "context7_resolve-library-id",
				arguments: { libraryName: "example", query: "example" },
			},
		});
		expect(view?.response).toEqual({
			jsonrpc: "2.0",
			result: {
				content: [{ type: "text", text: "ok" }],
			},
		});
		expect(view?.context).toEqual({
			server_id: "SERV9THgrQkrQGMO",
			server_name: "Context7",
			mode: "native",
			session_id: "INSPSESIDsMsfVDAL13",
			timeout_ms: 15000,
			call_id: "INSPCALLoUei9eTadTUk",
			elapsed_ms: 1459,
		});
	});

	it("returns null for platform housekeeping events", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "mcp_list",
				list_kind: "tools",
				server_id: "srv-1",
				refresh: "cache",
				mode: "native",
			},
		});
		expect(buildInspectorEventProtocolView(entry)).toBeNull();
	});

	it("maps session handshake exchanges to MCP JSON-RPC payloads", () => {
		const initializeRequest = {
			jsonrpc: "2.0",
			id: 1,
			method: "initialize",
			params: {
				protocolVersion: "2025-03-26",
				capabilities: {},
				clientInfo: { name: "mcpmate-proxy", version: "0.1.0" },
			},
		};
		const entry = createInspectorLogEntry({
			data: {
				event: "mcp_exchange",
				direction: "outbound",
				method: "initialize",
				server_id: "srv-1",
				mode: "native",
				session_id: "sess-1",
			},
			request: initializeRequest,
		});

		const view = buildInspectorEventProtocolView(entry);
		expect(view?.request).toEqual(initializeRequest);
		expect(view?.context).toEqual({
			server_id: "srv-1",
			mode: "native",
			session_id: "sess-1",
		});
	});
});
