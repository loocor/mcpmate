import { describe, expect, test } from "bun:test";

import {
	HANDOFF_TTL_MS,
	buildHandoffPageUrl,
	buildMcpMateImportUrl,
	consumeHandoffRecord,
	createHandoffRecord,
	handoffStorageKey,
	isFreshHandoffRecord,
	readHandoffRecord,
	removeHandoffRecord,
	writeHandoffRecord,
} from "./import-handoff.mjs";

describe("import handoff helpers", () => {
	function createMockChromeStorage() {
		const data = {};
		return {
			chromeLike: {
				storage: {
					local: {
						async get(key) {
							return { [key]: data[key] };
						},
						async set(items) {
							Object.assign(data, items);
						},
						async remove(key) {
							delete data[key];
						},
					},
				},
			},
		};
	}

	test("encodes MCPMate import URLs with URL-safe payloads", () => {
		const url = buildMcpMateImportUrl({
			text: JSON.stringify({
				mcpServers: {
					time: { command: "uvx", args: ["mcp-server-time"] },
				},
			}),
			format: "json",
			source: { type: "browser" },
		});

		expect(url.startsWith("mcpmate://import/server?p=")).toBe(true);
		const encoded = url.slice("mcpmate://import/server?p=".length);
		expect(encoded).not.toContain("+");
		expect(encoded).not.toContain("/");
		expect(encoded).not.toContain("=");
	});

	test("creates namespaced storage keys", () => {
		expect(handoffStorageKey("abc")).toBe("mcpmate.importHandoff.abc");
	});

	test("marks records stale after ttl", () => {
		const record = createHandoffRecord({ text: "payload" }, 1000);

		expect(isFreshHandoffRecord(record, 1000 + HANDOFF_TTL_MS - 1)).toBe(
			true,
		);
		expect(isFreshHandoffRecord(record, 1000 + HANDOFF_TTL_MS + 1)).toBe(
			false,
		);
	});

	test("rejects malformed or empty records", () => {
		expect(isFreshHandoffRecord(null, 1000)).toBe(false);
		expect(isFreshHandoffRecord({ createdAt: 1000 }, 1000)).toBe(false);
		expect(
			isFreshHandoffRecord({ payload: { text: "" }, createdAt: 1000 }, 1000),
		).toBe(false);
	});

	test("builds extension handoff page URLs", () => {
		const runtime = { getURL: (path) => `chrome-extension://id/${path}` };

		expect(buildHandoffPageUrl("abc", runtime)).toBe(
			"chrome-extension://id/handoff.html?id=abc",
		);
		expect(buildHandoffPageUrl("a b+c", runtime)).toBe(
			"chrome-extension://id/handoff.html?id=a%20b%2Bc",
		);
	});

	test("reads, writes, and removes records through extension local storage", async () => {
		const { chromeLike } = createMockChromeStorage();
		const record = createHandoffRecord({ text: "payload" }, 1000);

		await writeHandoffRecord("abc", record, chromeLike);
		expect(await readHandoffRecord("abc", chromeLike)).toEqual(record);

		await removeHandoffRecord("abc", chromeLike);
		expect(await readHandoffRecord("abc", chromeLike)).toBeNull();
	});

	test("consumes fresh records and removes them from extension storage", async () => {
		const { chromeLike } = createMockChromeStorage();
		const record = createHandoffRecord({ text: "payload" }, 1000);

		await writeHandoffRecord("fresh", record, chromeLike);

		expect(await consumeHandoffRecord("fresh", chromeLike, 1000)).toEqual(
			record,
		);
		expect(await readHandoffRecord("fresh", chromeLike)).toBeNull();
	});

	test("removes stale records when consuming them", async () => {
		const { chromeLike } = createMockChromeStorage();
		const record = createHandoffRecord({ text: "payload" }, 1000);

		await writeHandoffRecord("stale", record, chromeLike);

		expect(
			await consumeHandoffRecord(
				"stale",
				chromeLike,
				1000 + HANDOFF_TTL_MS + 1,
			),
		).toBeNull();
		expect(await readHandoffRecord("stale", chromeLike)).toBeNull();
	});
});
