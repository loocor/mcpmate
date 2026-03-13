import { Moon, RotateCcw, Sun } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../components/ui/dialog";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Segment, type SegmentOption } from "../../components/ui/segment";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { API_BASE_URL, setApiBaseUrl } from "../../lib/api";
import { SUPPORTED_LANGUAGES } from "../../lib/i18n/index";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import {
	detectTauriEnvironment,
	isTauriEnvironmentSync,
} from "../../lib/platform";
import {
	type ClientBackupStrategy,
	type ClientDefaultMode,
	type ClientListDefaultFilter,
	type DashboardAppMode,
	type DashboardDefaultView,
	type DashboardLanguage,
	type DashboardSettings,
	type DefaultMarket,
	type MarketBlacklistEntry,
	type MenuBarIconMode,
	useAppStore,
} from "../../lib/store";
import type { OpenSourceDocument } from "../../types/open-source";
import { mergePortalOverrides } from "../market/portal-registry";
import { AboutLicensesSection } from "./about-licenses-section";

// Options for Segment components
const THEME_CONFIG = [
	{
		value: "light" as const,
		icon: Sun,
		labelKey: "settings:options.theme.light",
		fallback: "Light",
	},
	{
		value: "dark" as const,
		icon: Moon,
		labelKey: "settings:options.theme.dark",
		fallback: "Dark",
	},
];

const CLIENT_FILTER_CONFIG = [
	{
		value: "all" as const,
		labelKey: "settings:clients.defaultVisibility.all",
		fallback: "All",
	},
	{
		value: "detected" as const,
		labelKey: "settings:clients.defaultVisibility.detected",
		fallback: "Detected",
	},
	{
		value: "managed" as const,
		labelKey: "settings:clients.defaultVisibility.managed",
		fallback: "Managed",
	},
];

const DEFAULT_VIEW_CONFIG = [
	{
		value: "list" as const,
		labelKey: "settings:options.defaultView.list",
		fallback: "List",
	},
	{
		value: "grid" as const,
		labelKey: "settings:options.defaultView.grid",
		fallback: "Grid",
	},
];

const APPLICATION_MODE_CONFIG = [
	{
		value: "express" as const,
		labelKey: "settings:options.appMode.express",
		fallback: "Express",
	},
	{
		value: "expert" as const,
		labelKey: "settings:options.appMode.expert",
		fallback: "Expert",
	},
];

const CLIENT_MODE_CONFIG = [
	{
		value: "hosted" as const,
		labelKey: "settings:options.clientMode.hosted",
		fallback: "Hosted",
	},
	{
		value: "transparent" as const,
		labelKey: "settings:options.clientMode.transparent",
		fallback: "Transparent",
	},
];

const BACKUP_STRATEGY_CONFIG = [
	{
		value: "keep_n" as const,
		labelKey: "settings:options.backup.keepN",
		fallback: "Keep N",
	},
	{
		value: "keep_last" as const,
		labelKey: "settings:options.backup.keepLast",
		fallback: "Keep Last",
	},
	{
		value: "none" as const,
		labelKey: "settings:options.backup.none",
		fallback: "None",
	},
];

interface ShellPreferencesResponse {
	menu_bar_icon_mode: MenuBarIconMode;
	show_dock_icon: boolean;
}

const MENU_BAR_ICON_OPTIONS: ReadonlyArray<{
	value: MenuBarIconMode;
	labelKey: string;
	fallback: string;
}> = [
	{
		value: "runtime",
		labelKey: "settings:options.menuBar.runtime",
		fallback: "Visible When Running",
	},
	{
		value: "hidden",
		labelKey: "settings:options.menuBar.hidden",
		fallback: "Hidden",
	},
];

export function SettingsPage() {
	usePageTranslations("settings");
	const languageId = useId();
	const backupLimitId = useId();
	const menuBarSelectId = useId();
	const { t, i18n } = useTranslation();

	const theme = useAppStore((state) => state.theme);
	const setTheme = useAppStore((state) => state.setTheme);
	const dashboardSettings = useAppStore((state) => state.dashboardSettings);
	const setDashboardSetting = useAppStore((state) => state.setDashboardSetting);
	const updateDashboardSettings = useAppStore(
		(state) => state.updateDashboardSettings,
	);
	const removeFromMarketBlacklist = useAppStore(
		(state) => state.removeFromMarketBlacklist,
	);
	const [licenseDocument, setLicenseDocument] =
		useState<OpenSourceDocument | null>(null);
	const [licenseLoaded, setLicenseLoaded] = useState(false);
	const [isTauriShell, setIsTauriShell] = useState(() =>
		isTauriEnvironmentSync(),
	);

	// Developer → Backend ports (API/MCP)
	const [apiPort, setApiPort] = useState<number | "">("");
	const [mcpPort, setMcpPort] = useState<number | "">("");
	const [loadingPorts, setLoadingPorts] = useState(false);
	const [applyBusy, setApplyBusy] = useState(false);
	const [webDialogOpen, setWebDialogOpen] = useState(false);
	const devUrl =
		typeof window !== "undefined" && window.location?.origin?.includes(":")
			? window.location.origin
			: "http://localhost:5173";

	const loadRuntimePorts = useCallback(async () => {
		setLoadingPorts(true);
		try {
			// seed from local persistence if available
			let hadCached = false;
			try {
				const cachedApi = window.localStorage?.getItem(
					"mcpmate.system.api_port",
				);
				const cachedMcp = window.localStorage?.getItem(
					"mcpmate.system.mcp_port",
				);
				if (cachedApi) {
					setApiPort(Number(cachedApi));
					hadCached = true;
				}
				if (cachedMcp) {
					setMcpPort(Number(cachedMcp));
					hadCached = true;
				}
			} catch {}
			if (isTauriEnvironmentSync()) {
				const { invoke } = await import("@tauri-apps/api/core");
				const resp = (await invoke("mcp_shell_read_runtime_ports")) as {
					api_port: number;
					mcp_port: number;
					api_url: string;
					mcp_http_url: string;
					mcp_sse_url: string;
				};
				// In web mode, if user has configured ports (cached), keep them visible.
				// Only initialize from backend when there is no cached value.
				if (!hadCached) {
					setApiPort(resp.api_port);
					setMcpPort(resp.mcp_port);
					try {
						window.localStorage?.setItem(
							"mcpmate.system.api_port",
							String(resp.api_port),
						);
						window.localStorage?.setItem(
							"mcpmate.system.mcp_port",
							String(resp.mcp_port),
						);
					} catch {}
				}
				if (resp.api_url && !API_BASE_URL.includes(`:${resp.api_port}`)) {
					setApiBaseUrl(resp.api_url);
				}
				return;
			}

			const apiBase = API_BASE_URL || "";
			const url = apiBase ? `${apiBase}/api/system/ports` : `/api/system/ports`;
			const response = await fetch(url, { cache: "no-store" });
			if (response.ok) {
				const data = await response.json();
				const d = data?.data ?? data;
				if (!hadCached) {
					if (typeof d?.api_port === "number") {
						setApiPort(d.api_port);
						try {
							window.localStorage?.setItem(
								"mcpmate.system.api_port",
								String(d.api_port),
							);
						} catch {}
					}
					if (typeof d?.mcp_port === "number") {
						setMcpPort(d.mcp_port);
						try {
							window.localStorage?.setItem(
								"mcpmate.system.mcp_port",
								String(d.mcp_port),
							);
						} catch {}
					}
				}
			}
		} finally {
			setLoadingPorts(false);
		}
	}, []);

	const tabTriggerClass =
		"justify-start px-3 py-2 text-left text-sm font-medium text-slate-600 data-[state=active]:text-emerald-700 dark:text-slate-300";

	const themeOptions = useMemo<SegmentOption[]>(
		() =>
			THEME_CONFIG.map(({ value, icon: Icon, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
				icon: <Icon className="h-4 w-4" />,
			})),
		[t, i18n.language],
	);

	const defaultViewOptions = useMemo<SegmentOption[]>(
		() =>
			DEFAULT_VIEW_CONFIG.map(({ value, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
			})),
		[t, i18n.language],
	);

	const applicationModeOptions = useMemo<SegmentOption[]>(
		() =>
			APPLICATION_MODE_CONFIG.map(({ value, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
			})),
		[t, i18n.language],
	);

	const clientModeOptions = useMemo<SegmentOption[]>(
		() =>
			CLIENT_MODE_CONFIG.map(({ value, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
			})),
		[t, i18n.language],
	);

	const clientFilterOptions = useMemo<SegmentOption[]>(
		() =>
			CLIENT_FILTER_CONFIG.map(({ value, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
			})),
		[t, i18n.language],
	);

	const backupStrategyOptions = useMemo<SegmentOption[]>(
		() =>
			BACKUP_STRATEGY_CONFIG.map(({ value, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
			})),
		[t, i18n.language],
	);

	const languageOptions = useMemo(
		() =>
			SUPPORTED_LANGUAGES.map(({ store, i18n: langCode, fallback }) => ({
				value: store,
				label: t(`languageNames.${langCode}`, { defaultValue: fallback }),
			})),
		[t, i18n.language],
	);

	const menuBarOptions = useMemo(
		() =>
			MENU_BAR_ICON_OPTIONS.map((option) => ({
				...option,
				label: t(option.labelKey, {
					defaultValue: option.fallback,
				}),
			})),
		[t, i18n.language],
	);

	// Build available portals list for Default Market selector
	const availablePortals = useMemo(
		() =>
			Object.values(mergePortalOverrides(dashboardSettings.marketPortals)).sort(
				(a, b) => a.label.localeCompare(b.label),
			),
		[dashboardSettings.marketPortals],
	);

	useEffect(() => {
		if (isTauriShell) {
			return undefined;
		}
		let cancelled = false;
		const detect = async () => {
			const result = await detectTauriEnvironment();
			if (!cancelled) {
				setIsTauriShell(result);
			}
		};
		void detect();
		return () => {
			cancelled = true;
		};
	}, [isTauriShell]);

	useEffect(() => {
		let cancelled = false;
		const noticesUrl = `${import.meta.env.BASE_URL}open-source-notices.json`;

		const loadLicenses = async () => {
			try {
				const response = await fetch(noticesUrl, {
					cache: "no-store",
				});

				if (!response.ok) {
					return;
				}

				const data = (await response.json()) as OpenSourceDocument;
				if (!cancelled && data && Array.isArray(data.sections)) {
					setLicenseDocument(data);
				}
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn(
						"[SettingsPage] Unable to load open-source notices:",
						error,
					);
				}
			} finally {
				if (!cancelled) {
					setLicenseLoaded(true);
				}
			}
		};

		void loadLicenses();

		return () => {
			cancelled = true;
		};
	}, []);

	useEffect(() => {
		void loadRuntimePorts();
	}, [loadRuntimePorts]);

	useEffect(() => {
		if (!isTauriShell) {
			return undefined;
		}

		let cancelled = false;
		const apply = async () => {
			try {
				const { invoke } = await import("@tauri-apps/api/core");
				const prefs =
					(await invoke<ShellPreferencesResponse>(
						"mcp_shell_read_preferences",
					)) ?? null;
				if (!cancelled && prefs) {
					updateDashboardSettings({
						menuBarIconMode: prefs.menu_bar_icon_mode,
						showDockIcon: prefs.show_dock_icon,
					});
				}
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn(
						"[SettingsPage] Failed to load desktop shell preferences",
						error,
					);
				}
			}
		};

		void apply();
		return () => {
			cancelled = true;
		};
	}, [isTauriShell, updateDashboardSettings]);

	const showLicenseTab = licenseLoaded && licenseDocument !== null;

	return (
		<div className="space-y-4">
			<div className="flex items-center gap-2 min-w-0">
				<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
					{t("settings:title", { defaultValue: "Settings" })}
				</p>
			</div>

			<Tabs
				defaultValue="general"
				orientation="vertical"
				className="flex flex-col gap-4 xl:flex-row xl:items-start"
			>
				<TabsList className="flex w-full flex-row flex-wrap gap-2 overflow-x-auto rounded-lg p-2 xl:w-64 xl:flex-col xl:overflow-visible xl:p-3 xl:self-start">
					<TabsTrigger value="general" className={tabTriggerClass}>
						{t("settings:tabs.general", { defaultValue: "General" })}
					</TabsTrigger>
					<TabsTrigger value="appearance" className={tabTriggerClass}>
						{t("settings:tabs.appearance", { defaultValue: "Appearance" })}
					</TabsTrigger>
					<TabsTrigger value="servers" className={tabTriggerClass}>
						{t("settings:tabs.serverControls", {
							defaultValue: "Server Controls",
						})}
					</TabsTrigger>
					<TabsTrigger value="clients" className={tabTriggerClass}>
						{t("settings:tabs.clientDefaults", {
							defaultValue: "Client Defaults",
						})}
					</TabsTrigger>
					<TabsTrigger value="market" className={tabTriggerClass}>
						{t("settings:tabs.market", { defaultValue: "MCP Market" })}
					</TabsTrigger>
					<TabsTrigger value="develop" className={tabTriggerClass}>
						{t("settings:tabs.developer", { defaultValue: "Developer" })}
					</TabsTrigger>
					<TabsTrigger value="system" className={tabTriggerClass}>
						{t("settings:tabs.system", { defaultValue: "System" })}
					</TabsTrigger>
					{showLicenseTab && (
						<TabsTrigger value="about" className={tabTriggerClass}>
							{t("settings:tabs.about", { defaultValue: "About & Licenses" })}
						</TabsTrigger>
					)}
				</TabsList>

				<div className="flex-1">
					<TabsContent value="general" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:general.title", { defaultValue: "General" })}
								</CardTitle>
								<CardDescription>
									{t("settings:general.description", {
										defaultValue:
											"Baseline preferences for the main workspace views.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-4">
								{/* Default View */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:general.defaultView", {
												defaultValue: "Default View",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:general.defaultViewDescription", {
												defaultValue:
													"Choose the default layout for displaying items.",
											})}
										</p>
									</div>
									<div className="w-48">
										<Segment
											options={defaultViewOptions}
											value={dashboardSettings.defaultView}
											onValueChange={(value) =>
												setDashboardSetting(
													"defaultView",
													value as DashboardDefaultView,
												)
											}
											showDots={false}
										/>
									</div>
								</div>

								{/* Application Mode */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:general.appMode", {
												defaultValue: "Application Mode",
											})}{" "}
											<sup>{t("wipTag", { defaultValue: "(WIP)" })}</sup>
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:general.appModeDescription", {
												defaultValue: "Select the interface complexity level.",
											})}
										</p>
									</div>
									<div className="w-48">
										<Segment
											options={applicationModeOptions}
											value={dashboardSettings.appMode}
											onValueChange={(value) =>
												setDashboardSetting(
													"appMode",
													value as DashboardAppMode,
												)
											}
											showDots={false}
										/>
									</div>
								</div>

								{/* Language Selection */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:general.language", {
												defaultValue: "Language",
											})}{" "}
											<sup>{t("wipTag", { defaultValue: "(WIP)" })}</sup>
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:general.languageDescription", {
												defaultValue:
													"Multi-language support is currently in development.",
											})}
										</p>
									</div>
									<Select
										value={dashboardSettings.language}
										onValueChange={(value: DashboardLanguage) =>
											setDashboardSetting("language", value)
										}
									>
										<SelectTrigger id={languageId} className="w-48">
											<SelectValue
												placeholder={t("settings:general.languagePlaceholder", {
													defaultValue: "Select language",
												})}
											/>
										</SelectTrigger>
										<SelectContent>
											{languageOptions.map((option) => (
												<SelectItem key={option.value} value={option.value}>
													{option.label}
												</SelectItem>
											))}
										</SelectContent>
									</Select>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="appearance" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:appearance.title", {
										defaultValue: "Appearance",
									})}
								</CardTitle>
								<CardDescription>
									{t("settings:appearance.description", {
										defaultValue:
											"Customize the look and feel of the dashboard.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-4">
								<div className="space-y-4">
									<div className="flex items-center justify-between gap-4">
										<div className="space-y-0.5">
											<h3 className="text-base font-medium">
												{t("settings:appearance.themeTitle", {
													defaultValue: "Theme",
												})}
											</h3>
											<p className="text-sm text-slate-500">
												{t("settings:appearance.themeDescription", {
													defaultValue: "Switch between light and dark mode.",
												})}
											</p>
										</div>
										<div className="w-48">
											<Segment
												options={themeOptions}
												value={theme === "system" ? "light" : theme}
												onValueChange={(value) =>
													setTheme(value as "light" | "dark")
												}
												showDots={false}
											/>
										</div>
									</div>

									<div className="flex items-center justify-between gap-4">
										<div className="space-y-0.5">
											<h3 className="text-base font-medium">
												{t("settings:appearance.systemPreferenceTitle", {
													defaultValue: "System Preference",
												})}
											</h3>
											<p className="text-sm text-slate-500">
												{t("settings:appearance.systemPreferenceDescription", {
													defaultValue:
														"Follow the operating system preference automatically.",
												})}
											</p>
										</div>
										<Switch
											checked={theme === "system"}
											onCheckedChange={(checked) =>
												setTheme(checked ? "system" : "light")
											}
										/>
									</div>

									{isTauriShell && (
										<div className="space-y-4 dark:border-slate-800">
											<div className="flex items-center justify-between gap-4">
												<div className="space-y-0.5">
													<h3 className="text-base font-medium">
														{t("settings:appearance.menuBarTitle", {
															defaultValue: "Menu Bar Icon",
														})}{" "}
														<sup>{t("wipTag", { defaultValue: "(WIP)" })}</sup>
													</h3>
													<p className="text-sm text-slate-500">
														{t("settings:appearance.menuBarDescription", {
															defaultValue:
																"Choose when the desktop tray icon should appear.",
														})}
													</p>
												</div>
												<Select
													value={dashboardSettings.menuBarIconMode}
													onValueChange={(value: MenuBarIconMode) =>
														setDashboardSetting("menuBarIconMode", value)
													}
												>
													<SelectTrigger id={menuBarSelectId} className="w-56">
														<SelectValue
															placeholder={t("placeholders.menuBarVisibility", {
																defaultValue: "Menu bar visibility",
															})}
														/>
													</SelectTrigger>
													<SelectContent>
														{menuBarOptions.map((option) => (
															<SelectItem
																key={option.value}
																value={option.value}
																disabled={
																	option.value === "hidden" &&
																	!dashboardSettings.showDockIcon
																}
															>
																{option.label}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>

											<div className="flex items-center justify-between gap-4">
												<div className="space-y-0.5">
													<h3 className="text-base font-medium">
														{t("settings:appearance.dockTitle", {
															defaultValue: "Dock Icon",
														})}{" "}
														<sup>{t("wipTag", { defaultValue: "(WIP)" })}</sup>
													</h3>
													<p className="text-sm text-slate-500">
														{t("settings:appearance.dockDescription", {
															defaultValue:
																"Display MCPMate in the macOS Dock or run silently from the menu bar.",
														})}
													</p>
												</div>
												<Switch
													checked={dashboardSettings.showDockIcon}
													onCheckedChange={(checked) =>
														setDashboardSetting("showDockIcon", checked)
													}
												/>
											</div>

											{!dashboardSettings.showDockIcon && (
												<p className="text-xs leading-relaxed text-slate-500">
													{t("settings:appearance.dockHiddenNotice", {
														defaultValue:
															"The Dock icon is hidden. The menu bar icon will remain visible so you can reopen MCPMate.",
													})}
												</p>
											)}
										</div>
									)}
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="servers" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:servers.title", {
										defaultValue: "Server Controls",
									})}
								</CardTitle>
								<CardDescription>
									{t("settings:servers.description", {
										defaultValue:
											"Decide how server operations propagate across clients.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:servers.syncTitle", {
												defaultValue: "Sync Global Start/Stop",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:servers.syncDescription", {
												defaultValue:
													"Push global enable state to managed clients instantly.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.syncServerStateToClients}
										onCheckedChange={(checked) =>
											setDashboardSetting("syncServerStateToClients", checked)
										}
									/>
								</div>

								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:servers.autoAddTitle", {
												defaultValue: "Auto Add To Default Profile",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:servers.autoAddDescription", {
												defaultValue:
													"Include new servers in the default profile automatically.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.autoAddServerToDefaultProfile}
										onCheckedChange={(checked) =>
											setDashboardSetting(
												"autoAddServerToDefaultProfile",
												checked,
											)
										}
									/>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="clients" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:clients.title", {
										defaultValue: "Client Defaults",
									})}
								</CardTitle>
								<CardDescription>
									{t("settings:clients.description", {
										defaultValue:
											"Configure default rollout and backup behavior for client apps.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-4">
								{/* {t("settings:clients.modeTitle", { defaultValue: "Client Application Mode" })} */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.modeTitle", {
												defaultValue: "Client Application Mode",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:clients.modeDescription", {
												defaultValue:
													"Choose how client applications should operate by default.",
											})}
										</p>
									</div>
									<div className="w-64">
										<Segment
											options={clientModeOptions}
											value={dashboardSettings.clientDefaultMode}
											onValueChange={(value) =>
												setDashboardSetting(
													"clientDefaultMode",
													value as ClientDefaultMode,
												)
											}
											showDots={false}
										/>
									</div>
								</div>

								{/* {t("settings:clients.backupStrategyTitle", { defaultValue: "Client Backup Strategy" })} */}
								{/* Default Client Visibility */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.defaultVisibilityTitle", {
												defaultValue: "Default Client Visibility",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:clients.defaultVisibilityDescription", {
												defaultValue:
													"Choose which client statuses are shown by default on the Clients page.",
											})}
										</p>
									</div>
									<div className="w-64">
										<Segment
											options={clientFilterOptions}
											value={dashboardSettings.clientListDefaultFilter}
											onValueChange={(value) =>
												setDashboardSetting(
													"clientListDefaultFilter",
													value as ClientListDefaultFilter,
												)
											}
											showDots={false}
										/>
									</div>
								</div>

								{/* {t("settings:clients.backupStrategyTitle", { defaultValue: "Client Backup Strategy" })} */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.backupStrategyTitle", {
												defaultValue: "Client Backup Strategy",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:clients.backupStrategyDescription", {
												defaultValue:
													"Define how client configurations should be backed up.",
											})}
										</p>
									</div>
									<div className="w-64">
										<Segment
											options={backupStrategyOptions}
											value={dashboardSettings.clientBackupStrategy}
											onValueChange={(value) =>
												setDashboardSetting(
													"clientBackupStrategy",
													value as ClientBackupStrategy,
												)
											}
											showDots={false}
										/>
									</div>
								</div>

								{/* {t("settings:clients.backupLimitTitle", { defaultValue: "Maximum Backup Copies" })} */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.backupLimitTitle", {
												defaultValue: "Maximum Backup Copies",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:clients.backupLimitDescription", {
												defaultValue:
													"Set the maximum number of backup copies to keep. Applied when the strategy is set to Keep N. Values below 1 are rounded up.",
											})}
										</p>
									</div>
									<Input
										id={backupLimitId}
										type="number"
										min={1}
										value={dashboardSettings.clientBackupLimit}
										onChange={(event) => {
											const next = parseInt(event.target.value, 10);
											if (!Number.isNaN(next) && next > 0) {
												setDashboardSetting("clientBackupLimit", next);
											}
										}}
										disabled={
											dashboardSettings.clientBackupStrategy !== "keep_n"
										}
										className="w-64"
									/>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					{/* System tab: Backend ports configuration */}
					<TabsContent value="system" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:system.title", { defaultValue: "System" })}
								</CardTitle>
								<CardDescription>
									{t("settings:system.description", {
										defaultValue:
											"Configure backend ports used by the local MCP proxy and API.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-6">
								{/* Row: API Port */}
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
									<div className="space-y-1.5">
										<Label htmlFor="api-port">
											{t("settings:system.apiPortTitle", {
												defaultValue: "Backend API Port",
											})}
										</Label>
										<p className="text-xs text-slate-500">
											{t("settings:system.apiPortDescription", {
												defaultValue:
													"Port for REST and dashboard access (default 8080).",
											})}
										</p>
									</div>
									<div className="flex sm:justify-end">
										<Input
											id="api-port"
											type="number"
											min={1}
											value={apiPort}
											onChange={(e) =>
												setApiPort(
													e.target.value === "" ? "" : Number(e.target.value),
												)
											}
											className="w-56"
										/>
									</div>
								</div>

								{/* Row: MCP Port */}
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
									<div className="space-y-1.5">
										<Label htmlFor="mcp-port">
											{t("settings:system.mcpPortTitle", {
												defaultValue: "Backend MCP Server Port",
											})}
										</Label>
										<p className="text-xs text-slate-500">
											{t("settings:system.mcpPortDescription", {
												defaultValue:
													"Port for MCP transports (/mcp for HTTP, /sse for SSE). Default 8000.",
											})}
										</p>
									</div>
									<div className="flex sm:justify-end">
										<Input
											id="mcp-port"
											type="number"
											min={1}
											value={mcpPort}
											onChange={(e) =>
												setMcpPort(
													e.target.value === "" ? "" : Number(e.target.value),
												)
											}
											className="w-56"
										/>
									</div>
								</div>

								{/* Bottom actions */}
								<div className="mt-1 flex items-center justify-between">
									<p className="text-xs text-slate-500">
										{isTauriShell
											? t("settings:system.helperTauri", {
													defaultValue:
														"Tauri: Apply ports and restart backend in-place.",
												})
											: t("settings:system.helperWeb", {
													defaultValue:
														"Web: Change ports then restart the backend process externally.",
												})}
									</p>
									<div className="space-x-2">
										<Button
											variant="secondary"
											onClick={() => loadRuntimePorts()}
											disabled={loadingPorts}
										>
											{loadingPorts
												? t("loading", { defaultValue: "Loading…" })
												: t("reload", { defaultValue: "Reload" })}
										</Button>
										<Button
											variant="default"
											disabled={
												applyBusy ||
												typeof apiPort !== "number" ||
												typeof mcpPort !== "number" ||
												apiPort <= 0 ||
												mcpPort <= 0 ||
												apiPort === mcpPort
											}
											onClick={async () => {
												if (isTauriShell) {
													try {
														setApplyBusy(true);
														setApiBaseUrl(`http://127.0.0.1:${apiPort}`);
														const { invoke } = await import(
															"@tauri-apps/api/core"
														);
														await invoke(
															"mcp_shell_restart_backend_with_ports",
															{ api_port: apiPort, mcp_port: mcpPort },
														);
														try {
															window.localStorage?.setItem(
																"mcpmate.system.api_port",
																String(apiPort),
															);
															window.localStorage?.setItem(
																"mcpmate.system.mcp_port",
																String(mcpPort),
															);
														} catch {}
													} finally {
														setApplyBusy(false);
													}
												} else {
													// In web mode, switch dashboard to use the new API port immediately
													setApiBaseUrl(`http://127.0.0.1:${apiPort}`);
													try {
														window.localStorage?.setItem(
															"mcpmate.system.api_port",
															String(apiPort),
														);
														window.localStorage?.setItem(
															"mcpmate.system.mcp_port",
															String(mcpPort),
														);
													} catch {}
													setWebDialogOpen(true);
												}
											}}
										>
											{applyBusy
												? "Restarting…"
												: t("settings:system.apply", {
														defaultValue: "Apply & Restart",
													})}
										</Button>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					{/* Web-mode helper dialog for Apply & Restart */}
					<Dialog open={webDialogOpen} onOpenChange={setWebDialogOpen}>
						<DialogContent>
							<DialogHeader>
								<DialogTitle>
									{t("settings:system.webDialogTitle", {
										defaultValue: "Apply & Restart (Web)",
									})}
								</DialogTitle>
								<DialogDescription>
									{t("settings:system.webDialogDesc", {
										defaultValue:
											"The browser cannot restart the backend. Use one of the commands below with the selected ports.",
									})}
								</DialogDescription>
							</DialogHeader>
							<div className="space-y-3">
								<div>
									<p className="mb-1 text-sm font-medium">
										{t("settings:system.optionCargoTitle", {
											defaultValue: "Option A — cargo run (dev)",
										})}
									</p>
									<div className="rounded-md bg-slate-950/90 p-3 font-mono text-xs text-slate-100">
										{`MCPMATE_API_PORT=${typeof apiPort === "number" ? apiPort : 8080} MCPMATE_MCP_PORT=${typeof mcpPort === "number" ? mcpPort : 8000} MCPMATE_ALLOWED_ORIGINS=${devUrl} cargo run -p app-mcpmate`}
									</div>
									<div className="mt-2 flex gap-2">
										<Button
											variant="secondary"
											onClick={() =>
												navigator.clipboard.writeText(
													`MCPMATE_API_PORT=${typeof apiPort === "number" ? apiPort : 8080} MCPMATE_MCP_PORT=${typeof mcpPort === "number" ? mcpPort : 8000} MCPMATE_ALLOWED_ORIGINS=${devUrl} cargo run -p app-mcpmate`,
												)
											}
										>
											{t("settings:system.copy", { defaultValue: "Copy" })}
										</Button>
										<Button
											variant="outline"
											onClick={async () => {
												const url = API_BASE_URL
													? `${API_BASE_URL}/api/system/shutdown`
													: "/api/system/shutdown";
												try {
													await fetch(url, { method: "POST" });
												} catch {}
											}}
										>
											{t("settings:system.stopCurrent", {
												defaultValue: "Stop current backend",
											})}
										</Button>
									</div>
								</div>
								<div>
									<p className="mb-1 text-sm font-medium">
										{t("settings:system.optionBinaryTitle", {
											defaultValue: "Option B — binary (release)",
										})}
									</p>
									<div className="rounded-md bg-slate-950/90 p-3 font-mono text-xs text-slate-100">
										{`./app-mcpmate --api-port ${typeof apiPort === "number" ? apiPort : 8080} --mcp-port ${typeof mcpPort === "number" ? mcpPort : 8000}`}
									</div>
									<div className="mt-2">
										<Button
											variant="secondary"
											onClick={() =>
												navigator.clipboard.writeText(
													`./app-mcpmate --api-port ${typeof apiPort === "number" ? apiPort : 8080} --mcp-port ${typeof mcpPort === "number" ? mcpPort : 8000}`,
												)
											}
										>
											{t("settings:system.copy", { defaultValue: "Copy" })}
										</Button>
									</div>
								</div>
							</div>
							<DialogFooter>
								<Button onClick={() => setWebDialogOpen(false)}>
									{t("settings:system.close", { defaultValue: "Close" })}
								</Button>
							</DialogFooter>
						</DialogContent>
					</Dialog>

					<TabsContent value="develop" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:developer.title", { defaultValue: "Developer" })}
								</CardTitle>
								<CardDescription>
									{t("settings:developer.description", {
										defaultValue:
											"Experimental toggles for internal inspection and navigation visibility.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								{/* ports block moved to System tab */}
								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:developer.enableServerDebugTitle", {
												defaultValue: "Enable Server Inspection",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:developer.enableServerDebugDescription", {
												defaultValue:
													"Expose inspection instrumentation for newly added servers.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.enableServerDebug}
										onCheckedChange={(checked) =>
											setDashboardSetting("enableServerDebug", checked)
										}
									/>
								</div>

								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:developer.openDebugInNewWindowTitle", {
												defaultValue: "Open Inspect Views In New Window",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:developer.openDebugInNewWindowDescription", {
												defaultValue:
													"When enabled, Inspect buttons launch a separate tab instead of navigating the current view.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.openDebugInNewWindow}
										onCheckedChange={(checked) =>
											setDashboardSetting("openDebugInNewWindow", checked)
										}
									/>
								</div>

								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:developer.showApiDocsTitle", {
												defaultValue: "Show API Docs Menu",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:developer.showApiDocsDescription", {
												defaultValue:
													"Reveal the API Docs shortcut in the sidebar navigation.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.showApiDocsMenu}
										onCheckedChange={(checked) =>
											setDashboardSetting("showApiDocsMenu", checked)
										}
									/>
								</div>

								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:developer.showRawJsonTitle", {
												defaultValue: "Show Raw Capability JSON",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:developer.showRawJsonDescription", {
												defaultValue:
													"Display raw JSON payloads under Details in capability lists (Server details and Uni‑Import preview).",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.showRawCapabilityJson}
										onCheckedChange={(checked) =>
											setDashboardSetting("showRawCapabilityJson", checked)
										}
									/>
								</div>

								{/* Show Default Headers (redacted) */}
								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:developer.showDefaultHeadersTitle", {
												defaultValue: "Show Default HTTP Headers",
											})}
										</h3>
										<p className="text-sm text-slate-500">
											{t("settings:developer.showDefaultHeadersDescription", {
												defaultValue:
													"Display the server's default HTTP headers (values are redacted) in Server Details. Use only for inspection.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.showDefaultHeaders}
										onCheckedChange={(checked) =>
											setDashboardSetting("showDefaultHeaders", checked)
										}
									/>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="market" className="mt-0 h-full">
						<MarketBlacklistCard
							entries={dashboardSettings.marketBlacklist}
							onRestore={removeFromMarketBlacklist}
							setDashboardSetting={setDashboardSetting}
							dashboardSettings={dashboardSettings}
							availablePortals={availablePortals}
						/>
					</TabsContent>
					{showLicenseTab && licenseDocument && (
						<TabsContent value="about" className="mt-0 h-full">
							<AboutLicensesSection document={licenseDocument} />
						</TabsContent>
					)}
				</div>
			</Tabs>
		</div>
	);
}

interface MarketBlacklistCardProps {
	entries: MarketBlacklistEntry[];
	onRestore: (serverId: string) => void;
	setDashboardSetting: <K extends keyof DashboardSettings>(
		key: K,
		value: DashboardSettings[K],
	) => void;
	dashboardSettings: DashboardSettings;
	availablePortals: Array<{ id: string; label: string }>;
}

function MarketBlacklistCard({
	entries,
	onRestore,
	setDashboardSetting,
	dashboardSettings,
	availablePortals,
}: MarketBlacklistCardProps) {
	const { t } = useTranslation();
	const searchId = useId();
	const sortId = useId();
	const enableBlacklistId = useId();

	const [searchTerm, setSearchTerm] = useState("");
	const [sortOrder, setSortOrder] = useState<"recent" | "name">("recent");

	const enableMarketBlacklist = useAppStore(
		(state) => state.dashboardSettings.enableMarketBlacklist,
	);

	const filteredEntries = useMemo(() => {
		const query = searchTerm.trim().toLowerCase();
		const list = query
			? entries.filter(
					(entry) =>
						entry.label.toLowerCase().includes(query) ||
						entry.serverId.toLowerCase().includes(query) ||
						(entry.description?.toLowerCase() ?? "").includes(query),
				)
			: entries;

		return [...list].sort((a, b) => {
			if (sortOrder === "name") {
				return a.label.localeCompare(b.label, undefined, {
					sensitivity: "base",
				});
			}
			return b.hiddenAt - a.hiddenAt;
		});
	}, [entries, searchTerm, sortOrder]);

	return (
		<Card className="h-full">
			<CardHeader>
				<CardTitle>
					{t("settings:market.title", { defaultValue: "MCP Market" })}
				</CardTitle>
				<CardDescription>
					{t("settings:market.description", {
						defaultValue:
							"Configure default market and manage hidden marketplace servers.",
					})}
				</CardDescription>
			</CardHeader>
			<CardContent className="flex h-full flex-col gap-4">
				{/* Default Market 设置项 */}
				<div className="flex items-center justify-between gap-4">
					<div className="space-y-0.5">
						<h3 className="text-base font-medium">
							{t("settings:market.defaultMarketTitle", {
								defaultValue: "Default Market",
							})}
						</h3>
						<p className="text-sm text-slate-500">
							{t("settings:market.defaultMarketDescription", {
								defaultValue:
									"Choose which market appears first and cannot be closed.",
							})}
						</p>
					</div>
					<Select
						value={dashboardSettings.defaultMarket}
						onValueChange={(value) =>
							setDashboardSetting("defaultMarket", value as DefaultMarket)
						}
					>
						<SelectTrigger className="w-48">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="official">
								{t("settings:market.officialPortal", {
									defaultValue: "Official MCP Registry",
								})}
							</SelectItem>
							{availablePortals.map((portal) => (
								<SelectItem key={portal.id} value={portal.id}>
									{portal.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>

				{/* Enable Blacklist 设置项 */}
				<div className="flex items-center justify-between gap-4">
					<div className="space-y-0.5">
						<h3 className="text-base font-medium">
							{t("settings:market.enableBlacklistTitle", {
								defaultValue: "Enable Blacklist",
							})}
						</h3>
						<p className="text-sm text-slate-500">
							{t("settings:market.enableBlacklistDescription", {
								defaultValue:
									"Hide quality-poor or unavailable content from the market to keep it clean",
							})}
						</p>
					</div>
					<Switch
						id={enableBlacklistId}
						checked={enableMarketBlacklist}
						onCheckedChange={(checked) =>
							setDashboardSetting("enableMarketBlacklist", checked)
						}
					/>
				</div>

				<div className="flex flex-col gap-3 md:flex-row md:items-center">
					<div className="flex w-full flex-col gap-2 md:flex-row md:items-center md:gap-3">
						<div className="grow">
							<Label htmlFor="market-blacklist-search" className="sr-only">
								{t("settings:market.searchHiddenServers", {
									defaultValue: "Search hidden servers",
								})}
							</Label>
							<Input
								id={searchId}
								placeholder={t("placeholders.searchHiddenServers", {
									defaultValue: "Search hidden servers...",
								})}
								value={searchTerm}
								onChange={(event) => setSearchTerm(event.target.value)}
							/>
						</div>
						<div className="w-full md:ml-auto md:w-52">
							<Label htmlFor="market-blacklist-sort" className="sr-only">
								{t("settings:market.sortHiddenServers", {
									defaultValue: "Sort hidden servers",
								})}
							</Label>
							<Select
								value={sortOrder}
								onValueChange={(value) =>
									setSortOrder(value as "recent" | "name")
								}
							>
								<SelectTrigger id={sortId}>
									<SelectValue
										placeholder={t("settings:market.sortPlaceholder", {
											defaultValue: "Sort",
										})}
									/>
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="recent">
										{t("sort.recent", {
											defaultValue: "Most Recently Hidden",
										})}
									</SelectItem>
									<SelectItem value="name">
										{t("sort.name", { defaultValue: "Name (A-Z)" })}
									</SelectItem>
								</SelectContent>
							</Select>
						</div>
					</div>
				</div>

				{filteredEntries.length === 0 ? (
					<div className="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-slate-200 p-8 text-center text-sm text-slate-500 dark:border-slate-800 dark:text-slate-400">
						<p>
							{t("settings:market.emptyTitle", {
								defaultValue: "No hidden servers currently.",
							})}
						</p>
						<p className="mt-1 text-xs text-slate-400">
							{t("settings:market.emptyDescription", {
								defaultValue:
									"Hide servers from the Market list to keep this space tidy. They will appear here for recovery.",
							})}
						</p>
					</div>
				) : (
					<div className="flex-1 space-y-4 overflow-y-auto pr-1">
						{filteredEntries.map((entry) => {
							const hiddenDate = new Date(entry.hiddenAt);
							const hiddenLabel = Number.isNaN(hiddenDate.getTime())
								? "Unknown"
								: hiddenDate.toLocaleString();
							return (
								<div
									key={entry.serverId}
									className="flex items-center justify-between gap-4 rounded-md border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900"
								>
									<div className="flex flex-col gap-1">
										<p className="text-sm font-semibold text-slate-900 dark:text-slate-100">
											{entry.label}
										</p>
										<p className="text-xs text-slate-500">
											{entry.description?.trim() ||
												t("settings:market.noNotes", {
													defaultValue: "No notes added.",
												})}
										</p>
										<p className="text-xs text-slate-400">
											{t("settings:market.hiddenOn", {
												defaultValue: "Hidden on {{value}}",
												value: hiddenLabel,
											})}
										</p>
									</div>
									<Button
										variant="outline"
										size="sm"
										onClick={() => onRestore(entry.serverId)}
										className="flex items-center gap-2"
									>
										<RotateCcw className="h-4 w-4" />
										<span>
											{t("settings:market.restore", {
												defaultValue: "Restore",
											})}
										</span>
									</Button>
								</div>
							);
						})}
					</div>
				)}
			</CardContent>
		</Card>
	);
}
