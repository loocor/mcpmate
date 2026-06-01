export type Platform = 'mac' | 'windows' | 'linux';
export type MacVariant = 'arm64' | 'x64';
export type DesktopPlatform = 'macos' | 'windows' | 'linux';
export type DesktopArchitecture = 'arm64' | 'x64';

interface NavigatorUserAgentDataLike {
  platform?: string;
  getHighEntropyValues?: (hints: string[]) => Promise<{
    architecture?: string;
    platform?: string;
  }>;
}

interface NavigatorWithOscpu extends Navigator {
  oscpu?: string;
}

const X64_HINTS = ['x64', 'x86_64', 'amd64', 'intel', 'i386', 'x86', 'wow64'];
const ARM_HINTS = ['arm64', 'aarch64', 'armv8'];

function getDesktopHints(): string {
  if (typeof navigator === 'undefined') {
    return '';
  }

  const userAgentData = getNavigatorUserAgentData();
  const osCpu = ((navigator as NavigatorWithOscpu).oscpu || '').toLowerCase();
  return `${userAgentData?.platform || ''} ${navigator.platform || ''} ${navigator.userAgent || ''} ${osCpu}`.toLowerCase();
}

function hasHint(text: string, hints: readonly string[]): boolean {
  return hints.some((hint) => text.includes(hint));
}

function getNavigatorUserAgentData(): NavigatorUserAgentDataLike | undefined {
  if (typeof navigator === 'undefined') {
    return undefined;
  }

  return (navigator as Navigator & { userAgentData?: NavigatorUserAgentDataLike }).userAgentData;
}

export function detectPlatform(): Platform {
  if (typeof window === 'undefined') return 'mac';
  const platform = (window.navigator.platform || '').toLowerCase();
  if (platform.includes('mac')) return 'mac';
  if (platform.includes('win')) return 'windows';
  return 'linux';
}

export function detectDesktopPlatform(): DesktopPlatform {
  if (typeof navigator === 'undefined') {
    return 'linux';
  }

  const uaDataPlatform = getNavigatorUserAgentData()?.platform ?? '';
  const osCpu = ((navigator as NavigatorWithOscpu).oscpu || '').toLowerCase();
  const platform = `${uaDataPlatform} ${navigator.platform || ''} ${navigator.userAgent || ''} ${osCpu}`.toLowerCase();

  if (platform.includes('mac')) return 'macos';
  if (platform.includes('win')) return 'windows';
  if (platform.includes('linux') || platform.includes('x11')) return 'linux';

  return 'linux';
}

export function detectDesktopArchitectureSync(): DesktopArchitecture {
  if (typeof navigator === 'undefined') {
    return 'x64';
  }

  const platformHints = getDesktopHints();

  if (hasHint(platformHints, ARM_HINTS)) {
    return 'arm64';
  }
  if (hasHint(platformHints, X64_HINTS)) {
    return 'x64';
  }

  return 'x64';
}

export async function detectDesktopArchitecture(): Promise<DesktopArchitecture> {
  const uaData = getNavigatorUserAgentData();
  if (!uaData?.getHighEntropyValues) {
    return detectDesktopArchitectureSync();
  }

  try {
    const values = await uaData.getHighEntropyValues(['architecture']);
    const architecture = values.architecture?.toLowerCase() ?? '';
    if (architecture.includes('arm') || architecture.includes('aarch64')) {
      return 'arm64';
    }
    if (architecture.includes('x86') || architecture.includes('x64') || architecture.includes('amd64')) {
      return 'x64';
    }
  } catch {
    return detectDesktopArchitectureSync();
  }

  return detectDesktopArchitectureSync();
}

export function getPreviewVersion(): string {
  const env = import.meta.env as Record<string, string | undefined>;
  return env.VITE_PREVIEW_VERSION || '0.1.0-preview';
}

export function getPreviewExpiry(): Date | null {
  const env = import.meta.env as Record<string, string | undefined>;
  const s = env.VITE_PREVIEW_EXPIRES_AT || '2025-11-01';
  // Interpret YYYY-MM-DD as a local-date deadline that expires at next day's 00:00 (local time).
  // This matches product copy: valid until end of that calendar day (local 00:00 next day).
  const ymd = /^\d{4}-\d{2}-\d{2}$/;
  if (ymd.test(s)) {
    const [y, m, d] = s.split('-').map((n) => parseInt(n, 10));
    // Expire at local 00:00 of the next day, so last valid moment is 23:59:59 of the given date.
    return new Date(y, (m as number) - 1, (d as number) + 1, 0, 0, 0, 0);
  }
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? null : d;
}

export function getCountdown(expiry: Date, now: Date = new Date()): { days: number; hours: number; minutes: number; seconds: number; expired: boolean } {
  const diff = Math.max(0, expiry.getTime() - now.getTime());
  const expired = diff === 0;
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff / (1000 * 60 * 60)) % 24);
  const minutes = Math.floor((diff / (1000 * 60)) % 60);
  const seconds = Math.floor((diff / 1000) % 60);
  return { days, hours, minutes, seconds, expired };
}

export function getDocsUrl(): string {
  const env = import.meta.env as Record<string, string | undefined>;
  return env.VITE_DOCS_URL || 'https://mcp.umate.ai/docs';
}

export function getInstallScriptUrl(): string | null {
  const env = import.meta.env as Record<string, string | undefined>;
  return env.VITE_INSTALL_URL || null;
}

export function isPreviewSuspended(): boolean {
  const env = import.meta.env as Record<string, string | undefined>;
  return (env.VITE_PREVIEW_SUSPENDED || "").toLowerCase() === "true";
}

export function getMacVariantUrl(variant: MacVariant): string | null {
  const env = import.meta.env as Record<string, string | undefined>;
  if (variant === 'arm64') return env.VITE_MAC_ARM64_URL || null;
  if (variant === 'x64') return env.VITE_MAC_X64_URL || null;
  return null;
}

export function getMacVariantSha256(variant: MacVariant): string | null {
  const env = import.meta.env as Record<string, string | undefined>;
  if (variant === 'arm64') return env.VITE_MAC_ARM64_SHA256 || null;
  if (variant === 'x64') return env.VITE_MAC_X64_SHA256 || null;
  return null;
}

export function getPlatformUrl(platform: Platform): string | null {
  const env = import.meta.env as Record<string, string | undefined>;
  if (platform === 'windows') return env.VITE_WIN_URL || null;
  if (platform === 'linux') return env.VITE_LINUX_URL || null;
  return null;
}
