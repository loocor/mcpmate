import {
	createContext,
	type ReactNode,
	useContext,
	useEffect,
	useState,
} from "react";
import enDict from "../i18n/en";
import zhDict from "../i18n/zh";

export type Language = "en" | "zh";

// 英文翻译
// Temporary reference to silence unused warnings for legacy in-file dictionaries
// during migration to src/i18n/*. Not used at runtime.

// Dev-only: check missing keys against English base
if (
	typeof window !== "undefined" &&
	(import.meta as unknown as { env?: { DEV?: boolean } })?.env?.DEV
) {
	const baseKeys = new Set(Object.keys(enDict as Record<string, string>));
	const check = (name: string, dict: Record<string, string>) => {
		const missing = [...baseKeys].filter((k) => !(k in dict));
		if (missing.length)
			console.warn(
				`[i18n] Missing ${missing.length} keys in ${name}:`,
				missing.slice(0, 10),
				missing.length > 10 ? "..." : "",
			);
	};
	check("zh", zhDict as Record<string, string>);
}

// 中文翻译
interface LanguageContextType {
	language: Language;
	setLanguage: (lang: Language) => void;
	t: (key: string) => string;
}

const LanguageContext = createContext<LanguageContextType | undefined>(
	undefined,
);

export function LanguageProvider({ children }: { children: ReactNode }) {
	const [language, setLanguage] = useState<Language>(() => {
		if (typeof window !== "undefined") {
			try {
				const params = new URLSearchParams(window.location.search);
				const urlLang = params.get("lang") as Language | null;
				if (urlLang && (urlLang === "en" || urlLang === "zh")) {
					// Persist and prefer explicit language from query param
					localStorage.setItem("language", urlLang);
					return urlLang;
				}
			} catch {
				/* ignore */
			}

			const savedLanguage = localStorage.getItem("language") as Language | null;
			if (savedLanguage) return savedLanguage;

			// 根据浏览器语言设置默认语言
			const browserLang = navigator.language.toLowerCase();
			if (browserLang.startsWith("zh")) return "zh";
			return "en";
		}
		return "en";
	});

	useEffect(() => {
		// 存储语言偏好
		localStorage.setItem("language", language);
	}, [language]);

	const t = (key: string): string => {
		const dict = { en: enDict, zh: zhDict } as const;
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
