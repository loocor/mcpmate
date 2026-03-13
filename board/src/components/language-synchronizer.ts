import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
	ensureI18n,
	resolveI18nLanguage,
	SUPPORTED_LANGUAGES,
} from "../lib/i18n/index";
import { useAppStore } from "../lib/store";

const FALLBACK_STORE_LANGUAGE = "en";

function mapI18nToStore(code: string): string {
	const resolved = resolveI18nLanguage(code);
	const match = SUPPORTED_LANGUAGES.find((entry) => entry.i18n === resolved);
	return match?.store ?? FALLBACK_STORE_LANGUAGE;
}

function mapStoreToI18n(code: string): string {
	const match = SUPPORTED_LANGUAGES.find((entry) => entry.store === code);
	return match?.i18n ?? resolveI18nLanguage(code);
}

export function LanguageSynchronizer() {
	const dashboardLanguage = useAppStore(
		(state) => state.dashboardSettings.language,
	);
	const setDashboardSetting = useAppStore((state) => state.setDashboardSetting);
	const { i18n } = useTranslation();
	const initialisedRef = useRef(false);

	useEffect(() => {
		if (!initialisedRef.current) {
			initialisedRef.current = true;
			void (async () => {
				const instance = await ensureI18n();
				const detectedStore = mapI18nToStore(instance.language);
				if (dashboardLanguage !== detectedStore) {
					setDashboardSetting(
						"language",
						detectedStore as typeof dashboardLanguage,
					);
				}
			})();
		}
	}, [dashboardLanguage, setDashboardSetting]);

	useEffect(() => {
		void (async () => {
			const target = mapStoreToI18n(dashboardLanguage);
			if (i18n.language !== target) {
				await i18n.changeLanguage(target);
			}
		})();
	}, [dashboardLanguage, i18n]);

	return null;
}
