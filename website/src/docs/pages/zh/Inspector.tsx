import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function Inspector() {
	return (
		<DocLayout
			meta={{
				title: "检视器",
				description: "无需离开控制台，即可深入查看服务器状态、日志和诊断信息",
			}}
		>
			<P>
				MCPMate 检视器提供了一个强大的界面，用于监控和调试您的 MCP
				服务器。实时了解服务器行为，检查日志，诊断问题——所有这些都可以在
				MCPMate 控制台内完成。
			</P>

			<h2>功能特性</h2>
			<ul>
				<li>
					<strong>实时监控：</strong>实时观察服务器活动
				</li>
				<li>
					<strong>日志查看器：</strong>浏览和搜索服务器日志
				</li>
				<li>
					<strong>请求/响应检视器：</strong>详细检查 MCP 协议消息
				</li>
				<li>
					<strong>性能指标：</strong>跟踪响应时间和资源使用情况
				</li>
				<li>
					<strong>错误诊断：</strong>快速识别和排查问题
				</li>
			</ul>

			<h2>使用场景</h2>
			<ul>
				<li>调试服务器配置问题</li>
				<li>监控生产环境中的服务器性能</li>
				<li>理解 MCP 协议交互</li>
				<li>排查客户端-服务器通信问题</li>
			</ul>

			<P>详细的检视器界面文档即将推出。</P>
		</DocLayout>
	);
}
