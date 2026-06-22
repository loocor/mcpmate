import DocLayout from "../../layout/DocLayout";

const currentFocus = [
	{
		title: "让 MCP Server 采用过程更安全",
		description:
			"用户应该能发现一个 Server、看懂它来自哪里、预览它会暴露什么能力，并在导入前判断这份配置是否值得信任。",
	},
	{
		title: "让客户端投放保持可控",
		description:
			"MCPMate 会继续明确哪些客户端获得哪些 Server、工具、资源与提示词，让本地 MCP 变更不再散落在多个配置文件里。",
	},
	{
		title: "把上手流程变成可观察的工作流",
		description:
			"一次配置变更前后都应该有足够证据：可读的来源上下文、dry-run 校验、凭证就绪状态、运行时状态，以及便于支持排障的诊断材料。",
	},
];

const nextBets = [
	{
		title: "可复用的团队工作流",
		description:
			"配置集与能力组合应该更容易共享、审阅和复用，让团队可以从验证过的 MCP 设置开始，而不是反复重建同一套操作模型。",
	},
	{
		title: "远程与分离式运行",
		description:
			"Core Server、控制台和未来远程入口需要形成更清晰的运行模型，服务那些已经超出单机桌面工作流的使用场景。",
	},
	{
		title: "更强的治理信号",
		description:
			"日志、审计证据、权限边界和高风险工具控制，需要帮助操作者理解发生了什么变化、谁或什么可以使用它，以及什么时候需要介入。",
	},
	{
		title: "更聪明的工作流辅助",
		description:
			"Inspector 驱动的检查、类似 Skills 的工作流，以及 Prompt 或 Provider 辅助能力，可以减少手动配置，但必须保持可解释并受操作者控制。",
	},
	{
		title: "用量与成本可见性",
		description:
			"更长期来看，MCPMate 应该让 Server 级别的使用模式与 Token 成本权衡足够可见，帮助操作者更有信心地调整能力暴露范围。",
	},
];

const shippedFoundation = [
	{
		title: "导入与发现基础",
		description:
			"浏览器发现、GitHub MCP 导入、Cursor.directory 交接、Market README 展示、来源元数据、多 Server 导入预览和 dry-run 校验，已经组成第一条端到端采用路径。",
	},
	{
		title: "凭证与 OAuth 托管",
		description:
			"Secure Store、OAuth Token 托管、生命周期视图、降级状态提示、重连提醒和清理控制，已经把敏感 Server 状态从明文配置文件中移出。",
	},
	{
		title: "托管式客户端配置",
		description:
			"配置集、批量 include / exclude、后端维护的兼容标准、诊断导出和更稳的 Inspector 生命周期，让 MCP 变更更容易审阅和支持。",
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
				<h2>当前重点</h2>
				<p>
					MCPMate 当前关注的是让 MCP 采用不再像手动编辑分散的客户端配置文件，而更像一个受管理的工作流：发现可用能力、验证即将发生的变化，并把合适能力暴露给合适客户端。
				</p>
				<ul className="space-y-2">
					{currentFocus.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>下一阶段方向</h2>
				<p>
					这些是战略方向，不是具体版本承诺。我们会结合真实使用、支持反馈与发布约束来决定优先级。
				</p>
				<ul className="space-y-2">
					{nextBets.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>近期已形成的基础</h2>
				<p>
					0.3.x 的重点是为这条工作流打基础。更新日志仍然保留完整版本记录；本页只保留产品层面的关键积木。
				</p>
				<ul className="space-y-2">
					{shippedFoundation.map((item) => (
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
