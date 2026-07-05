import { describe, expect, test } from "bun:test";
import { draftToServerConfig } from "./draft-to-server-config";

describe("draftToServerConfig", () => {
	test("preserves source for pending OAuth server creation and publish", () => {
		const config = draftToServerConfig(
			{
				name: "google-ads",
				kind: "streamable_http",
				url: "https://mcp.adramp.ai",
				source: { type: "registry", ref: "google-ads" },
			},
			{
				pending_import: true,
				enabled: false,
			},
		);

		expect(config).toEqual({
			name: "google-ads",
			kind: "streamable_http",
			command: undefined,
			url: "https://mcp.adramp.ai",
			args: undefined,
			env: undefined,
			headers: undefined,
			source: { type: "registry", ref: "google-ads" },
			meta: undefined,
			pending_import: true,
			enabled: false,
		});
	});

	test("sets url to undefined for non-stdio when draft url is absent", () => {
		const config = draftToServerConfig({
			name: "minimal-server",
			kind: "streamable_http",
		});

		expect(config.url).toBeUndefined();
		expect(config.command).toBeUndefined();
		expect(config.headers).toBeUndefined();
	});

	test("merges URL params into non-stdio server URL", () => {
		const config = draftToServerConfig({
			name: "http-server",
			kind: "streamable_http",
			url: "https://example.com/mcp?existing=1",
			urlParams: {
				token: "abc",
				existing: "2",
			},
		});

		expect(config.url).toBe("https://example.com/mcp?existing=2&token=abc");
	});

	test("preserves non-registry source without modification", () => {
		const config = draftToServerConfig({
			name: "admin-server",
			kind: "stdio",
			command: "npx",
			source: { type: "catalog", ref: "github" },
		});

		expect(config.source).toEqual({ type: "catalog", ref: "github" });
	});
});
