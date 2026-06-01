import type { WebsiteClientPreset } from "../lib/admin-discovery";

/**
 * Built-in compatibility list for the marketing site when the public discovery API
 * is unreachable (offline dev, network blocks, or transient outages).
 * Keep in sync with the public catalog when identifiers change.
 */
export const COMPATIBLE_CLIENTS_FALLBACK: WebsiteClientPreset[] = [
	{ identifier: "claude_code", displayName: "Claude Code", logoUrl: "", homepageUrl: "https://www.anthropic.com/claude-code" },
	{ identifier: "claude_desktop", displayName: "Claude Desktop", logoUrl: "", homepageUrl: "https://claude.ai/download" },
	{ identifier: "cline", displayName: "Cline", logoUrl: "", homepageUrl: "https://cline.bot" },
	{ identifier: "codebuddy", displayName: "CodeBuddy", logoUrl: "", homepageUrl: "" },
	{ identifier: "codex", displayName: "Codex", logoUrl: "", homepageUrl: "" },
	{ identifier: "continue", displayName: "Continue", logoUrl: "", homepageUrl: "https://continue.dev" },
	{ identifier: "cursor", displayName: "Cursor", logoUrl: "", homepageUrl: "https://cursor.com" },
	{ identifier: "hermes", displayName: "Hermes", logoUrl: "", homepageUrl: "" },
	{ identifier: "kiro", displayName: "Kiro", logoUrl: "", homepageUrl: "" },
	{ identifier: "openclaw", displayName: "OpenClaw", logoUrl: "", homepageUrl: "" },
	{ identifier: "opencode", displayName: "OpenCode", logoUrl: "", homepageUrl: "" },
	{ identifier: "qoder", displayName: "Qoder", logoUrl: "", homepageUrl: "" },
	{ identifier: "trae", displayName: "Trae", logoUrl: "", homepageUrl: "" },
	{ identifier: "trae_cn", displayName: "Trae CN", logoUrl: "", homepageUrl: "" },
	{ identifier: "vscode", displayName: "VS Code", logoUrl: "", homepageUrl: "https://code.visualstudio.com" },
	{ identifier: "windsurf", displayName: "Windsurf", logoUrl: "", homepageUrl: "https://windsurf.com" },
	{ identifier: "zed", displayName: "Zed", logoUrl: "", homepageUrl: "https://zed.dev" },
];
