/** GitHub REST: single release payload fields we consume. */
export interface GitHubReleaseAsset {
	name: string;
	browser_download_url: string;
	download_count: number;
}

export interface GitHubLatestRelease {
	tag_name: string;
	html_url: string;
	assets: GitHubReleaseAsset[];
	draft?: boolean;
	prerelease?: boolean;
}

export const MCPMATE_GITHUB_OWNER = "loocor";
export const MCPMATE_GITHUB_REPO = "mcpmate";

export const LATEST_RELEASE_API_URL = `https://api.github.com/repos/${MCPMATE_GITHUB_OWNER}/${MCPMATE_GITHUB_REPO}/releases/latest`;

export const LIST_RELEASES_API_URL = `https://api.github.com/repos/${MCPMATE_GITHUB_OWNER}/${MCPMATE_GITHUB_REPO}/releases`;

export const RELEASES_PAGE_URL = `https://github.com/${MCPMATE_GITHUB_OWNER}/${MCPMATE_GITHUB_REPO}/releases`;

export type DesktopBuildRowId =
	| "macos-aarch64"
	| "macos-x64"
	| "windows-x64"
	| "windows-arm64"
	| "linux-x64"
	| "linux-arm64";

/** macOS installers are treated as stable; Windows/Linux remain marked unstable in UI. */
export type BuildTier = "stable" | "unstable";

export interface DesktopBuildRow {
	id: DesktopBuildRowId;
	platformI18nKey: "download.platform_macos" | "download.platform_windows" | "download.platform_linux";
	archI18nKey: "download.arch_arm64" | "download.arch_x64";
	tier: BuildTier;
	/** First asset name match wins (preferred installer per platform). */
	assetSuffixes: readonly string[];
	asset?: GitHubReleaseAsset;
}

export const DESKTOP_BUILD_ROWS: readonly DesktopBuildRow[] = [
	{
		id: "macos-aarch64",
		platformI18nKey: "download.platform_macos",
		archI18nKey: "download.arch_arm64",
		tier: "stable",
		assetSuffixes: ["macos_aarch64.dmg", "macos_arm64.dmg"],
	},
	{
		id: "macos-x64",
		platformI18nKey: "download.platform_macos",
		archI18nKey: "download.arch_x64",
		tier: "stable",
		assetSuffixes: ["macos_x86_64.dmg", "macos_x64.dmg"],
	},
	{
		id: "windows-x64",
		platformI18nKey: "download.platform_windows",
		archI18nKey: "download.arch_x64",
		tier: "unstable",
		assetSuffixes: ["windows_x64.msi"],
	},
	{
		id: "windows-arm64",
		platformI18nKey: "download.platform_windows",
		archI18nKey: "download.arch_arm64",
		tier: "unstable",
		assetSuffixes: ["windows_arm64.msi"],
	},
	{
		id: "linux-x64",
		platformI18nKey: "download.platform_linux",
		archI18nKey: "download.arch_x64",
		tier: "unstable",
		assetSuffixes: ["linux_x64.deb", "linux_amd64.deb"],
	},
	{
		id: "linux-arm64",
		platformI18nKey: "download.platform_linux",
		archI18nKey: "download.arch_arm64",
		tier: "unstable",
		assetSuffixes: ["linux_arm64.deb", "linux_aarch64.deb"],
	},
] as const;

function findAssetForRow(assets: readonly GitHubReleaseAsset[], row: DesktopBuildRow): GitHubReleaseAsset | undefined {
	for (const suffix of row.assetSuffixes) {
		const lower = suffix.toLowerCase();
		const hit = assets.find((a) => a.name.toLowerCase().endsWith(lower));
		if (hit) {
			return hit;
		}
	}
	return undefined;
}

export function attachAssetsToBuildRows(
	release: GitHubLatestRelease,
): Array<DesktopBuildRow & { asset?: GitHubReleaseAsset }> {
	return DESKTOP_BUILD_ROWS.map((row) => ({
		...row,
		asset: findAssetForRow(release.assets, row),
	}));
}

/** Sum `download_count` for this row’s installer pattern across every published release. */
export function cumulativeDownloadsForRow(
	releases: readonly Pick<GitHubLatestRelease, "assets">[],
	row: DesktopBuildRow,
): number {
	let total = 0;
	for (const rel of releases) {
		const asset = findAssetForRow(rel.assets, row);
		if (asset) {
			total += asset.download_count ?? 0;
		}
	}
	return total;
}

const RELEASES_PER_PAGE = 100;
const MAX_RELEASE_PAGES = 50;

/**
 * Paginates `GET /repos/.../releases` (newest first). Skips draft and prerelease entries.
 */
export async function fetchAllPublishedReleases(signal: AbortSignal): Promise<GitHubLatestRelease[]> {
	const out: GitHubLatestRelease[] = [];
	for (let page = 1; page <= MAX_RELEASE_PAGES; page += 1) {
		const url = `${LIST_RELEASES_API_URL}?per_page=${RELEASES_PER_PAGE}&page=${page}`;
		const res = await fetch(url, {
			signal,
			headers: { Accept: "application/vnd.github+json" },
		});
		if (!res.ok) {
			throw new Error(`releases list HTTP ${res.status}`);
		}
		const batch = (await res.json()) as GitHubLatestRelease[];
		if (!Array.isArray(batch) || batch.length === 0) {
			break;
		}
		for (const r of batch) {
			if (r?.draft === true || r?.prerelease === true) {
				continue;
			}
			if (r?.tag_name && Array.isArray(r.assets)) {
				out.push(r);
			}
		}
		if (batch.length < RELEASES_PER_PAGE) {
			break;
		}
	}
	return out;
}
