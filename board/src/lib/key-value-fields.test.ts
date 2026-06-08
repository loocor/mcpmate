import { describe, expect, it } from "vitest";
import {
	compactKeyValueFields,
	isBlankKeyValuePair,
	shouldAppendKeyValueRow,
} from "./key-value-fields";

describe("key-value-fields", () => {
	it("detects blank rows", () => {
		expect(isBlankKeyValuePair({ key: "", value: "" })).toBe(true);
		expect(isBlankKeyValuePair({ key: "A", value: "" })).toBe(false);
	});

	it("blocks duplicate ghost appends when trailing row is blank", () => {
		expect(shouldAppendKeyValueRow([])).toBe(true);
		expect(
			shouldAppendKeyValueRow([{ key: "A", value: "1" }]),
		).toBe(true);
		expect(
			shouldAppendKeyValueRow([{ key: "", value: "" }]),
		).toBe(false);
	});

	it("compacts duplicate blank rows", () => {
		expect(
			compactKeyValueFields([
				{ key: "", value: "" },
				{ key: "A", value: "1" },
				{ key: "", value: "" },
			]),
		).toEqual([{ key: "A", value: "1" }]);
	});
});
