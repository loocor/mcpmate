import { describe, expect, it } from "bun:test";
import { createInspectorLogEntry } from "./inspector-event-log";
import {
	extractMcpProtocolEnvelopeBody,
	inferInspectorCapabilityKindFromEntry,
	mapInspectorMcpResponseModeToSegment,
	pickDefaultInspectorMcpResponseSegmentMode,
	pickDefaultInspectorMcpResponseViewMode,
	resolveActiveInspectorMcpResponseSegmentMode,
	resolveEffectiveInspectorMcpResponseViewMode,
	resolveInspectorMcpResponseViewModeForSegment,
} from "./inspector-mcp-response-view";

describe("inspector mcp response view", () => {
	it("extracts JSON-RPC result bodies from MCP envelopes", () => {
		expect(
			extractMcpProtocolEnvelopeBody({
				jsonrpc: "2.0",
				id: 1,
				result: { content: [{ type: "text", text: "ok" }] },
			}),
		).toEqual({ content: [{ type: "text", text: "ok" }] });
	});

	it("falls back to json for structured handshake responses", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: { protocolVersion: "2025-03-26" },
		};
		expect(pickDefaultInspectorMcpResponseViewMode(response, "tool")).toBe("json");
		expect(pickDefaultInspectorMcpResponseSegmentMode(response, "tool")).toBe("json");
		expect(resolveInspectorMcpResponseViewModeForSegment("outline", response, "tool")).toBe(
			"outline",
		);
		expect(resolveActiveInspectorMcpResponseSegmentMode(response, "tool", "outline")).toBe(
			"outline",
		);
	});

	it("maps rich markdown content to the preview segment", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: {
				content: [{ type: "text", text: "# Hello", mimeType: "text/markdown" }],
			},
		};
		expect(pickDefaultInspectorMcpResponseViewMode(response, "tool")).toBe("markdown");
		expect(mapInspectorMcpResponseModeToSegment("markdown")).toBe("preview");
		expect(pickDefaultInspectorMcpResponseSegmentMode(response, "tool")).toBe("preview");
		expect(resolveInspectorMcpResponseViewModeForSegment("preview", response, "tool")).toBe(
			"markdown",
		);
	});

	it("keeps preview segment active for plain text and image responses", () => {
		const textResponse = {
			jsonrpc: "2.0",
			id: 1,
			result: { content: [{ type: "text", text: "hello" }] },
		};
		expect(pickDefaultInspectorMcpResponseSegmentMode(textResponse, "tool")).toBe("preview");

		const imageResponse = {
			jsonrpc: "2.0",
			id: 1,
			result: { content: [{ type: "image", mimeType: "image/png", data: "abc123" }] },
		};
		expect(pickDefaultInspectorMcpResponseSegmentMode(imageResponse, "tool")).toBe("preview");
	});

	it("falls back preview segment selection to json when rich content is unavailable", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: { protocolVersion: "2025-03-26" },
		};
		expect(
			resolveActiveInspectorMcpResponseSegmentMode(response, "tool", "preview"),
		).toBe("json");
		expect(resolveEffectiveInspectorMcpResponseViewMode(response, "tool", "preview")).toBe(
			"json",
		);
	});

	it("falls back raw segment selection to json when raw text is unavailable", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: { protocolVersion: "2025-03-26" },
		};
		expect(resolveActiveInspectorMcpResponseSegmentMode(response, "tool", "raw")).toBe("json");
	});

	it("infers capability kind from event metadata", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "prompt_get",
				name: "summarize",
				server_id: "srv-1",
				mode: "native",
			},
		});
		expect(inferInspectorCapabilityKindFromEntry(entry)).toBe("prompt");
	});
});
