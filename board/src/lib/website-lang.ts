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
 * Public docs on the marketing site only ship `en` and `zh` (see website DocRoutes).
 */
export function websiteDocsLocale(i18nLanguage: string | undefined): "en" | "zh" {
	return websiteLangParam(i18nLanguage) === "zh" ? "zh" : "en";
}
