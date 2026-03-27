export function detectLocale(pathname: string): "en" | "zh" | "ja" {
	if (pathname.startsWith("/docs/zh")) return "zh";
	if (pathname.startsWith("/docs/ja")) return "ja";
	return "en";
}

export function getLocalizedText(
	locale: "en" | "zh" | "ja",
	key: "search" | "noResults",
): string {
	const texts = {
		search: {
			zh: "搜索文档…",
			ja: "ドキュメントを検索…",
			en: "Search docs…",
		},
		noResults: {
			zh: "无匹配结果",
			ja: "一致する結果がありません",
			en: "No results",
		},
	};
	return texts[key][locale];
}

export function getGroupName(
	locale: "en" | "zh" | "ja",
	group: string,
): string {
	const groupMappings: Record<string, Record<string, string>> = {
		Features: { zh: "功能特性", ja: "機能", en: "Features" },
		Guides: { zh: "操作指南", ja: "操作ガイド", en: "Guides" },
	};
	return groupMappings[group]?.[locale] ?? group;
}
