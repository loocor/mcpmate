import { Link } from "react-router-dom";

import DocLayout from "../../layout/DocLayout";

const currentFocus = [
	{
		title: "Make MCP server adoption safer",
		description:
			"Users should be able to discover a server, understand where it came from, preview what it exposes, and import it without guessing whether the configuration is safe to trust.",
	},
	{
		title: "Keep client rollouts under control",
		description:
			"MCPMate is moving toward clearer control over which clients receive which servers, tools, resources, and prompts, so local MCP changes stay intentional instead of scattered across config files.",
	},
	{
		title: "Turn setup into an observable workflow",
		description:
			"The setup path should show enough evidence before and after a change: readable source context, dry-run checks, credential readiness, runtime state, and support-friendly diagnostics.",
	},
	{
		title: "Build a Standalone Inspector",
		description:
			"A focused Inspector entry point should make it possible to connect to an MCP Server, discover its capabilities, and verify calls without first entering the full management workflow.",
	},
];

const nextBets = [
	{
		title: "Reusable team workflows",
		description:
			"Profiles and capability sets should become easier to share, review, and reuse so teams can start from proven MCP setups instead of rebuilding the same operating model repeatedly.",
	},
	{
		title: "Remote and split operation",
		description:
			"Core Server, dashboard, and future remote entry points should be easier to package as a clear operating model for users who outgrow a single local desktop workflow.",
	},
	{
		title: "Stronger governance signals",
		description:
			"Logs, audit evidence, permission boundaries, and higher-risk tool controls should help operators understand what changed, who or what can use it, and when intervention is needed.",
	},
	{
		title: "Smarter workflow assistance",
		description:
			"Inspector-driven checks, skill-like workflows, and prompt or provider helpers can reduce manual setup work when they stay explainable and remain under operator control.",
	},
	{
		title: "Usage and cost visibility",
		description:
			"Longer term, MCPMate should make server-level usage patterns and token cost tradeoffs visible enough for operators to tune tool exposure with confidence.",
	},
];

const shippedFoundation = [
	{
		title: "Import and discovery foundation",
		description:
			"Browser discovery, GitHub MCP import, Cursor.directory handoff, Market README rendering, source metadata, multi-server import preview, and dry-run validation now form the first end-to-end adoption path.",
	},
	{
		title: "Credential and OAuth custody",
		description:
			"Secure Store, OAuth token custody, lifecycle views, degraded-state guidance, reconnect prompts, and cleanup controls now move sensitive server state out of plain configuration files.",
	},
	{
		title: "Managed client configuration",
		description:
			"Profiles, bulk include and exclude controls, backend-maintained compatibility standards, diagnostics export, and improved Inspector lifecycle handling make MCP changes easier to review and support.",
	},
];

const Roadmap = () => {
	const meta = {
		title: "Roadmap",
		description: "What MCPMate is actively improving next.",
	};

	return (
		<DocLayout meta={meta}>
			<div className="space-y-6">
				<h2>Current Focus</h2>
				<p>
					MCPMate is focused on making MCP adoption feel less like editing
					scattered client files and more like a managed workflow: discover
					what is available, verify what will change, and expose the right
					capabilities to the right client.
				</p>
				<ul className="space-y-2">
					{currentFocus.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>Next Bets</h2>
				<p>
					These are strategic directions, not release promises. We use real
					usage, support signals, and rollout constraints to decide sequencing.
				</p>
				<ul className="space-y-2">
					{nextBets.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>Recently Shipped Foundation</h2>
				<p>
					0.3.x has been about building the foundation for that workflow. The
					changelog remains the detailed release record; this page keeps only
					the product-level building blocks.
				</p>
				<ul className="space-y-2">
					{shippedFoundation.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>Follow the moving pieces</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						For the freshest shipped record, use the{" "}
						<Link
							to="/docs/en/changelog"
							className="font-medium text-blue-700 underline underline-offset-2 dark:text-blue-300"
						>
							changelog
						</Link>
						. It reflects what has already landed, while this page captures the
						direction we are actively shaping.
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
