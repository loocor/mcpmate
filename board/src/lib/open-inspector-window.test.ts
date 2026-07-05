import { describe, expect, it } from "vitest";
import {
	buildInspectorWindowUrl,
	shouldOpenInspectorInSameTab,
} from "./open-inspector-window";

describe("openInspectorWindow helpers", () => {
	it("builds absolute inspector URLs from relative paths", () => {
		const originalWindow = globalThis.window;
		globalThis.window = {
			location: { origin: "http://localhost:5173" },
		} as Window & typeof globalThis.window;

		try {
			expect(buildInspectorWindowUrl("/inspector")).toBe(
				"http://localhost:5173/inspector",
			);
			expect(buildInspectorWindowUrl("/inspector?server_id=abc")).toBe(
				"http://localhost:5173/inspector?server_id=abc",
			);
		} finally {
			globalThis.window = originalWindow;
		}
	});

	it("allows modified clicks to use default browser navigation", () => {
		expect(
			shouldOpenInspectorInSameTab({
				metaKey: true,
				ctrlKey: false,
				shiftKey: false,
				altKey: false,
				button: 0,
			}),
		).toBe(true);
		expect(
			shouldOpenInspectorInSameTab({
				metaKey: false,
				ctrlKey: false,
				shiftKey: false,
				altKey: false,
				button: 1,
			}),
		).toBe(true);
	});
});
