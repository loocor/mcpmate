import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "桌面发布链路",
		description:
			"我们正在继续打磨以 GitHub Releases 为起点的交付路径，包括自动更新行为、预发布处理，以及 macOS、Windows、Linux 三端的打包一致性。",
	},
	{
		title: "平台成熟度补齐",
		description:
			"当前 macOS 仍是最稳定的桌面外壳；接下来的重点，是让 Windows 与 Linux 在安装包、运行时表现和桌面体验上尽快追平。",
	},
	{
		title: "容器与分离部署",
		description:
			"我们正在增强适合容器化交付的核心服务形态，并继续完善 Core Server / UI 分离运行的远程与多机部署路径。",
	},
	{
		title: "客户端治理打磨",
		description:
			"针对已检测客户端的发布流程、可写目标校验，以及应用 / 清理路径，我们还在持续优化，让托管式变更更容易信任。",
	},
	{
		title: "文档与引导对齐",
		description:
			"网站、快速开始和仪表盘文案会继续随着真实已交付行为保持同步，避免发布链路变化后出现过时引导。",
	},
];

const exploringNext = [
	{
		title: "内置自动更新体验",
		description:
			"在首条发布链路已经成形之后，下一步是把桌面更新做得更顺手、更接近日常习惯。",
	},
	{
		title: "配置集共享",
		description:
			"我们希望团队可以复用已经验证过的配置集组合，而不是每次都重新搭一遍能力集合。",
	},
	{
		title: "轻量账户层",
		description:
			"可选的账户关联能力与轻量云同步仍然值得探索，但前提是继续保持 MCPMate 的本地优先边界。",
	},
	{
		title: "更安全的沙箱控制",
		description:
			"对于高风险工具，我们在评估更细的隔离与审批护栏，让能力暴露更可控。",
	},
	{
		title: "用量与成本可见性",
		description:
			"更长期来看，我们希望让运维侧更容易看清服务器级别的使用模式与 Token 成本权衡。",
	},
];

const Roadmap = () => {
	const meta = {
		title: "开发规划",
		description: "MCPMate 接下来重点改进的方向。",
	};

	return (
		<DocLayout meta={meta}>
			<div className="space-y-6">
				<h2>进行中</h2>
				<p>
					这一组工作最贴近当前用户体验：发布交付、平台成熟度、客户端治理安全性，以及更清晰的上手路径。
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>近期已交付</h2>
				<ul className="space-y-2">
					<li>
						现已支持面向 Streamable HTTP MCP 服务的 OAuth 上游能力，包括元数据发现、授权流程与令牌刷新。
					</li>
					<li>
						审计日志已经上线，支持筛选与游标分页；同时，Core Server 与 UI 也已经可以分离运行，适合拆分部署。
					</li>
					<li>
						服务源与导入流程现在带有更丰富的注册表元数据、更完整的预览细节，以及浏览器辅助片段捕获能力。
					</li>
					<li>
						桌面分发现已具备基于 GitHub Releases 的交付路径，并整合了打包流程与容器发布覆盖。
					</li>
				</ul>

				<h2>继续评估中的方向</h2>
				<p>
					下面这些更像明确的候选方向，而不是硬性承诺。我们会结合真实反馈与发布约束来决定先后顺序。
				</p>
				<ul className="space-y-2">
					{exploringNext.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>跟踪最新进展</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						如果你想看最接近真实落地状态的信号，优先关注 GitHub Releases 与更新日志；那里反映的是已经交付的内容，而本页更多描述的是我们正在塑形的方向。
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
