import { describe, expect, test } from "bun:test";

import { buildInfoSource, formatBuildDate } from "./write-build-info.mjs";

describe("browser extension build info", () => {
	test("formats build dates with the configured time zone", () => {
		const date = new Date("2026-05-22T04:37:09.000Z");

		expect(formatBuildDate(date, "Asia/Singapore")).toBe("20260522123709");
	});

	test("generates a static global build info script", () => {
		expect(buildInfoSource("20260522123709")).toBe(
			'globalThis.MCPMATE_EXTENSION_BUILD = Object.freeze({\n\tbuildDate: "20260522123709",\n});\n',
		);
	});
});
