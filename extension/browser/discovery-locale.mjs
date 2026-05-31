/** Maps browser language tags to extension popup language (`en` | `zh-cn` | `ja`). */
export function extensionLanguageFromBrowser(
	languages = globalThis.navigator?.languages ?? [globalThis.navigator?.language ?? "en"],
) {
	for (const tag of languages) {
		const lower = String(tag ?? "").toLowerCase();
		if (lower.startsWith("zh")) {
			return "zh-cn";
		}
		if (lower.startsWith("ja")) {
			return "ja";
		}
		if (lower.startsWith("en")) {
			return "en";
		}
	}
	return "en";
}

/** Maps extension popup language to Admin public discovery locale (`en` | `zh` | `ja`). */
export function discoveryLocaleFromLanguage(language) {
	if (language === "zh" || language === "zh-cn") {
		return "zh";
	}
	if (language === "ja") {
		return "ja";
	}
	return "en";
}

export function discoveryAcceptLanguage(locale) {
	switch (locale) {
		case "zh":
			return "zh-CN,zh;q=0.9,en;q=0.8";
		case "ja":
			return "ja-JP,ja;q=0.9,en;q=0.8";
		default:
			return "en-US,en;q=0.9";
	}
}
