import { describe, expect, test } from "bun:test";

import {
	parseCursorMcpInstallLink,
	splitMergedStdioCommand,
} from "./cursor-deeplink.mjs";

describe("cursor deeplink parsing", () => {
	test("splits merged stdio command when args are missing", () => {
		expect(splitMergedStdioCommand("npx -y mcp-mermaid", undefined)).toEqual({
			command: "npx",
			args: ["-y", "mcp-mermaid"],
		});
	});

	test("preserves separate command and args", () => {
		expect(
			splitMergedStdioCommand("npx", ["-y", "@phantom/mcp-server"]),
		).toEqual({
			command: "npx",
			args: ["-y", "@phantom/mcp-server"],
		});
	});

	test("leaves single-token command unchanged", () => {
		expect(splitMergedStdioCommand("node", undefined)).toEqual({
			command: "node",
			args: undefined,
		});
	});

	test("splits merged command when args is an empty array", () => {
		expect(splitMergedStdioCommand("npx -y mcp-mermaid", [])).toEqual({
			command: "npx",
			args: ["-y", "mcp-mermaid"],
		});
	});

	test("parses cursor.directory mcp-mermaid install link", () => {
		const config = btoa(JSON.stringify({ command: "npx -y mcp-mermaid" }));
		const href = `cursor://anysphere.cursor-deeplink/mcp/install?name=mcp-mermaid&config=${config}`;
		expect(parseCursorMcpInstallLink(href)).toBe(
			JSON.stringify({
				mcpServers: {
					"mcp-mermaid": {
						type: "stdio",
						command: "npx",
						args: ["-y", "mcp-mermaid"],
					},
				},
			}),
		);
	});

	test("parses cursor.directory phantom install link without merging env", () => {
		const config = btoa(
			JSON.stringify({
				command: "npx",
				args: ["-y", "@phantom/mcp-server"],
				env: { PHANTOM_APP_ID: "your-phantom-app-id" },
			}),
		);
		const href = `cursor://anysphere.cursor-deeplink/mcp/install?name=phantom-mcp-server&config=${config}`;
		expect(parseCursorMcpInstallLink(href)).toBe(
			JSON.stringify({
				mcpServers: {
					"phantom-mcp-server": {
						type: "stdio",
						command: "npx",
						args: ["-y", "@phantom/mcp-server"],
						env: { PHANTOM_APP_ID: "your-phantom-app-id" },
					},
				},
			}),
		);
	});

	test("parses cursor.directory remote install links with explicit transport", () => {
		const config = btoa(
			JSON.stringify({
				url: "https://example.com/mcp",
				transport: "streamable-http",
			}),
		);
		const href = `cursor://anysphere.cursor-deeplink/mcp/install?name=remote-api&config=${config}`;
		expect(parseCursorMcpInstallLink(href)).toBe(
			JSON.stringify({
				mcpServers: {
					"remote-api": {
						url: "https://example.com/mcp",
						type: "streamable_http",
					},
				},
			}),
		);
	});

	test("uses remote URL path as a last resort transport hint", () => {
		const config = btoa(
			JSON.stringify({
				url: "https://example.com/events/sse",
			}),
		);
		const href = `cursor://anysphere.cursor-deeplink/mcp/install?name=remote-api&config=${config}`;
		expect(parseCursorMcpInstallLink(href)).toBe(
			JSON.stringify({
				mcpServers: {
					"remote-api": {
						url: "https://example.com/events/sse",
						type: "sse",
					},
				},
			}),
		);
	});
});
