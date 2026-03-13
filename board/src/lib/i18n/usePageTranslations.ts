import { useEffect } from "react";
import i18n from "./index";
import { loadPageTranslations } from "./index";

export const usePageTranslations = (
	page: keyof typeof loadPageTranslations,
) => {
	useEffect(() => {
		// 确保 i18n 已初始化后再加载翻译
		const loadTranslations = async () => {
			if (!i18n.isInitialized) {
				await i18n.init();
			}
			loadPageTranslations[page]();
		};

		void loadTranslations();
	}, [page]);
};
