import { describe, expect, test } from "bun:test";
import type { TFunction } from "i18next";

import { resolveClientIdentifierForSave } from "../../../components/client-form-identifiers";
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

	test("limits strict client identifier format validation to create mode", () => {
		const echoT = ((key: string) => key) as TFunction;
		const values = {
			identifier: "claude_desktop",
			displayName: "Claude Desktop",
			configFileChoice: "without_config_file",
			supportedTransports: [],
			configFileParseFormat: "json",
			configFileParseContainerType: "standard",
		};

		const createResult = createClientFormSchema(echoT, "create").safeParse(values);
		expect(createResult.success).toBe(false);
		if (!createResult.success) {
			expect(createResult.error.flatten().fieldErrors.identifier).toEqual([
				"detail.form.validation.identifierFormat",
			]);
		}

		const editResult = createClientFormSchema(echoT, "edit").safeParse(values);
		expect(editResult.success).toBe(true);
	});

	test("preserves legacy client identifiers when saving edit mode", () => {
		expect(resolveClientIdentifierForSave("create", "Claude Desktop")).toBe("claude-desktop");
		expect(resolveClientIdentifierForSave("edit", "claude_desktop", "claude_desktop")).toBe(
			"claude_desktop",
		);
	});
});
