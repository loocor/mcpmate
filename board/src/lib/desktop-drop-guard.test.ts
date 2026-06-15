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

function targetWithClosest({
	isEditable = false,
	isDesktopDropTarget = false,
	isOtherDesktopDropTarget = false,
}: {
	isEditable?: boolean;
	isDesktopDropTarget?: boolean;
	isOtherDesktopDropTarget?: boolean;
}): EventTarget {
	return {
		closest: (selector: string) => {
			if (isEditable && selector.includes("input")) {
				return {};
			}
			if (
				isDesktopDropTarget &&
				selector === "[data-desktop-drop-target='server-import']"
			) {
				return {};
			}
			if (
				isOtherDesktopDropTarget &&
				selector === "[data-desktop-drop-target='other']"
			) {
				return {};
			}
			return null;
		},
	} as unknown as EventTarget;
}

describe("desktop drop guard", () => {
	test("blocks dropped files so the WebView cannot navigate to local content", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["Files"]),
				targetWithClosest({ isEditable: true }),
			),
		).toBe(true);
	});

	test("allows dropped files on explicit desktop drop targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["Files"]),
				targetWithClosest({ isDesktopDropTarget: true }),
			),
		).toBe(false);
	});

	test("blocks dropped text outside editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/plain"]),
				targetWithClosest({}),
			),
		).toBe(true);
	});

	test("allows dropped text in editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/plain"]),
				targetWithClosest({ isEditable: true }),
			),
		).toBe(false);
	});

	test("allows dropped text on explicit desktop drop targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/plain"]),
				targetWithClosest({ isDesktopDropTarget: true }),
			),
		).toBe(false);
	});

	test("allows dropped URI lists on explicit desktop drop targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/uri-list"]),
				targetWithClosest({ isDesktopDropTarget: true }),
			),
		).toBe(false);
	});

	test("blocks dropped text on unrelated desktop drop targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["text/plain"]),
				targetWithClosest({ isOtherDesktopDropTarget: true }),
			),
		).toBe(true);
	});

	test("blocks unrelated drag payloads outside editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["application/x-custom"]),
				targetWithClosest({}),
			),
		).toBe(true);
	});

	test("allows unrelated drag payloads in editable targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["application/x-custom"]),
				targetWithClosest({ isEditable: true }),
			),
		).toBe(false);
	});

	test("blocks unrelated drag payloads on explicit desktop drop targets", () => {
		expect(
			shouldBlockDesktopDropNavigation(
				dataTransferWithTypes(["application/x-custom"]),
				targetWithClosest({ isDesktopDropTarget: true }),
			),
		).toBe(true);
	});
});
