import { describe, expect, test } from "bun:test";

import {
	clientCatalogMeta,
	entryUrl,
	iconUrl,
} from "./catalog-entry.mjs";

const ADMIN_ORIGIN = "https://public.mcp.umate.ai";

describe("browser extension catalog entry helpers", () => {
	test("summarizes client catalog category without config internals", () => {
		expect(
			clientCatalogMeta({
				tags: ["application"],
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
			signal: "application",
			meta: "",
		});
	});

	test("accepts HTTPS catalog icon URLs from Admin values", () => {
		expect(
			iconUrl(
				{
					icon: {
						url: "https://www.anthropic.com/favicon.ico",
					},
				},
				ADMIN_ORIGIN,
			),
		).toBe("https://www.anthropic.com/favicon.ico");
	});

	test("accepts portal iconUrl values", () => {
		expect(
			iconUrl(
				{
					iconUrl: "https://composio.dev/toolkits/graphics/composio_ogImage.png",
				},
				ADMIN_ORIGIN,
			),
		).toBe("https://composio.dev/toolkits/graphics/composio_ogImage.png");
	});

	test("accepts base64 raster image icons from Admin values", () => {
		const icon =
			"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMB/axjYioAAAAASUVORK5CYII=";
		expect(
			iconUrl(
				{
					icon: {
						url: icon,
					},
				},
				ADMIN_ORIGIN,
			),
		).toBe(icon);
	});

	test("rejects non-HTTPS icon URLs", () => {
		expect(
			iconUrl(
				{
					iconUrl: "http://example.com/icon.png",
				},
				ADMIN_ORIGIN,
			),
		).toBe("");
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
