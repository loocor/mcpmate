import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "Desktop release pipeline",
		description:
			"We are tightening the GitHub Releases-first delivery path, including updater behavior, prerelease handling, and packaging consistency across macOS, Windows, and Linux.",
	},
	{
		title: "Platform maturity catch-up",
		description:
			"macOS is the most stable shell today; current work is focused on bringing Windows and Linux installers, runtime behavior, and desktop polish closer to that bar.",
	},
	{
		title: "Container and split deployment",
		description:
			"We are strengthening container-friendly core delivery and the separated core-server/UI story for remote or multi-machine operation.",
	},
	{
		title: "Client governance polish",
		description:
			"Detected-client rollout, writable target validation, and apply or cleanup flows are being refined so managed client changes are easier to trust.",
	},
	{
		title: "Docs and onboarding alignment",
		description:
			"The website, Quick Start, and dashboard wording are being kept in sync with shipped behavior so setup paths stay clear while release workflows evolve.",
	},
];

const exploringNext = [
	{
		title: "Built-in auto-update polish",
		description:
			"Now that the first release pipeline is in place, the next step is making desktop updates feel smoother and more routine.",
	},
	{
		title: "Profile sharing",
		description:
			"We want teams to be able to package and reuse proven profile bundles instead of rebuilding the same capability sets repeatedly.",
	},
	{
		title: "Lightweight account layer",
		description:
			"Optional account-linked helpers and cloud-backed sync remain interesting, as long as MCPMate keeps its local-first boundaries explicit.",
	},
	{
		title: "Safer sandboxing",
		description:
			"Additional guardrails for higher-risk tools are being evaluated so operators can expose powerful capabilities more deliberately.",
	},
	{
		title: "Usage and cost visibility",
		description:
			"Longer term, we want better operator-facing visibility into server-level usage patterns and token cost tradeoffs.",
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
				<h2>In Progress</h2>
				<p>
					This is the work closest to users right now: release delivery,
					platform maturity, client rollout safety, and clearer onboarding.
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>Recently Delivered</h2>
				<ul className="space-y-2">
					<li>
						OAuth upstream support now works for Streamable HTTP MCP servers,
						including metadata discovery, authorization flow, and token refresh.
					</li>
					<li>
						Audit Logs is live with filtering and cursor pagination, and core
						server plus UI can now run separately for split deployments.
					</li>
					<li>
						Market and import flows now carry richer registry metadata, better
						preview detail, and browser-assisted snippet capture.
					</li>
					<li>
						Desktop distribution now has a GitHub Releases-driven path with
						consolidated packaging and container publishing coverage.
					</li>
				</ul>

				<h2>Exploring Next</h2>
				<p>
					These are strong candidates, not hard promises. We use real user
					feedback and rollout constraints to decide sequencing.
				</p>
				<ul className="space-y-2">
					{exploringNext.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>Follow the moving pieces</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						If you want the freshest signal, watch GitHub Releases and the
						changelog first. They reflect what has already landed, while this
						page captures the direction we are actively shaping.
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
