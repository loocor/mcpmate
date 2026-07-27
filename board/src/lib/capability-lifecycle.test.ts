import { describe, expect, test } from "bun:test";

import {
	formatCapabilityLifecycle,
	resolveCapabilityLifecycle,
	shouldDisplayCapabilityKind,
} from "./capability-lifecycle";
import type { CapabilityKindSummary, SnapshotState } from "./types";

const kind = (
	overrides: Partial<CapabilityKindSummary> = {},
): CapabilityKindSummary => ({
	declaration: "supported",
	inventory: "complete",
	currentCount: 1,
	currentAvailable: true,
	lastError: null,
	...overrides,
});

describe("resolveCapabilityLifecycle", () => {
	test("applies failed, unsupported, unknown, empty, ready precedence", () => {
		const cases: Array<
			[SnapshotState, CapabilityKindSummary, string]
		> = [
			["unavailable", kind({ declaration: "unsupported" }), "unavailable"],
			["ready", kind({ declaration: "unsupported", currentCount: 99 }), "unsupported"],
			["ready", kind({ declaration: "unknown", currentCount: 99 }), "unknown"],
			["ready", kind({ inventory: "unknown", currentCount: 99 }), "unknown"],
			["ready", kind({ currentCount: 0 }), "empty"],
			["ready", kind({ currentCount: 3 }), "ready"],
			["ready", kind({ inventory: "failed", currentCount: 3 }), "unavailable"],
		];

		for (const [snapshotState, summary, expected] of cases) {
			expect(resolveCapabilityLifecycle(snapshotState, summary)).toBe(expected);
		}
	});

	test("renders missing state as unknown without fabricating a zero count", () => {
		expect(
			formatCapabilityLifecycle(undefined, "tools", {
				unavailable: "Unavailable",
				unsupported: "Unsupported",
				unknown: "Unknown",
				empty: "Empty",
				ready: "Ready",
			}),
		).toBe("Unknown");
	});

	test("hides unsupported capability kinds from lifecycle summaries", () => {
		expect(
			shouldDisplayCapabilityKind(
				{
					snapshotState: "ready",
					revision: 1,
					observedAt: "2026-01-01T00:00:00Z",
					tools: kind({ currentCount: 1 }),
					prompts: kind({ declaration: "unsupported", currentCount: 0 }),
					resources: kind({ declaration: "unsupported", currentCount: 0 }),
					resourceTemplates: kind({
						declaration: "unsupported",
						currentCount: 0,
					}),
				},
				"prompts",
			),
		).toBe(false);
		expect(
			formatCapabilityLifecycle(
				{
					snapshotState: "ready",
					revision: 1,
					observedAt: "2026-01-01T00:00:00Z",
					tools: kind({ currentCount: 1 }),
					prompts: kind({ declaration: "unsupported", currentCount: 0 }),
					resources: kind({ declaration: "unsupported", currentCount: 0 }),
					resourceTemplates: kind({
						declaration: "unsupported",
						currentCount: 0,
					}),
				},
				"prompts",
				{
					unavailable: "Unavailable",
					unsupported: "Unsupported",
					unknown: "Unknown",
					empty: "Empty",
					ready: "Ready",
				},
			),
		).toBeNull();
	});
});
