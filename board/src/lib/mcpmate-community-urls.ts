/** MCPMate community Discord (onboarding community step + app shell footer). */
export const MCPMATE_DISCORD_COMMUNITY_HREF =
	"https://discord.gg/pc5YfEVbKj" as const;

/** MCPMate Feishu community group invite (Chinese locale onboarding). */
export const MCPMATE_FEISHU_COMMUNITY_HREF =
	"https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=bd4hb1f5-7dd8-4e89-9e83-103364a81fbf&qr_code=true" as const;

export function prefersFeishuCommunity(language: string | undefined): boolean {
	return (language ?? "").toLowerCase().startsWith("zh");
}
