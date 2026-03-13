import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";
// 导入所有翻译文件
import { clientsTranslations } from "../../pages/clients/i18n";
import { dashboardTranslations } from "../../pages/dashboard/i18n";
import { marketTranslations } from "../../pages/market/i18n";
import { profilesTranslations } from "../../pages/profiles/i18n";
import { runtimeTranslations } from "../../pages/runtime/i18n";
import { settingsTranslations } from "../../pages/settings/i18n";
import { serversTranslations } from "../../pages/servers/i18n";
import { inspectorTranslations } from "../../components/i18n/inspector";
import { commonTranslations } from "./common";
import { navigationTranslations } from "./navigation";

// 基础翻译资源
const baseResources = {
	en: {
		translation: {
			...commonTranslations.en,
			...navigationTranslations.en,
		},
	},
	"zh-CN": {
		translation: {
			...commonTranslations["zh-CN"],
			...navigationTranslations["zh-CN"],
		},
	},
	"ja-JP": {
		translation: {
			...commonTranslations["ja-JP"],
			...navigationTranslations["ja-JP"],
		},
	},
};

// 动态加载翻译文件的函数
export const loadTranslations = (namespace: string, translations: any) => {
	// 为每种语言添加翻译
	Object.keys(translations).forEach((lang) => {
		// 始终添加/更新资源束，deep=true, overwrite=true 确保正确合并
		i18n.addResourceBundle(lang, namespace, translations[lang], true, true);
	});
};

// 页面特定的翻译加载器
export const loadPageTranslations = {
	dashboard: () => {
		loadTranslations("dashboard", dashboardTranslations);
	},
	profiles: () => {
		loadTranslations("profiles", profilesTranslations);
	},
	settings: () => {
		loadTranslations("settings", settingsTranslations);
	},
	market: () => {
		loadTranslations("market", marketTranslations);
	},
	runtime: () => {
		loadTranslations("runtime", runtimeTranslations);
	},
	clients: () => {
		loadTranslations("clients", clientsTranslations);
	},
	servers: () => {
		loadTranslations("servers", serversTranslations);
	},
	inspector: () => {
		loadTranslations("inspector", inspectorTranslations);
	},
};

// 初始化 i18n
i18n
	.use(LanguageDetector)
	.use(initReactI18next)
	.init({
		resources: baseResources,
		fallbackLng: "en",
		debug: false,
		interpolation: {
			escapeValue: false,
		},
		detection: {
			order: ["localStorage", "navigator", "htmlTag"],
			caches: ["localStorage"],
		},
	});

// 导出语言支持相关的类型和常量
export type SupportedI18nCode = "en" | "zh-CN" | "ja-JP";

export const SUPPORTED_LANGUAGES = [
	{ store: "en", i18n: "en" as SupportedI18nCode, fallback: "English" },
	{ store: "zh-cn", i18n: "zh-CN" as SupportedI18nCode, fallback: "简体中文" },
	{ store: "ja", i18n: "ja-JP" as SupportedI18nCode, fallback: "日本語" },
] as const;

const FALLBACK_LANGUAGE: SupportedI18nCode = "en";

export function resolveI18nLanguage(language?: string): SupportedI18nCode {
	if (!language) return FALLBACK_LANGUAGE;
	const lower = language.toLowerCase();
	if (lower.startsWith("zh")) {
		return "zh-CN";
	}
	if (lower.startsWith("ja")) {
		return "ja-JP";
	}
	return "en";
}

export async function ensureI18n() {
	await i18n.isInitialized;
	return i18n;
}

export default i18n;
