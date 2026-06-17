import { describe, expect, test } from "bun:test";

import {
	DISCOVERY_CACHE_TTL_MS,
	discoveryCacheKey,
	discoverySessionKey,
	isCacheEntryFresh,
} from "./discovery-cache.mjs";

describe("discovery cache helpers", () => {
	test("builds stable discovery cache keys", () => {
		const context = { mode: "account", origin: "https://public.mcp.umate.ai" };
		const url = "https://public.mcp.umate.ai/discovery/servers?surface=extension&limit=6&offset=0";

		expect(discoveryCacheKey({ ...context, kind: "servers", requestUrl: url })).toBe(
			`mcpmate.discovery.cache.account.${context.origin}.servers.${encodeURIComponent(url)}`,
		);
	});

	test("builds locale-scoped session keys", () => {
		const context = { mode: "account", origin: "https://public.mcp.umate.ai" };
		expect(discoverySessionKey({ ...context, locale: "en", kind: "servers" })).toBe(
			"mcpmate.discovery.session.account.https://public.mcp.umate.ai.en.servers",
		);
	});

	test("treats fresh cache entries as valid within ttl", () => {
		expect(isCacheEntryFresh({ cachedAt: Date.now() })).toBe(true);
		expect(
			isCacheEntryFresh({ cachedAt: Date.now() - DISCOVERY_CACHE_TTL_MS - 1 }),
		).toBe(false);
	});
});
