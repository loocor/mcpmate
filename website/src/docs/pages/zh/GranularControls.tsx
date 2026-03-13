import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function GranularControls() {
	return (
		<DocLayout
			meta={{
				title: "精细控制",
				description: "逐项开关每个能力，按需启用/禁用",
			}}
		>
			<P>
				MCPMate 对 MCP
				服务器的每个方面都提供细粒度控制。您可以有选择地启用或禁用每个服务器内的单个工具、提示词和资源，而不是采用全有或全无的方法。
			</P>

			<h2>控制级别</h2>
			<ul>
				<li>
					<strong>服务器级别：</strong>启用或禁用整个服务器
				</li>
				<li>
					<strong>能力级别：</strong>独立切换工具、提示词和资源
				</li>
				<li>
					<strong>单项级别：</strong>控制服务器内的特定工具或提示词
				</li>
			</ul>

			<h2>使用场景</h2>
			<ul>
				<li>
					<strong>安全性：</strong>禁用潜在的危险操作
				</li>
				<li>
					<strong>性能：</strong>通过禁用未使用的功能来减少开销
				</li>
				<li>
					<strong>专注：</strong>隐藏无关工具以减少混乱
				</li>
				<li>
					<strong>测试：</strong>在开发期间逐步启用功能
				</li>
			</ul>

			<h2>优势</h2>
			<P>
				精细控制使您能够精确控制客户端可用的功能。这在团队环境中特别有价值，因为不同用户可能需要不同级别的访问权限。
			</P>

			<P>详细的控制界面文档即将推出。</P>
		</DocLayout>
	);
}
