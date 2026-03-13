import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function GuidesOverview() {
	return (
		<DocLayout
			meta={{
				title: "操作指南概览",
				description: "学习如何高效使用 MCPMate",
			}}
		>
			<P>
				本节与 MCPMate 控制台（<code>board/</code> 子项目）的侧边栏导航完全一致。每篇文章都会说明页面要解决的核心问题、面板数据来源以及常见验证步骤，方便你一边阅读一边在仪表盘实操。
			</P>

			<Callout type="info" title="指南如何映射到产品">
				控制台、配置集、客户端、服务器、市场、运行时、API 文档与设置文章的顺序，与应用内左侧导航保持同步。建议在浏览文档时同时打开控制台页面，随读随点更容易理解。
			</Callout>

			<H2>导航支柱</H2>
			<Ul>
				<Li>
					<strong>控制台</strong> —— 每 30 秒刷新代理健康、活跃配置集以及 CPU/内存曲线。
				</Li>
				<Li>
					<strong>配置集</strong> —— 管理可复用的套件，支持激活开关、统计卡片与详情抽屉。
				</Li>
				<Li>
					<strong>客户端</strong> —— 追踪检测到的编辑器，切换托管状态并查看 MCP 配置路径。
				</Li>
				<Li>
					<strong>服务器</strong> —— 查看能力摘要、启停实例，并通过 Uni-Import 或手动表单导入新服务器。
				</Li>
				<Li>
					<strong>市场</strong> —— 浏览官方/自建门户、搜索筛选条目并一键送入安装向导。
				</Li>
				<Li>
					<strong>运行时</strong> —— 监控 uv 与 Bun 状态、清理缓存、查看能力缓存统计。
				</Li>
				<Li>
					<strong>API 文档</strong> —— 打开运行中代理所提供的 REST 与 MCP 参考。
				</Li>
				<Li>
					<strong>设置</strong> —— 调整外观、默认视图、市场偏好与开发者开关。
				</Li>
			</Ul>

			<H2>推荐路径</H2>
			<H3>1. 先观察再决策</H3>
			<P>
				从控制台与运行时入手，确认代理是否稳定运行、资源占用是否异常，再决定要深入排查的对象。
			</P>

			<H3>2. 配置要运行的内容</H3>
			<P>
				依次进入配置集、服务器、客户端三大管理页。它们共享同一套工具栏（搜索、排序、视图切换、筛选器），熟悉一种页面后即可无缝迁移。
			</P>

			<H3>3. 拓展能力</H3>
			<P>
				在市场里导入新的 MCP 服务器，随后到运行时确认环境与缓存正常，最后在设置里固定默认视图、市场门户以及 API 文档入口。
			</P>

			<H2>通用交互模式</H2>
			<Ul>
				<Li>
					<strong>统计概览卡</strong> —— 每个管理页面都会用四个卡片给出激活数、启用数与实例总数。
				</Li>
				<Li>
					<strong>工具栏</strong> —— 统一提供搜索、排序、列表/网格切换以及可选过滤条件。
				</Li>
				<Li>
					<strong>抽屉式操作</strong> —— 创建新配置集或安装服务器时使用侧边抽屉，保证上下文不丢失。
				</Li>
				<Li>
					<strong>调试助手</strong> —— 在设置 → 开发者中开启后，可查看原始 JSON、日志或导入 payload，便于排查。
				</Li>
			</Ul>

			<Callout type="warning" title="前置条件">
				请先在 <code>backend/</code> 中执行 <code>cargo run -p app-mcpmate</code>（或连接到既有部署），确保 <code>http://127.0.0.1:8080</code> 可访问。若后端未启动，卡片和图表会停留在空白或旧数据状态。
			</Callout>
		</DocLayout>
	);
}
