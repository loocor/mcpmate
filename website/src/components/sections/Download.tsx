import { ArrowRight, Download, ExternalLink, RefreshCw } from "lucide-react";
import { useCallback, useId, useMemo } from "react";
import { useLanguage } from "../LanguageProvider";
import { useNavigate } from "react-router-dom";
import Button from "../ui/Button";
import Section from "../ui/Section";
import { useLatestGitHubRelease } from "../../hooks/useLatestGitHubRelease";
import { trackMCPMateEvents } from "../../utils/analytics";
import { NIGHTLY_RELEASE_PAGE_URL, attachAssetsToBuildRows } from "../../utils/githubRelease";

function getNumberLocale(language: string): string {
  if (language === "zh") {
    return "zh-CN";
  }

  if (language === "ja") {
    return "ja-JP";
  }

  return "en-US";
}

function getQuickstartPath(language: string): string {
  if (language === "zh") {
    return "/docs/zh/quickstart";
  }

  if (language === "ja") {
    return "/docs/ja/quickstart";
  }

  return "/docs/en/quickstart";
}

function getStatusContent(
  loading: boolean,
  hasAsset: boolean,
  isBeta: boolean,
  t: (key: string) => string,
): JSX.Element {
  if (loading) {
    return <span className="section-muted-soft">{t("download.loading")}</span>;
  }

  if (!hasAsset) {
    return <span className="section-muted-soft">{t("download.coming_soon")}</span>;
  }

  if (isBeta) {
    return <span className="text-amber-700 dark:text-amber-400 font-medium">{t("download.status_beta")}</span>;
  }

  return <span className="text-brand-accent">{t("download.available")}</span>;
}

const QuickStartSection = () => {
  const tableCaptionId = useId();
  const { t, language } = useLanguage();
  const navigate = useNavigate();
  const releaseState = useLatestGitHubRelease();

  const numberFmt = useMemo(() => new Intl.NumberFormat(getNumberLocale(language)), [language]);

  const rowsWithAssets = useMemo(() => {
    if (releaseState.status !== "ok") {
      return null;
    }
    return attachAssetsToBuildRows(releaseState.latest);
  }, [releaseState]);

  const showRetry = releaseState.status === "error";

  const onDownloadClick = useCallback((rowId: string, url: string) => {
    trackMCPMateEvents.downloadClick(rowId);
    trackMCPMateEvents.externalLinkClick(url);
  }, []);

  return (
    <Section
      id="download"
      snap
      className="py-16 md:py-20"
    >
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-12">
          <h2 className="text-3xl md:text-4xl font-bold mb-2 text-brand-foreground">{t("download.quick_start")}</h2>
          <p className="text-lg section-muted mt-3 max-w-3xl mx-auto">{t("download.subtitle")}</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          <div className="min-w-0 glass-card rounded-2xl p-5 md:p-6">
            <div className="flex flex-wrap items-center justify-between gap-2 mb-4">
              <h3 className="text-lg font-semibold text-brand-foreground">{t("download.official_builds")}</h3>
              <div className="flex items-center gap-2">
                {showRetry ? (
                  <button
                    type="button"
                    onClick={() => releaseState.refetch()}
                    className="inline-flex items-center gap-1 text-sm text-brand-accent hover:underline"
                  >
                    <RefreshCw size={14} aria-hidden />
                    {t("download.retry")}
                  </button>
                ) : null}
                {releaseState.status === "ok" ? (
                  <span className="flex items-center gap-x-1 text-sm section-muted">
                    <span className="font-medium text-brand-foreground">{t("download.latest_label")}: </span>
                    <code className="bg-brand-overlay px-1.5 py-0.5 rounded font-mono text-brand-accent">{releaseState.latest.tag_name}</code>
                  </span>
                ) : null}
              </div>
            </div>

            {releaseState.status === "error" ? <p className="text-sm text-amber-400 mb-3">{t("download.load_error")}</p> : null}

            <div className="overflow-x-auto rounded-lg border border-brand-border bg-brand-overlay">
              <table className="w-full text-sm text-left" aria-describedby={tableCaptionId}>
                <caption id={tableCaptionId} className="sr-only">
                  {t("download.table_caption")}
                </caption>
                <thead className="bg-brand-overlay-strong text-brand-foreground">
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
                <tbody className="divide-y divide-brand-border">
                  {(rowsWithAssets ?? attachAssetsToBuildRows({ tag_name: "", html_url: "", assets: [] })).map((row) => {
                    const url = row.asset?.browser_download_url;
                    const hasAsset = Boolean(row.asset);
                    const loading = releaseState.status === "loading";
                    const isBeta = row.tier === "beta";
                    const latestDownloads = row.asset?.download_count ?? null;

                    return (
                      <tr key={row.id} className="text-brand-foreground/90">
                        <td className="px-2 py-1 whitespace-nowrap">{t(row.platformI18nKey)}</td>
                        <td className="px-2 py-1 whitespace-nowrap">{t(row.archI18nKey)}</td>
                        <td className="px-2 py-1">
                          {getStatusContent(loading, hasAsset, isBeta, t)}
                        </td>
                        <td className="px-2 py-1 text-center tabular-nums section-muted">
                          {loading || latestDownloads === null ? "-" : numberFmt.format(latestDownloads)}
                        </td>
                        <td className="px-1 py-0.5 text-center">
                          {url ? (
                            <a
                              href={url}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="inline-flex p-1.5 rounded-md text-brand-accent hover:bg-brand-overlay-hover"
                              aria-label={`${t("download.btn")} ${t(row.platformI18nKey)} ${t(row.archI18nKey)}`}
                              onClick={() => onDownloadClick(row.id, url)}
                            >
                              <Download size={16} aria-hidden />
                            </a>
                          ) : !loading ? (
                            <span className="inline-flex w-9 justify-center text-slate-500" title={t("download.unavailable")}>
                              -
                            </span>
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

            <p className="mt-2 text-xs section-muted">
              {t("download.nightly.note")}
              <a
                href={NIGHTLY_RELEASE_PAGE_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="ml-1 inline-flex items-center gap-1 font-medium text-amber-700 hover:underline dark:text-amber-400"
                onClick={() => trackMCPMateEvents.externalLinkClick(NIGHTLY_RELEASE_PAGE_URL)}
              >
                {t("download.nightly.cta")}
                <ExternalLink size={12} aria-hidden />
              </a>
            </p>
          </div>

          <div className="space-y-6 min-w-0">
            <div className="glass-card rounded-2xl p-5 md:p-6">
              <h3 className="text-lg font-semibold mb-2 text-brand-foreground">{t("download.getting_started")}</h3>
              <p className="section-muted mb-4">{t("download.getting_started.desc")}</p>
              <Button
                variant="outline"
                className="w-full flex items-center justify-center gap-2"
                onClick={() => navigate(getQuickstartPath(language))}
              >
                <span>{t("download.read_guide")}</span>
                <ArrowRight size={16} />
              </Button>
            </div>

            <div className="glass-card rounded-2xl p-5 md:p-6">
              <h3 className="text-lg font-semibold mb-2 text-brand-foreground">{t("contact.github")}</h3>
              <p className="section-muted mb-4">{t("download.github.desc")}</p>
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
        </div>
      </div>
    </Section>
  );
};

export default QuickStartSection;
