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
			{
				id: "$",
				depth: 0,
				label: "$",
				type: "object",
				summary: "3 keys",
				summaryMeta: { kind: "keys", count: 3 },
				hasChildren: true,
			},
			{
				id: "$.jsonrpc",
				depth: 1,
				label: "jsonrpc",
				type: "string",
				summary: '"2.0"',
				hasChildren: false,
			},
			{
				id: "$.result",
				depth: 1,
				label: "result",
				type: "object",
				summary: "1 key",
				summaryMeta: { kind: "keys", count: 1 },
				hasChildren: true,
			},
			{
				id: "$.result.content",
				depth: 2,
				label: "content",
				type: "array",
				summary: "2 items",
				summaryMeta: { kind: "items", count: 2 },
				hasChildren: true,
			},
			{
				id: "$.result.content[0]",
				depth: 3,
				label: "[0]",
				type: "object",
				summary: "2 keys",
				summaryMeta: { kind: "keys", count: 2 },
				hasChildren: true,
			},
			{
				id: "$.result.content[0].type",
				depth: 4,
				label: "type",
				type: "string",
				summary: '"text"',
				hasChildren: false,
			},
			{
				id: "$.result.content[0].text",
				depth: 4,
				label: "text",
				type: "string",
				summary: '"ok"',
				hasChildren: false,
			},
			{
				id: "$.result.content[1]",
				depth: 3,
				label: "[1]",
				type: "object",
				summary: "2 keys",
				summaryMeta: { kind: "keys", count: 2 },
				hasChildren: true,
			},
			{
				id: "$.result.content[1].type",
				depth: 4,
				label: "type",
				type: "string",
				summary: '"image"',
				hasChildren: false,
			},
			{
				id: "$.result.content[1].data",
				depth: 4,
				label: "data",
				type: "string",
				summary: '"abc123"',
				hasChildren: false,
			},
			{
				id: "$.id",
				depth: 1,
				label: "id",
				type: "number",
				summary: "1",
				hasChildren: false,
			},
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
			{
				id: "$",
				depth: 0,
				label: "$",
				type: "object",
				summary: "1 key",
				summaryMeta: { kind: "keys", count: 1 },
				hasChildren: true,
			},
			{
				id: "$.items",
				depth: 1,
				label: "items",
				type: "array",
				summary: "5 items",
				summaryMeta: { kind: "items", count: 5 },
				hasChildren: true,
			},
			{
				id: "$.items[0]",
				depth: 2,
				label: "[0]",
				type: "number",
				summary: "1",
				hasChildren: false,
			},
			{
				id: "$.items.__maxRows",
				depth: 2,
				label: "...",
				type: "truncated",
				summary: "Additional entries hidden after max rows",
				summaryMeta: { kind: "truncatedRows" },
				hasChildren: false,
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
			{
				id: "$",
				depth: 0,
				label: "$",
				type: "object",
				summary: "1 key",
				summaryMeta: { kind: "keys", count: 1 },
				hasChildren: true,
			},
			{
				id: "$.a",
				depth: 1,
				label: "a",
				type: "object",
				summary: "1 key",
				summaryMeta: { kind: "keys", count: 1 },
				hasChildren: false,
			},
			{
				id: "$.a.__maxDepth",
				depth: 2,
				label: "...",
				type: "truncated",
				summary: "Nested entries hidden after max depth",
				summaryMeta: { kind: "truncatedDepth" },
				hasChildren: false,
			},
		]);
	});

	it("summarizes empty containers compactly", () => {
		expect(
			buildInspectorJsonOutline({
				emptyObject: {},
				emptyArray: [],
			}),
		).toEqual([
			{
				id: "$",
				depth: 0,
				label: "$",
				type: "object",
				summary: "2 keys",
				summaryMeta: { kind: "keys", count: 2 },
				hasChildren: true,
			},
			{
				id: "$.emptyObject",
				depth: 1,
				label: "emptyObject",
				type: "object",
				summary: "{}",
				summaryMeta: { kind: "emptyObject" },
				hasChildren: false,
			},
			{
				id: "$.emptyArray",
				depth: 1,
				label: "emptyArray",
				type: "array",
				summary: "[]",
				summaryMeta: { kind: "emptyArray" },
				hasChildren: false,
			},
		]);
	});
});
