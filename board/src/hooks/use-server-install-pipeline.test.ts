import { describe, expect, test } from "bun:test";

import { buildPreviewPayload } from "./use-server-install-pipeline";

describe("buildPreviewPayload", () => {
	test("builds preview items with tagged candidate transports", () => {
		expect(
			buildPreviewPayload([
				{
					name: "stdio-secret",
					serverId: "server-stdio",
					kind: "stdio",
					command: "example-server",
					args: ["--serve"],
					env: {
						API_KEY: "[[secret:stdio-token]]",
						LOG_LEVEL: "debug",
					},
				},
				{
					name: "sse-secret",
					serverId: "server-sse",
					kind: "sse",
					url: "https://example.com/sse",
					headers: {
						Authorization: "[[secret:sse-token]]",
						"X-Client": "board",
					},
				},
				{
					name: "streamable-literal",
					kind: "streamable_http",
					url: "https://example.com/mcp?existing=one",
					urlParams: { next: "two" },
					headers: {
						Authorization: "Bearer [[secret:not-an-alias]]",
					},
				},
			]),
		).toEqual({
			include_details: true,
			servers: [
				{
					name: "stdio-secret",
					server_id: "server-stdio",
					transport: {
						kind: "stdio",
						command: "example-server",
						args: ["--serve"],
						env: {
							API_KEY: { kind: "secret_ref", alias: "stdio-token" },
							LOG_LEVEL: { kind: "literal", value: "debug" },
						},
					},
				},
				{
					name: "sse-secret",
					server_id: "server-sse",
					transport: {
						kind: "http",
						protocol: "sse",
						endpoint: "https://example.com/sse",
						headers: {
							Authorization: { kind: "secret_ref", alias: "sse-token" },
							"X-Client": { kind: "literal", value: "board" },
						},
					},
				},
				{
					name: "streamable-literal",
					transport: {
						kind: "http",
						protocol: "streamable_http",
						endpoint: "https://example.com/mcp?existing=one&next=two",
						headers: {
							Authorization: {
								kind: "literal",
								value: "Bearer [[secret:not-an-alias]]",
							},
						},
					},
				},
			],
		});
	});
});
