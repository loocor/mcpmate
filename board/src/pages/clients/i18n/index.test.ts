import { describe, expect, test } from "bun:test";
import type { TFunction } from "i18next";

import { createClientFormSchema } from "../../../components/client-form-schema";
import { clientsTranslations } from ".";

describe("clients translations", () => {
	test("defines preset catalog diagnostics copy for all supported locales", () => {
		for (const locale of ["en", "zh-CN", "ja-JP"] as const) {
			expect(
				clientsTranslations[locale].detail.form.adminCatalog.partialWarning,
			).toBeTruthy();
		}
	});

	test("does not expose Admin as a user-facing catalog source", () => {
		for (const locale of ["en", "zh-CN", "ja-JP"] as const) {
			const copy = Object.values(clientsTranslations[locale].detail.form.adminCatalog).join("\n");
			expect(copy).not.toContain("Admin");
			expect(copy.toLowerCase()).not.toContain("recommendation");
			expect(copy).not.toContain("推荐");
			expect(copy).not.toContain("おすすめ");
		}
	});

	test("uses translated validation messages for required client fields", () => {
		const echoT = ((key: string) => key) as TFunction;
		const schema = createClientFormSchema(echoT);
		const result = schema.safeParse({
			identifier: "",
			displayName: "",
			configFileChoice: "without_config_file",
			supportedTransports: [],
			configFileParseFormat: "json",
			configFileParseContainerType: "standard",
		});

		expect(result.success).toBe(false);
		if (!result.success) {
			expect(result.error.flatten().fieldErrors.identifier).toEqual([
				"detail.form.validation.identifierRequired",
			]);
			expect(result.error.flatten().fieldErrors.displayName).toEqual([
				"detail.form.validation.displayNameRequired",
			]);
		}
	});
});
