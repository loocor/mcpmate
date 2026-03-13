import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function ResourceOptimization() {
	return (
		<DocLayout
			meta={{
				title: "资源优化",
				description: "智能管理服务器资源，减少系统开销并提高性能",
			}}
		>
			<P>
				MCPMate
				智能管理服务器资源，在最大化性能的同时最小化系统开销。通过智能池化和生命周期管理，MCPMate
				确保您的 MCP 服务器高效运行。
			</P>

			<h2>主要特性</h2>
			<ul>
				<li>
					<strong>连接池：</strong>在多个客户端之间共享服务器实例
				</li>
				<li>
					<strong>自动生命周期管理：</strong>按需启动服务器，不使用时停止
				</li>
				<li>
					<strong>内存优化：</strong>通过智能资源共享减少内存占用
				</li>
				<li>
					<strong>性能监控：</strong>跟踪资源使用并识别优化机会
				</li>
			</ul>

			<h2>优势</h2>
			<P>
				MCPMate
				可以在所有应用程序之间共享单个服务器实例，而不是为不同客户端运行同一服务器的多个实例，从而大幅降低
				CPU 和内存使用率。
			</P>

			<P>性能基准测试和优化技巧即将推出。</P>
		</DocLayout>
	);
}
