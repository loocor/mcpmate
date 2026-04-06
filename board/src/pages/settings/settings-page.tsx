import { isProfileTokenEstimateMethod } from "../../lib/profile-token-estimate-method";
import type { AuditRetentionPolicy } from "../../lib/types";
import { useQueryClient, useQuery, useMutation } from "@tanstack/react-query";
import {
	Activity,
	AppWindow,
	BookText,
	Download,
	ExternalLink,
	FileSearch,
	Bug,
	LayoutGrid,
	Moon,
	Palette,
	RotateCcw,
	Server,
	Sliders,
	Store,
	Sun,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
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
import {
	auditApi,
	API_BASE_URL,
	notificationsService,
	setApiBaseUrl,
	systemApi,
} from "../../lib/api";
import {
	type DesktopCoreSourceResponse,
	useDesktopCoreState,
} from "../../lib/desktop-core-state";
import {
	notifyError,
	notifySuccess,
	stringifyError,
} from "../../lib/notify";
import { SUPPORTED_LANGUAGES } from "../../lib/i18n/index";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import {
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
	type MarketBlacklistEntry,
	type MenuBarIconMode,
	useAppStore,
} from "../../lib/store";
import type { OpenSourceDocument } from "../../types/open-source";
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
		value: "unify" as const,
		labelKey: "settings:options.clientMode.unify",
		fallback: "Unify",
	},
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

const CHROME_EXTENSION_URL =
	"https://chromewebstore.google.com/detail/mcpmate-server-import/jngogcgclencgillbmeeimkcjjnobidf";
const EDGE_EXTENSION_URL =
	"https://microsoftedge.microsoft.com/addons/detail/mcpmate-server-import/nbpdfanhajcjghegoocfmjkpaklidckn";

interface ShellPreferencesResponse {
	menuBarIconMode: MenuBarIconMode;
	showDockIcon: boolean;
}

type LocalhostRuntimeMode = "service" | "desktop_managed";

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

function persistLocalPorts(api: number, mcp: number): void {
	try {
		window.localStorage?.setItem("mcpmate.system.api_port", String(api));
		window.localStorage?.setItem("mcpmate.system.mcp_port", String(mcp));
	} catch {
		// LocalStorage write is best-effort
	}
}

function readCachedLocalPort(raw: string | null | undefined): number | undefined {
	if (!raw) {
		return undefined;
	}

	const value = Number(raw);
	return !Number.isNaN(value) && value > 0 ? value : undefined;
}

function getServiceInstallLabel(params: {
	busyAction: string | null;
	installed: boolean;
	t: ReturnType<typeof useTranslation>["t"];
}): string {
	const { busyAction, installed, t } = params;
	if (busyAction === "install" || busyAction === "uninstall") {
		return t("settings:system.serviceActionBusy", {
			defaultValue: "Working…",
		});
	}

	if (installed) {
		return t("settings:system.uninstallAction", {
			defaultValue: "Uninstall",
		});
	}

	return t("settings:system.installAction", {
		defaultValue: "Install",
	});
}

function buildSettingsTabSearchParams(
	searchParams: URLSearchParams,
	value: string,
): URLSearchParams {
	const next = new URLSearchParams(searchParams);
	if (value === "about") {
		next.set("tab", "about");
	} else {
		next.delete("tab");
	}
	return next;
}

export function SettingsPage() {
	usePageTranslations("settings");
	const queryClient = useQueryClient();
	const languageId = useId();
	const backupLimitId = useId();
	const menuBarSelectId = useId();
	const { t, i18n } = useTranslation();
	const {
		isTauriShell,
		coreView,
		busyAction,
		refreshCoreView,
		manageLocalCore,
	} = useDesktopCoreState();
	const [searchParams, setSearchParams] = useSearchParams();

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
	const [coreSource, setCoreSource] = useState<"localhost" | "remote">(
		"localhost",
	);
	const [localhostRuntimeMode, setLocalhostRuntimeMode] =
		useState<LocalhostRuntimeMode>("service");
	const [remoteBaseUrl, setRemoteBaseUrl] = useState("");
	const [localService, setLocalService] = useState<
		DesktopCoreSourceResponse["localService"]
	>({
		status: "not_installed",
		label: "Not Installed",
		detail: "",
		level: "user",
		installed: false,
		running: false,
	});
	const [serviceStatusExpanded, setServiceStatusExpanded] = useState(false);

	// Developer → Backend ports (API/MCP)
	const [apiPort, setApiPort] = useState<number | "">("");
	const [mcpPort, setMcpPort] = useState<number | "">("");
	const [loadingPorts, setLoadingPorts] = useState(false);
	const [applyBusy, setApplyBusy] = useState(false);
	const sourceOptions = useMemo<SegmentOption[]>(
		() => [
			{
				value: "localhost",
				label: t("settings:system.sourceOptions.localhost", {
					defaultValue: "Localhost",
				}),
			},
			{
				value: "remote",
				label: t("settings:system.sourceOptions.remote", {
					defaultValue: "Remote",
				}),
				status: t("wipTag", { defaultValue: "(WIP)" }),
			},
		],
		[t, i18n.language],
	);
	const [webDialogOpen, setWebDialogOpen] = useState(false);
	const [policyType, setPolicyType] = useState<string>("combined");
	const [policyDays, setPolicyDays] = useState<number>(30);
	const [policyCount, setPolicyCount] = useState<number>(100000);
	const [sweepInterval, setSweepInterval] = useState<number>(3600);

	const policyQuery = useQuery({
		queryKey: ["audit", "policy"],
		queryFn: () => auditApi.getPolicy(),
	});

	const defaultClientModeQuery = useQuery({
		queryKey: ["system", "default-client-mode"],
		queryFn: () => systemApi.getDefaultClientMode(),
	});

	useEffect(() => {
		if (policyQuery.data) {
			const p = policyQuery.data.policy;
			if (p === "off") {
				setPolicyType("off");
			} else if (typeof p === "object" && "keep_days" in p) {
				setPolicyType("keep_days");
				setPolicyDays(p.keep_days.days);
			} else if (typeof p === "object" && "keep_count" in p) {
				setPolicyType("keep_count");
				setPolicyCount(p.keep_count.count);
			} else if (typeof p === "object" && "combined" in p) {
				setPolicyType("combined");
				setPolicyDays(p.combined.days);
				setPolicyCount(p.combined.count);
			}
			setSweepInterval(policyQuery.data.sweep_interval_secs);
		}
	}, [policyQuery.data]);

	useEffect(() => {
		if (defaultClientModeQuery.data?.default_config_mode) {
			setDashboardSetting(
				"clientDefaultMode",
				defaultClientModeQuery.data.default_config_mode,
			);
		}
	}, [defaultClientModeQuery.data, setDashboardSetting]);

	const policyMutation = useMutation({
		mutationFn: (data: { policy: AuditRetentionPolicy; sweep_interval_secs: number }) =>
			auditApi.setPolicy(data),
		onSuccess: () => {
			notifySuccess(t("settings:audit.saved", { defaultValue: "Retention policy saved" }));
			policyQuery.refetch();
		},
		onError: (e) => {
			notifyError(t("settings:audit.saveFailed", { defaultValue: "Failed to save policy" }), String(e));
		},
	});

	const defaultClientModeMutation = useMutation({
		mutationFn: (mode: ClientDefaultMode) => systemApi.setDefaultClientMode(mode),
		onSuccess: (data) => {
			setDashboardSetting("clientDefaultMode", data.default_config_mode);
			notifySuccess(
				t("settings:clients.modeTitle", {
					defaultValue: "Client Management Mode",
				}),
				t("settings:system.applySuccessDescription", {
					source: data.default_config_mode,
					apiPort: effectiveApiPort,
					mcpPort: effectiveMcpPort,
					defaultValue: "Default mode updated to {{source}}.",
				}),
			);
			void defaultClientModeQuery.refetch();
		},
		onError: (error) => {
			notifyError(
				t("settings:clients.modeTitle", {
					defaultValue: "Client Management Mode",
				}),
				stringifyError(error),
			);
		},
	});

	const handleSavePolicy = useCallback(() => {
		let policy: AuditRetentionPolicy;
		switch (policyType) {
			case "off":
				policy = "off";
				break;
			case "keep_days":
				policy = { keep_days: { days: policyDays } };
				break;
			case "keep_count":
				policy = { keep_count: { count: policyCount } };
				break;
			case "combined":
			default:
				policy = { combined: { days: policyDays, count: policyCount } };
		}
		policyMutation.mutate({ policy, sweep_interval_secs: sweepInterval });
	}, [policyType, policyDays, policyCount, sweepInterval, policyMutation]);

	const applyCoreSourceView = useCallback(
		(response: DesktopCoreSourceResponse) => {
			setCoreSource(response.selectedSource);
			setLocalhostRuntimeMode(response.localhostRuntimeMode);
			setRemoteBaseUrl(response.remoteBaseUrl || "");
			setLocalService(response.localService);
			setApiPort(response.localhostApiPort);
			setMcpPort(response.localhostMcpPort);
			persistLocalPorts(response.localhostApiPort, response.localhostMcpPort);
		},
		[],
	);

	const seedPortsFromLocalStorage = useCallback(() => {
		try {
			const cachedApi = window.localStorage?.getItem("mcpmate.system.api_port");
			const cachedMcp = window.localStorage?.getItem("mcpmate.system.mcp_port");
			const nextApiPort = readCachedLocalPort(cachedApi);
			const nextMcpPort = readCachedLocalPort(cachedMcp);

			if (nextApiPort !== undefined) {
				setApiPort(nextApiPort);
				if (isTauriShell) {
					setApiBaseUrl(`http://127.0.0.1:${nextApiPort}`);
				}
			}

			if (nextMcpPort !== undefined) {
				setMcpPort(nextMcpPort);
			}
		} catch {
			// Cache read is best-effort
		}
	}, [isTauriShell]);

	const wireDashboardToCoreSource = useCallback(
		async (
			apiBaseUrl: string,
			api?: number,
			mcp?: number,
		) => {
			setApiBaseUrl(apiBaseUrl);
			notificationsService.reconnectAfterApiBaseChanged();
			await queryClient.invalidateQueries({ predicate: () => true });
			if (typeof api === "number" && typeof mcp === "number") {
				persistLocalPorts(api, mcp);
			}
		},
		[queryClient],
	);

	const devUrl =
		typeof window !== "undefined" && window.location?.origin?.includes(":")
			? window.location.origin
			: "http://localhost:5173";

	const effectiveApiPort = typeof apiPort === "number" ? apiPort : 8080;
	const effectiveMcpPort = typeof mcpPort === "number" ? mcpPort : 8000;

	const loadRuntimePorts = useCallback(async () => {
		setLoadingPorts(true);
		try {
			const applyAuthorityPorts = (api: number, mcp: number) => {
				setApiPort(api);
				setMcpPort(mcp);
				persistLocalPorts(api, mcp);
				setCoreSource("localhost");
				setLocalhostRuntimeMode("service");
				setRemoteBaseUrl("");
				setLocalService((current) => ({
					...current,
					status: "not_installed",
					label: "Not Installed",
					running: false,
					installed: false,
				}));
				if (isTauriShell) {
					setApiBaseUrl(`http://127.0.0.1:${api}`);
				} else {
					setApiBaseUrl("");
				}
			};

			if (isTauriEnvironmentSync()) {
				try {
					const resp = await refreshCoreView();
					if (
						resp &&
						typeof resp.localhostApiPort === "number" &&
						typeof resp.localhostMcpPort === "number"
					) {
						applyCoreSourceView(resp);
						setApiBaseUrl(resp.apiBaseUrl || `http://127.0.0.1:${resp.localhostApiPort}`);
					}
				} catch {
					seedPortsFromLocalStorage();
					notifyError(
						t("settings:system.portsReloadFailedTitle", {
							defaultValue: "Could not load ports from shell",
						}),
						t("settings:system.portsReloadFailedDescription", {
							defaultValue:
								"Showing cached values if any. Check the desktop app is healthy and try Reload again.",
						}),
					);
				}
				return;
			}

			const apiBase = API_BASE_URL || "";
			const url = apiBase ? `${apiBase}/api/system/ports` : `/api/system/ports`;
			const response = await fetch(url, { cache: "no-store" });
			if (response.ok) {
				const data = (await response.json()) as unknown;
				let raw: unknown = data;
				if (
					data &&
					typeof data === "object" &&
					"data" in data &&
					(data as { data?: unknown }).data !== undefined
				) {
					raw = (data as { data: unknown }).data;
				}
				const d = raw as { api_port?: unknown; mcp_port?: unknown };
				if (
					typeof d?.api_port === "number" &&
					typeof d?.mcp_port === "number"
				) {
					applyAuthorityPorts(d.api_port, d.mcp_port);
					return;
				}
			}

			seedPortsFromLocalStorage();
		} finally {
			setLoadingPorts(false);
		}
		// API_BASE_URL is a live module binding; reads inside this async fn stay current without listing it in deps.
	}, [applyCoreSourceView, refreshCoreView, seedPortsFromLocalStorage, isTauriShell, t, i18n.language]);

	const runtimeModeOptions = useMemo<SegmentOption[]>(
		() => [
			{
				value: "service",
				label: t("settings:system.runtimeModeOptions.service", {
					defaultValue: "Service",
				}),
			},
			{
				value: "desktop_managed",
				label: t("settings:system.runtimeModeOptions.desktopManaged", {
					defaultValue: "Desktop",
				}),
			},
		],
		[t, i18n.language],
	);

	const currentRuntimeModeLabel = useMemo(
		() =>
			localhostRuntimeMode === "service"
				? t("settings:system.runtimeModeOptions.service", {
					defaultValue: "Service",
				})
				: t("settings:system.runtimeModeOptions.desktopManaged", {
					defaultValue: "Desktop",
				}),
		[localhostRuntimeMode, t, i18n.language],
	);

	const applyDisabled =
		applyBusy ||
		typeof apiPort !== "number" ||
		typeof mcpPort !== "number" ||
		apiPort <= 0 ||
		mcpPort <= 0 ||
		apiPort === mcpPort ||
		(coreSource === "remote" && !remoteBaseUrl.trim());

	const serviceInstallAction = localService.installed ? "uninstall" : "install";
	const serviceInstallIcon = localService.installed ? (
		<Trash2 className="mr-2 h-4 w-4" />
	) : (
		<Download className="mr-2 h-4 w-4" />
	);
	const serviceInstallLabel = getServiceInstallLabel({
		busyAction,
		installed: localService.installed,
		t,
	});

	const handleApplyCoreSource = useCallback(async () => {
		if (typeof apiPort !== "number" || typeof mcpPort !== "number") {
			return;
		}

		if (!isTauriShell) {
			void wireDashboardToCoreSource(
				"",
				apiPort,
				mcpPort,
			);
			setWebDialogOpen(true);
			return;
		}

		try {
			setApplyBusy(true);
			const { invoke } = await import("@tauri-apps/api/core");
			const response = (await invoke("mcp_shell_apply_core_source", {
				payload: {
					selectedSource: coreSource,
					localhostRuntimeMode,
					localhostApiPort: apiPort,
					localhostMcpPort: mcpPort,
					remoteBaseUrl,
				},
			})) as DesktopCoreSourceResponse;

			applyCoreSourceView(response);
			await wireDashboardToCoreSource(
				response.apiBaseUrl,
				response.localhostApiPort,
				response.localhostMcpPort,
			);
			notifySuccess(
				t("settings:system.applySuccessTitle", {
					defaultValue: "Core source updated",
				}),
				t("settings:system.applySuccessDescription", {
					source: response.selectedSource,
					apiPort: response.localhostApiPort,
					mcpPort: response.localhostMcpPort,
					defaultValue:
						"Desktop is now attached to the selected {{source}} core. Existing service definitions were refreshed if needed.",
				}),
			);
		} catch (error) {
			notifyError(
				t("settings:system.applyFailedTitle", {
					defaultValue: "Could not update core source",
				}),
				stringifyError(error),
			);
		} finally {
			setApplyBusy(false);
		}
	}, [
		apiPort,
		applyCoreSourceView,
		coreSource,
		isTauriShell,
		localhostRuntimeMode,
		mcpPort,
		remoteBaseUrl,
		t,
		wireDashboardToCoreSource,
	]);

	const tabTriggerClass =
		"w-full justify-center gap-2 px-2 py-2 text-left text-sm font-medium text-slate-600 data-[state=active]:text-emerald-700 md:justify-start md:px-3 dark:text-slate-300";
	const settingItemTitleClass = "text-base font-medium";
	const settingItemDescriptionClass = "text-sm text-muted-foreground";

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
		if (!coreView) {
			return;
		}
		applyCoreSourceView(coreView);
	}, [applyCoreSourceView, coreView]);

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
						menuBarIconMode: prefs.menuBarIconMode,
						showDockIcon: prefs.showDockIcon,
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
	const requestedTab = searchParams.get("tab");
	const [activeTab, setActiveTab] = useState("general");

	useEffect(() => {
		if (requestedTab === "about" && showLicenseTab) {
			setActiveTab("about");
			return;
		}
		if (activeTab === "about" && !showLicenseTab) {
			setActiveTab("general");
		}
	}, [activeTab, requestedTab, showLicenseTab]);

	const handleTabChange = useCallback(
		(value: string) => {
			setActiveTab(value);
			const next = buildSettingsTabSearchParams(searchParams, value);
			setSearchParams(next, { replace: true });
		},
		[searchParams, setSearchParams],
	);

	return (
		<div className="space-y-4">
			<div className="flex items-center gap-2 min-w-0">
				<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
					{t("settings:title", { defaultValue: "Settings" })}
				</p>
			</div>

			<Tabs
				value={activeTab}
				onValueChange={handleTabChange}
				orientation="vertical"
				className="flex items-start gap-4"
			>
				<TabsList className="sticky top-4 flex w-14 shrink-0 flex-col gap-1 self-start rounded-lg p-1 md:w-56 md:p-2">
					<TabsTrigger value="general" className={tabTriggerClass}>
						<LayoutGrid className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.general", { defaultValue: "General" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="appearance" className={tabTriggerClass}>
						<Palette className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.appearance", { defaultValue: "Appearance" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="servers" className={tabTriggerClass}>
						<Server className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.serverControls", {
								defaultValue: "Server",
							})}
						</span>
					</TabsTrigger>
					<TabsTrigger value="clients" className={tabTriggerClass}>
						<AppWindow className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.clientDefaults", {
								defaultValue: "Client",
							})}
						</span>
					</TabsTrigger>
					<TabsTrigger value="profile" className={tabTriggerClass}>
						<Sliders className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.profile", { defaultValue: "Profile" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="market" className={tabTriggerClass}>
						<Store className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.market", { defaultValue: "Market" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="develop" className={tabTriggerClass}>
						<Bug className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.developer", { defaultValue: "Developer" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="audit" className={tabTriggerClass}>
						<FileSearch className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.audit", { defaultValue: "Audit" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="system" className={tabTriggerClass}>
						<Activity className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.system", { defaultValue: "System" })}
						</span>
					</TabsTrigger>
					{showLicenseTab && (
						<TabsTrigger value="about" className={tabTriggerClass}>
							<BookText className="h-4 w-4 shrink-0" />
							<span className="hidden md:inline truncate">
								{t("settings:tabs.about", { defaultValue: "About" })}
							</span>
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
							<CardContent className="space-y-5">
								{/* Default View */}
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:general.defaultView", {
												defaultValue: "Default View",
											})}
										</h3>
										<p className="text-xs text-muted-foreground">
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
										<p className="text-sm text-muted-foreground">
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
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:general.languageDescription", {
												defaultValue: "Select the dashboard language.",
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
							<CardContent className="space-y-5">
								<div className="space-y-4">
									<div className="flex items-center justify-between gap-4">
										<div className="space-y-0.5">
											<h3 className="text-base font-medium">
												{t("settings:appearance.themeTitle", {
													defaultValue: "Theme",
												})}
											</h3>
											<p className="text-xs text-muted-foreground">
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
											<p className="text-xs text-muted-foreground">
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
										<div className="space-y-4">
											<div className="flex items-center justify-between gap-4">
												<div className="space-y-0.5">
													<h3 className="text-base font-medium">
														{t("settings:appearance.menuBarTitle", {
															defaultValue: "Menu Bar Icon",
														})}
													</h3>
													<p className="text-sm text-muted-foreground">
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
															defaultValue: "Dock / Taskbar Icon",
														})}
													</h3>
													<p className="text-sm text-muted-foreground">
														{t("settings:appearance.dockDescription", {
															defaultValue:
																"Show MCPMate in the Dock (macOS), taskbar (Windows/Linux), or run from the tray or menu bar only.",
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
												<p className="text-sm leading-relaxed text-muted-foreground">
													{t("settings:appearance.dockHiddenNotice", {
														defaultValue:
															"The Dock or taskbar entry is hidden. The tray icon stays visible so you can reopen MCPMate.",
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
										defaultValue: "Server",
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
										<p className="text-sm text-muted-foreground">
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
										<p className="text-sm text-muted-foreground">
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
								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:servers.liveLogsTitle", {
												defaultValue: "Server Detail Logs",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:servers.liveLogsDescription", {
												defaultValue:
													"Show paginated live logs on the Server detail page.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.showServerLiveLogs}
										onCheckedChange={(checked) =>
											setDashboardSetting("showServerLiveLogs", checked)
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
										defaultValue: "Client Controls",
									})}
								</CardTitle>
								<CardDescription>
									{t("settings:clients.description", {
										defaultValue:
											"Configure default rollout and backup behavior for client apps.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.modeTitle", {
												defaultValue: "Client Application Mode",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
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
											defaultClientModeMutation.mutate(value as ClientDefaultMode)
										}
										disabled={defaultClientModeMutation.isPending}
										showDots={false}
									/>
									</div>
								</div>

								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.defaultVisibilityTitle", {
												defaultValue: "Default Client Visibility",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
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

								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.backupStrategyTitle", {
												defaultValue: "Client Backup Strategy",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
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

								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.backupLimitTitle", {
												defaultValue: "Maximum Backup Copies",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
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
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:clients.liveLogsTitle", {
												defaultValue: "Client Detail Logs",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:clients.liveLogsDescription", {
												defaultValue:
													"Show paginated live logs on the Client detail page.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.showClientLiveLogs}
										onCheckedChange={(checked) =>
											setDashboardSetting("showClientLiveLogs", checked)
										}
									/>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="profile" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:profile.title", { defaultValue: "Profile Controls" })}
								</CardTitle>
								<CardDescription>
									{t("settings:profile.description", {
										defaultValue:
											"Token estimates, profile detail logs, and related options.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
									<div className="space-y-1.5">
										<h3 className="text-base font-medium">
											{t("settings:profile.tokenEstimateTitle", {
												defaultValue: "Profile token estimate",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:profile.tokenEstimateDescription", {
												defaultValue:
													"Tokenizer used for profile capability size on the chart and dashboard.",
											})}
										</p>
									</div>
									<div className="flex sm:justify-end">
										<Select
											value={dashboardSettings.profileTokenEstimateMethod}
											onValueChange={(value) => {
												if (!isProfileTokenEstimateMethod(value)) return;
												setDashboardSetting("profileTokenEstimateMethod", value);
											}}
										>
											<SelectTrigger className="w-full sm:w-72">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="openai_cl100k">
													{t("settings:profile.tokenEstimateOpenAI", {
														defaultValue: "OpenAI (cl100k_base)",
													})}
												</SelectItem>
												<SelectItem value="anthropic_claude">
													{t("settings:profile.tokenEstimateAnthropic", {
														defaultValue: "Anthropic Claude",
													})}
												</SelectItem>
											</SelectContent>
										</Select>
									</div>
								</div>
								<div className="flex items-center justify-between gap-4">
									<div className="space-y-0.5">
										<h3 className="text-base font-medium">
											{t("settings:profile.liveLogsTitle", {
												defaultValue: "Profile Detail Logs",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:profile.liveLogsDescription", {
												defaultValue:
													"Show paginated live logs on the Profile detail page.",
											})}
										</p>
									</div>
									<Switch
										checked={dashboardSettings.showProfileLiveLogs}
										onCheckedChange={(checked) =>
											setDashboardSetting("showProfileLiveLogs", checked)
										}
									/>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					{/* System tab: Backend ports configuration */}
					<TabsContent value="audit" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:audit.title", { defaultValue: "Audit Policy" })}
								</CardTitle>
								<CardDescription>
									{t("settings:audit.description", {
										defaultValue:
											"Manage how long audit events are retained in the database.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
									<div className="space-y-1.5">
										<h3 className={settingItemTitleClass}>
											{t("settings:audit.typeTitle", {
												defaultValue: "Retention Strategy",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:audit.typeDescription", { defaultValue: "Select how events are automatically pruned." })}
										</p>
									</div>
									<div className="flex sm:justify-end">
										<Select value={policyType} onValueChange={setPolicyType}>
											<SelectTrigger className="w-full sm:w-64">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="combined">{t("settings:audit.typeCombined", { defaultValue: "Combined (days + count)" })}</SelectItem>
												<SelectItem value="keep_days">{t("settings:audit.typeDays", { defaultValue: "Keep by days" })}</SelectItem>
												<SelectItem value="keep_count">{t("settings:audit.typeCount", { defaultValue: "Keep by count" })}</SelectItem>
												<SelectItem value="off">{t("settings:audit.typeOff", { defaultValue: "Disabled (keep all)" })}</SelectItem>
											</SelectContent>
										</Select>
									</div>
								</div>

								{(policyType === "keep_days" || policyType === "combined") && (
									<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
										<div className="space-y-1.5">
											<h3 className={settingItemTitleClass}>
												{t("settings:audit.daysTitle", {
													defaultValue: "Days to keep",
												})}
											</h3>
											<p className={settingItemDescriptionClass}>
												{t("settings:audit.daysDescription", { defaultValue: "Events older than this number of days will be deleted." })}
											</p>
										</div>
										<div className="flex sm:justify-end">
											<Input
												type="number"
												min={1}
												value={policyDays}
												onChange={(e) => setPolicyDays(Number(e.target.value))}
												className="w-full sm:w-64"
											/>
										</div>
									</div>
								)}

								{(policyType === "keep_count" || policyType === "combined") && (
									<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
										<div className="space-y-1.5">
											<h3 className={settingItemTitleClass}>
												{t("settings:audit.countTitle", {
													defaultValue: "Max events",
												})}
											</h3>
											<p className={settingItemDescriptionClass}>
												{t("settings:audit.countDescription", { defaultValue: "If event count exceeds this limit, oldest events will be deleted." })}
											</p>
										</div>
										<div className="flex sm:justify-end">
											<Input
												type="number"
												min={1}
												value={policyCount}
												onChange={(e) => setPolicyCount(Number(e.target.value))}
												className="w-full sm:w-64"
											/>
										</div>
									</div>
								)}

								<div className="mt-4 flex justify-end gap-2">
									<Button
										variant="default"
										disabled={policyMutation.isPending}
										onClick={handleSavePolicy}
									>
										{policyMutation.isPending
											? t("settings:audit.saving", { defaultValue: "Saving..." })
											: t("settings:audit.save", { defaultValue: "Save Policy" })}
									</Button>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="system" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:system.title", { defaultValue: "System" })}
								</CardTitle>
								<CardDescription>
									{t("settings:system.description", {
										defaultValue:
											"Manage how MCPMate Desktop connects to and controls its service, including runtime mode and local ports.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
									<div className="space-y-1.5">
										<h3 className={settingItemTitleClass}>
											{t("settings:system.sourceTitle", {
												defaultValue: "Service Target",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:system.sourceDescription", {
												defaultValue:
													"Choose whether Desktop should attach to the built-in local service or a remote service endpoint.",
											})}
										</p>
									</div>
									<div className="flex sm:justify-end">
										<div className="w-56">
											<Segment
												value={coreSource}
												onValueChange={(value) => {
													if (value === "localhost") {
														setCoreSource("localhost");
													}
												}}
												options={sourceOptions}
												showDots={false}
											/>
										</div>
									</div>
								</div>

								{coreSource === "remote" ? (
									<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
										<div className="space-y-1.5">
											<h3 className={settingItemTitleClass}>
												{t("settings:system.remoteUrlTitle", {
													defaultValue: "Remote Core URL",
												})}
											</h3>
											<p className={settingItemDescriptionClass}>
												{t("settings:system.remoteUrlDescription", {
													defaultValue:
														"Store the remote core endpoint for future attach support. This phase still prioritizes localhost service management.",
												})}
											</p>
										</div>
										<div className="flex sm:justify-end">
											<Input
												id="remote-core-url"
												type="url"
												value={remoteBaseUrl}
												onChange={(event) => setRemoteBaseUrl(event.target.value)}
												placeholder={t("settings:system.remoteUrlPlaceholder", {
													defaultValue: "https://your-core.example.com",
												})}
												className="w-full sm:w-80"
											/>
										</div>
									</div>
								) : null}

								{coreSource === "localhost" ? (
									<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
										<div className="space-y-1.5">
											<h3 className={settingItemTitleClass}>
												{t("settings:system.runtimeModeTitle", {
													defaultValue: "Local Runtime Mode",
												})}
											</h3>
											<p className={settingItemDescriptionClass}>
												{t("settings:system.runtimeModeDescription", {
													defaultValue:
														"Choose whether localhost core is managed as an OS service or tied to the MCPMate desktop lifecycle.",
												})}
												{isTauriShell ? (
													<>
														{" "}
														<button
															type="button"
															onClick={() => setServiceStatusExpanded((prev) => !prev)}
															className="inline-flex whitespace-nowrap text-xs font-semibold text-sky-600 hover:text-sky-500 dark:text-sky-400 dark:hover:text-sky-300"
														>
															{serviceStatusExpanded
																? t("settings:system.lessToggle", { defaultValue: "Less" })
																: t("settings:system.moreToggle", { defaultValue: "More" })}
														</button>
													</>
												) : null}
											</p>
										</div>
										<div className="flex sm:justify-end">
											<div className="w-56">
												<Segment
													value={localhostRuntimeMode}
													onValueChange={(value) =>
														setLocalhostRuntimeMode(
															value as "service" | "desktop_managed",
														)
													}
													options={runtimeModeOptions}
													showDots={false}
												/>
											</div>
										</div>
									</div>
								) : null}

								{isTauriShell && coreSource === "localhost" && serviceStatusExpanded ? (
									<div className="rounded-lg border border-slate-200 bg-slate-50 p-3 dark:border-slate-800 dark:bg-slate-900/60">
										<div className="flex items-start justify-between gap-4">
											<div className="space-y-0.5">
												<p className="text-sm font-medium text-slate-900 dark:text-slate-100">
													{t("settings:system.serviceStatusTitle", {
														defaultValue: "Local Service Status",
													})}
												</p>
												<p className="text-sm text-slate-600 dark:text-slate-300">
													{localService.label}
												</p>
												<p className="text-xs text-muted-foreground">
													{t("settings:system.runtimeModeCurrent", {
														defaultValue: "Current runtime mode: {{value}}",
														value: currentRuntimeModeLabel,
													})}{" "}
													{" · "}
													{t("settings:system.serviceLevel", {
														defaultValue: "Service level: {{value}}",
														value: localService.level,
													})}
												</p>
												<p className="text-xs leading-5 text-muted-foreground">
													{localService.detail ||
														t("settings:system.serviceStatusFallback", {
															defaultValue:
																"The desktop will attach to the configured localhost core service when it is available.",
														})}
												</p>
											</div>
											{localhostRuntimeMode === "service" ? (
												<div className="flex shrink-0 items-center gap-2">
													<Button
														variant={localService.installed ? "destructive" : "secondary"}
														disabled={busyAction !== null}
														onClick={() =>
															void manageLocalCore(serviceInstallAction)
														}
													>
														{serviceInstallIcon}
														{serviceInstallLabel}
													</Button>
												</div>
											) : null}
										</div>
									</div>
								) : null}

								{/* Row: API Port */}
								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
									<div className="space-y-1.5">
										<h3 className={settingItemTitleClass}>
											{t("settings:system.apiPortTitle", {
												defaultValue: "Localhost Core API Port",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:system.apiPortDescription", {
												defaultValue:
													"Port for localhost REST and dashboard access (default 8080).",
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
										<h3 className={settingItemTitleClass}>
											{t("settings:system.mcpPortTitle", {
												defaultValue: "Localhost Core MCP Port",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:system.mcpPortDescription", {
												defaultValue:
													"Port for the localhost MCP proxy endpoint (/mcp). Default 8000.",
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
								<div className="mt-1 space-y-2">
									<div className="flex items-center justify-end gap-4">
										<div className="flex shrink-0 flex-wrap justify-end gap-2">
											<Button
												variant="secondary"
												onClick={() => loadRuntimePorts()}
												disabled={loadingPorts || applyBusy}
											>
												{loadingPorts
													? t("loading", { defaultValue: "Loading…" })
													: t("reload", { defaultValue: "Reload" })}
											</Button>
											<Button
												variant="default"
												disabled={applyDisabled}
												onClick={() => {
													void handleApplyCoreSource();
												}}
											>
												{applyBusy
													? t("settings:system.applyButtonBusy", {
														defaultValue: "Applying…",
													})
													: t("settings:system.apply", {
														defaultValue: "Apply Core Source",
													})}
											</Button>
										</div>
									</div>
									{applyBusy && isTauriShell ? (
										<p className="text-xs text-amber-700 dark:text-amber-400/90">
											{t("settings:system.applyProgressHint", {
												defaultValue:
													"Updating the selected core source. API requests may fail briefly while the desktop reconnects.",
											})}
										</p>
									) : null}
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
										{`MCPMATE_API_PORT=${effectiveApiPort} MCPMATE_MCP_PORT=${effectiveMcpPort} MCPMATE_ALLOWED_ORIGINS=${devUrl} cargo run -p app-mcpmate`}
									</div>
									<div className="mt-2 flex gap-2">
										<Button
											variant="secondary"
											onClick={() =>
												navigator.clipboard.writeText(
													`MCPMATE_API_PORT=${effectiveApiPort} MCPMATE_MCP_PORT=${effectiveMcpPort} MCPMATE_ALLOWED_ORIGINS=${devUrl} cargo run -p app-mcpmate`,
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
												} catch {
													// Shutdown request is fire-and-forget
												}
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
										{`./app-mcpmate --api-port ${effectiveApiPort} --mcp-port ${effectiveMcpPort}`}
									</div>
									<div className="mt-2">
										<Button
											variant="secondary"
											onClick={() =>
												navigator.clipboard.writeText(
													`./app-mcpmate --api-port ${effectiveApiPort} --mcp-port ${effectiveMcpPort}`,
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
										<p className="text-sm text-muted-foreground">
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
										<p className="text-sm text-muted-foreground">
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
										<p className="text-sm text-muted-foreground">
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
										<p className="text-sm text-muted-foreground">
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
										<p className="text-sm text-muted-foreground">
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
}

function MarketBlacklistCard({
	entries,
	onRestore,
	setDashboardSetting,
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
					{t("settings:market.title", { defaultValue: "Market" })}
				</CardTitle>
				<CardDescription>
					{t("settings:market.description", {
						defaultValue:
							"Manage hidden entries for the Official MCP Registry market.",
					})}
				</CardDescription>
			</CardHeader>
			<CardContent className="flex h-full flex-col gap-5">
				{/* Enable Blacklist settings */}
				<div className="flex items-center justify-between gap-4">
					<div className="space-y-0.5">
						<h3 className="text-base font-medium">
							{t("settings:market.enableBlacklistTitle", {
								defaultValue: "Enable Blacklist",
							})}
						</h3>
						<p className="text-sm text-muted-foreground">
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
							<Label htmlFor={searchId} className="sr-only">
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
							<Label htmlFor={sortId} className="sr-only">
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
					<div className="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-slate-200 p-8 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
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
									className="flex items-center justify-between gap-4 rounded-md border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-700 dark:bg-slate-900"
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

				<div className="space-y-4 border-t border-slate-200 pt-4 dark:border-slate-700">
					<div className="flex items-center justify-between gap-4">
						<div className="space-y-0.5">
							<h3 className="text-base font-medium">
								{t("settings:market.installChromeExtension", {
									defaultValue: "Install Chrome Extension",
								})}
							</h3>
							<p className="text-sm text-muted-foreground">
								{t("settings:market.installChromeExtensionDescription", {
									defaultValue:
										"Add the MCPMate Chrome extension to detect importable MCP server snippets and one-click import them into MCPMate.",
								})}
							</p>
						</div>
						<Button asChild variant="outline" size="sm" className="w-52 shrink-0">
							<a
								href={CHROME_EXTENSION_URL}
								target="_blank"
								rel="noopener noreferrer"
								className="inline-flex w-full items-center justify-center"
							>
								{t("settings:market.installChromeExtension", {
									defaultValue: "Install Chrome Extension",
								})}
								<ExternalLink className="ml-1 h-3.5 w-3.5 shrink-0" />
							</a>
						</Button>
					</div>

					<div className="flex items-center justify-between gap-4">
						<div className="space-y-0.5">
							<h3 className="text-base font-medium">
								{t("settings:market.installEdgeExtension", {
									defaultValue: "Install Edge Extension",
								})}
							</h3>
							<p className="text-sm text-muted-foreground">
								{t("settings:market.installEdgeExtensionDescription", {
									defaultValue:
										"Add the MCPMate Edge extension to discover importable MCP server configurations on web pages and import with one click.",
								})}
							</p>
						</div>
						<Button asChild variant="outline" size="sm" className="w-52 shrink-0">
							<a
								href={EDGE_EXTENSION_URL}
								target="_blank"
								rel="noopener noreferrer"
								className="inline-flex w-full items-center justify-center"
							>
								{t("settings:market.installEdgeExtension", {
									defaultValue: "Install Edge Extension",
								})}
								<ExternalLink className="ml-1 h-3.5 w-3.5 shrink-0" />
							</a>
						</Button>
					</div>

					<p className="text-xs text-muted-foreground">
						{t("settings:market.browserExtensionsAvailableHint", {
							defaultValue:
								"Browser extensions are now available on Chrome Web Store and Microsoft Edge Add-ons.",
						})}
					</p>
				</div>
			</CardContent>
		</Card>
	);
}
