import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function FeaturesOverview() {
	return (
		<DocLayout
			meta={{
				title: "功能特性概览",
				description: "探索 MCPMate 的强大功能",
			}}
		>
			<P>
				MCPMate
				提供了一整套功能特性，旨在让 MCP 服务器的使用更简单、更高效、更强大。
			</P>

			<h2>核心功能</h2>
			<P>
				我们的功能特性涵盖了从集中配置、资源优化到高级工具和无缝集成等方方面面。每个功能都以用户体验和开发者生产力为设计核心。
			</P>

			<h3>配置与管理</h3>
			<ul>
				<li>
					<strong>集中配置：</strong>一次配置，随处使用，跨所有客户端生效
				</li>
				<li>
					<strong>无缝上下文切换：</strong>在不同工作场景之间瞬间切换
				</li>
				<li>
					<strong>精细控制：</strong>通过精确的开关微调每一项能力
				</li>
			</ul>

			<h3>性能与优化</h3>
			<ul>
				<li>
					<strong>资源优化：</strong>智能的服务器资源管理，带来更好的性能
				</li>
				<li>
					<strong>协议桥接：</strong>无需修改即可将基于 stdio 的客户端连接到 SSE
					服务
				</li>
			</ul>

			<h3>开发者工具</h3>
			<ul>
				<li>
					<strong>检视器：</strong>深入了解服务器状态、日志和诊断信息
				</li>
				<li>
					<strong>自动发现与导入：</strong>自动检测并导入现有配置
				</li>
				<li>
					<strong>全能导入：</strong>通过拖放或粘贴轻松配置
				</li>
			</ul>

			<h3>生态系统</h3>
			<ul>
				<li>
					<strong>内联商城：</strong>内建官方注册中心与 mcpmarket.cn 集成
				</li>
			</ul>

			<P>
				通过下方的各个章节深入了解每个功能特性，探索 MCPMate 如何增强您的 MCP
				工作流程。
			</P>
		</DocLayout>
	);
}
