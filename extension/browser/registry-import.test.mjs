import { describe, expect, test } from "bun:test";

import {
	convertRegistryToMcpMate,
	normalizeRegistryTransport,
} from "./registry-import.mjs";

describe("registry import conversion", () => {
	test("normalizes registry transport aliases", () => {
		expect(normalizeRegistryTransport("stdio")).toBe("stdio");
		expect(normalizeRegistryTransport("server-sent-events")).toBe("sse");
		expect(normalizeRegistryTransport("streamable-http")).toBe(
			"streamable_http",
		);
		expect(normalizeRegistryTransport("http")).toBe("streamable_http");
		expect(normalizeRegistryTransport("", "streamable_http")).toBe(
			"streamable_http",
		);
		expect(normalizeRegistryTransport("websocket", "streamable_http")).toBeNull();
	});

	test("emits explicit stdio type for package imports", () => {
		expect(
			JSON.parse(
				convertRegistryToMcpMate({
					name: "time",
					packages: [{ identifier: "mcp-server-time", registryType: "pypi" }],
				}),
			),
		).toEqual({
			mcpServers: {
				"mcp-server-time": {
					type: "stdio",
					command: "uvx",
					args: ["mcp-server-time"],
				},
			},
		});
	});

	test("selects highest recommended transport for same registry identifier", () => {
		expect(
			JSON.parse(
				convertRegistryToMcpMate({
					name: "remote-api",
					packages: [{ identifier: "remote-api", registryType: "npm" }],
					remotes: [
						{
							identifier: "remote-api",
							type: "sse",
							url: "https://example.com/sse",
						},
						{
							identifier: "remote-api",
							type: "streamable-http",
							url: "https://example.com/mcp",
						},
					],
				}),
			),
		).toEqual({
			mcpServers: {
				"remote-api": {
					type: "streamable_http",
					url: "https://example.com/mcp",
				},
			},
		});
	});

	test("keeps lower ranked transport when it has a distinct identifier", () => {
		expect(
			JSON.parse(
				convertRegistryToMcpMate({
					name: "remote-api",
					remotes: [
						{
							identifier: "remote-api-sse",
							type: "sse",
							url: "https://example.com/sse",
						},
						{
							identifier: "remote-api-http",
							type: "streamable_http",
							url: "https://example.com/mcp",
						},
					],
				}),
			),
		).toEqual({
			mcpServers: {
				"remote-api-sse": {
					type: "sse",
					url: "https://example.com/sse",
				},
				"remote-api-http": {
					type: "streamable_http",
					url: "https://example.com/mcp",
				},
			},
		});
	});

	test("uses URL path as last resort transport hint", () => {
		expect(
			JSON.parse(
				convertRegistryToMcpMate({
					name: "remote-api",
					remotes: [
						{
							identifier: "remote-api-sse",
							url: "https://example.com/api/sse",
						},
						{
							identifier: "remote-api-http",
							url: "https://example.com/api/mcp",
						},
					],
				}),
			),
		).toEqual({
			mcpServers: {
				"remote-api-sse": {
					type: "sse",
					url: "https://example.com/api/sse",
				},
				"remote-api-http": {
					type: "streamable_http",
					url: "https://example.com/api/mcp",
				},
			},
		});
	});
});
