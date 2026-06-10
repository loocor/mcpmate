import { describe, expect, test } from "bun:test";
import { shouldBlockDesktopDropNavigation } from "./desktop-drop-guard";

function dataTransferWithTypes(types: string[]): DataTransfer {
	return {
		types: {
			length: types.length,
			contains: (value: string) => types.includes(value),
		},
	} as unknown as DataTransfer;
}

function targetWithClosest(isEditable: boolean): EventTarget {
	return {
		closest: () => (isEditable ? {} : null),
	} as unknown as EventTarget;
}

describe("desktop drop guard", () => {
	test("blocks dropped files so the WebView cannot navigate to local content", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["Files"]),
				targetWithClosest(true),
			),
		).toBe(true);
	});

	test("blocks dropped text outside editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/plain"]),
				targetWithClosest(false),
			),
		).toBe(true);
	});

	test("allows dropped text in editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/plain"]),
				targetWithClosest(true),
			),
		).toBe(false);
	});

	test("blocks unrelated drag payloads outside editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["application/x-custom"]),
				targetWithClosest(false),
			),
		).toBe(true);
	});

	test("allows unrelated drag payloads in editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["application/x-custom"]),
				targetWithClosest(true),
			),
		).toBe(false);
	});
});
