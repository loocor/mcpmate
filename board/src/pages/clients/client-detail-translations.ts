import { usePageTranslations } from "../../lib/i18n/usePageTranslations";

export function useClientDetailTranslations(): void {
	usePageTranslations("clients");
	usePageTranslations("servers");
}
