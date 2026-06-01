import type { Language } from "../components/LanguageProvider";

export const LANGUAGE_OPTIONS: readonly {
	code: Language;
	badge: string;
	label: string;
}[] = [
	{ code: "en", badge: "EN", label: "English" },
	{ code: "zh", badge: "中", label: "中文" },
	{ code: "ja", badge: "日", label: "日本語" },
] as const;

export function getLanguageOption(code: Language) {
	return LANGUAGE_OPTIONS.find((option) => option.code === code) ?? LANGUAGE_OPTIONS[0];
}
