import { describe, expect, test } from "bun:test";

import {
	resolveImportDrawerOpen,
	shouldAcceptImportDrawerChange,
} from "./import-drawer-lifecycle";

describe("import drawer lifecycle", () => {
	test("keeps every interactive close path blocked while import is pending", () => {
		expect(shouldAcceptImportDrawerChange(false, true)).toBeFalse();
		expect(resolveImportDrawerOpen(false, true)).toBeTrue();
	});

	test("restores close behavior after import settles", () => {
		expect(shouldAcceptImportDrawerChange(false, false)).toBeTrue();
		expect(resolveImportDrawerOpen(false, false)).toBeFalse();
	});
});
