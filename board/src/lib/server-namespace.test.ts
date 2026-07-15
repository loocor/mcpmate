import { describe, expect, test } from "bun:test";

import {
	isCanonicalServerNamespace,
	namespaceInputIsReadOnly,
	serverNamespaceImportPreview,
	suggestServerNamespace,
} from "./server-namespace";

describe("server namespace", () => {
	test("suggests a visible canonical value for safe transformations", () => {
		expect(suggestServerNamespace("  Sequential Thinking-v2  ")).toBe(
			"sequential_thinking_v2",
		);
		expect(suggestServerNamespace("PaddleOCR-VL-1.6")).toBe(
			"paddleocr_vl_1_6",
		);
	});

	test("accepts digits after the first letter", () => {
		expect(isCanonicalServerNamespace("sequential_thinking_v2")).toBe(true);
		expect(isCanonicalServerNamespace("server_7zip")).toBe(true);
	});

	test("rejects non-canonical and unsafe values", () => {
		for (const value of [
			"SequentialThinking",
			"sequential-thinking",
			"sequential thinking",
			"7zip",
			"server__name",
			"server_",
			"server.name",
			"序列思考",
			"",
		]) {
			expect(isCanonicalServerNamespace(value)).toBe(false);
		}
	});

	test("rejects namespaces longer than 64 characters", () => {
		expect(isCanonicalServerNamespace("a".repeat(64))).toBe(true);
		expect(isCanonicalServerNamespace("a".repeat(65))).toBe(false);
	});

	test("does not invent unsafe suggestions", () => {
		expect(suggestServerNamespace("123")).toBeNull();
		expect(suggestServerNamespace("序列思考")).toBeNull();
		expect(suggestServerNamespace("server\u00a0name")).toBeNull();
	});

	test("locks the namespace after creation or OAuth draft creation", () => {
		expect(namespaceInputIsReadOnly("create", false)).toBe(false);
		expect(namespaceInputIsReadOnly("market", false)).toBe(false);
		expect(namespaceInputIsReadOnly("edit", false)).toBe(true);
		expect(namespaceInputIsReadOnly("edit", false, true)).toBe(false);
		expect(namespaceInputIsReadOnly("create", true)).toBe(true);
	});

	test("preserves the original import label for namespace preview", () => {
		expect(
			serverNamespaceImportPreview(
				"Sequential Thinking-v2",
				"sequential_thinking_v2",
			),
		).toEqual({
			original: "Sequential Thinking-v2",
			namespace: "sequential_thinking_v2",
		});
		expect(
			serverNamespaceImportPreview("sequential_thinking_v2", "sequential_thinking_v2"),
		).toBeNull();
	});
});
