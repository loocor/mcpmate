import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

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

			<H2>工作原理</H2>
			<P>
				在 MCPMate
				中创建多个配置集，每个配置集都有自己的服务器和设置。只需单击即可在配置集之间切换，所有连接的客户端都会自动更新以使用新配置。
			</P>

			<H2>示例场景</H2>
			<Ul>
				<Li>
					<strong>开发配置集：</strong>代码辅助、文件操作、Git 集成
				</Li>
				<Li>
					<strong>写作配置集：</strong>语法检查、研究工具、引文管理
				</Li>
				<Li>
					<strong>分析配置集：</strong>数据处理、可视化、统计工具
				</Li>
				<Li>
					<strong>团队配置集：</strong>共享资源和协作工具
				</Li>
			</Ul>

			<H2>落地建议</H2>
			<Ul>
				<Li>至少保留一个默认锚点配置集，作为基础兜底能力。</Li>
				<Li>需要即时切换时优先使用托管模式；若希望改用会话内建控制而不是仪表板侧配置集切换，则使用统一模式。</Li>
				<Li>分离部署场景先确认 API 连通，再执行配置集切换。</Li>
				<Li>关键切换后到审计日志复核时序，便于多人协作追踪。</Li>
			</Ul>

			<P>
				推荐流程是：先预建场景化配置集，再按任务切换，最后通过审计日志做操作回溯。
			</P>
		</DocLayout>
	);
}
