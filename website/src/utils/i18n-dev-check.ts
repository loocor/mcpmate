import enDict from "../i18n/en";
import zhDict from "../i18n/zh";

export function checkI18nKeys() {
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
}
