import i18n, { SUPPORTED_LANGUAGES } from "./index";
import { loadPageTranslations } from "./index";

export function usePageTranslations(
	page: keyof typeof loadPageTranslations,
): void {
	const hasPageTranslations = SUPPORTED_LANGUAGES.every(({ i18n: language }) =>
		i18n.hasResourceBundle(language, page),
	);

	if (!hasPageTranslations) {
		loadPageTranslations[page]();
	}
}
