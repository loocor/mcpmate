import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "Usability improvements after hardening",
		description:
			"With the 0.2.3 stability pass in place, the next release work is reducing setup friction across onboarding, client setup, Market installs, and support feedback loops.",
	},
	{
		title: "Desktop release pipeline",
		description:
			"We are tightening the GitHub Releases-first delivery path, including updater behavior, prerelease handling, and packaging consistency across macOS, Windows, and Linux.",
	},
	{
		title: "Platform maturity catch-up",
		description:
			"macOS, Windows, and Linux desktop builds are all treated as Beta while we keep tightening installer behavior, runtime detection, and desktop polish across platforms.",
	},
	{
		title: "Client governance and credential safety",
		description:
			"Detected-client rollout, writable target validation, apply or cleanup flows, and sensitive token handling are being refined so managed client changes are easier to trust.",
	},
	{
		title: "Docs and onboarding alignment",
		description:
			"The website, Quick Start, extension install paths, and dashboard wording are being kept in sync with shipped behavior so setup paths stay clear while release workflows evolve.",
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
		title: "Container and split deployment polish",
		description:
			"Core Server and UI can already run separately; future work is making remote, container, and multi-machine operation easier to package and explain.",
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
					After the 0.2.3 stability pass, the work closest to users is
					usability polish, release delivery, platform maturity, client rollout
					safety, and clearer onboarding.
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>Recently Delivered</h2>
				<ul className="space-y-2">
					<li>
						Onboarding and new-client setup now use backend-maintained
						compatibility standards so users can receive fresher matching
						client configuration.
					</li>
					<li>
						Automatic refresh foundations were strengthened, including OAuth
						token refresh for authorized Streamable HTTP servers.
					</li>
					<li>
						Desktop diagnostics export gives users a cleaner way to share local
						feedback when support investigation is needed.
					</li>
					<li>
						Inspector lifecycle management and Registry install handling were
						hardened to reduce repeated work, confusing status, and broken
						install drafts.
					</li>
					<li>
						Browser extension, onboarding, and website documentation were
						refreshed so install and upgrade paths are easier to follow.
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
