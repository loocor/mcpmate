import { describe, expect, test } from "bun:test";

import { onboardingTranslations } from ".";

function collectStrings(value: unknown): string[] {
	if (typeof value === "string") return [value];
	if (!value || typeof value !== "object") return [];
	return Object.values(value).flatMap(collectStrings);
}

describe("onboarding translations", () => {
	test("does not expose Admin or recommendations as a user-facing source", () => {
		for (const locale of ["en", "zh-CN", "ja-JP"] as const) {
			const text = collectStrings(onboardingTranslations[locale]).join("\n");
			expect(text).not.toContain("MCPMate Admin");
			expect(text).not.toContain("Admin recommendation");
			expect(text).not.toContain("Admin 推荐");
			expect(text).not.toContain("Admin のおすすめ");
			expect(text.toLowerCase()).not.toContain("recommendation");
			expect(text).not.toContain("推荐");
			expect(text).not.toContain("おすすめ");
		}
	});
});
