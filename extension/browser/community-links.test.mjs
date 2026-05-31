import { describe, expect, test } from "bun:test";
import {
	communityFooterForLanguage,
	MCPMATE_DISCORD_COMMUNITY_HREF,
	MCPMATE_FEISHU_COMMUNITY_HREF,
} from "./community-links.mjs";

describe("community links", () => {
	test("selects Feishu footer for Chinese extension languages", () => {
		expect(communityFooterForLanguage("zh-cn")).toEqual({
			iconKey: "feishu",
			href: MCPMATE_FEISHU_COMMUNITY_HREF,
		});
		expect(communityFooterForLanguage("zh")).toEqual({
			iconKey: "feishu",
			href: MCPMATE_FEISHU_COMMUNITY_HREF,
		});
	});

	test("selects Discord footer for non-Chinese extension languages", () => {
		expect(communityFooterForLanguage("en")).toEqual({
			iconKey: "discord",
			href: MCPMATE_DISCORD_COMMUNITY_HREF,
		});
		expect(communityFooterForLanguage("ja")).toEqual({
			iconKey: "discord",
			href: MCPMATE_DISCORD_COMMUNITY_HREF,
		});
	});
});
