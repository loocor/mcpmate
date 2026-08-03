import { describe, expect, test } from "bun:test";

import {
	getDefaultMarketPageSize,
	getMarketPageSizeOptions,
	parseMarketListPerPageParam,
	readMarketListSelectedPageSize,
	snapMarketPageSize,
} from "./market-list-pagination-storage";

describe("market list pagination storage", () => {
	test("page size options are multiples of the responsive grid column count", () => {
		expect(getMarketPageSizeOptions(1)).toEqual([3, 9, 18, 24]);
		expect(getMarketPageSizeOptions(2)).toEqual([6, 18, 36, 48]);
		expect(getMarketPageSizeOptions(3)).toEqual([9, 27, 54, 72]);
	});

	test("default page size matches three grid rows at the current column count", () => {
		expect(getDefaultMarketPageSize(1)).toBe(3);
		expect(getDefaultMarketPageSize(2)).toBe(6);
		expect(getDefaultMarketPageSize(3)).toBe(9);
	});

	test("parseMarketListPerPageParam snaps invalid values to the responsive default", () => {
		expect(parseMarketListPerPageParam("9", 2)).toBe(6);
		expect(parseMarketListPerPageParam("27", 3)).toBe(27);
		expect(parseMarketListPerPageParam(null, 2)).toBe(6);
	});

	test("snapMarketPageSize picks the nearest valid option", () => {
		expect(snapMarketPageSize(9, 2)).toBe(6);
		expect(snapMarketPageSize(27, 2)).toBe(18);
		expect(snapMarketPageSize(72, 3)).toBe(72);
	});

	test("readMarketListSelectedPageSize returns null for the responsive default", () => {
		expect(readMarketListSelectedPageSize(null, 2)).toBe(null);
		expect(readMarketListSelectedPageSize("18", 2)).toBe(18);
		expect(readMarketListSelectedPageSize("6", 2)).toBe(null);
	});
});
