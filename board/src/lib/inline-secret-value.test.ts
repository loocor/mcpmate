import { describe, expect, it } from "vitest";
import {
	appendInlineText,
	backspaceInlineAtEnd,
	backspaceSecretBeforeTextBoundary,
	buildInlineDisplayItems,
	isFlexibleInlineTextSlot,
	insertInlineSecretAtTarget,
	insertInlineSecretPlaceholder,
	insertSecretPlaceholderIntoFieldValue,
	insertSecretIntoPlainValue,
	parseInlineSecretValue,
	prependInlineText,
	removeInlineSecretSegment,
	resolveFocusAfterAppendInlineText,
	resolveFocusAfterBackspaceAtTextBoundary,
	resolveFocusAfterPrependInlineText,
	resolveInlineFocusTargetAfterUpdate,
	serializeInlineSecretSegments,
	shouldUseInlineEditor,
	updateInlineSecretTextSegment,
} from "./inline-secret-value";

describe("inline-secret-value", () => {
	it("parses mixed text and multiple secret placeholders", () => {
		expect(
			parseInlineSecretValue("prefix [[secret:a]] middle [[secret:b]] suffix"),
		).toEqual([
			{ kind: "text", text: "prefix " },
			{ kind: "secret", alias: "a" },
			{ kind: "text", text: " middle " },
			{ kind: "secret", alias: "b" },
			{ kind: "text", text: " suffix" },
		]);
	});

	it("builds display items with prefix and trailing slots around secrets", () => {
		expect(buildInlineDisplayItems("[[secret:token]]")).toEqual([
			{ key: "prefix", kind: "prefix" },
			{ key: "secret-0", kind: "secret", storedIndex: 0, alias: "token" },
			{ key: "trailing", kind: "trailing" },
		]);
		expect(buildInlineDisplayItems("[[secret:token]]tail")).toEqual([
			{ key: "prefix", kind: "prefix" },
			{ key: "secret-0", kind: "secret", storedIndex: 0, alias: "token" },
			{ key: "text-1", kind: "text", storedIndex: 1, text: "tail" },
		]);
		expect(buildInlineDisplayItems("Bearer [[secret:token]]")).toEqual([
			{ key: "text-0", kind: "text", storedIndex: 0, text: "Bearer " },
			{ key: "secret-1", kind: "secret", storedIndex: 1, alias: "token" },
			{ key: "trailing", kind: "trailing" },
		]);
	});

	it("marks only the last text slot flexible when trailing is absent", () => {
		const mixed = buildInlineDisplayItems("ddd[[secret:token]]ddd");
		const leadingText = mixed.find((item) => item.key === "text-0");
		const trailingText = mixed.find((item) => item.key === "text-2");
		expect(leadingText).toBeDefined();
		expect(trailingText).toBeDefined();
		expect(isFlexibleInlineTextSlot(leadingText!, mixed)).toBe(false);
		expect(isFlexibleInlineTextSlot(trailingText!, mixed)).toBe(true);
	});

	it("keeps sandwiched text fixed-width when value ends with a secret", () => {
		const items = buildInlineDisplayItems("[[secret:a]]dd dd[[secret:b]]");
		const midText = items.find((item) => item.key === "text-1");
		const trailing = items.find((item) => item.kind === "trailing");
		expect(midText).toBeDefined();
		expect(trailing).toBeDefined();
		expect(isFlexibleInlineTextSlot(midText!, items)).toBe(false);
		expect(isFlexibleInlineTextSlot(trailing!, items)).toBe(true);
	});

	it("serializes segments back to stored value", () => {
		const value = serializeInlineSecretSegments([
			{ kind: "text", text: "Bearer " },
			{ kind: "secret", alias: "token" },
			{ kind: "text", text: " tail" },
		]);
		expect(value).toBe("Bearer [[secret:token]] tail");
	});

	it("uses inline editor for whole-secret and mixed values", () => {
		expect(shouldUseInlineEditor("[[secret:token]]")).toBe(true);
		expect(shouldUseInlineEditor("Bearer [[secret:token]] tail")).toBe(true);
		expect(shouldUseInlineEditor("***REDACTED***")).toBe(false);
	});

	it("prepends and appends text around secrets", () => {
		expect(prependInlineText("[[secret:token]]", "pre")).toBe(
			"pre[[secret:token]]",
		);
		expect(appendInlineText("[[secret:token]]", "!")).toBe(
			"[[secret:token]]!",
		);
		expect(appendInlineText("[[secret:token]]hi", "!")).toBe(
			"[[secret:token]]hi!",
		);
	});

	it("inserts secrets at cursor position inside text", () => {
		const inserted = insertInlineSecretAtTarget(
			"hello world",
			"[[secret:token]]",
			{ segmentIndex: 0, offset: 5 },
		);
		expect(inserted).toBe("hello[[secret:token]] world");
	});

	it("supports backspace boundaries around secrets", () => {
		expect(backspaceSecretBeforeTextBoundary("[[secret:token]]tail", 1)).toBe(
			"tail",
		);
		expect(backspaceInlineAtEnd("[[secret:token]]d")).toBe("[[secret:token]]");
		expect(backspaceInlineAtEnd("[[secret:token]]")).toBe("");
	});

	it("resolves focus targets after structural updates", () => {
		expect(resolveInlineFocusTargetAfterUpdate("[[secret:token]]")).toEqual({
			mode: "inline",
			inputKey: "trailing",
			caretOffset: 0,
		});
		expect(resolveInlineFocusTargetAfterUpdate("[[secret:token]]tail")).toEqual({
			mode: "inline",
			inputKey: "text-1",
			caretOffset: 4,
		});
		expect(
			resolveFocusAfterAppendInlineText("[[secret:token]]t"),
		).toEqual({
			mode: "inline",
			inputKey: "text-1",
			caretOffset: 1,
		});
		expect(resolveFocusAfterPrependInlineText("a[[secret:token]]")).toEqual({
			mode: "inline",
			inputKey: "text-0",
			caretOffset: 1,
		});
		expect(resolveFocusAfterBackspaceAtTextBoundary("tail", 1, "[[secret:token]]tail")).toEqual({
			mode: "plain",
			caretOffset: 0,
		});
		expect(
			resolveFocusAfterBackspaceAtTextBoundary(
				"hello world",
				2,
				"hello [[secret:token]] world",
			),
		).toEqual({
			mode: "plain",
			caretOffset: 6,
		});
	});

	it("inserts and edits inline secret placeholders", () => {
		expect(insertInlineSecretPlaceholder("", "[[secret:token]]")).toBe(
			"[[secret:token]]",
		);
		expect(
			insertInlineSecretPlaceholder("prefix ", "[[secret:token]]"),
		).toBe("prefix [[secret:token]]");
		expect(
			insertInlineSecretPlaceholder("[[secret:a]]", "[[secret:b]]"),
		).toBe("[[secret:a]][[secret:b]]");
		expect(
			insertInlineSecretPlaceholder("prefix ", "[[secret:a]]", {
				segmentIndex: 0,
				offset: 0,
			}),
		).toBe("[[secret:a]]prefix ");

		expect(appendInlineText("[[secret:token]]", " tail")).toBe(
			"[[secret:token]] tail",
		);

		expect(removeInlineSecretSegment("a [[secret:x]] b", 1)).toBe("a  b");
	});

	it("replaces redacted masks when picking a secret", () => {
		expect(
			insertSecretPlaceholderIntoFieldValue(
				"***REDACTED***",
				"[[secret:token]]",
			),
		).toBe("[[secret:token]]");
		expect(
			insertSecretPlaceholderIntoFieldValue(
				"***REDACTED***",
				"[[secret:token]]",
				{ headerKey: "authorization" },
			),
		).toBe("Bearer [[secret:token]]");
	});

	it("inserts into plain or inline field values without replacing existing text", () => {
		expect(
			insertSecretPlaceholderIntoFieldValue("npx", "[[secret:token]]"),
		).toBe("npx[[secret:token]]");
		expect(
			insertSecretPlaceholderIntoFieldValue("npx", "[[secret:token]]", {
				target: { segmentIndex: 0, offset: 3 },
			}),
		).toBe("npx[[secret:token]]");
		expect(
			insertSecretPlaceholderIntoFieldValue(
				"prefix [[secret:a]] suffix",
				"[[secret:b]]",
			),
		).toBe("prefix [[secret:a]] suffix[[secret:b]]");
	});

	it("preserves the stored value when updating an out-of-bounds text segment", () => {
		expect(
			updateInlineSecretTextSegment("[[secret:token]]", 9, "oops"),
		).toBe("[[secret:token]]");
	});

	it("inserts secrets into plain text values", () => {
		expect(insertSecretIntoPlainValue("", "[[secret:token]]")).toBe(
			"[[secret:token]]",
		);
		expect(insertSecretIntoPlainValue("hello", "[[secret:token]]")).toBe(
			"hello[[secret:token]]",
		);
		expect(
			insertSecretIntoPlainValue("hello world", "[[secret:token]]", {
				target: { segmentIndex: 0, offset: 5 },
			}),
		).toBe("hello[[secret:token]] world");
		expect(
			insertSecretIntoPlainValue("", "[[secret:token]]", {
				headerKey: "authorization",
			}),
		).toBe("Bearer [[secret:token]]");
	});
});
