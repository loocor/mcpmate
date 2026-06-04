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
});
