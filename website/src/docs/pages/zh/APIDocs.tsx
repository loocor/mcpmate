import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function APIDocsZH() {
	return (
		<DocLayout meta={{ title: "API 文档", description: "REST 与 MCP 接口参考" }}>
			<P>
				MCPMate 在 <code>http://127.0.0.1:8080/docs</code> 暴露交互式文档（由后端代理生成）。本页说明如何在控制台显示快捷入口以及进站后可以使用的能力。
			</P>

			<H2>启用侧边栏入口</H2>
			<Ul>
				<Li>前往 “设置 → 开发者”，打开“显示 API 文档菜单”。侧边栏会出现新的快捷链接。</Li>
				<Li>若已调整后端端口（设置 → 系统），请同步更新 API 基址，确保链接指向正确的服务。</Li>
				<Li>点击链接会在新标签页打开文档；也可直接访问该 URL 以便收藏。</Li>
			</Ul>

			<H2>文档内容</H2>
			<H3>REST 操作</H3>
			<P>
				OpenAPI 视图按模块划分接口（系统、服务器、配置集、运行时、市场等）。展开条目可以查看请求/响应结构，并通过 Try
				It 按钮在线调用。
			</P>

			<H3>MCP 相关端点</H3>
			<P>
				文档同样列出 MCP 传输端点（HTTP SSE、WebSocket 桥接），用于核对头信息、载荷字段与认证要求，方便编写客户端。
			</P>

			<H2>使用建议</H2>
			<Ul>
				<Li>测试接口时保持控制台页面开启，便于立即观察配置或服务器状态的变化。</Li>
				<Li>执行破坏性操作前记录请求示例，方便团队复现与回滚。</Li>
				<Li>如后续版本启用鉴权，可通过文档页面的 Authorize 按钮注入 token，与实际调用保持一致。</Li>
			</Ul>

			<Callout type="warning" title="必须先启动代理">
				仅当后端进程运行时 <code>/docs</code> 才可访问。若页面无法加载，请确认已执行{" "}
				<code>cargo run -p app-mcpmate</code>（或连接到部署环境），并确保浏览器能访问所配置的 API 端口。
			</Callout>
		</DocLayout>
	);
}
