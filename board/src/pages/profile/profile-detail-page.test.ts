import { describe, expect, test } from "bun:test";

import {
	resolveProfileDetailReviewTab,
	resolveProfileDetailTab,
} from "./profile-detail-tabs";

describe("resolveProfileDetailTab", () => {
	test("preserves a URL tab while the profile detail is loading", () => {
		expect(resolveProfileDetailTab("materials", undefined)).toBe("materials");
	});

	test("keeps workflow tabs and rejects capability-only tabs", () => {
		expect(resolveProfileDetailTab("materials", "workflow")).toBe("materials");
		expect(resolveProfileDetailTab("capabilities", "workflow")).toBe(
			"workflow",
		);
	});

	test("rejects workflow-only tabs for capability profiles", () => {
		expect(resolveProfileDetailTab("workflow", "capability")).toBe(
			"overview",
		);
		expect(resolveProfileDetailTab("materials", "capability")).toBe(
			"overview",
		);
	});

	test("keeps an explicit URL tab when a review item is present", () => {
		expect(resolveProfileDetailReviewTab("materials", true, true)).toBe(
			"materials",
		);
		expect(resolveProfileDetailReviewTab("overview", true, false)).toBe(
			"capabilities",
		);
	});
});
