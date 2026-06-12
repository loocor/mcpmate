import { useTranslation } from "react-i18next";
import { usePageTranslations } from "../i18n/usePageTranslations";

/** Load secrets page bundles and return the secrets namespace translator. */
export function useSecretsTranslations() {
	usePageTranslations("secrets");
	return useTranslation("secrets");
}
