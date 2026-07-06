import { describe, expect, test } from "bun:test";
import {
	handoffCopyForLanguage,
	normalizeHandoffLanguage,
} from "./handoff-copy.mjs";

describe("handoff copy", () => {
	test("normalizes extension and browser language tags", () => {
		expect(normalizeHandoffLanguage("zh")).toBe("zh-cn");
		expect(normalizeHandoffLanguage("zh-Hant")).toBe("zh-cn");
		expect(normalizeHandoffLanguage("ja-JP")).toBe("ja");
		expect(normalizeHandoffLanguage("fr-FR")).toBe("en");
	});

	test("returns Chinese copy for Chinese handoff pages", () => {
		const copy = handoffCopyForLanguage("zh-cn");
		expect(copy.documentLanguage).toBe("zh-CN");
		expect(copy.title).toBe("继续在 MCPMate 中添加");
		expect(copy.actions.open).toBe("打开 MCPMate");
		expect(copy.help.community).toBe("加入飞书社群");
	});

	test("returns Japanese copy for Japanese handoff pages", () => {
		const copy = handoffCopyForLanguage("ja");
		expect(copy.documentLanguage).toBe("ja");
		expect(copy.title).toBe("MCPMate で続ける");
		expect(copy.actions.install).toBe("MCPMate をインストール");
		expect(copy.help.community).toBe("Discord に参加");
	});
});
