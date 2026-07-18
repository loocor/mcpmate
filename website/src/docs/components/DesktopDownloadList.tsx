import { Download, RefreshCw } from "lucide-react";
import { useMemo } from "react";
import { useLatestGitHubRelease } from "../../hooks/useLatestGitHubRelease";
import { trackMCPMateEvents } from "../../utils/analytics";
import {
	DESKTOP_BUILD_ROWS,
	attachAssetsToBuildRows,
} from "../../utils/githubRelease";

type DocsLocale = "en" | "zh" | "ja";

type DesktopDownloadListProps = {
	locale: DocsLocale;
};

const copy = {
	en: {
		platform: "Platform",
		architecture: "Architecture",
		status: "Status",
		action: "Download",
		macos: "macOS",
		windows: "Windows",
		linux: "Linux",
		arm64: "ARM64",
		x64: "x64",
		beta: "Beta",
		loading: "Loading",
		unavailable: "Unavailable",
		loadError: "The download list could not be loaded.",
		retry: "Retry",
		latest: "Latest release",
	},
	zh: {
		platform: "平台",
		architecture: "架构",
		status: "状态",
		action: "下载",
		macos: "macOS",
		windows: "Windows",
		linux: "Linux",
		arm64: "ARM64",
		x64: "x64",
		beta: "Beta",
		loading: "加载中",
		unavailable: "暂不可用",
		loadError: "无法加载下载列表。",
		retry: "重试",
		latest: "最新版本",
	},
	ja: {
		platform: "プラットフォーム",
		architecture: "アーキテクチャ",
		status: "状態",
		action: "ダウンロード",
		macos: "macOS",
		windows: "Windows",
		linux: "Linux",
		arm64: "ARM64",
		x64: "x64",
		beta: "Beta",
		loading: "読み込み中",
		unavailable: "利用できません",
		loadError: "ダウンロード一覧を読み込めませんでした。",
		retry: "再試行",
		latest: "最新リリース",
	},
} as const;

const platformLabels = {
	"download.platform_macos": "macos",
	"download.platform_windows": "windows",
	"download.platform_linux": "linux",
} as const;

const architectureLabels = {
	"download.arch_arm64": "arm64",
	"download.arch_x64": "x64",
} as const;

function getDownloadStatus(
	releaseStatus: "loading" | "error" | "ok",
	hasDownload: boolean,
	labels: (typeof copy)[DocsLocale],
): string {
	if (releaseStatus === "loading") {
		return labels.loading;
	}

	if (hasDownload) {
		return labels.beta;
	}

	return labels.unavailable;
}

export default function DesktopDownloadList({
	locale,
}: DesktopDownloadListProps): JSX.Element {
	const releaseState = useLatestGitHubRelease();
	const labels = copy[locale];
	const rows = useMemo(() => {
		if (releaseState.status === "ok") {
			return attachAssetsToBuildRows(releaseState.latest);
		}

		return DESKTOP_BUILD_ROWS.map((row) => ({ ...row }));
	}, [releaseState]);

	return (
		<div className="not-prose overflow-hidden rounded-lg border border-brand-border bg-brand-elevated">
			<div className="flex flex-wrap items-center justify-between gap-3 border-b border-brand-border px-4 py-3">
				<span className="text-sm font-medium text-brand-foreground">
					{labels.latest}
					{releaseState.status === "ok" ? (
						<code className="ml-2 rounded bg-brand-overlay px-1.5 py-0.5 text-brand-accent">
							{releaseState.latest.tag_name}
						</code>
					) : null}
				</span>
				{releaseState.status === "error" ? (
					<div className="flex items-center gap-3 text-sm text-amber-600 dark:text-amber-400">
						<span>{labels.loadError}</span>
						<button
							type="button"
							onClick={releaseState.refetch}
							className="inline-flex items-center gap-1 font-medium hover:underline"
						>
							<RefreshCw size={14} aria-hidden />
							{labels.retry}
						</button>
					</div>
				) : null}
			</div>

			<div className="overflow-x-auto">
				<table className="w-full min-w-[34rem] text-left text-sm">
					<thead className="bg-brand-overlay text-brand-foreground">
						<tr>
							<th scope="col" className="px-4 py-2 font-semibold">{labels.platform}</th>
							<th scope="col" className="px-4 py-2 font-semibold">{labels.architecture}</th>
							<th scope="col" className="px-4 py-2 font-semibold">{labels.status}</th>
							<th scope="col" className="px-4 py-2 text-right font-semibold">{labels.action}</th>
						</tr>
					</thead>
					<tbody className="divide-y divide-brand-border">
						{rows.map((row) => {
							const platform = labels[platformLabels[row.platformI18nKey]];
							const architecture = labels[architectureLabels[row.archI18nKey]];
							const url = row.asset?.browser_download_url;
							const status = getDownloadStatus(
								releaseState.status,
								Boolean(url),
								labels,
							);

							return (
								<tr key={row.id} className="text-brand-muted">
									<td className="px-4 py-2 text-brand-foreground">{platform}</td>
									<td className="px-4 py-2">{architecture}</td>
									<td className="px-4 py-2">{status}</td>
									<td className="px-4 py-2 text-right">
										{url ? (
											<a
												href={url}
												target="_blank"
												rel="noopener noreferrer"
												onClick={() => {
													trackMCPMateEvents.downloadClick(row.id);
													trackMCPMateEvents.externalLinkClick(url);
												}}
												className="inline-flex items-center gap-1 font-medium text-brand-accent hover:underline"
												aria-label={`${labels.action}: ${platform} ${architecture}`}
											>
												<Download size={14} aria-hidden />
												{labels.action}
											</a>
										) : (
											<span className="text-brand-muted-soft">—</span>
										)}
									</td>
								</tr>
							);
						})}
					</tbody>
				</table>
			</div>
		</div>
	);
}
