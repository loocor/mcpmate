import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function ContextSwitching() {
	return (
		<DocLayout
			meta={{
				title: "无缝上下文切换",
				description: "通过即时配置更改在不同工作场景之间切换",
			}}
		>
			<P>
				MCPMate
				通过其配置集系统实现无缝的上下文切换。即时在不同的服务器和配置集之间切换，以匹配您当前的工作场景——无论您是在编码、写作、研究还是协作。
			</P>

			<h2>工作原理</h2>
			<P>
				在 MCPMate
				中创建多个配置集，每个配置集都有自己的服务器和设置。只需单击即可在配置集之间切换，所有连接的客户端都会自动更新以使用新配置。
			</P>

			<h2>示例场景</h2>
			<ul>
				<li>
					<strong>开发配置集：</strong>代码辅助、文件操作、Git 集成
				</li>
				<li>
					<strong>写作配置集：</strong>语法检查、研究工具、引文管理
				</li>
				<li>
					<strong>分析配置集：</strong>数据处理、可视化、统计工具
				</li>
				<li>
					<strong>团队配置集：</strong>共享资源和协作工具
				</li>
			</ul>

			<h2>优势</h2>
			<ul>
				<li>无需手动重新配置客户端</li>
				<li>通过只关注相关工具来减少认知负担</li>
				<li>通过只加载所需内容来优化性能</li>
				<li>为不同任务创建专门的工作流程</li>
			</ul>

			<P>配置集管理最佳实践即将推出。</P>
		</DocLayout>
	);
}
