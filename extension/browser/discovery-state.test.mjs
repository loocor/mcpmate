import { describe, expect, test } from "bun:test";

import {
	buildDiscoveryUrl,
	discoveryPageState,
	discoveryQueryForPage,
	entriesForPageRender,
	nextDiscoveryPageState,
	responseMetadata,
	shouldClearEntriesBeforeLoad,
	shouldRenderPanel,
	shouldStartPullRefresh,
} from "./discovery-state.mjs";

describe("browser extension discovery pagination", () => {
	test("builds account discovery page URLs for the extension surface", () => {
		const query = discoveryQueryForPage({ kind: "servers", limit: 6, offset: 12 });

		expect(query).toEqual({
			limit: 6,
			offset: 12,
			surface: "extension",
		});
		expect(
			buildDiscoveryUrl("https://public.mcp.umate.ai/discovery/servers", query),
		).toBe("https://public.mcp.umate.ai/discovery/servers?limit=6&offset=12&surface=extension");
	});

	test("does not paginate portal requests", () => {
		expect(discoveryQueryForPage({ kind: "portals", limit: 6, offset: 0 })).toEqual({
			surface: "extension",
		});
	});

	test("prefers v2 page metadata over legacy metadata fields", () => {
		expect(
			responseMetadata({
				page: { hasMore: true, nextOffset: 6, mode: "page" },
				metadata: { hasMore: false, nextOffset: null, mode: "page" },
				meta: { hasMore: false, nextOffset: null, mode: "page" },
			}),
		).toEqual({ hasMore: true, nextOffset: 6, mode: "page" });
	});

	test("uses v2 page metadata for paginated discovery responses", () => {
		const metadata = responseMetadata({
			clients: [{ identifier: "claude_desktop" }],
			page: {
				total: 11,
				limit: 1,
				offset: 1,
				hasMore: true,
				nextOffset: 2,
				mode: "page",
			},
		});
		const state = discoveryPageState({
			kind: "clients",
			entries: [{ identifier: "claude_desktop" }],
			metadata,
			limit: 1,
			offset: 1,
		});

		expect(state).toEqual({
			entries: [{ identifier: "claude_desktop" }],
			hasMore: true,
			nextOffset: 2,
		});
	});

	test("does not require legacy page mode when v2 page exposes next offset", () => {
		const state = discoveryPageState({
			kind: "clients",
			entries: [{ identifier: "cursor" }],
			metadata: responseMetadata({
				clients: [{ identifier: "cursor" }],
				page: { hasMore: true, nextOffset: 12 },
			}),
			limit: 6,
			offset: 6,
		});

		expect(state).toEqual({
			entries: [{ identifier: "cursor" }],
			hasMore: true,
			nextOffset: 12,
		});
	});

	test("tracks next page metadata for paginated discovery responses", () => {
		const state = discoveryPageState({
			kind: "clients",
			entries: [{ identifier: "claude_desktop" }],
			metadata: {
				total: 11,
				limit: 1,
				offset: 1,
				hasMore: true,
				nextOffset: 2,
				mode: "page",
			},
			limit: 1,
			offset: 1,
		});

		expect(state).toEqual({
			entries: [{ identifier: "claude_desktop" }],
			hasMore: true,
			nextOffset: 2,
		});
	});

	test("replaces entries on refresh and appends entries on next page", () => {
		const current = {
			entries: [{ identifier: "claude_desktop" }],
			hasMore: true,
			nextOffset: 1,
		};
		const nextPage = discoveryPageState({
			kind: "clients",
			entries: [{ identifier: "zed" }],
			metadata: { hasMore: false, nextOffset: null, mode: "page" },
			limit: 1,
			offset: 1,
		});
		const refreshed = discoveryPageState({
			kind: "clients",
			entries: [{ identifier: "cursor" }],
			metadata: { hasMore: true, nextOffset: 1, mode: "page" },
			limit: 1,
			offset: 0,
		});

		expect(nextDiscoveryPageState(current, nextPage, { reset: false })).toEqual({
			entries: [{ identifier: "claude_desktop" }, { identifier: "zed" }],
			hasMore: false,
			nextOffset: null,
		});
		expect(nextDiscoveryPageState(current, refreshed, { reset: true })).toEqual({
			entries: [{ identifier: "cursor" }],
			hasMore: true,
			nextOffset: 1,
		});
	});

	test("renders only new entries when adding a next page", () => {
		const current = {
			entries: [{ identifier: "claude_desktop" }],
			hasMore: true,
			nextOffset: 1,
		};
		const page = discoveryPageState({
			kind: "clients",
			entries: [{ identifier: "zed" }],
			metadata: { hasMore: false, nextOffset: null, mode: "page" },
			limit: 1,
			offset: 1,
		});
		const next = nextDiscoveryPageState(current, page, { reset: false });

		expect(entriesForPageRender(next, page, { reset: false })).toEqual([
			{ identifier: "zed" },
		]);
		expect(entriesForPageRender(next, page, { reset: true })).toEqual([
			{ identifier: "claude_desktop" },
			{ identifier: "zed" },
		]);
	});

	test("keeps existing entries visible while refreshing", () => {
		expect(
			shouldClearEntriesBeforeLoad(
				{ entries: [] },
				{ reset: true },
			),
		).toBe(true);
		expect(
			shouldClearEntriesBeforeLoad(
				{ entries: [{ identifier: "claude_desktop" }] },
				{ reset: true },
			),
		).toBe(false);
		expect(
			shouldClearEntriesBeforeLoad(
				{ entries: [{ identifier: "claude_desktop" }] },
				{ reset: false },
			),
		).toBe(false);
	});

	test("renders only unloaded active panels unless refresh bypasses cache", () => {
		expect(shouldRenderPanel({ panelName: "settings", loaded: false })).toBe(false);
		expect(shouldRenderPanel({ panelName: "clients", loaded: false })).toBe(true);
		expect(shouldRenderPanel({ panelName: "clients", loaded: true })).toBe(false);
		expect(
			shouldRenderPanel({ panelName: "clients", loaded: true, bypassCache: true }),
		).toBe(true);
	});

	test("starts pull refresh only for touch gestures at the top", () => {
		expect(
			shouldStartPullRefresh({
				button: 0,
				pointerType: "mouse",
				scrollTop: 0,
				panelName: "clients",
				selectionType: "None",
			}),
		).toBe(false);
		expect(
			shouldStartPullRefresh({
				button: 0,
				pointerType: "touch",
				scrollTop: 0,
				panelName: "clients",
				selectionType: "Range",
			}),
		).toBe(false);
		expect(
			shouldStartPullRefresh({
				button: 0,
				pointerType: "touch",
				scrollTop: 8,
				panelName: "clients",
				selectionType: "None",
			}),
		).toBe(false);
		expect(
			shouldStartPullRefresh({
				button: 0,
				pointerType: "touch",
				scrollTop: 0,
				panelName: "clients",
				selectionType: "None",
			}),
		).toBe(true);
	});
});
