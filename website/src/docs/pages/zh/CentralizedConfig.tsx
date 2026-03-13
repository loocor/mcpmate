import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function CentralizedConfig() {
	return (
		<DocLayout
			meta={{
				title: "集中配置",
				description: "一次配置，随处使用。消除跨不同客户端的重复设置",
			}}
		>
			<P>
				MCPMate
				的核心功能之一是集中配置管理。您无需为每个 MCP
				客户端维护单独的配置，只需在 MCPMate
				中配置一次服务器，它们就会自动在所有连接的客户端中可用。
			</P>

			<h2>优势</h2>
			<ul>
				<li>
					<strong>单一数据源：</strong>所有 MCP 服务器配置都在一个地方管理
				</li>
				<li>
					<strong>无需重复：</strong>消除在不同客户端之间复制设置的需要
				</li>
				<li>
					<strong>一致体验：</strong>确保所有客户端使用相同的服务器配置
				</li>
				<li>
					<strong>轻松更新：</strong>一次更改配置即可应用到所有地方
				</li>
			</ul>

			<h2>工作原理</h2>
			<P>
				MCPMate 充当所有 MCP 服务器的中心枢纽。当您在 MCPMate
				中配置服务器时，它会自动对所有连接的客户端可用。这消除了为每个客户端应用程序手动编辑配置文件的传统工作流程。
			</P>

			<h2>使用场景</h2>
			<ul>
				<li>在 Claude Desktop、Cursor 和 VS Code 中使用相同的服务器</li>
				<li>管理团队范围的服务器配置</li>
				<li>快速接入新客户端而无需重新配置</li>
			</ul>

			<P>详细的配置示例即将推出。</P>
		</DocLayout>
	);
}
