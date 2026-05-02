import { ArrowRight, Download, ExternalLink, RefreshCw } from "lucide-react";
import { useCallback, useId, useMemo } from "react";
import { useLanguage } from "../LanguageProvider";
import { useNavigate } from "react-router-dom";
import Button from "../ui/Button";
import Section from "../ui/Section";
import { useLatestGitHubRelease } from "../../hooks/useLatestGitHubRelease";
import { trackMCPMateEvents } from "../../utils/analytics";
import { RELEASES_PAGE_URL, attachAssetsToBuildRows, cumulativeDownloadsForRow } from "../../utils/githubRelease";

const QuickStartSection = () => {
  const tableCaptionId = useId();
  const { t, language } = useLanguage();
  const navigate = useNavigate();
  const releaseState = useLatestGitHubRelease();

  const numberFmt = useMemo(() => new Intl.NumberFormat(language === "zh" ? "zh-CN" : language === "ja" ? "ja-JP" : "en-US"), [language]);

  const rowsWithAssets = useMemo(() => {
    if (releaseState.status !== "ok") {
      return null;
    }
    return attachAssetsToBuildRows(releaseState.latest);
  }, [releaseState]);

  const historyUnavailable = releaseState.status === "ok" && releaseState.allReleases === null;
  const showRetry = releaseState.status === "error" || historyUnavailable;

  const onDownloadClick = useCallback((rowId: string, url: string) => {
    trackMCPMateEvents.downloadClick(rowId);
    trackMCPMateEvents.externalLinkClick(url);
  }, []);

  return (
    <Section
      id="download"
      className="border-t border-slate-200/70 dark:border-slate-800/60"
    >
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-12">
          <h2 className="text-3xl md:text-4xl font-bold mb-2">{t("download.quick_start")}</h2>
          <p className="text-lg text-slate-600 dark:text-slate-400 mt-3 max-w-3xl mx-auto">{t("download.subtitle")}</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          <div className="space-y-6 min-w-0">
            <div>
              <h3 className="text-lg font-semibold mb-2">{t("download.getting_started")}</h3>
              <p className="text-slate-600 dark:text-slate-400 mb-4">{t("download.getting_started.desc")}</p>
              <Button
                variant="outline"
                className="w-full flex items-center justify-center gap-2"
                onClick={() => navigate(language === "zh" ? "/docs/zh/quickstart" : language === "ja" ? "/docs/ja/quickstart" : "/docs/en/quickstart")}
              >
                <span>{t("download.read_guide")}</span>
                <ArrowRight size={16} />
              </Button>
            </div>

            <div>
              <h3 className="text-lg font-semibold mb-2">{t("contact.github")}</h3>
              <p className="text-slate-600 dark:text-slate-400 mb-4">{t("contact.github.desc")}</p>
              <Button
                variant="outline"
                className="w-full flex items-center justify-center gap-2"
                onClick={() => {
                  const u = "https://github.com/loocor/mcpmate";
                  trackMCPMateEvents.externalLinkClick(u);
                  window.open(u, "_blank");
                }}
              >
                <span>github.com/loocor/mcpmate</span>
                <ArrowRight size={16} />
              </Button>
            </div>
          </div>

          <div className="min-w-0">
            <div className="flex flex-wrap items-center justify-between gap-2 mb-4">
              <h3 className="text-lg font-semibold">{t("download.official_builds")}</h3>
              <div className="flex items-center gap-2">
                {showRetry ? (
                  <button
                    type="button"
                    onClick={() => releaseState.refetch()}
                    className="inline-flex items-center gap-1 text-sm text-blue-600 dark:text-blue-400 hover:underline"
                  >
                    <RefreshCw size={14} aria-hidden />
                    {t("download.retry")}
                  </button>
                ) : null}
                <a
                  href={RELEASES_PAGE_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 text-sm text-blue-600 dark:text-blue-400 hover:underline"
                  onClick={() => trackMCPMateEvents.externalLinkClick(RELEASES_PAGE_URL)}
                >
                  {t("download.all_releases")}
                  <ExternalLink size={14} aria-hidden />
                </a>
              </div>
            </div>

            {releaseState.status === "ok" ? (
              <p className="mb-3 flex flex-wrap items-center gap-x-1 gap-y-0.5 text-sm text-slate-600 dark:text-slate-400">
                <span className="font-medium text-slate-800 dark:text-slate-200">{t("download.latest_label")}: </span>
                <code className="text-sm bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5 rounded">{releaseState.latest.tag_name}</code>
              </p>
            ) : null}

            {releaseState.status === "error" ? <p className="text-sm text-amber-700 dark:text-amber-400 mb-3">{t("download.load_error")}</p> : null}

            <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-950/40 shadow-sm">
              <table className="w-full text-sm text-left" aria-describedby={tableCaptionId}>
                <caption id={tableCaptionId} className="sr-only">
                  {t("download.table_caption")}
                </caption>
                <thead className="bg-slate-100 dark:bg-slate-800/80 text-slate-700 dark:text-slate-300">
                  <tr>
                    <th scope="col" className="px-2 py-1.5 font-semibold">
                      {t("download.col_platform")}
                    </th>
                    <th scope="col" className="px-2 py-1.5 font-semibold">
                      {t("download.col_arch")}
                    </th>
                    <th scope="col" className="px-2 py-1.5 font-semibold">
                      {t("download.col_status")}
                    </th>
                    <th scope="col" className="px-2 py-1.5 font-semibold text-center whitespace-nowrap">
                      {t("download.col_downloads")}
                    </th>
                    <th scope="col" className="px-1 py-1.5 w-11" />
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-200 dark:divide-slate-700">
                  {(rowsWithAssets ?? attachAssetsToBuildRows({ tag_name: "", html_url: "", assets: [] })).map((row) => {
                    const url = row.asset?.browser_download_url;
                    const hasAsset = Boolean(row.asset);
                    const loading = releaseState.status === "loading";
                    const unstable = row.tier === "unstable";
                    const lifetimeDownloads = releaseState.status === "ok" && releaseState.allReleases !== null
                      ? cumulativeDownloadsForRow(releaseState.allReleases, row)
                      : null;

                    return (
                      <tr key={row.id} className="text-slate-800 dark:text-slate-200">
                        <td className="px-2 py-1 whitespace-nowrap">{t(row.platformI18nKey)}</td>
                        <td className="px-2 py-1 whitespace-nowrap">{t(row.archI18nKey)}</td>
                        <td className="px-2 py-1">
                          {loading ? (
                            <span className="text-slate-400">{t("download.loading")}</span>
                          ) : hasAsset ? (
                            unstable ? (
                              <span className="text-amber-700 dark:text-amber-400 font-medium">{t("download.status_unstable")}</span>
                            ) : (
                              <span className="text-emerald-700 dark:text-emerald-400">{t("download.available")}</span>
                            )
                          ) : (
                            <span className="text-slate-500">{t("download.coming_soon")}</span>
                          )}
                        </td>
                        <td className="px-2 py-1 text-center tabular-nums text-slate-600 dark:text-slate-400">
                          {loading || lifetimeDownloads === null ? "—" : numberFmt.format(lifetimeDownloads)}
                        </td>
                        <td className="px-1 py-0.5 text-center">
                          {url ? (
                            <a
                              href={url}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="inline-flex p-1.5 rounded-md text-blue-600 dark:text-blue-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                              aria-label={`${t("download.btn")} ${t(row.platformI18nKey)} ${t(row.archI18nKey)}`}
                              onClick={() => onDownloadClick(row.id, url)}
                            >
                              <Download size={16} aria-hidden />
                            </a>
                          ) : !loading ? (
                            <a
                              href={RELEASES_PAGE_URL}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="inline-flex p-1.5 rounded-md text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"
                              aria-label={t("download.all_releases")}
                              onClick={() => trackMCPMateEvents.externalLinkClick(RELEASES_PAGE_URL)}
                            >
                              <ExternalLink size={16} aria-hidden />
                            </a>
                          ) : (
                            <span className="inline-block w-9" />
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>

            <p className="text-xs text-slate-500 dark:text-slate-500 mt-2">{t("download.platform_availability_note")}</p>
          </div>
        </div>
      </div>
    </Section>
  );
};

export default QuickStartSection;
