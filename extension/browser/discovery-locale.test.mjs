import { describe, expect, test } from "bun:test";
import {
	discoveryAcceptLanguage,
	discoveryLocaleFromLanguage,
	extensionLanguageFromBrowser,
} from "./discovery-locale.mjs";

describe("discovery locale", () => {
	test("maps extension languages to Admin discovery locales", () => {
		expect(discoveryLocaleFromLanguage("zh-cn")).toBe("zh");
		expect(discoveryLocaleFromLanguage("ja")).toBe("ja");
		expect(discoveryLocaleFromLanguage("en")).toBe("en");
	});

	test("builds Accept-Language headers for discovery requests", () => {
		expect(discoveryAcceptLanguage("zh")).toContain("zh-CN");
		expect(discoveryAcceptLanguage("ja")).toContain("ja-JP");
		expect(discoveryAcceptLanguage("en")).toContain("en-US");
	});

	test("derives popup language from browser language tags", () => {
		expect(extensionLanguageFromBrowser(["zh-CN", "en"])).toBe("zh-cn");
		expect(extensionLanguageFromBrowser(["ja-JP", "en-US"])).toBe("ja");
		expect(extensionLanguageFromBrowser(["en-US", "en"])).toBe("en");
		expect(extensionLanguageFromBrowser(["fr-FR", "de-DE"])).toBe("en");
	});
});
