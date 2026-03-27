import {
	createContext,
	type ReactNode,
	useState,
	useEffect,
	useContext,
} from "react";
import enDict from "../i18n/en";
import zhDict from "../i18n/zh";
import jaDict from "../i18n/ja";
import { checkI18nKeys } from "../utils/i18n-dev-check";

export type Language = "en" | "zh" | "ja";

export interface LanguageContextType {
	language: Language;
	setLanguage: (lang: Language) => void;
	t: (key: string) => string;
}

export const LanguageContext = createContext<LanguageContextType | undefined>(
	undefined,
);

checkI18nKeys();

export function LanguageProvider({ children }: { children: ReactNode }) {
	const [language, setLanguage] = useState<Language>(() => {
		if (typeof window !== "undefined") {
			try {
				const params = new URLSearchParams(window.location.search);
				const urlLang = params.get("lang");
				const validLanguages: Language[] = ["en", "zh", "ja"];
				if (urlLang && validLanguages.includes(urlLang as Language)) {
					const lang = urlLang as Language;
					localStorage.setItem("language", lang);
					return lang;
				}
			} catch {
				/* ignore */
			}

			const savedLanguage = localStorage.getItem("language") as Language | null;
			if (savedLanguage) return savedLanguage;

			const browserLang = navigator.language.toLowerCase();
			if (browserLang.startsWith("zh")) return "zh";
			if (browserLang.startsWith("ja")) return "ja";
			return "en";
		}
		return "en";
	});

	useEffect(() => {
		localStorage.setItem("language", language);
	}, [language]);

	const t = (key: string): string => {
		const dict = { en: enDict, zh: zhDict, ja: jaDict } as const;
		const primary = dict[language] as Record<string, string>;
		const fallback = dict["en"] as Record<string, string>;
		const translation = primary[key] ?? fallback[key];
		if (!translation) {
			console.warn(`Translation missing for key: ${key} in ${language}`);
			return key;
		}
		return translation.replace("{year}", new Date().getFullYear().toString());
	};

	return (
		<LanguageContext.Provider value={{ language, setLanguage, t }}>
			{children}
		</LanguageContext.Provider>
	);
}

export function useLanguage() {
	const context = useContext(LanguageContext);
	if (context === undefined) {
		throw new Error("useLanguage must be used within a LanguageProvider");
	}
	return context;
}
