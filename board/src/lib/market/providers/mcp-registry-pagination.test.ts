import { describe, expect, it } from "bun:test";
import type { CatalogEntry } from "../types";
import {
	resolveCatalogPageNextCursor,
	upstreamPageHasUnseenEntries,
} from "./mcp-registry-pagination";

function entry(name: string): CatalogEntry {
	return { name, version: "1.0.0" };
}

describe("resolveCatalogPageNextCursor", () => {
	it("clears next cursor for partial pages", () => {
		expect(
			resolveCatalogPageNextCursor({
				entries: [entry("alpha")],
				cappedLimit: 9,
				upstreamNextCursor: "cursor-2",
				hasMoreUniqueEntriesAhead: true,
			}),
		).toBeUndefined();
	});

	it("clears next cursor for empty pages", () => {
		expect(
			resolveCatalogPageNextCursor({
				entries: [],
				cappedLimit: 9,
				upstreamNextCursor: "cursor-2",
				hasMoreUniqueEntriesAhead: true,
			}),
		).toBeUndefined();
	});

	it("keeps next cursor only when a full page has unseen entries ahead", () => {
		expect(
			resolveCatalogPageNextCursor({
				entries: Array.from({ length: 9 }, (_, index) => entry(`server-${index}`)),
				cappedLimit: 9,
				upstreamNextCursor: "cursor-2",
				hasMoreUniqueEntriesAhead: true,
			}),
		).toBe("cursor-2");
	});

	it("drops upstream next cursor when peek finds no unseen entries", () => {
		expect(
			resolveCatalogPageNextCursor({
				entries: Array.from({ length: 9 }, (_, index) => entry(`server-${index}`)),
				cappedLimit: 9,
				upstreamNextCursor: "cursor-2",
				hasMoreUniqueEntriesAhead: false,
			}),
		).toBeUndefined();
	});
});

describe("upstreamPageHasUnseenEntries", () => {
	it("returns true when upstream page contains a new canonical id", () => {
		const seen = new Set(["alpha"]);
		expect(
			upstreamPageHasUnseenEntries(
				{
					servers: [{ server: entry("beta") }],
				},
				seen,
			),
		).toBe(true);
	});

	it("returns false when upstream page only repeats seen ids", () => {
		const seen = new Set(["alpha", "beta"]);
		expect(
			upstreamPageHasUnseenEntries(
				{
					servers: [{ server: entry("alpha") }, { server: entry("beta") }],
				},
				seen,
			),
		).toBe(false);
	});
});
