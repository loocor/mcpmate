export type Locale = "en" | "zh" | "ja";

export type DocPage = {
	id: string;
	path: string; // full path including /docs/:locale/...
	title: string;
	summary?: string;
	keywords?: string[];
	// Lazy component import to keep first load small
	component: () => Promise<{ default: React.ComponentType<Record<string, unknown>> }>;
};

export type DocGroup = { group: string; pages: (DocPage | DocGroup)[] };

export type DocNav = { locale: Locale; groups: DocGroup[] };

export const docsNav: DocNav[] = [
	{
		locale: "en",
		groups: [
			// Root-level items (no collapsible header)
			{
				group: "",
				pages: [
					{
						id: "quickstart",
						path: "/docs/en/quickstart",
						title: "Quick Start",
						summary: "Install and run MCPMate in minutes.",
						keywords: ["install", "setup"],
						component: () => import("./pages/en/Quickstart"),
					},
				],
			},
			// Feature concepts
			{
				group: "Features",
				pages: [
					{
						id: "features-overview",
						path: "/docs/en/features-overview",
						title: "Overview",
						summary: "Explore MCPMate's powerful features",
						component: () => import("./pages/en/FeaturesOverview"),
					},
					{
						id: "centralized-config",
						path: "/docs/en/centralized-config",
						title: "Centralized Configuration",
						component: () => import("./pages/en/CentralizedConfig"),
					},
					{
						id: "resource-optimization",
						path: "/docs/en/resource-optimization",
						title: "Resource Optimization",
						component: () => import("./pages/en/ResourceOptimization"),
					},
					{
						id: "inspector",
						path: "/docs/en/inspector",
						title: "Inspector",
						component: () => import("./pages/en/Inspector"),
					},
					{
						id: "context-switching",
						path: "/docs/en/context-switching",
						title: "Seamless Context Switching",
						component: () => import("./pages/en/ContextSwitching"),
					},
					{
						id: "protocol-bridging",
						path: "/docs/en/protocol-bridging",
						title: "Protocol Bridging",
						component: () => import("./pages/en/ProtocolBridging"),
					},
					{
						id: "marketplace",
						path: "/docs/en/marketplace",
						title: "Market Install Flow",
						component: () => import("./pages/en/Marketplace"),
					},
					{
						id: "granular-controls",
						path: "/docs/en/granular-controls",
						title: "Granular Controls",
						component: () => import("./pages/en/GranularControls"),
					},
					{
						id: "auto-discovery",
						path: "/docs/en/auto-discovery",
						title: "Auto Discovery & Import",
						component: () => import("./pages/en/AutoDiscovery"),
					},
					{
						id: "uni-import",
						path: "/docs/en/uni-import",
						title: "Uni-Import",
						component: () => import("./pages/en/UniImport"),
					},
				],
			},
			// Guides group ordered by UI sections
			{
				group: "Guides",
				pages: [
					{
						id: "guides-overview",
						path: "/docs/en/guides-overview",
						title: "Overview",
						summary: "Learn how to use MCPMate effectively",
						component: () => import("./pages/en/GuidesOverview"),
					},
					{
						id: "dashboard",
						path: "/docs/en/dashboard",
						title: "Dashboard",
						summary: "Overview of the main console and status.",
						component: () => import("./pages/en/Dashboard"),
					},
					{
						group: "Profiles",
						pages: [
							{
								id: "profile",
								path: "/docs/en/profile",
								title: "Overview",
								summary: "What Profiles are for and how the module is organized.",
								component: () => import("./pages/en/Profile"),
							},
							{
								id: "profile-presets",
								path: "/docs/en/profile-presets",
								title: "Preset Templates",
								component: () => import("./pages/en/ProfilePresets"),
							},
							{
								id: "profile-detail-overview",
								path: "/docs/en/profile-detail-overview",
								title: "Detail Overview",
								component: () => import("./pages/en/ProfileDetailOverview"),
							},
							{
								id: "profile-capabilities",
								path: "/docs/en/profile-capabilities",
								title: "Capability Tabs",
								component: () => import("./pages/en/ProfileCapabilities"),
							},
						],
					},
					{
						group: "Clients",
						pages: [
							{
								id: "clients",
								path: "/docs/en/clients",
								title: "Overview",
								summary: "Discover client apps and understand the detail workflow.",
								component: () => import("./pages/en/ClientApps"),
							},
							{
								id: "client-detail-overview",
								path: "/docs/en/client-detail-overview",
								title: "Detail Overview",
								component: () => import("./pages/en/ClientDetailOverview"),
							},
							{
								id: "client-configuration",
								path: "/docs/en/client-configuration",
								title: "Configuration",
								component: () => import("./pages/en/ClientConfiguration"),
							},
							{
								id: "client-backups",
								path: "/docs/en/client-backups",
								title: "Backups",
								component: () => import("./pages/en/ClientBackups"),
							},
						],
					},
					{
						group: "Servers",
						pages: [
							{
								id: "servers",
								path: "/docs/en/servers",
								title: "Overview",
								component: () => import("./pages/en/Servers"),
							},
							{
								id: "server-import-preview",
								path: "/docs/en/server-import-preview",
								title: "Import & Preview",
								component: () => import("./pages/en/ServerImportPreview"),
							},
							{
								id: "server-detail-overview",
								path: "/docs/en/server-detail-overview",
								title: "Detail Overview",
								component: () => import("./pages/en/ServerDetailOverview"),
							},
							{
								id: "server-capabilities",
								path: "/docs/en/server-capabilities",
								title: "Capabilities",
								component: () => import("./pages/en/ServerCapabilities"),
							},
							{
								id: "server-inspector",
								path: "/docs/en/server-inspector",
								title: "Inspector",
								component: () => import("./pages/en/ServerInspector"),
							},
							{
								id: "server-instances",
								path: "/docs/en/server-instances",
								title: "Instances",
								component: () => import("./pages/en/ServerInstances"),
							},
						],
					},
					{
						id: "market",
						path: "/docs/en/market",
						title: "Market",
						component: () => import("./pages/en/Market"),
					},
					{
						id: "runtime",
						path: "/docs/en/runtime",
						title: "Runtime",
						component: () => import("./pages/en/Runtime"),
					},
					{
						id: "logs",
						path: "/docs/en/logs",
						title: "Audit Logs",
						component: () => import("./pages/en/Logs"),
					},
					{
						id: "api-docs",
						path: "/docs/en/api-docs",
						title: "API Docs",
						component: () => import("./pages/en/APIDocs"),
					},
					{
						id: "settings",
						path: "/docs/en/settings",
						title: "Settings",
						component: () => import("./pages/en/Settings"),
					},
				],
			},
			// Keep Changelog below feature concepts
			{
				group: "",
				pages: [
					{
						id: "changelog",
						path: "/docs/en/changelog",
						title: "Changelog",
						component: () => import("./pages/en/Changelog"),
					},
					{
						id: "roadmap",
						path: "/docs/en/roadmap",
						title: "Roadmap",
						component: () => import("./pages/en/Roadmap"),
					},
				],
			},
		],
	},
	{
		locale: "zh",
		groups: [
			// Root-level items (no collapsible header)
			{
				group: "",
				pages: [
					{
						id: "quickstart",
						path: "/docs/zh/quickstart",
						title: "快速开始",
						summary: "几分钟内完成 MCPMate 安装与运行。",
						keywords: ["安装", "上手"],
						component: () => import("./pages/zh/Quickstart"),
					},
				],
			},
			// New feature concepts group
			{
				group: "功能特性",
				pages: [
					{
						id: "features-overview",
						path: "/docs/zh/features-overview",
						title: "概览",
						summary: "探索 MCPMate 的强大功能",
						component: () => import("./pages/zh/FeaturesOverview"),
					},
					{
						id: "centralized-config",
						path: "/docs/zh/centralized-config",
						title: "集中配置",
						component: () => import("./pages/zh/CentralizedConfig"),
					},
					{
						id: "resource-optimization",
						path: "/docs/zh/resource-optimization",
						title: "资源优化",
						component: () => import("./pages/zh/ResourceOptimization"),
					},
					{
						id: "inspector",
						path: "/docs/zh/inspector",
						title: "检视器",
						component: () => import("./pages/zh/Inspector"),
					},
					{
						id: "context-switching",
						path: "/docs/zh/context-switching",
						title: "无缝上下文切换",
						component: () => import("./pages/zh/ContextSwitching"),
					},
					{
						id: "protocol-bridging",
						path: "/docs/zh/protocol-bridging",
						title: "协议桥接",
						component: () => import("./pages/zh/ProtocolBridging"),
					},
					{
						id: "marketplace",
						path: "/docs/zh/marketplace",
						title: "服务源安装流程",
						component: () => import("./pages/zh/Marketplace"),
					},
					{
						id: "granular-controls",
						path: "/docs/zh/granular-controls",
						title: "精细控制",
						component: () => import("./pages/zh/GranularControls"),
					},
					{
						id: "auto-discovery",
						path: "/docs/zh/auto-discovery",
						title: "自动发现与导入",
						component: () => import("./pages/zh/AutoDiscovery"),
					},
					{
						id: "uni-import",
						path: "/docs/zh/uni-import",
						title: "全能导入",
						component: () => import("./pages/zh/UniImport"),
					},
				],
			},
			{
				group: "操作指南",
				pages: [
					{
						id: "guides-overview",
						path: "/docs/zh/guides-overview",
						title: "概览",
						summary: "学习如何高效使用 MCPMate",
						component: () => import("./pages/zh/GuidesOverview"),
					},
					{
						id: "dashboard",
						path: "/docs/zh/dashboard",
						title: "控制台",
						component: () => import("./pages/zh/Dashboard"),
					},
					{
						group: "配置集",
						pages: [
							{
								id: "profile",
								path: "/docs/zh/profile",
								title: "概览",
								summary: "理解配置集模块的价值、范围与文档结构。",
								component: () => import("./pages/zh/Profile"),
							},
							{
								id: "profile-presets",
								path: "/docs/zh/profile-presets",
								title: "预设模板",
								component: () => import("./pages/zh/ProfilePresets"),
							},
							{
								id: "profile-detail-overview",
								path: "/docs/zh/profile-detail-overview",
								title: "详情概览",
								component: () => import("./pages/zh/ProfileDetailOverview"),
							},
							{
								id: "profile-capabilities",
								path: "/docs/zh/profile-capabilities",
								title: "能力标签页",
								component: () => import("./pages/zh/ProfileCapabilities"),
							},
						],
					},
					{
						group: "客户端",
						pages: [
							{
								id: "clients",
								path: "/docs/zh/clients",
								title: "概览",
								summary: "了解客户端列表与详情流程的分工。",
								component: () => import("./pages/zh/ClientApps"),
							},
							{
								id: "client-detail-overview",
								path: "/docs/zh/client-detail-overview",
								title: "详情概览",
								component: () => import("./pages/zh/ClientDetailOverview"),
							},
							{
								id: "client-configuration",
								path: "/docs/zh/client-configuration",
								title: "配置管理",
								component: () => import("./pages/zh/ClientConfiguration"),
							},
							{
								id: "client-backups",
								path: "/docs/zh/client-backups",
								title: "备份与恢复",
								component: () => import("./pages/zh/ClientBackups"),
							},
						],
					},
					{
						group: "服务器",
						pages: [
							{
								id: "servers",
								path: "/docs/zh/servers",
								title: "概览",
								component: () => import("./pages/zh/Servers"),
							},
							{
								id: "server-import-preview",
								path: "/docs/zh/server-import-preview",
								title: "导入与预览",
								component: () => import("./pages/zh/ServerImportPreview"),
							},
							{
								id: "server-detail-overview",
								path: "/docs/zh/server-detail-overview",
								title: "详情概览",
								component: () => import("./pages/zh/ServerDetailOverview"),
							},
							{
								id: "server-capabilities",
								path: "/docs/zh/server-capabilities",
								title: "能力浏览",
								component: () => import("./pages/zh/ServerCapabilities"),
							},
							{
								id: "server-inspector",
								path: "/docs/zh/server-inspector",
								title: "Inspector",
								component: () => import("./pages/zh/ServerInspector"),
							},
							{
								id: "server-instances",
								path: "/docs/zh/server-instances",
								title: "实例管理",
								component: () => import("./pages/zh/ServerInstances"),
							},
						],
					},
					{
						id: "market",
						path: "/docs/zh/market",
						title: "服务源",
						component: () => import("./pages/zh/Market"),
					},
					{
						id: "runtime",
						path: "/docs/zh/runtime",
						title: "运行时",
						component: () => import("./pages/zh/Runtime"),
					},
					{
						id: "logs",
						path: "/docs/zh/logs",
						title: "审计日志",
						component: () => import("./pages/zh/Logs"),
					},
					{
						id: "api-docs",
						path: "/docs/zh/api-docs",
						title: "API 文档",
						component: () => import("./pages/zh/APIDocs"),
					},
					{
						id: "settings",
						path: "/docs/zh/settings",
						title: "设置",
						component: () => import("./pages/zh/Settings"),
					},
				],
			},
			// Place Changelog below feature concepts
			{
				group: "",
				pages: [
					{
						id: "changelog",
						path: "/docs/zh/changelog",
						title: "更新日志",
						component: () => import("./pages/zh/Changelog"),
					},
					{
						id: "roadmap",
						path: "/docs/zh/roadmap",
						title: "开发规划",
						component: () => import("./pages/zh/Roadmap"),
					},
				],
			},
		],
	},
	{
		locale: "ja",
		groups: [
			{
				group: "",
				pages: [
					{
						id: "quickstart",
						path: "/docs/ja/quickstart",
						title: "クイックスタート",
						summary: "数分で MCPMate をインストールして実行。",
						keywords: ["インストール", "セットアップ"],
						component: () => import("./pages/ja/Quickstart"),
					},
				],
			},
			{
				group: "機能",
				pages: [
					{
						id: "features-overview",
						path: "/docs/ja/features-overview",
						title: "概要",
						summary: "MCPMate の強力な機能を探索",
						component: () => import("./pages/ja/FeaturesOverview"),
					},
					{
						id: "centralized-config",
						path: "/docs/ja/centralized-config",
						title: "一元管理",
						component: () => import("./pages/ja/CentralizedConfig"),
					},
					{
						id: "resource-optimization",
						path: "/docs/ja/resource-optimization",
						title: "リソース最適化",
						component: () => import("./pages/ja/ResourceOptimization"),
					},
					{
						id: "inspector",
						path: "/docs/ja/inspector",
						title: "インスペクター",
						component: () => import("./pages/ja/Inspector"),
					},
					{
						id: "context-switching",
						path: "/docs/ja/context-switching",
						title: "シームレスなコンテキスト切り替え",
						component: () => import("./pages/ja/ContextSwitching"),
					},
					{
						id: "protocol-bridging",
						path: "/docs/ja/protocol-bridging",
						title: "プロトコルブリッジ",
						component: () => import("./pages/ja/ProtocolBridging"),
					},
					{
						id: "marketplace",
						path: "/docs/ja/marketplace",
						title: "マーケット導入フロー",
						component: () => import("./pages/ja/Marketplace"),
					},
					{
						id: "granular-controls",
						path: "/docs/ja/granular-controls",
						title: "きめ細かい制御",
						component: () => import("./pages/ja/GranularControls"),
					},
					{
						id: "auto-discovery",
						path: "/docs/ja/auto-discovery",
						title: "自動検出とインポート",
						component: () => import("./pages/ja/AutoDiscovery"),
					},
					{
						id: "uni-import",
						path: "/docs/ja/uni-import",
						title: "ユニインポート",
						component: () => import("./pages/ja/UniImport"),
					},
				],
			},
			{
				group: "操作ガイド",
				pages: [
					{
						id: "guides-overview",
						path: "/docs/ja/guides-overview",
						title: "概要",
						summary: "MCPMate を効率的に使いこなす",
						component: () => import("./pages/ja/GuidesOverview"),
					},
					{
						id: "dashboard",
						path: "/docs/ja/dashboard",
						title: "ダッシュボード",
						component: () => import("./pages/ja/Dashboard"),
					},
					{
						group: "プロファイル",
						pages: [
							{
								id: "profile",
								path: "/docs/ja/profile",
								title: "概要",
								summary: "プロファイルモジュールの価値、範囲、ドキュメント構成を理解する。",
								component: () => import("./pages/ja/Profile"),
							},
							{
								id: "profile-presets",
								path: "/docs/ja/profile-presets",
								title: "プリセットテンプレート",
								component: () => import("./pages/ja/ProfilePresets"),
							},
							{
								id: "profile-detail-overview",
								path: "/docs/ja/profile-detail-overview",
								title: "詳細概要",
								component: () => import("./pages/ja/ProfileDetailOverview"),
							},
							{
								id: "profile-capabilities",
								path: "/docs/ja/profile-capabilities",
								title: "能力タブ",
								component: () => import("./pages/ja/ProfileCapabilities"),
							},
						],
					},
					{
						group: "クライアント",
						pages: [
							{
								id: "clients",
								path: "/docs/ja/clients",
								title: "概要",
								summary: "クライアントアプリの一覧と詳細フローの役割分担を理解する。",
								component: () => import("./pages/ja/ClientApps"),
							},
							{
								id: "client-detail-overview",
								path: "/docs/ja/client-detail-overview",
								title: "詳細概要",
								component: () => import("./pages/ja/ClientDetailOverview"),
							},
							{
								id: "client-configuration",
								path: "/docs/ja/client-configuration",
								title: "設定管理",
								component: () => import("./pages/ja/ClientConfiguration"),
							},
							{
								id: "client-backups",
								path: "/docs/ja/client-backups",
								title: "バックアップと復元",
								component: () => import("./pages/ja/ClientBackups"),
							},
						],
					},
					{
						group: "サーバー",
						pages: [
							{
								id: "servers",
								path: "/docs/ja/servers",
								title: "概要",
								component: () => import("./pages/ja/Servers"),
							},
							{
								id: "server-import-preview",
								path: "/docs/ja/server-import-preview",
								title: "インポートとプレビュー",
								component: () => import("./pages/ja/ServerImportPreview"),
							},
							{
								id: "server-detail-overview",
								path: "/docs/ja/server-detail-overview",
								title: "詳細概要",
								component: () => import("./pages/ja/ServerDetailOverview"),
							},
							{
								id: "server-capabilities",
								path: "/docs/ja/server-capabilities",
								title: "能力閲覧",
								component: () => import("./pages/ja/ServerCapabilities"),
							},
							{
								id: "server-inspector",
								path: "/docs/ja/server-inspector",
								title: "インスペクター",
								component: () => import("./pages/ja/ServerInspector"),
							},
							{
								id: "server-instances",
								path: "/docs/ja/server-instances",
								title: "インスタンス管理",
								component: () => import("./pages/ja/ServerInstances"),
							},
						],
					},
					{
						id: "market",
						path: "/docs/ja/market",
						title: "マーケット",
						component: () => import("./pages/ja/Market"),
					},
					{
						id: "runtime",
						path: "/docs/ja/runtime",
						title: "ランタイム",
						component: () => import("./pages/ja/Runtime"),
					},
					{
						id: "logs",
						path: "/docs/ja/logs",
						title: "監査ログ",
						component: () => import("./pages/ja/Logs"),
					},
					{
						id: "api-docs",
						path: "/docs/ja/api-docs",
						title: "API ドキュメント",
						component: () => import("./pages/ja/APIDocs"),
					},
					{
						id: "settings",
						path: "/docs/ja/settings",
						title: "設定",
						component: () => import("./pages/ja/Settings"),
					},
				],
			},
			{
				group: "",
				pages: [
					{
						id: "changelog",
						path: "/docs/ja/changelog",
						title: "変更履歴",
						component: () => import("./pages/ja/Changelog"),
					},
					{
						id: "roadmap",
						path: "/docs/ja/roadmap",
						title: "ロードマップ",
						component: () => import("./pages/ja/Roadmap"),
					},
				],
			},
		],
	},
];

export function flattenPages(nav: DocNav): DocPage[] {
	const out: DocPage[] = [];
	for (const g of nav.groups) {
		const walk = (node: DocGroup | DocPage) => {
			if ("path" in node) {
				out.push(node);
			} else {
				node.pages.forEach(walk);
			}
		};
		g.pages.forEach(walk);
	}
	return out;
}

export function findRouteByPath(
	list: DocPage[],
	path: string,
): DocPage | undefined {
	return list.find((p) => p.path === path);
}
