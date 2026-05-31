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
