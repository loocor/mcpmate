import { create } from "zustand";
import type { MarketPortalDefinition } from "../pages/market/portal-registry";
import { isTauriEnvironmentSync } from "./platform";
import type { ProfileTokenEstimateMethod } from "./profile-token-estimate-method";
import {
	isProfileTokenEstimateMethod,
	PROFILE_TOKEN_ESTIMATE_METHOD_DEFAULT,
} from "./profile-token-estimate-method";
import type { Theme } from "./types";

/** Persisted third-party market portal metadata (re-export shape for market UI). */
export type MarketPortalMeta = MarketPortalDefinition;

export type DashboardDefaultView = "list" | "grid";
export type DashboardAppMode = "express" | "expert";
export type DashboardLanguage = "en" | "zh-cn" | "ja";
export type ClientDefaultMode = "unify" | "hosted" | "transparent";
export type ClientListDefaultFilter = "all" | "detected" | "managed";
export type ClientBackupStrategy = "keep_n" | "keep_last" | "none";
export type MenuBarIconMode = "runtime" | "hidden";
/** Default Market portal selection (`official` or a registered third-party id). */
export type DefaultMarket = string;

export interface DashboardSettings {
	defaultView: DashboardDefaultView;
	appMode: DashboardAppMode;
	language: DashboardLanguage;
	syncServerStateToClients: boolean;
	autoAddServerToDefaultProfile: boolean;
	enableServerDebug: boolean;
	openDebugInNewWindow: boolean;
	showRawCapabilityJson: boolean;
	showDefaultHeaders: boolean;
	menuBarIconMode: MenuBarIconMode;
	showDockIcon: boolean;
	clientDefaultMode: ClientDefaultMode;
	clientListDefaultFilter: ClientListDefaultFilter;
	clientBackupStrategy: ClientBackupStrategy;
	clientBackupLimit: number;
	marketBlacklist: MarketBlacklistEntry[];
	enableMarketBlacklist: boolean;
	showApiDocsMenu: boolean;
	showClientLiveLogs: boolean;
	showServerLiveLogs: boolean;
	showProfileLiveLogs: boolean;
	/** Tokenizer used for profile capability payload estimates (ledger + dashboard charts). */
	profileTokenEstimateMethod: ProfileTokenEstimateMethod;
	defaultMarket: DefaultMarket;
	marketPortals: Record<string, MarketPortalMeta>;
}

export interface MarketBlacklistEntry {
	serverId: string;
	label: string;
	hiddenAt: number;
	description?: string;
}

/** Payload from `mcpmate://import/server` (desktop) for Uni-Import on Servers page. */
export interface PendingServerDeepLinkImport {
	text: string;
	format?: string;
	source?: string;
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
	pendingServerDeepLinkImport: PendingServerDeepLinkImport | null;
	setPendingServerDeepLinkImport: (
		payload: PendingServerDeepLinkImport | null,
	) => void;
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
	clientDefaultMode: "unify",
	clientListDefaultFilter: "all",
	clientBackupStrategy: "keep_n",
	clientBackupLimit: 5,
	marketBlacklist: [],
	enableMarketBlacklist: false,
	showApiDocsMenu: true,
	showClientLiveLogs: true,
	showServerLiveLogs: true,
	showProfileLiveLogs: true,
	profileTokenEstimateMethod: PROFILE_TOKEN_ESTIMATE_METHOD_DEFAULT,
	defaultMarket: "official",
	marketPortals: {},
};

function normalizeDashboardSettings(
	base: DashboardSettings,
	patch?: Partial<DashboardSettings>,
): DashboardSettings {
	if (!patch || typeof patch !== "object") {
		return {
			...base,
			defaultMarket: "official",
			marketPortals: {},
		};
	}

	const next: DashboardSettings = {
		...base,
		marketPortals: {},
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

	if (typeof patch.showClientLiveLogs === "boolean") {
		next.showClientLiveLogs = patch.showClientLiveLogs;
	}

	if (typeof patch.showServerLiveLogs === "boolean") {
		next.showServerLiveLogs = patch.showServerLiveLogs;
	}

	if (typeof patch.showProfileLiveLogs === "boolean") {
		next.showProfileLiveLogs = patch.showProfileLiveLogs;
	}

	if (isProfileTokenEstimateMethod(patch.profileTokenEstimateMethod)) {
		next.profileTokenEstimateMethod = patch.profileTokenEstimateMethod;
	}

	if (typeof patch.enableMarketBlacklist === "boolean") {
		next.enableMarketBlacklist = patch.enableMarketBlacklist;
	}

	if (
		patch.clientDefaultMode === "unify" ||
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

	next.defaultMarket = "official";
	next.marketPortals = {};

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
	pendingServerDeepLinkImport: null,
	setPendingServerDeepLinkImport: (payload) =>
		set({ pendingServerDeepLinkImport: payload }),
}));
