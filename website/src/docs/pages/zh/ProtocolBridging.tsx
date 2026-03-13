import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function ProtocolBridging() {
	return (
		<DocLayout
			meta={{
				title: "协议桥接",
				description: "无需修改客户端即可将基于 stdio 的客户端连接到 SSE 服务",
			}}
		>
			<P>
				MCPMate 的协议桥接功能允许基于 stdio 的 MCP 客户端连接到基于服务器发送事件
				(SSE) 的服务，而无需修改客户端代码。这使得部署和使用 MCP
				服务器具有更大的灵活性。
			</P>

			<h2>工作原理</h2>
			<P>
				MCPMate
				充当不同传输协议之间的透明桥接。当基于 stdio 的客户端连接到 MCPMate
				时，它可以与 SSE 服务器通信，就像它们是原生 stdio
				服务器一样。协议转换在后台无缝进行。
			</P>

			<h2>使用场景</h2>
			<ul>
				<li>
					<strong>远程服务器访问：</strong>将本地客户端连接到云托管的 MCP 服务器
				</li>
				<li>
					<strong>混合部署：</strong>在同一工作流程中混合使用本地和远程服务器
				</li>
				<li>
					<strong>旧客户端支持：</strong>在仅支持 stdio 的旧客户端上使用现代 SSE
					服务器
				</li>
				<li>
					<strong>服务迁移：</strong>在不中断客户端的情况下从 stdio 逐步迁移到 SSE
				</li>
			</ul>

			<h2>优势</h2>
			<ul>
				<li>无需更改客户端代码</li>
				<li>所有传输类型的统一界面</li>
				<li>实现灵活的部署架构</li>
				<li>为您的 MCP 基础设施做好未来准备</li>
			</ul>

			<P>协议桥接配置示例即将推出。</P>
		</DocLayout>
	);
}
