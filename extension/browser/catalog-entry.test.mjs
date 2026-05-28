import { describe, expect, test } from "bun:test";

import {
	clientConfigMeta,
	entryUrl,
	iconUrl,
} from "./catalog-entry.mjs";

const ADMIN_ORIGIN = "https://public.mcp.umate.ai";

describe("browser extension catalog entry helpers", () => {
	test("summarizes file client paths and container keys together", () => {
		expect(
			clientConfigMeta({
				config: {
					kind: "file",
					file: {
						paths: {
							macos: "~/Library/Application Support/Claude/claude_desktop_config.json",
							windows: "%APPDATA%\\Claude\\claude_desktop_config.json",
						},
						container: {
							keys: ["mcpServers"],
						},
					},
				},
			}),
		).toEqual({
			signal: "config.kind=file",
			meta: "paths: macos, windows; keys: mcpServers",
		});
	});

	test("rejects arbitrary third-party icon URLs", () => {
		expect(
			iconUrl(
				{
					icon: {
						url: "https://www.anthropic.com/favicon.ico",
					},
				},
				ADMIN_ORIGIN,
			),
		).toBe("");
	});

	test("accepts same-origin HTTPS Admin icon URLs", () => {
		expect(
			iconUrl(
				{
					icon: {
						url: "https://public.mcp.umate.ai/catalog/icons/claude.png",
					},
				},
				ADMIN_ORIGIN,
			),
		).toBe("https://public.mcp.umate.ai/catalog/icons/claude.png");
	});

	test("falls back when catalog links use unsafe schemes", () => {
		expect(
			entryUrl({
				links: {
					homepage: "javascript:alert(1)",
					docs: "chrome://extensions",
					support: "file:///tmp/catalog.html",
				},
				url: "javascript:alert(2)",
			}),
		).toBe("https://mcp.umate.ai");
	});
});
