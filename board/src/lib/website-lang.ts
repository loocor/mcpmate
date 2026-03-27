/** Query param for mcp.umate.ai legal/marketing pages (`terms`, `privacy`, etc.). */
export type WebsiteLangParam = "en" | "zh" | "ja";

/**
 * Map i18next language to site `?lang=` value (zh / ja / en).
 */
export function websiteLangParam(i18nLanguage: string | undefined): WebsiteLangParam {
	const lang = (i18nLanguage ?? "").toLowerCase();
	if (lang.startsWith("zh")) {
		return "zh";
	}
	if (lang.startsWith("ja")) {
		return "ja";
	}
	return "en";
}

/**
 * Public docs on the marketing site ship `en`, `zh`, and `ja` (see website DocRoutes).
 */
export function websiteDocsLocale(i18nLanguage: string | undefined): "en" | "zh" | "ja" {
	const lang = websiteLangParam(i18nLanguage);
	if (lang === "zh") return "zh";
	if (lang === "ja") return "ja";
	return "en";
}
