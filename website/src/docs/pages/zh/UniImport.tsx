import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function UniImport() {
	return (
		<DocLayout
			meta={{
				title: "全能导入",
				description: "拖拽或粘贴轻松配置；支持 JSON/TOML，mcpb 即将支持",
			}}
		>
			<P>
				全能导入是 MCPMate
				的灵活配置导入系统。无论您有 JSON 文件、TOML 配置还是文本片段，都可以通过简单的拖放或粘贴操作将其导入到
				MCPMate 中。
			</P>

			<h2>支持的格式</h2>
			<ul>
				<li>
					<strong>JSON：</strong>标准 MCP 配置格式
				</li>
				<li>
					<strong>TOML：</strong>替代配置格式
				</li>
				<li>
					<strong>MCPB：</strong>即将推出 - MCPMate 的打包格式，用于共享完整设置
				</li>
			</ul>

			<h2>导入方式</h2>
			<ul>
				<li>
					<strong>拖放：</strong>只需将配置文件拖放到 MCPMate 中
				</li>
				<li>
					<strong>粘贴：</strong>复制配置文本并粘贴到导入对话框中
				</li>
				<li>
					<strong>文件浏览器：</strong>传统的文件选择对话框
				</li>
			</ul>

			<h2>智能解析</h2>
			<P>
				全能导入会自动检测配置格式并在导入前进行验证。如果有任何问题，MCPMate
				会提供清晰的错误消息和修复建议。
			</P>

			<h2>使用场景</h2>
			<ul>
				<li>导入共享的团队配置</li>
				<li>从其他 MCP 工具迁移</li>
				<li>从文档示例快速设置</li>
				<li>从备份恢复</li>
			</ul>

			<P>全能导入示例和支持的架构即将推出。</P>
		</DocLayout>
	);
}
