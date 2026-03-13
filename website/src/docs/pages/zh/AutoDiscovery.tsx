import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "自动发现与导入",
				description: "自动检测并导入现有配置，无需手工编辑",
			}}
		>
			<P>
				MCPMate 可以自动发现系统上现有的 MCP
				服务器配置，并一键导入。这消除了在新工具中手动重新创建设置的繁琐过程。
			</P>

			<h2>工作原理</h2>
			<P>MCPMate 扫描流行 MCP 客户端使用的常见配置位置：</P>
			<ul>
				<li>Claude Desktop 配置文件</li>
				<li>VS Code MCP 扩展设置</li>
				<li>Cursor MCP 配置</li>
				<li>其他标准 MCP 客户端设置</li>
			</ul>

			<h2>导入流程</h2>
			<ol>
				<li>MCPMate 自动扫描现有配置</li>
				<li>在导入界面中显示发现的服务器</li>
				<li>您查看并选择要导入的服务器</li>
				<li>MCPMate 将配置导入到您的活动配置集中</li>
			</ol>

			<h2>优势</h2>
			<ul>
				<li>
					<strong>快速上手：</strong>立即开始使用 MCPMate
				</li>
				<li>
					<strong>无需手动工作：</strong>避免手动复制配置详细信息
				</li>
				<li>
					<strong>保留现有设置：</strong>您的原始配置保持不变
				</li>
				<li>
					<strong>防止错误：</strong>减少手动输入导致的配置错误
				</li>
			</ul>

			<P>自动发现演练和截图即将推出。</P>
		</DocLayout>
	);
}
