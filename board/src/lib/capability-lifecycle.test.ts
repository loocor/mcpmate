import { describe, expect, test } from "bun:test";

import {
	formatCapabilityLifecycle,
	resolveCapabilityLifecycle,
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

	test("never treats a zero count as unsupported", () => {
		expect(resolveCapabilityLifecycle("ready", kind({ currentCount: 0 }))).toBe(
			"empty",
		);
		expect(
			resolveCapabilityLifecycle(
				"ready",
				kind({ declaration: "unknown", currentCount: 0 }),
			),
		).toBe("unknown");
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
});
