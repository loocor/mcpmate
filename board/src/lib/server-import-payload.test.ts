import { describe, expect, test } from "bun:test";
import { buildMcpServersImportBodyFromDrafts } from "./server-import-payload";

describe("server import payload", () => {
	test("preserves remote headers in the shared import request payload", () => {
		const body = buildMcpServersImportBodyFromDrafts([
			{
				name: "remote-api",
				kind: "streamable_http",
				url: "https://api.example.com/mcp",
				headers: {
					Authorization: "",
				},
			},
		]);

		expect(body).toEqual({
			mcpServers: {
				"remote-api": {
					type: "streamable_http",
					url: "https://api.example.com/mcp",
					headers: {
						Authorization: "",
					},
				},
			},
		});
	});

	test("filters draft imports to the selected server names", () => {
		const body = buildMcpServersImportBodyFromDrafts(
			[
				{
					name: "alpha",
					kind: "stdio",
					command: "node",
					args: ["alpha.js"],
				},
				{
					name: "beta",
					kind: "streamable_http",
					url: "https://beta.example.com/mcp",
				},
			],
			new Set(["beta"]),
		);

		expect(body).toEqual({
			mcpServers: {
				beta: {
					type: "streamable_http",
					url: "https://beta.example.com/mcp",
				},
			},
		});
	});
});
