/** MCPMate community Discord (extension footer + board onboarding). */
export const MCPMATE_DISCORD_COMMUNITY_HREF =
	"https://discord.gg/pc5YfEVbKj";

/** MCPMate Feishu community group invite (Chinese locale surfaces). */
export const MCPMATE_FEISHU_COMMUNITY_HREF =
	"https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=bd4hb1f5-7dd8-4e89-9e83-103364a81fbf&qr_code=true";

function prefersFeishuCommunity(language) {
	return String(language ?? "")
		.toLowerCase()
		.startsWith("zh");
}

/** Footer community CTA: Feishu for zh, Discord otherwise. */
export function communityFooterForLanguage(language) {
	if (prefersFeishuCommunity(language)) {
		return {
			iconKey: "feishu",
			href: MCPMATE_FEISHU_COMMUNITY_HREF,
		};
	}
	return {
		iconKey: "discord",
		href: MCPMATE_DISCORD_COMMUNITY_HREF,
	};
}
