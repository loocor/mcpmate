export const HANDOFF_SETTINGS_KEY = "mcpmate.discovery.settings";
export const HANDOFF_DOWNLOAD_URL = "https://mcp.umate.ai/#download";
export const HANDOFF_ISSUES_URL = "https://github.com/loocor/mcpmate/issues";

const HANDOFF_COPY = {
	en: {
		documentTitle: "MCPMate Handoff",
		documentLanguage: "en",
		eyebrow: "MCPMate",
		title: "Continue in MCPMate",
		summary:
			"MCPMate Desktop previews, validates, and saves this server configuration before it becomes part of your local setup.",
		actions: {
			open: "Open MCPMate",
			install: "Install MCPMate",
		},
		status: {
			loading: "Loading import request...",
			ready: "Trying to open MCPMate. If nothing opens, use the options below.",
			missingId: "Import request id is missing.",
			expired: "This import request expired. Start the import again.",
		},
		unavailable: {
			summary:
				"This import request is not available. Return to the source page and choose Add to MCPMate again.",
			note:
				"If MCPMate is not installed yet, install it first and then repeat the import from the source page.",
		},
		note:
			"If MCPMate is already installed, choose Open MCPMate. If nothing opens, install MCPMate and return to this page.",
		help: {
			title: "Need help installing MCPMate?",
			community: "Join Discord",
			issues: "GitHub Issues",
		},
	},
	"zh-cn": {
		documentTitle: "MCPMate 交接",
		documentLanguage: "zh-CN",
		eyebrow: "MCPMate",
		title: "继续在 MCPMate 中添加",
		summary:
			"MCPMate Desktop 会先预览、校验并保存这个服务器配置，再把它纳入你的本地 MCP 设置。",
		actions: {
			open: "打开 MCPMate",
			install: "安装 MCPMate",
		},
		status: {
			loading: "正在加载导入请求...",
			ready: "正在尝试打开 MCPMate。如果没有反应，请使用下面的选项。",
			missingId: "缺少导入请求 ID。",
			expired: "这个导入请求已过期，请重新发起导入。",
		},
		unavailable: {
			summary: "这个导入请求不可用。请回到来源页面，重新选择添加到 MCPMate。",
			note: "如果还没有安装 MCPMate，请先安装，然后从来源页面重新发起导入。",
		},
		note:
			"如果已经安装 MCPMate，请选择打开 MCPMate。如果没有反应，请先安装 MCPMate，然后回到这个页面继续。",
		help: {
			title: "安装 MCPMate 时需要帮助？",
			community: "加入飞书社群",
			issues: "GitHub Issues",
		},
	},
	ja: {
		documentTitle: "MCPMate ハンドオフ",
		documentLanguage: "ja",
		eyebrow: "MCPMate",
		title: "MCPMate で続ける",
		summary:
			"MCPMate Desktop がこのサーバー設定をプレビュー、検証、保存してからローカル設定に追加します。",
		actions: {
			open: "MCPMate を開く",
			install: "MCPMate をインストール",
		},
		status: {
			loading: "インポート要求を読み込み中...",
			ready:
				"MCPMate を開こうとしています。何も起きない場合は、下の選択肢を使用してください。",
			missingId: "インポート要求 ID がありません。",
			expired: "このインポート要求は期限切れです。もう一度インポートしてください。",
		},
		unavailable: {
			summary:
				"このインポート要求は利用できません。元のページに戻り、Add to MCPMate をもう一度選択してください。",
			note:
				"MCPMate がまだインストールされていない場合は、先にインストールしてから元のページでインポートをやり直してください。",
		},
		note:
			"MCPMate がインストール済みの場合は MCPMate を開いてください。何も起きない場合は、MCPMate をインストールしてからこのページに戻ってください。",
		help: {
			title: "MCPMate のインストールでお困りですか？",
			community: "Discord に参加",
			issues: "GitHub Issues",
		},
	},
};

export function normalizeHandoffLanguage(language) {
	const lower = String(language ?? "").toLowerCase();
	if (lower === "zh" || lower === "zh-cn" || lower.startsWith("zh-")) {
		return "zh-cn";
	}
	if (lower === "ja" || lower.startsWith("ja-")) {
		return "ja";
	}
	return "en";
}

export function handoffCopyForLanguage(language) {
	return HANDOFF_COPY[normalizeHandoffLanguage(language)];
}
