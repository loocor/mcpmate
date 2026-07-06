import { describe, expect, it } from "bun:test";
import { buildInspectorJsonOutline } from "./inspector-response-preview";

describe("inspector response preview", () => {
	it("builds a flattened JSON outline with container and primitive summaries", () => {
		expect(
			buildInspectorJsonOutline({
				jsonrpc: "2.0",
				result: {
					content: [
						{ type: "text", text: "ok" },
						{ type: "image", data: "abc123" },
					],
				},
				id: 1,
			}),
		).toEqual([
			{ id: "$", depth: 0, label: "$", type: "object", summary: "3 keys" },
			{ id: "$.jsonrpc", depth: 1, label: "jsonrpc", type: "string", summary: '"2.0"' },
			{ id: "$.result", depth: 1, label: "result", type: "object", summary: "1 key" },
			{ id: "$.result.content", depth: 2, label: "content", type: "array", summary: "2 items" },
			{ id: "$.result.content[0]", depth: 3, label: "[0]", type: "object", summary: "2 keys" },
			{ id: "$.result.content[0].type", depth: 4, label: "type", type: "string", summary: '"text"' },
			{ id: "$.result.content[0].text", depth: 4, label: "text", type: "string", summary: '"ok"' },
			{ id: "$.result.content[1]", depth: 3, label: "[1]", type: "object", summary: "2 keys" },
			{ id: "$.result.content[1].type", depth: 4, label: "type", type: "string", summary: '"image"' },
			{ id: "$.result.content[1].data", depth: 4, label: "data", type: "string", summary: '"abc123"' },
			{ id: "$.id", depth: 1, label: "id", type: "number", summary: "1" },
		]);
	});

	it("caps outline rows with a truncation marker", () => {
		expect(
			buildInspectorJsonOutline(
				{
					items: [1, 2, 3, 4, 5],
				},
				{ maxRows: 4 },
			),
		).toEqual([
			{ id: "$", depth: 0, label: "$", type: "object", summary: "1 key" },
			{ id: "$.items", depth: 1, label: "items", type: "array", summary: "5 items" },
			{ id: "$.items[0]", depth: 2, label: "[0]", type: "number", summary: "1" },
			{
				id: "$.items.__maxRows",
				depth: 2,
				label: "...",
				type: "truncated",
				summary: "Additional entries hidden after max rows",
			},
		]);
	});

	it("caps outline depth with a truncation marker", () => {
		expect(
			buildInspectorJsonOutline(
				{
					a: {
						b: 1,
					},
				},
				{ maxDepth: 1 },
			),
		).toEqual([
			{ id: "$", depth: 0, label: "$", type: "object", summary: "1 key" },
			{ id: "$.a", depth: 1, label: "a", type: "object", summary: "1 key" },
			{
				id: "$.a.__maxDepth",
				depth: 2,
				label: "...",
				type: "truncated",
				summary: "Nested entries hidden after max depth",
			},
		]);
	});
});
