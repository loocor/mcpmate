export interface MarketPortalDefinition {
  id: string;
  label: string;
  remoteOrigin: string;
  proxyPath: string;
  adapter: string;
  favicon?: string;
  proxyFavicon?: string;
  locales?: string[];
  localeParam?: {
    strategy?: "query" | "path-prefix";
    key: string;
    mapping?: Record<string, string>;
    fallback?: string;
  };
}

export const BUILTIN_MARKET_PORTALS: MarketPortalDefinition[] = [];

export const MARKET_PORTAL_MAP: Record<string, MarketPortalDefinition> = {};

export const createRegistryPortalMap = (): Record<string, MarketPortalDefinition> => ({});

export const mergePortalOverrides = (
  _overrides?: Record<string, Partial<MarketPortalDefinition> | undefined>,
): Record<string, MarketPortalDefinition> => ({});

export type MarketPortalId = string;

export const buildPortalUrlWithLocale = (
  _portal: MarketPortalDefinition,
  baseUrl: string,
  _language: string | undefined,
): string => baseUrl;
