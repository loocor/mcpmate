import { create } from "zustand";
import { isTauriEnvironmentSync } from "./platform";
import type {
	MarketPortalDefinition,
	MarketPortalId,
} from "../pages/market/portal-registry";
import {
	MARKET_PORTAL_MAP,
	mergePortalOverrides,
} from "../pages/market/portal-registry";
import type { Theme } from "./types";

export type DashboardDefaultView = "list" | "grid";
export type DashboardAppMode = "express" | "expert";
export type DashboardLanguage = "en" | "zh-cn" | "ja";
export type ClientDefaultMode = "hosted" | "transparent";
export type ClientListDefaultFilter = "all" | "detected" | "managed";
export type ClientBackupStrategy = "keep_n" | "keep_last" | "none";
export type MenuBarIconMode = "runtime" | "hidden";
export type DefaultMarket = "official" | MarketPortalId;

export type MarketPortalMeta = MarketPortalDefinition;

export const DEFAULT_MARKET_PORTALS: Readonly<
	Record<string, MarketPortalMeta>
> = Object.freeze(mergePortalOverrides());

function cloneMarketPortals(
	portals: Record<string, MarketPortalMeta>,
): Record<string, MarketPortalMeta> {
	return Object.fromEntries(
		Object.entries(portals).map(([key, meta]) => [key, { ...meta }]),
	);
}

function sanitizeMarketPortalMeta(
	id: string,
	value: unknown,
	fallback?: MarketPortalMeta,
): MarketPortalMeta | null {
	if (!value || typeof value !== "object") return null;
	const input = value as Partial<MarketPortalMeta>;
	const targetId =
		typeof input.id === "string" && input.id.trim() ? input.id.trim() : id;
	const canonical = MARKET_PORTAL_MAP[targetId];
	if (!canonical) return null;

	const label =
		typeof input.label === "string" && input.label.trim()
			? input.label.trim()
			: (fallback?.label ?? canonical.label);
	const remoteOrigin =
		typeof (input as Partial<MarketPortalDefinition>).remoteOrigin ===
			"string" &&
		(input as Partial<MarketPortalDefinition>).remoteOrigin?.trim()
			? (input as Partial<MarketPortalDefinition>).remoteOrigin.trim()
			: (fallback?.remoteOrigin ?? canonical.remoteOrigin);
	const proxyPathRaw =
		typeof input.proxyPath === "string" && input.proxyPath.trim()
			? input.proxyPath.trim()
			: (fallback?.proxyPath ?? canonical.proxyPath);
	if (!label || !proxyPathRaw) return null;
	const proxyPath = proxyPathRaw.endsWith("/")
		? proxyPathRaw
		: `${proxyPathRaw}/`;
	const favicon =
		typeof input.favicon === "string" && input.favicon.trim()
			? input.favicon.trim()
			: (fallback?.favicon ?? canonical.favicon);
	const proxyFavicon =
		typeof input.proxyFavicon === "string" && input.proxyFavicon.trim()
			? input.proxyFavicon.trim()
			: (fallback?.proxyFavicon ?? canonical.proxyFavicon);
	const adapter =
		typeof (input as Partial<MarketPortalDefinition>).adapter === "string" &&
		(input as Partial<MarketPortalDefinition>).adapter?.trim()
			? (input as Partial<MarketPortalDefinition>).adapter!.trim()
			: (fallback?.adapter ?? canonical.adapter ?? "default");
	const locales =
		Array.isArray((input as Partial<MarketPortalDefinition>).locales) &&
		((input as Partial<MarketPortalDefinition>).locales?.length ?? 0) > 0
			? ((input as Partial<MarketPortalDefinition>).locales ?? []).filter(
					(locale): locale is string =>
						typeof locale === "string" && locale.trim().length > 0,
				)
			: (fallback?.locales ?? canonical.locales);
  const localeParamInput =
		(input as Partial<MarketPortalDefinition>).localeParam ??
		fallback?.localeParam ??
		canonical.localeParam;
  let localeParam: MarketPortalDefinition["localeParam"] | undefined;
  if (localeParamInput && typeof localeParamInput === "object") {
		const keyRaw = (localeParamInput as { key?: unknown }).key;
		const key =
			typeof keyRaw === "string" && keyRaw.trim().length > 0
				? keyRaw.trim()
				: undefined;
		const strategyRaw = (localeParamInput as { strategy?: unknown }).strategy;
		const strategy =
			strategyRaw === "path-prefix" || strategyRaw === "query"
				? (strategyRaw as "path-prefix" | "query")
				: undefined;
		const mappingRaw = (localeParamInput as { mapping?: unknown }).mapping;
		const mapping =
			mappingRaw && typeof mappingRaw === "object"
				? Object.fromEntries(
						Object.entries(mappingRaw as Record<string, unknown>).reduce(
								(acc, [mapKey, mapValue]) => {
									if (typeof mapKey !== "string") return acc;
									const trimmedKey = mapKey.trim().toLowerCase();
									if (!trimmedKey) return acc;
									if (typeof mapValue !== "string") return acc;
									const trimmedValue = mapValue.trim();
									if (!trimmedValue) return acc;
									acc.push([trimmedKey, trimmedValue]);
									return acc;
								}, [] as Array<[string, string]>),
					)
				: undefined;
		const fallbackLocale =
			typeof (localeParamInput as { fallback?: unknown }).fallback === "string"
				? ((localeParamInput as { fallback?: string }).fallback ?? "").trim()
				: undefined;
		if (key || strategy) {
			localeParam = {
				// Keep key for backward compatibility even if using path-prefix
				...(key ? { key } : {}),
				...(strategy ? { strategy } : {}),
				...(mapping ? { mapping } : {}),
				...(fallbackLocale ? { fallback: fallbackLocale } : {}),
			};
		}
  }

	return {
		id: targetId,
		label,
		remoteOrigin,
		proxyPath,
		favicon,
		proxyFavicon,
		adapter,
		...(locales ? { locales } : {}),
		...(localeParam ? { localeParam } : {}),
	};
}

export interface DashboardSettings {
	defaultView: DashboardDefaultView;
	appMode: DashboardAppMode;
	language: DashboardLanguage;
	syncServerStateToClients: boolean;
	autoAddServerToDefaultProfile: boolean;
	enableServerDebug: boolean;
	openDebugInNewWindow: boolean;
	showRawCapabilityJson: boolean;
	// Developer: show default HTTP headers (redacted) in Server Details
	showDefaultHeaders: boolean;
	menuBarIconMode: MenuBarIconMode;
	showDockIcon: boolean;
	clientDefaultMode: ClientDefaultMode;
	// Default filter applied to Clients list page
	clientListDefaultFilter: ClientListDefaultFilter;
	clientBackupStrategy: ClientBackupStrategy;
	clientBackupLimit: number;
	marketBlacklist: MarketBlacklistEntry[];
	enableMarketBlacklist: boolean;
	showApiDocsMenu: boolean;
	defaultMarket: DefaultMarket;
	marketPortals: Record<string, MarketPortalMeta>;
}

export interface MarketBlacklistEntry {
	serverId: string;
	label: string;
	hiddenAt: number;
	description?: string;
}

interface AppState {
	theme: Theme;
	setTheme: (theme: Theme) => void;
	sidebarOpen: boolean;
	toggleSidebar: () => void;
	setSidebarOpen: (open: boolean) => void;
	inspectorViewMode: "browse" | "debug";
	setInspectorViewMode: (mode: "browse" | "debug") => void;
	dashboardSettings: DashboardSettings;
	setDashboardSetting: <K extends keyof DashboardSettings>(
		key: K,
		value: DashboardSettings[K],
	) => void;
	updateDashboardSettings: (patch: Partial<DashboardSettings>) => void;
	removeFromMarketBlacklist: (serverId: string) => void;
	addToMarketBlacklist: (entry: MarketBlacklistEntry) => void;
}

const DASHBOARD_SETTINGS_KEY = "mcp_dashboard_settings";

const defaultDashboardSettings: DashboardSettings = {
	defaultView: "grid",
	appMode: "expert",
	language: "en",
	syncServerStateToClients: false,
	autoAddServerToDefaultProfile: false,
	enableServerDebug: true,
	openDebugInNewWindow: false,
	showRawCapabilityJson: false,
	showDefaultHeaders: true,
	menuBarIconMode: "runtime",
	showDockIcon: true,
	clientDefaultMode: "hosted",
	clientListDefaultFilter: "all",
	clientBackupStrategy: "keep_n",
	clientBackupLimit: 5,
	marketBlacklist: [],
	enableMarketBlacklist: false,
	showApiDocsMenu: false,
	defaultMarket: "mcpmarket",
	marketPortals: cloneMarketPortals(DEFAULT_MARKET_PORTALS),
};

function normalizeDashboardSettings(
	base: DashboardSettings,
	patch?: Partial<DashboardSettings>,
): DashboardSettings {
	if (!patch || typeof patch !== "object") {
		return {
			...base,
			marketPortals: cloneMarketPortals(base.marketPortals),
		};
	}

	const next: DashboardSettings = {
		...base,
		marketPortals: cloneMarketPortals(base.marketPortals),
	};

	if (patch.defaultView === "list" || patch.defaultView === "grid") {
		next.defaultView = patch.defaultView;
	}

	if (patch.appMode === "express" || patch.appMode === "expert") {
		next.appMode = patch.appMode;
	}

	if (
		patch.language === "en" ||
		patch.language === "zh-cn" ||
		patch.language === "ja"
	) {
		next.language = patch.language;
	}

	if (typeof patch.syncServerStateToClients === "boolean") {
		next.syncServerStateToClients = patch.syncServerStateToClients;
	}

	if (typeof patch.autoAddServerToDefaultProfile === "boolean") {
		next.autoAddServerToDefaultProfile = patch.autoAddServerToDefaultProfile;
	}

	if (typeof patch.enableServerDebug === "boolean") {
		next.enableServerDebug = patch.enableServerDebug;
	}

	if (typeof patch.openDebugInNewWindow === "boolean") {
		next.openDebugInNewWindow = patch.openDebugInNewWindow;
	}

	if (typeof patch.showRawCapabilityJson === "boolean") {
		next.showRawCapabilityJson = patch.showRawCapabilityJson;
	}

	if (typeof patch.showDefaultHeaders === "boolean") {
		next.showDefaultHeaders = patch.showDefaultHeaders;
	}

	if (
		patch.menuBarIconMode === "runtime" ||
		patch.menuBarIconMode === "hidden"
	) {
		next.menuBarIconMode = patch.menuBarIconMode;
	}

	if (typeof patch.showDockIcon === "boolean") {
		next.showDockIcon = patch.showDockIcon;
	}

	if (!next.showDockIcon) {
		next.menuBarIconMode = "runtime";
	}

	if (typeof patch.showApiDocsMenu === "boolean") {
		next.showApiDocsMenu = patch.showApiDocsMenu;
	}

	if (typeof patch.enableMarketBlacklist === "boolean") {
		next.enableMarketBlacklist = patch.enableMarketBlacklist;
	}

	// Accept any known portal id (built-in or merged) in addition to "official"
	if (patch.defaultMarket) {
		if (patch.defaultMarket === "official") {
			next.defaultMarket = "official";
		} else if (MARKET_PORTAL_MAP[patch.defaultMarket as string]) {
			next.defaultMarket = patch.defaultMarket as any;
		}
	}

	if (
		patch.clientDefaultMode === "hosted" ||
		patch.clientDefaultMode === "transparent"
	) {
		next.clientDefaultMode = patch.clientDefaultMode;
	}

	if (
		patch.clientListDefaultFilter === "all" ||
		patch.clientListDefaultFilter === "detected" ||
		patch.clientListDefaultFilter === "managed"
	) {
		next.clientListDefaultFilter = patch.clientListDefaultFilter;
	}

	if (
		patch.clientBackupStrategy === "keep_n" ||
		patch.clientBackupStrategy === "keep_last" ||
		patch.clientBackupStrategy === "none"
	) {
		next.clientBackupStrategy = patch.clientBackupStrategy;
	}

	if (patch.clientBackupLimit !== undefined) {
		const candidate = Number(patch.clientBackupLimit);
		if (Number.isFinite(candidate) && candidate > 0) {
			next.clientBackupLimit = Math.max(1, Math.round(candidate));
		}
	}

	if (Array.isArray(patch.marketBlacklist)) {
		const unique = new Map<string, MarketBlacklistEntry>();
		for (const item of patch.marketBlacklist) {
			if (!item || typeof item !== "object") continue;
			const serverId = String(item.serverId || "").trim();
			const label = String(item.label || "").trim();
			const hiddenAt = Number(item.hiddenAt);
			const description =
				item.description !== undefined
					? String(item.description).trim()
					: undefined;
			if (!serverId || !label || !Number.isFinite(hiddenAt)) continue;
			const entry: MarketBlacklistEntry = {
				serverId,
				label,
				hiddenAt,
				...(description ? { description } : {}),
			};
			unique.set(serverId, entry);
		}
		next.marketBlacklist = Array.from(unique.values()).sort(
			(a, b) => b.hiddenAt - a.hiddenAt,
		);
	}

	if (patch.marketPortals && typeof patch.marketPortals === "object") {
		const merged = cloneMarketPortals(next.marketPortals);
		for (const [rawId, value] of Object.entries(patch.marketPortals)) {
			const fallback = merged[rawId] ?? DEFAULT_MARKET_PORTALS[rawId];
			const sanitized = sanitizeMarketPortalMeta(rawId, value, fallback);
			if (!sanitized) continue;
			if (sanitized.id !== rawId) {
				delete merged[rawId];
			}
			merged[sanitized.id] = sanitized;
		}
		next.marketPortals = merged;
	}

	return next;
}

function readDashboardSettings(): DashboardSettings {
	if (typeof window === "undefined") {
		return { ...defaultDashboardSettings };
	}

	try {
		const saved = window.localStorage.getItem(DASHBOARD_SETTINGS_KEY);
		if (!saved) return { ...defaultDashboardSettings };
		const parsed = JSON.parse(saved) as Partial<DashboardSettings> | null;
		return normalizeDashboardSettings(
			defaultDashboardSettings,
			parsed ?? undefined,
		);
	} catch {
		return { ...defaultDashboardSettings };
	}
}

function persistDashboardSettings(settings: DashboardSettings) {
	try {
		if (typeof window !== "undefined") {
			window.localStorage.setItem(
				DASHBOARD_SETTINGS_KEY,
				JSON.stringify(settings),
			);
		}
	} catch {
		// Swallow persistence errors to avoid blocking UI updates.
	}
	void syncDesktopShellPreferences(settings);
}

async function syncDesktopShellPreferences(settings: DashboardSettings) {
	if (!isTauriEnvironmentSync()) {
		return;
	}

	try {
		const { invoke } = await import("@tauri-apps/api/core");
		await invoke("mcp_shell_apply_preferences", {
			payload: {
				menuBarIconMode: settings.menuBarIconMode,
				showDockIcon: settings.showDockIcon,
			},
		});
	} catch (error) {
		if (import.meta.env.DEV) {
			console.warn("[store] Failed to sync desktop shell preferences", error);
		}
	}
}

function getInitialTheme(): Theme {
	try {
		const saved =
			typeof window !== "undefined" ? localStorage.getItem("mcp_theme") : null;
		if (saved === "light" || saved === "dark" || saved === "system")
			return saved;
	} catch {
		/* noop */
	}
	return "system";
}

function getInitialInspectorMode(): "browse" | "debug" {
	try {
		const saved =
			typeof window !== "undefined"
				? localStorage.getItem("mcp_inspector_view")
				: null;
		if (saved === "debug") return "debug";
	} catch {
		/* noop */
	}
	return "browse";
}

export const useAppStore = create<AppState>((set) => ({
	theme: getInitialTheme(),
	setTheme: (theme) => {
		try {
			if (typeof window !== "undefined")
				localStorage.setItem("mcp_theme", theme);
		} catch {
			/* noop */
		}
		set({ theme });
	},
	sidebarOpen: true,
	toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
	setSidebarOpen: (open) => set({ sidebarOpen: open }),
	inspectorViewMode: getInitialInspectorMode(),
	setInspectorViewMode: (mode) => {
		try {
			if (typeof window !== "undefined")
				localStorage.setItem("mcp_inspector_view", mode);
		} catch {
			/* noop */
		}
		set({ inspectorViewMode: mode });
	},
	dashboardSettings: readDashboardSettings(),
	setDashboardSetting: (key, value) => {
		set((state) => {
			const next = normalizeDashboardSettings(state.dashboardSettings, {
				[key]: value,
			} as Partial<DashboardSettings>);
			persistDashboardSettings(next);
			return { dashboardSettings: next };
		});
	},
	updateDashboardSettings: (patch) => {
		set((state) => {
			const next = normalizeDashboardSettings(state.dashboardSettings, patch);
			persistDashboardSettings(next);
			return { dashboardSettings: next };
		});
	},
	removeFromMarketBlacklist: (serverId) => {
		set((state) => {
			const filtered = state.dashboardSettings.marketBlacklist.filter(
				(entry) => entry.serverId !== serverId,
			);
			const next = normalizeDashboardSettings(state.dashboardSettings, {
				marketBlacklist: filtered,
			});
			persistDashboardSettings(next);
			return { dashboardSettings: next };
		});
	},
	addToMarketBlacklist: (entry) => {
		set((state) => {
			const existing = state.dashboardSettings.marketBlacklist.filter(
				(item) => item.serverId !== entry.serverId,
			);
			const updated = [...existing, entry];
			const next = normalizeDashboardSettings(state.dashboardSettings, {
				marketBlacklist: updated,
			});
			persistDashboardSettings(next);
			return { dashboardSettings: next };
		});
	},
}));
