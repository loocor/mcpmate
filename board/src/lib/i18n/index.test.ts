import { describe, expect, test } from "bun:test";

import { accountTranslations } from "../../pages/account/i18n";
import { auditTranslations } from "../../pages/audit/i18n";
import { clientsTranslations } from "../../pages/clients/i18n";
import { dashboardTranslations } from "../../pages/dashboard/i18n";
import { marketTranslations } from "../../pages/market/i18n";
import { onboardingTranslations } from "../../pages/onboarding/i18n";
import { operatorTranslations } from "../../pages/operator/i18n";
import { profilesTranslations } from "../../pages/profiles/i18n";
import { runtimeTranslations } from "../../pages/runtime/i18n";
import { serversTranslations } from "../../pages/servers/i18n";
import { settingsTranslations } from "../../pages/settings/i18n";
import { systemTranslations } from "../../pages/system/i18n";

const locales = ["en", "zh-CN", "ja-JP"] as const;

const namespaces = {
	account: accountTranslations,
	audit: auditTranslations,
	clients: clientsTranslations,
	dashboard: dashboardTranslations,
	market: marketTranslations,
	onboarding: onboardingTranslations,
	operator: operatorTranslations,
	profiles: profilesTranslations,
	runtime: runtimeTranslations,
	servers: serversTranslations,
	settings: settingsTranslations,
	system: systemTranslations,
};

function collectStringPaths(value: unknown, prefix = ""): string[] {
	if (typeof value === "string") {
		return [prefix];
	}
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return [];
	}

	return Object.entries(value).flatMap(([key, child]) =>
		collectStringPaths(child, prefix ? `${prefix}.${key}` : key),
	);
}

function collectStrings(
	value: unknown,
	prefix = "",
): Record<string, string> {
	if (typeof value === "string") {
		return { [prefix]: value };
	}
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return {};
	}

	return Object.fromEntries(
		Object.entries(value).flatMap(([key, child]) =>
			Object.entries(collectStrings(child, prefix ? `${prefix}.${key}` : key)),
		),
	);
}

function interpolationVariables(value: string): string[] {
	return [...value.matchAll(/{{\s*([^},\s]+)[^}]*}}/g)]
		.map((match) => match[1])
		.sort();
}

describe("i18n translations", () => {
	test("keeps supported locale keys aligned for every namespace", () => {
		for (const [namespace, translations] of Object.entries(namespaces)) {
			const pathsByLocale = Object.fromEntries(
				locales.map((locale) => [
					locale,
					new Set(collectStringPaths(translations[locale])),
				]),
			);
			const allPaths = new Set(
				Object.values(pathsByLocale).flatMap((paths) => [...paths]),
			);

			for (const locale of locales) {
				const missing = [...allPaths].filter(
					(path) => !pathsByLocale[locale].has(path),
				);
				expect(missing, `${namespace}/${locale}`).toEqual([]);
			}
		}
	});

	test("keeps interpolation variables aligned across supported locales", () => {
		for (const [namespace, translations] of Object.entries(namespaces)) {
			const stringsByLocale = Object.fromEntries(
				locales.map((locale) => [locale, collectStrings(translations[locale])]),
			);
			const reference = stringsByLocale.en;
			for (const path of Object.keys(reference)) {
				const expected = interpolationVariables(reference[path]);
				for (const locale of locales.slice(1)) {
					expect(
						interpolationVariables(stringsByLocale[locale][path]),
						`${namespace}/${locale}/${path}`,
					).toEqual(expected);
				}
			}
		}
	});
});
