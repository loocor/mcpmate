import { readFileSync } from "node:fs";

const homepage = readFileSync("src/pages/Homepage.tsx", "utf8");
const hero = readFileSync("src/components/sections/Hero.tsx", "utf8");
const nav = readFileSync("src/docs/nav.ts", "utf8");
const desktopDownloads = readFileSync(
	"src/docs/components/DesktopDownloadList.tsx",
	"utf8",
);
const communityLinks = readFileSync(
	"src/docs/components/CommunityLinks.tsx",
	"utf8",
);
const copyableInlineCode = readFileSync(
	"src/docs/components/CopyableInlineCode.tsx",
	"utf8",
);
const locales = ["en", "zh", "ja"] as const;
const readmeByLocale = {
	en: "../README.md",
	zh: "../README_CN.md",
	ja: "../README_JP.md",
} as const;
const installationHeadingIds = [
	"supported-systems",
	"install",
	"desktop-install",
	"homebrew",
	"upgrade",
	"desktop-upgrade",
	"homebrew-upgrade",
	"uninstall",
	"desktop-uninstall",
	"homebrew-uninstall",
] as const;
const removedQuickstartSections = {
	en: [
		"Build from source when you want full control",
		"Run the dashboard from source",
		"Pick your shell: web or desktop",
		"Run core and UI separately",
		"If something fails at runtime",
		"Trace changes with Audit Logs",
	],
	zh: [
		"需要完全自控时再从源码构建",
		"从源码运行仪表盘",
		"选择外壳：Web 还是桌面版",
		"分离运行核心服务与 UI",
		"运行时出问题时",
		"用审计日志追踪变更",
	],
	ja: [
		"フルコントロールが必要ならソースからビルド",
		"ソースからダッシュボードを動かす",
		"Web とデスクトップ、どちらで使うか",
		"Core と UI を分離して動かす",
		"ランタイムで詰まったら",
		"監査ログで変更を追う",
	],
} as const;

function assertIncludes(source: string, expected: string, message: string): void {
	if (!source.includes(expected)) {
		throw new Error(message);
	}
}

function assertInstallationHeadings(source: string, locale: (typeof locales)[number]): void {
	const headingIds = [...source.matchAll(/<H[23] id="([^"]+)">/g)].map(
		(match) => match[1],
	);
	const uniqueHeadingIds = new Set(headingIds);

	for (const headingId of installationHeadingIds) {
		if (!uniqueHeadingIds.has(headingId)) {
			throw new Error(
				`${locale} Installation guide must expose the stable #${headingId} anchor.`,
			);
		}
	}

	if (headingIds.length !== uniqueHeadingIds.size) {
		throw new Error(`${locale} Installation guide heading IDs must be unique.`);
	}

	if (headingIds.filter((headingId) => headingId === "homebrew").length !== 1) {
		throw new Error(`${locale} Installation guide must expose exactly one #homebrew anchor.`);
	}
}

assertIncludes(homepage, 'import Hero from "../components/sections/Hero"', "Homepage must import the live Hero component.");
assertIncludes(homepage, "<Hero />", "Homepage must render the live Hero component.");
if (homepage.includes("components/sections/Download") || homepage.includes("<Download")) {
	throw new Error("Homepage must not render the stale Download section alongside Hero.");
}

assertIncludes(
	hero,
	'className="flex w-full flex-col gap-4 pt-2 sm:flex-row sm:items-start"',
	"Hero must preserve the main-branch download action layout.",
);
if (hero.includes("HomebrewInstallCard")) {
	throw new Error("Hero must not render a separate Homebrew card in the primary visual hierarchy.");
}
assertIncludes(
	hero,
	"to={`${getInstallationPath(language)}#homebrew`}",
	"Hero must link directly to the localized Homebrew installation section.",
);
if (hero.includes("download.homebrew.inline_cta")) {
	throw new Error("Hero download menu must not include a Homebrew item.");
}
assertIncludes(hero, 't(\'download.homebrew.hero_cta\')', "Hero must use the Homebrew install hint below the download CTA.");
assertIncludes(hero, "useLatestGitHubRelease()", "Hero must preserve the latest-release download source.");
assertIncludes(hero, "attachAssetsToBuildRows(releaseState.latest)", "Hero must preserve latest-release asset mapping.");
assertIncludes(hero, "RELEASES_PAGE_URL", "Hero must preserve its GitHub Releases fallback.");

const downloadAnchorCount = hero.match(/id="download"/g)?.length ?? 0;
if (downloadAnchorCount !== 1) {
	throw new Error(`Hero must render exactly one download anchor; found ${downloadAnchorCount}.`);
}

assertIncludes(
	desktopDownloads,
	"useLatestGitHubRelease()",
	"Documentation downloads must use the public Admin manifest hook.",
);
assertIncludes(
	desktopDownloads,
	"attachAssetsToBuildRows(releaseState.latest)",
	"Documentation downloads must map tracked assets through the shared release model.",
);
if (desktopDownloads.includes("RELEASES_PAGE_URL")) {
	throw new Error("Documentation downloads must not fall back to GitHub Releases.");
}

assertIncludes(communityLinks, "https://discord.gg/pc5YfEVbKj", "Community links must include Discord.");
assertIncludes(
	communityLinks,
	"https://applink.feishu.cn/client/chat/chatter/add_by_link",
	"Community links must include the Chinese Feishu community.",
);
assertIncludes(
	communityLinks,
	"https://github.com/loocor/MCPMate/issues",
	"Community links must include GitHub Issues.",
);
assertIncludes(
	communityLinks,
	"https://github.com/loocor/MCPMate/discussions",
	"Community links must include GitHub Discussions.",
);
assertIncludes(communityLinks, 'locale === "zh"', "Chinese docs must prefer the Feishu community.");
assertIncludes(
	copyableInlineCode,
	"navigator.clipboard.writeText(children)",
	"Copyable inline code must use the Clipboard API without a fallback mutation path.",
);
assertIncludes(
	copyableInlineCode,
	"group-hover/code:opacity-100",
	"Copyable inline code must reveal its copy action on hover.",
);

for (const locale of locales) {
	const overviewPath = `/docs/${locale}/guides-overview`;
	const installationPath = `/docs/${locale}/installation`;
	const onboardingPath = `/docs/${locale}/onboarding`;
	const overviewIndex = nav.indexOf(overviewPath);
	const installationIndex = nav.indexOf(installationPath);
	const onboardingIndex = nav.indexOf(onboardingPath);
	if (
		overviewIndex === -1 ||
		installationIndex === -1 ||
		onboardingIndex === -1 ||
		!(overviewIndex < installationIndex && installationIndex < onboardingIndex)
	) {
		throw new Error(`${locale} Installation must appear after Guides Overview and before Onboarding.`);
	}

	const guide = readFileSync(`src/docs/pages/${locale}/Installation.tsx`, "utf8");
	assertIncludes(guide, "brew install --cask loocor/tap/mcpmate@beta", `${locale} Homebrew guide must expose the exact install command.`);
	assertIncludes(
		guide,
		"Homebrew 5.1.12",
		`${locale} Homebrew guide must document the Linux Homebrew 5.1.12 minimum.`,
	);
	assertInstallationHeadings(guide, locale);

	const quickstart = readFileSync(`src/docs/pages/${locale}/Quickstart.tsx`, "utf8");
	assertIncludes(quickstart, "<DesktopDownloadList", `${locale} Quick Start must render tracked desktop downloads.`);
	assertIncludes(quickstart, "<CommunityLinks", `${locale} Quick Start must render localized community links.`);
	assertIncludes(
		quickstart,
		"brew install --cask loocor/tap/mcpmate@beta",
		`${locale} Quick Start must expose the standard Homebrew install command.`,
	);
	assertIncludes(
		quickstart,
		"<CopyableInlineCode",
		`${locale} Quick Start must render the Homebrew command as copyable inline code.`,
	);
	assertIncludes(
		quickstart,
		`to="/docs/${locale}/installation"`,
		`${locale} Quick Start must link to the Installation guide.`,
	);
	for (const removedSection of removedQuickstartSections[locale]) {
		if (quickstart.includes(removedSection)) {
			throw new Error(`${locale} Quick Start must not include "${removedSection}".`);
		}
	}

	const readme = readFileSync(readmeByLocale[locale], "utf8");
	assertIncludes(
		readme,
		"brew install --cask loocor/tap/mcpmate@beta",
		`${locale} README must expose the Homebrew install command.`,
	);
	assertIncludes(
		readme,
		`https://mcp.umate.ai/docs/${locale}/installation#homebrew`,
		`${locale} README must link to the localized Homebrew installation section.`,
	);

	const roadmap = readFileSync(`src/docs/pages/${locale}/Roadmap.tsx`, "utf8");
	assertIncludes(
		roadmap,
		"Standalone Inspector",
		`${locale} Roadmap must reflect the active Standalone Inspector work.`,
	);
	if (roadmap.includes("Homebrew")) {
		throw new Error(`${locale} Roadmap must treat Homebrew as normal maintenance, not future work.`);
	}
	assertIncludes(
		readme,
		"Standalone Inspector",
		`${locale} README Roadmap must reflect the active Standalone Inspector work.`,
	);
}

console.log("Homebrew homepage and installation docs reachability: PASS");
