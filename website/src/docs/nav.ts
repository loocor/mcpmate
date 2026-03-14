export type Locale = "en" | "zh";

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
						title: "Inline Marketplace",
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
						id: "profile",
						path: "/docs/en/profile",
						title: "Profiles",
						component: () => import("./pages/en/Profile"),
					},
					{
						id: "clients",
						path: "/docs/en/clients",
						title: "Clients",
						component: () => import("./pages/en/ClientApps"),
					},
					{
						id: "servers",
						path: "/docs/en/servers",
						title: "Servers",
						component: () => import("./pages/en/Servers"),
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
						title: "内联商城",
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
						id: "profile",
						path: "/docs/zh/profile",
						title: "配置集",
						component: () => import("./pages/zh/Profile"),
					},
					{
						id: "clients",
						path: "/docs/zh/clients",
						title: "客户端",
						component: () => import("./pages/zh/ClientApps"),
					},
					{
						id: "servers",
						path: "/docs/zh/servers",
						title: "服务器",
						component: () => import("./pages/zh/Servers"),
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
