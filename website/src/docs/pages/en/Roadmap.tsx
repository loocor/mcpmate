import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "OAuth integrations",
		description:
			"We plan to accept external tokens, map scopes, and refresh sessions without custom plumbing.",
	},
	{
		title: "Client-specific safeguards",
		description:
			"Granular tool visibility and session isolation are being polished so each client only sees what it should.",
	},
	{
		title: "End-to-end activity logging",
		description:
			"A lightweight audit trail will capture every MCP interaction with optional redaction, giving teams confidence in shared environments.",
	},
	{
		title: "Cross-platform packaging",
		description:
			"macOS, Windows, and Linux installers with auto-updates and system service support are on the way.",
	},
	{
		title: "Configuration history",
		description:
			"Building on the current backup system, we plan to add preview, diff, and rollback options so teams can review before restoring.",
	},
	{
		title: "Smart profile suggestions",
		description:
			"We're refining recommendations that turn natural-language requests into ready-to-use tool bundles without manual toggling.",
	},
];

const onTheHorizon = [
	{
		title: "Built-in services",
		description:
			"Richer in-place MCP management services that streamline day-to-day maintenance without leaving the dashboard.",
	},
	{
		title: "Profile sharing",
		description:
			"Publish and import curated profile bundles so teams can reuse proven tool sets with a single click.",
	},
	{
		title: "Cost center",
		description:
			"Track and reconcile token consumption per MCP server, giving finance and ops a clear view of usage.",
	},
	{
		title: "Account layer",
		description:
			"Lightweight cloud sync and hosted options that keep configurations aligned across environments.",
	},
	{
		title: "Audit hub",
		description:
			"A centralized surface for reviewing recorded events, highlighting anomalies, and coordinating follow-up.",
	},
	{
		title: "Master-follower mode",
		description:
			"Designate follower nodes to mirror a primary instance, enabling coordinated rollouts inside larger teams.",
	},
	{
		title: "Sandbox mode",
		description:
			"Isolated environments with rate limits and capability allowlists to safely exercise high-risk tools.",
	},
];

const Roadmap = () => {
	const meta = {
		title: "Roadmap",
		description: "Snapshot of upcoming MCPMate experiences.",
	};

	return (
		<DocLayout meta={meta}>
			<div className="space-y-6">
				<h2>In Progress</h2>
				<p>
					These initiatives are actively being shaped. Expect early previews and
					alpha access as we validate the experience with real workflows.
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>On the Horizon</h2>
				<p>
					Below is the longer-term wishlist. We’re listening for feedback to
					help us confirm sequencing, so feel free to reach out if something
					catches your eye.
				</p>
				<ul className="space-y-2">
					{onTheHorizon.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>Stay in the Loop</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						We share milestones and early access sign-ups through release notes
						and the community newsletter. Subscribe or drop us a line if you’d
						like to pilot a specific capability.
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
