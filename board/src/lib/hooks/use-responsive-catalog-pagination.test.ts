import { describe, expect, test } from "bun:test";

import {
	clampCatalogPage,
	getCatalogPageSize,
	getCatalogTotalPages,
	paginateCatalogItems,
} from "./use-responsive-catalog-pagination";

describe("responsive catalog pagination", () => {
	test("uses three grid rows at every responsive column count", () => {
		expect(getCatalogPageSize("grid", 1)).toBe(3);
		expect(getCatalogPageSize("grid", 2)).toBe(6);
		expect(getCatalogPageSize("grid", 3)).toBe(9);
	});

	test("uses six items in list view regardless of grid columns", () => {
		expect(getCatalogPageSize("list", 1)).toBe(6);
		expect(getCatalogPageSize("list", 3)).toBe(6);
	});

	test("calculates at least one page and rounds partial pages up", () => {
		expect(getCatalogTotalPages(0, 6)).toBe(1);
		expect(getCatalogTotalPages(7, 6)).toBe(2);
	});

	test("clamps invalid and stale page numbers", () => {
		expect(clampCatalogPage(0, 4)).toBe(1);
		expect(clampCatalogPage(8, 4)).toBe(4);
		expect(clampCatalogPage(2, 4)).toBe(2);
	});

	test("returns only the requested page slice", () => {
		expect(paginateCatalogItems([1, 2, 3, 4, 5, 6, 7], 2, 3)).toEqual([
			4,
			5,
			6,
		]);
	});
});
