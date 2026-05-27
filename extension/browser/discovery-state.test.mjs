import { describe, expect, test } from "bun:test";

import {
	buildDiscoveryUrl,
	discoveryPageState,
	discoveryQueryForPage,
	entriesForPageRender,
	nextDiscoveryPageState,
	shouldClearEntriesBeforeLoad,
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
			buildDiscoveryUrl("https://auth.mcp.umate.ai/discovery/servers", query),
		).toBe("https://auth.mcp.umate.ai/discovery/servers?limit=6&offset=12&surface=extension");
	});

	test("does not paginate portal requests", () => {
		expect(discoveryQueryForPage({ kind: "portals", limit: 6, offset: 0 })).toEqual({
			surface: "extension",
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
