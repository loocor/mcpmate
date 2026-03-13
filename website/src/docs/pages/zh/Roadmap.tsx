import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "OAuth 对接",
		description:
			"计划支持外部身份体系接入，实现令牌校验、刷新与能力映射。",
	},
	{
		title: "客户端隔离",
		description:
			"我们正在完善工具清单隔离与会话策略，确保不同客户端只看到自己的能力范围。",
	},
	{
		title: "全链路操作留痕",
		description:
			"即将上线的审计轨迹会记录每一次 MCP 调用，并支持脱敏与最小化开销，方便团队共享环境协作。",
	},
	{
		title: "跨平台发行",
		description:
			"macOS、Windows、Linux 的原生安装包、自动更新与系统服务集成已在规划中。",
	},
	{
		title: "配置历史管理",
		description:
			"在当前配置历史备份管理的基础上提供预览、差异比对、回滚能力，方便恢复评估。",
	},
	{
		title: "智能配置推荐",
		description:
			"正在打磨的自然语言配置助手，将自动组合工具套件并一键激活，无需手动切换开关。",
	},
];

const onTheHorizon = [
	{
		title: "内置服务",
		description:
			"提供更完备的内置 MCP 管理服务，可在控制台内完成丝滑的就地管理。",
	},
	{
		title: "配置集共享",
		description:
			"允许将自己组合的配置集分享给他人导入，打造更高效的协作流程。",
	},
	{
		title: "成本中心",
		description:
			"记录并核算每个 MCP 服务器的 Token 消耗，帮助团队掌握运营成本。",
	},
	{
		title: "账户体系",
		description:
			"提供配置同步、云端轻量托管等能力，让多端保持一致体验。",
	},
	{
		title: "审计中心",
		description:
			"集中展示审计事件、标记风险并协调后续处理，提升安全透明度。",
	},
	{
		title: "主从管理",
		description:
			"支持设置从属模式，方便团队在多节点之间协作与统一管理。",
	},
	{
		title: "沙箱模式",
		description:
			"隔离的执行环境、速率限制与能力白名单将为高风险工具提供更多保障。",
	},
];

const Roadmap = () => {
	const meta = {
		title: "开发规划",
		description: "面向用户的 MCPMate 远景概览。",
	};

	return (
		<DocLayout meta={meta}>
			<div className="space-y-6">
				<h2>进行中</h2>
				<p>这些能力正在打磨阶段，后续会第一时间通过内测邀请或更新日志向大家开放。</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>规划中</h2>
				<p>以下是中长期的愿望清单，我们会根据用户反馈与落地难度不断调整顺序。</p>
				<ul className="space-y-2">
					{onTheHorizon.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>欢迎持续关注</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						我们会通过版本公告与社区通讯分享最新进度。如希望抢先体验某一项功能，欢迎与我们联系。
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
