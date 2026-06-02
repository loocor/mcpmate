import { useEffect, useMemo, useState, useRef } from 'react';
import { ArrowRight, ChevronDown, Download, ExternalLink } from 'lucide-react';

import { useLatestGitHubRelease } from '../../hooks/useLatestGitHubRelease';
import { BROWSER_EXTENSION_LINKS } from '../../lib/browser-extensions';
import {
  attachAssetsToBuildRows,
  DESKTOP_BUILD_ROWS,
  RELEASES_PAGE_URL,
  type DesktopBuildRow,
  type GitHubReleaseAsset,
} from '../../utils/githubRelease';
import { trackMCPMateEvents } from '../../utils/analytics';
import {
  detectDownloadEnvironment,
  detectDownloadEnvironmentSync,
  type DesktopArchitecture,
  type DesktopPlatform,
  type DownloadEnvironment,
} from '../../utils/downloads';
import { useLanguage } from '../LanguageProvider';
import { useTheme } from '../ThemeProvider';
import Button from '../ui/Button';

type DesktopBuildWithAsset = DesktopBuildRow & { asset?: GitHubReleaseAsset };
type HeroSlide = {
  id: string;
  titleKey: 'hero.slide.dashboard' | 'hero.slide.profiles' | 'hero.slide.servers' | 'hero.slide.clients';
  imageLight: string;
  imageDark: string;
};

const RESTING_SCREEN_TRANSFORM = 'perspective(1000px) rotateX(0deg) rotateY(0deg) scale3d(1, 1, 1)';
const HOVER_SCALE_TRANSFORM = 'scale3d(1.01, 1.01, 1.01)';

const PLATFORM_DOWNLOADS: Array<{
  id: DesktopPlatform;
  labelKey: DesktopBuildRow['platformI18nKey'];
}> = [
  { id: 'macos', labelKey: 'download.platform_macos' },
  { id: 'windows', labelKey: 'download.platform_windows' },
  { id: 'linux', labelKey: 'download.platform_linux' },
];

function getArchitectureLabelKey(architecture: DesktopArchitecture): DesktopBuildRow['archI18nKey'] {
  return architecture === 'arm64' ? 'download.arch_arm64' : 'download.arch_x64';
}

function formatPlatformList(platforms: string[], language: string): string {
  if (platforms.length < 2) {
    return platforms[0] ?? '';
  }

  if (language === 'zh' || language === 'ja') {
    return platforms.join('、');
  }

  return `${platforms.slice(0, -1).join(', ')} and ${platforms[platforms.length - 1]}`;
}

function formatAlternativePlatforms(currentPlatform: DesktopPlatform, language: string, t: (key: string) => string): string {
  const alternatives = PLATFORM_DOWNLOADS.filter((platform) => platform.id !== currentPlatform).map((platform) => t(platform.labelKey));
  return formatPlatformList(alternatives, language);
}

function getTiltTransform(event: React.MouseEvent<HTMLDivElement>, element: HTMLDivElement): string {
  const rect = element.getBoundingClientRect();
  const x = event.clientX - rect.left - rect.width / 2;
  const y = event.clientY - rect.top - rect.height / 2;
  const normalizedX = x / (rect.width / 2);
  const normalizedY = y / (rect.height / 2);
  const rotateX = -normalizedY * 6;
  const rotateY = normalizedX * 6;

  return `perspective(1000px) rotateX(${rotateX}deg) rotateY(${rotateY}deg) ${HOVER_SCALE_TRANSFORM}`;
}

function getSlideClass(isActive: boolean): string {
  const visibilityClass = isActive ? 'opacity-100 z-10' : 'opacity-0 z-0';
  return `absolute inset-0 w-full h-full object-cover object-top transition-opacity duration-500 ease-in-out ${visibilityClass}`;
}

function Hero(): JSX.Element {
  const { theme } = useTheme();
  const { language, t } = useLanguage();
  const releaseState = useLatestGitHubRelease(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const [transformStyle, setTransformStyle] = useState<string>(RESTING_SCREEN_TRANSFORM);
  const [downloadEnvironment, setDownloadEnvironment] = useState<DownloadEnvironment>(() => detectDownloadEnvironmentSync());
  const [downloadMenuOpen, setDownloadMenuOpen] = useState(false);

  const carouselItems = useMemo<HeroSlide[]>(
    () => [
      {
        id: 'dashboard',
        titleKey: 'hero.slide.dashboard' as const,
        imageLight: '/screenshot/dashboard-light.png',
        imageDark: '/screenshot/dashboard-dark.png',
      },
      {
        id: 'profiles',
        titleKey: 'hero.slide.profiles' as const,
        imageLight: '/screenshot/profiles-light.png',
        imageDark: '/screenshot/profiles-dark.png',
      },
      {
        id: 'servers',
        titleKey: 'hero.slide.servers' as const,
        imageLight: '/screenshot/servers-light.png',
        imageDark: '/screenshot/servers-dark.png',
      },
      {
        id: 'clients',
        titleKey: 'hero.slide.clients' as const,
        imageLight: '/screenshot/clients-light.png',
        imageDark: '/screenshot/clients-dark.png',
      },
    ],
    [],
  );

  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setActiveIndex((previousIndex) => (previousIndex + 1) % carouselItems.length);
    }, 5000);

    return () => clearInterval(interval);
  }, [carouselItems.length]);

  useEffect(() => {
    let cancelled = false;

    void detectDownloadEnvironment().then((environment) => {
      if (!cancelled) {
        setDownloadEnvironment(environment);
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!containerRef.current) return;
    setTransformStyle(getTiltTransform(e, containerRef.current));
  };

  const handleMouseLeave = () => {
    setTransformStyle(RESTING_SCREEN_TRANSFORM);
  };

  function scrollToSection(id: string): void {
    const element = document.getElementById(id);

    if (!element) {
      return;
    }

    const offset = 80;
    const elementPosition = element.getBoundingClientRect().top;
    const offsetPosition = elementPosition + window.scrollY - offset;

    window.scrollTo({
      top: offsetPosition,
      behavior: 'smooth',
    });
  }

  const active = carouselItems[activeIndex];
  const slideAlt = t(active.titleKey);
  const downloadRows = useMemo<DesktopBuildWithAsset[]>(() => {
    if (releaseState.status === 'ok') {
      return attachAssetsToBuildRows(releaseState.latest);
    }

    return DESKTOP_BUILD_ROWS.map((row) => ({ ...row }));
  }, [releaseState]);

  const isDesktopDownload = downloadEnvironment.kind === 'desktop';
  const selectedPlatform = isDesktopDownload
    ? (PLATFORM_DOWNLOADS.find((platform) => platform.id === downloadEnvironment.platform) ?? PLATFORM_DOWNLOADS[0])
    : null;
  const selectedArchitectureLabelKey = isDesktopDownload ? getArchitectureLabelKey(downloadEnvironment.architecture) : null;
  const selectedPlatformRows = useMemo(
    () => selectedPlatform ? downloadRows.filter((row) => row.platformI18nKey === selectedPlatform.labelKey) : [],
    [downloadRows, selectedPlatform],
  );
  const preferredDownloadRow =
    selectedArchitectureLabelKey ? (selectedPlatformRows.find((row) => row.archI18nKey === selectedArchitectureLabelKey) ?? selectedPlatformRows[0]) : undefined;
  const primaryDownloadRow = isDesktopDownload && preferredDownloadRow?.asset
    ? preferredDownloadRow
    : selectedPlatformRows.find((row) => row.asset);
  const primaryDownloadUrl = isDesktopDownload ? (primaryDownloadRow?.asset?.browser_download_url ?? RELEASES_PAGE_URL) : RELEASES_PAGE_URL;
  const fallbackDownloadUrl = releaseState.status === 'ok' ? releaseState.latest.html_url : RELEASES_PAGE_URL;
  const primaryDownloadLabel = isDesktopDownload && selectedPlatform
    ? t('download.cta_for').replace('{platform}', t(selectedPlatform.labelKey))
    : t('download.desktop_downloads');
  const downloadSupportLabel = isDesktopDownload && selectedPlatform
    ? t('download.also_available_for').replace(
        '{platforms}',
        formatAlternativePlatforms(selectedPlatform.id, language, t),
      )
    : t('download.desktop_note');
  const downloadMenuStateClass = downloadMenuOpen
    ? 'pointer-events-auto visible translate-y-0'
    : 'pointer-events-none invisible translate-y-1';
  const downloadMenuClass = isDesktopDownload
    ? 'pointer-events-none invisible translate-y-1 group-hover/download:pointer-events-auto group-hover/download:visible group-hover/download:translate-y-0 group-focus-within/download:pointer-events-auto group-focus-within/download:visible group-focus-within/download:translate-y-0'
    : downloadMenuStateClass;
  const primaryDownloadButtonClass =
    'inline-flex w-full items-center justify-center gap-3 rounded-lg bg-brand-accent px-6 py-3 text-base font-semibold text-brand-accent-fg shadow-card shadow-glow-sm transition-all duration-200 hover:bg-brand-accent-hover focus:outline-none focus:ring-2 focus:ring-brand-accent focus:ring-offset-2 focus:ring-offset-brand-bg dark:hover:ring-2 dark:hover:ring-white dark:hover:ring-offset-2 dark:hover:ring-offset-brand-bg dark:focus-visible:ring-2 dark:focus-visible:ring-white dark:focus-visible:ring-offset-2 dark:focus-visible:ring-offset-brand-bg sm:w-auto';

  const onDownloadClick = (row: DesktopBuildWithAsset | undefined, url: string) => {
    setDownloadMenuOpen(false);
    if (row) {
      trackMCPMateEvents.downloadClick(row.id);
    }
    trackMCPMateEvents.externalLinkClick(url);
  };

  const toggleDownloadMenu = () => {
    setDownloadMenuOpen((open) => !open);
  };

  return (
    <div className="relative w-full pt-[calc(var(--banner-height,0px)+8.5rem)] pb-20 md:py-16 lg:py-14 [@media(max-height:52rem)]:pt-[calc(var(--banner-height,0px)+7.75rem)] [@media(max-height:52rem)]:pb-10">
      <div className="container mx-auto px-4 md:px-6">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-10 items-center">
          <div className="flex flex-col items-start space-y-6">
            <h1 className="text-4xl md:text-5xl lg:text-[3.4rem] font-bold tracking-tight leading-[1.08] text-brand-foreground">
              <span>{t('hero.title')}</span>
              <br />
              <span className="text-brand-accent">{t('hero.subtitle')}</span>
            </h1>

            <p className="text-lg md:text-xl text-brand-muted max-w-xl leading-relaxed">
              {t('hero.description')}
            </p>

            <p className="max-w-xl text-sm leading-relaxed section-muted">
              {t('hero.scenario')}
            </p>

            <div id="download" className="flex w-full flex-col gap-4 pt-2 sm:flex-row sm:items-start">
              <div className="flex w-full flex-col gap-1.5 sm:w-auto sm:items-center">
                <div className="group/download relative w-full sm:inline-block">
                  {isDesktopDownload ? (
                    <a
                      href={primaryDownloadUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => onDownloadClick(primaryDownloadRow, primaryDownloadUrl)}
                      className={primaryDownloadButtonClass}
                    >
                      <Download className="h-5 w-5" aria-hidden />
                      <span>{primaryDownloadLabel}</span>
                      <ChevronDown
                        size={16}
                        aria-hidden
                        className="shrink-0 transition-transform duration-200 group-hover/download:translate-y-0.5 group-focus-within/download:translate-y-0.5"
                      />
                    </a>
                  ) : (
                    <button
                      type="button"
                      aria-expanded={downloadMenuOpen}
                      onClick={toggleDownloadMenu}
                      className={primaryDownloadButtonClass}
                    >
                      <Download className="h-5 w-5" aria-hidden />
                      <span>{primaryDownloadLabel}</span>
                      <ChevronDown
                        size={16}
                        aria-hidden
                        className="shrink-0 transition-transform duration-200 group-hover/download:translate-y-0.5 group-focus-within/download:translate-y-0.5"
                      />
                    </button>
                  )}

                  <div className={`absolute inset-x-0 top-full z-30 w-full pt-2 transition-[transform,visibility] duration-200 ease-out ${downloadMenuClass}`}>
                    <div className="overflow-hidden rounded-xl border border-brand-border-subtle bg-brand-elevated/95 p-1 shadow-card backdrop-blur-md">
                      {downloadRows.map((row) => {
                        const url = row.asset?.browser_download_url;
                        const rowLabel = `${t(row.platformI18nKey)} ${t(row.archI18nKey)}`;

                        if (!url && releaseState.status === 'loading') {
                          return (
                            <span
                              key={row.id}
                              aria-disabled="true"
                              className="flex items-center justify-between gap-4 rounded-lg px-3 py-2 text-sm font-medium section-muted-soft opacity-60"
                            >
                              {rowLabel}
                            </span>
                          );
                        }

                        const targetUrl = url ?? fallbackDownloadUrl;
                        const isDirectAsset = Boolean(url);

                        return (
                          <a
                            key={row.id}
                            href={targetUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            onClick={() => onDownloadClick(isDirectAsset ? row : undefined, targetUrl)}
                            className={`flex items-center justify-between gap-4 rounded-lg px-3 py-2 text-sm font-medium transition-colors hover:bg-brand-overlay-hover hover:text-brand-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent ${
                              isDirectAsset ? 'text-brand-foreground' : 'section-muted'
                            }`}
                            aria-label={
                              isDirectAsset
                                ? `${t('download.btn')} ${rowLabel}`
                                : `${t('download.all_releases')} ${rowLabel}`
                            }
                          >
                            <span>{rowLabel}</span>
                            {isDirectAsset ? <Download size={14} aria-hidden /> : <ExternalLink size={14} aria-hidden />}
                          </a>
                        );
                      })}
                      <a
                        href={RELEASES_PAGE_URL}
                        target="_blank"
                        rel="noopener noreferrer"
                        onClick={() => onDownloadClick(undefined, RELEASES_PAGE_URL)}
                        className="mt-1 flex items-center justify-between gap-4 border-t border-brand-border-subtle px-3 py-2 text-sm font-medium text-brand-accent transition-colors hover:bg-brand-overlay-hover hover:text-brand-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent"
                      >
                        <span>{t('download.all_releases')}</span>
                        <ArrowRight size={14} aria-hidden />
                      </a>
                    </div>
                  </div>
                </div>
                <p className="text-center text-xs leading-tight section-muted-soft">{downloadSupportLabel}</p>
                <p className="flex flex-wrap items-center justify-center gap-x-2 gap-y-0.5 text-center text-xs leading-tight section-muted-soft">
                  <span>{t('browserExtensions.inlineLabel')}</span>
                  {BROWSER_EXTENSION_LINKS.map((link, index) => (
                    <span key={link.id} className="inline-flex items-center gap-2">
                      {index > 0 ? <span aria-hidden>/</span> : null}
                      <a
                        href={link.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        onClick={() => trackMCPMateEvents.externalLinkClick(link.url)}
                        className="font-medium text-brand-accent transition-colors hover:text-brand-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent"
                      >
                        {t(link.labelKey)}
                      </a>
                    </span>
                  ))}
                </p>
              </div>

              <Button
                variant="outline"
                size="lg"
                onClick={() => scrollToSection('how-it-works')}
                className="w-full sm:w-auto"
              >
                <span>{t('hero.cta.learn')}</span>
                <ArrowRight className="ml-2 h-5 w-5" />
              </Button>
            </div>

          </div>

          <div className="relative">
            <div className="absolute inset-0 rounded-2xl bg-brand-accent/10 blur-3xl opacity-40" aria-hidden />
            <div
              ref={containerRef}
              onMouseMove={handleMouseMove}
              onMouseLeave={handleMouseLeave}
              style={{ transform: transformStyle, transformStyle: 'preserve-3d' }}
              className="relative z-10 w-full md:max-w-[110%] md:ml-auto transition-all duration-200 ease-out"
            >
              <div
                style={{ transform: 'translateZ(20px)' }}
                className="overflow-hidden rounded-2xl glass-card shadow-glow ring-1 ring-brand-accent/15 flex flex-col"
              >
                <div className="flex items-center gap-2 border-b border-brand-border px-4 py-2 shrink-0">
                  <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
                  <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
                  <span className="h-2.5 w-2.5 rounded-full bg-brand-accent/80" />
                  <span className="ml-2 truncate text-xs font-mono text-brand-muted-soft">{slideAlt}</span>
                </div>
                <div className="relative w-full aspect-[126/79] overflow-hidden bg-brand-bg/20">
                  {carouselItems.map((item, index) => (
                    <img
                      key={item.id}
                      src={theme === 'dark' ? item.imageDark : item.imageLight}
                      alt={t(item.titleKey)}
                      className={getSlideClass(activeIndex === index)}
                    />
                  ))}
                </div>
              </div>

              <div className="flex justify-center mt-4 gap-2 flex-wrap">
                {carouselItems.map((item, index) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setActiveIndex(index)}
                    className={`h-2 rounded-full transition-all duration-200 ${activeIndex === index ? 'w-8 bg-brand-accent' : 'w-2 bg-brand-dot hover:bg-brand-dot-hover'}`}
                    aria-label={t(item.titleKey)}
                    aria-current={activeIndex === index ? 'true' : undefined}
                  />
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default Hero;
