import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { useTranslation } from "react-i18next";

import i18n, { loadPageTranslations } from "../../lib/i18n";
import { useClientDetailTranslations } from "./client-detail-translations";

function LifecycleLabelProbe() {
	useClientDetailTranslations();
	const { t } = useTranslation("clients");
	return <span>{t("servers:capabilityLifecycle.capabilityUnknown")}</span>;
}

test("loads server lifecycle labels during direct Client Detail navigation", async () => {
	await i18n.changeLanguage("en");
	for (const language of ["en", "zh-CN", "ja-JP"]) {
		i18n.removeResourceBundle(language, "clients");
		i18n.removeResourceBundle(language, "servers");
	}
	loadPageTranslations.clients();
	expect(i18n.hasResourceBundle("en", "clients")).toBe(true);
	expect(i18n.hasResourceBundle("en", "servers")).toBe(false);

	const markup = renderToStaticMarkup(<LifecycleLabelProbe />);

	expect(markup).toContain("Unknown");
	expect(markup).not.toContain("capabilityLifecycle.capabilityUnknown");
});
