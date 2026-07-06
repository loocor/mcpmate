import { describe, expect, it } from "bun:test";
import { createInspectorLogEntry } from "./inspector-event-log";
import {
	extractMcpProtocolEnvelopeBody,
	inferInspectorCapabilityKindFromEntry,
	mapInspectorMcpResponseModeToSegment,
	pickDefaultInspectorMcpResponseSegmentMode,
	pickDefaultInspectorMcpResponseViewMode,
	pickDefaultInspectorPayloadSegmentMode,
	resolveActiveInspectorMcpResponseSegmentMode,
	resolveAvailablePayloadSegments,
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

	it("defaults structured handshake responses to outline and json segments only", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: { protocolVersion: "2025-03-26" },
		};
		expect(resolveAvailablePayloadSegments(response, "tool")).toEqual(["outline", "json"]);
		expect(pickDefaultInspectorMcpResponseViewMode(response, "tool")).toBe("outline");
		expect(pickDefaultInspectorMcpResponseSegmentMode(response, "tool")).toBe("outline");
		expect(resolveInspectorMcpResponseViewModeForSegment("preview", response, "tool")).toBe(
			"preview",
		);
		expect(resolveActiveInspectorMcpResponseSegmentMode(response, "tool", "outline")).toBe(
			"outline",
		);
	});

	it("exposes preview, outline, json, and raw for text tool results", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: { content: [{ type: "text", text: "hello" }] },
		};
		expect(resolveAvailablePayloadSegments(response, "tool")).toEqual([
			"preview",
			"outline",
			"json",
			"raw",
		]);
		expect(pickDefaultInspectorPayloadSegmentMode(response, "tool")).toBe("preview");
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
		expect(resolveAvailablePayloadSegments(imageResponse, "tool")).toEqual([
			"preview",
			"outline",
			"json",
		]);
		expect(pickDefaultInspectorMcpResponseSegmentMode(imageResponse, "tool")).toBe("preview");
	});

	it("coerces unavailable raw selection back to the default segment", () => {
		const response = {
			jsonrpc: "2.0",
			id: 1,
			result: { protocolVersion: "2025-03-26" },
		};
		expect(resolveActiveInspectorMcpResponseSegmentMode(response, "tool", "raw")).toBe(
			"outline",
		);
		expect(resolveEffectiveInspectorMcpResponseViewMode(response, "tool", "raw")).toBe(
			"outline",
		);
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
