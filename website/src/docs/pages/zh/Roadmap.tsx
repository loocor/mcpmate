import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "安全上手链路收尾",
		description:
			"0.3.0 已经带来 Secure Store 与批量导入基础，接下来会继续打磨 OAuth 凭证归属、secret 生命周期清理，以及剩余的服务器字段绑定细节。",
	},
	{
		title: "桌面发布链路",
		description:
			"我们正在继续打磨以 GitHub Releases 为起点的交付路径，包括自动更新行为、预发布处理，以及 macOS、Windows、Linux 三端的打包一致性。",
	},
	{
		title: "平台成熟度补齐",
		description:
			"macOS、Windows 与 Linux 桌面构建目前均按 Beta 呈现；接下来的重点，是继续统一三端的安装包行为、运行时检测和桌面体验细节。",
	},
	{
		title: "客户端治理与投放安全",
		description:
			"针对已检测客户端的发布流程、可写目标校验、attach / detach 路径，以及能力批量调整，我们还在持续优化，让托管式变更更容易信任。",
	},
	{
		title: "文档与引导对齐",
		description:
			"网站、快速开始、浏览器扩展安装路径和控制台文案会继续随着真实已交付行为保持同步，避免发布链路变化后出现过时引导。",
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
		title: "容器与分离部署打磨",
		description:
			"Core Server 与 UI 已经可以分离运行；后续会继续让远程、容器和多机部署更容易打包、解释与使用。",
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
					0.3.0 beta 基础落地后，当前最贴近用户体验的工作是安全上手链路打磨、发布交付、平台成熟度、客户端投放安全，以及更清晰的上手路径。
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>近期已交付</h2>
				<ul className="space-y-2">
					<li>
						Secure Store 现在会把敏感服务器参数移出明文配置文件，在运行时解析加密 secret 记录，并在仪表盘中提供保护与使用位置相关流程。
					</li>
					<li>
						Server 安装流程现在可以接收类似 <code>mcp-servers.json</code> 的多服务器配置包，并让用户逐项查看草稿、预览服务器、执行 dry-run 校验，再导入选中的配置。
					</li>
					<li>
						配置集现在支持对服务器、工具、资源、提示和资源模板进行批量 include / exclude。
					</li>
					<li>
						首次使用与新增客户端流程现在使用后端维护的兼容标准，让用户可以拿到更新、更匹配的客户端配置。
					</li>
					<li>
						自动刷新基础能力已进一步增强，包括面向已授权 Streamable HTTP 服务器的 OAuth Token 刷新。
					</li>
					<li>
						桌面端诊断导出让用户在需要支持排障时，可以用更干净的方式提供本地反馈材料。
					</li>
					<li>
						检视器生命周期管理与注册中心安装处理已经加固，减少重复执行、状态误导和错误安装草稿。
					</li>
					<li>
						浏览器扩展、首次使用与网站文档已经更新，让安装和升级路径更容易跟随。
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
