/** Public download manifest asset fields consumed by the website. */
export interface PublicDownloadAsset {
	key: string;
	platform: "macos" | "windows" | "linux";
	arch: "arm64" | "x64";
	name: string;
	githubUrl: string;
	redirectUrl: string;
	githubDownloadCount: number;
	size: number;
	contentType: string | null;
	updatedAt: string | null;
}

export interface PublicDownloadManifest {
	schemaVersion: 1;
	tag: string;
	releaseName: string | null;
	releaseUrl: string;
	publishedAt: string | null;
	fetchedAt: string;
	assets: Record<string, PublicDownloadAsset>;
}

/** Release payload shape normalized from the public download manifest. */
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

export const DOWNLOADS_MANIFEST_API_URL = "https://public.mcp.umate.ai/downloads/latest";

export const RELEASES_PAGE_URL = `https://github.com/${MCPMATE_GITHUB_OWNER}/${MCPMATE_GITHUB_REPO}/releases`;

export const NIGHTLY_RELEASE_PAGE_URL = `${RELEASES_PAGE_URL}?q=nightly`;

export type DesktopBuildRowId =
	| "macos-aarch64"
	| "macos-x64"
	| "windows-x64"
	| "windows-arm64"
	| "linux-x64"
	| "linux-arm64";

/** Desktop installers are marked beta across macOS, Windows, and Linux. */
export type BuildTier = "stable" | "beta";

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
		tier: "beta",
		assetSuffixes: ["macos_aarch64.dmg", "macos_arm64.dmg"],
	},
	{
		id: "macos-x64",
		platformI18nKey: "download.platform_macos",
		archI18nKey: "download.arch_x64",
		tier: "beta",
		assetSuffixes: ["macos_x86_64.dmg", "macos_x64.dmg"],
	},
	{
		id: "windows-x64",
		platformI18nKey: "download.platform_windows",
		archI18nKey: "download.arch_x64",
		tier: "beta",
		assetSuffixes: ["windows_x64.msi"],
	},
	{
		id: "windows-arm64",
		platformI18nKey: "download.platform_windows",
		archI18nKey: "download.arch_arm64",
		tier: "beta",
		assetSuffixes: ["windows_arm64.msi"],
	},
	{
		id: "linux-x64",
		platformI18nKey: "download.platform_linux",
		archI18nKey: "download.arch_x64",
		tier: "beta",
		assetSuffixes: ["linux_x64.deb", "linux_amd64.deb"],
	},
	{
		id: "linux-arm64",
		platformI18nKey: "download.platform_linux",
		archI18nKey: "download.arch_arm64",
		tier: "beta",
		assetSuffixes: ["linux_arm64.deb", "linux_aarch64.deb"],
	},
] as const;

export function releaseFromDownloadManifest(manifest: PublicDownloadManifest): GitHubLatestRelease {
	return {
		tag_name: manifest.tag,
		html_url: manifest.releaseUrl,
		assets: Object.values(manifest.assets).map((asset) => ({
			name: asset.name,
			browser_download_url: asset.redirectUrl,
			download_count: asset.githubDownloadCount,
		})),
	};
}

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
