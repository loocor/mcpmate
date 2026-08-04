import { describe, expect, it } from "vitest";

import {
	getPageToolbarSelectLabel,
	type PageToolbarSelectOption,
} from "./page-toolbar-select";

describe("getPageToolbarSelectLabel", () => {
	const options: PageToolbarSelectOption[] = [
		{ value: "name", label: "Name" },
		{ value: "needs_review", label: "Needs review" },
	];

	it("uses the selected option label for width mirroring", () => {
		expect(getPageToolbarSelectLabel("name", options)).toBe("Name");
		expect(getPageToolbarSelectLabel("needs_review", options)).toBe("Needs review");
	});

	it("falls back to placeholder when value is missing", () => {
		expect(getPageToolbarSelectLabel("missing", options, "Filter")).toBe("Filter");
	});
});
