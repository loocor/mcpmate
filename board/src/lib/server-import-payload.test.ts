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
});
