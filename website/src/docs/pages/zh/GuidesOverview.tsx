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
				本节与 MCPMate 控制台（桌面应用内嵌的 <code>board/</code> 前端，或本地连接代理的 Web
				控制台）的实际结构一致。每篇文章说明该页的职责、依赖的代理接口以及常见操作路径，便于对照界面理解。
			</P>

			<Callout type="info" title="指南如何映射到产品">
				控制台、配置集、客户端、服务器、市场、运行时与侧边栏顺序一致。「API
				文档」需在「设置 → 开发者」中开启后才会出现在侧栏底部；其下还有「账户」与「设置」。建议阅读时同时打开控制台对照操作。
			</Callout>

			<H2>控制台布局：侧栏与顶栏</H2>
			<H3>侧栏</H3>
			<Ul>
				<Li>
					<strong>MAIN</strong> 自上而下：控制台、配置集、客户端、服务器、市场、运行时。不再提供单独的「工具」顶级入口；工具、提示、资源与模板均在各服务器的详情页中分标签管理。
				</Li>
				<Li>
					底部区域：<strong>API 文档</strong>（默认打开{" "}
					<code>http://127.0.0.1:8080/docs</code>，随代理端口而变）、<strong>账户</strong>（见下）、<strong>设置</strong>。
				</Li>
			</Ul>
			<H3>顶栏</H3>
			<Ul>
				<Li>一级页面显示标题；子页面显示返回上一级。</Li>
				<Li>
					<strong>反馈</strong> 打开预填收件人与正文的邮件，便于向团队提交意见。
				</Li>
				<Li>
					<strong>文档</strong> 在新标签打开 <code>mcp.umate.ai</code> 上的公开指南，并尽量跳转到与当前页面相关的章节。
				</Li>
				<Li>
					<strong>主题</strong> 切换亮/暗色；<strong>通知</strong> 汇总近期应用内提示（成功、警告、错误），便于在气泡消失后回看。
				</Li>
			</Ul>
			<H3>账户（桌面端）</H3>
			<P>
				在 macOS 的 Tauri 桌面版中，可通过 <strong>账户</strong> 关联 GitHub，为后续云端相关能力预留身份。浏览器内单独运行的控制台会显示同一入口，但实际登录仅在打包应用中可用；条款与隐私链接指向{" "}
				<code>mcp.umate.ai</code>。
			</P>

			<H2>导航支柱</H2>
			<Ul>
				<Li>
					<strong>控制台</strong> —— 系统状态、配置集、服务器与客户端四张摘要卡（约每 30
					秒轮询），以及约每 10 秒采样的 CPU/内存趋势图。
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
					<strong>API 文档</strong> —— 侧栏可选入口，打开代理托管的 OpenAPI 交互文档（默认路径{" "}
					<code>/docs</code>）。
				</Li>
				<Li>
					<strong>设置</strong> —— 外观、界面语言（中/英/日）、默认列表视图、客户端与市场默认值、开发者开关与系统端口等。
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
