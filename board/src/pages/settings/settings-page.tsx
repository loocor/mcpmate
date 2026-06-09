import { isProfileTokenEstimateMethod } from "../../lib/profile-token-estimate-method";
import type { AuditPolicyData, AuditRetentionPolicy } from "../../lib/types";
import { useQueryClient, useQuery, useMutation } from "@tanstack/react-query";
import {
	Activity,
	AppWindow,
	BookText,
	Download,
	ExternalLink,
	FileSearch,
	Bug,
	Grid3X3,
	LayoutGrid,
	List,
	Monitor,
	Moon,
	RotateCcw,
	Server,
	ShieldCheck,
	Sliders,
	Store,
	Sun,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { Button } from "../../components/ui/button";
import { LockScreen } from "../../components/lock-screen";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Segment, type SegmentOption } from "../../components/ui/segment";
import {
	ProtectionPasswordDialog,
	type ProtectionPasswordDialogMode,
} from "../../components/protection-password-dialog";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../../components/ui/alert-dialog";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import {
	auditApi,
	notificationsService,
	secretsApi,
	setApiBaseUrl,
	syncApiBaseUrlForRuntimePort,
	systemApi,
} from "../../lib/api";
import {
	type DesktopCoreSourceResponse,
	useDesktopCoreState,
} from "../../lib/desktop-core-state";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import {
	type ProtectionLevel,
	protectionScopeForLevel,
	requiresSettingsPasswordGate,
	resolveProtectionLevel,
} from "../../lib/protection-password";
import { SUPPORTED_LANGUAGES } from "../../lib/i18n/index";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import {
	isTauriEnvironmentSync,
} from "../../lib/platform";
import {
	type ClientBackupStrategy,
	type ClientDefaultMode,
	type ClientListDefaultFilter,
	type DashboardDefaultView,
	type DashboardLanguage,
	type DashboardSettings,
	type MarketBlacklistEntry,
	type MenuBarIconMode,
	useAppStore,
} from "../../lib/store";
import type { CapabilitySource, Theme } from "../../lib/types";
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
		value: "system" as const,
		icon: Monitor,
		labelKey: "settings:options.theme.auto",
		fallback: "Auto",
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
		value: "allowed" as const,
		labelKey: "settings:clients.defaultVisibility.allowed",
		fallback: "Allowed",
	},
	{
		value: "pending" as const,
		labelKey: "settings:clients.defaultVisibility.pending",
		fallback: "Pending",
	},
	{
		value: "denied" as const,
		labelKey: "settings:clients.defaultVisibility.denied",
		fallback: "Denied",
	},
];

const DEFAULT_VIEW_CONFIG = [
	{
		value: "list" as const,
		icon: List,
		labelKey: "settings:options.defaultView.list",
		fallback: "List",
	},
	{
		value: "grid" as const,
		icon: Grid3X3,
		labelKey: "settings:options.defaultView.grid",
		fallback: "Grid",
	},
];

const CLIENT_MODE_CONFIG: ReadonlyArray<{
	value: "unify" | "hosted" | "transparent";
	labelKey: string;
	fallback: string;
	disabled?: boolean;
	tooltipKey?: string;
	tooltipFallback?: string;
}> = [
		{
			value: "unify",
			labelKey: "settings:options.clientMode.unify",
			fallback: "Unify",
		},
		{
			value: "hosted",
			labelKey: "settings:options.clientMode.hosted",
			fallback: "Hosted",
		},
		{
			value: "transparent",
			labelKey: "settings:options.clientMode.transparent",
			fallback: "Transparent",
			disabled: true,
			tooltipKey: "settings:options.clientMode.transparentDisabledTooltip",
			tooltipFallback:
				"Transparent cannot be the workspace default. Enable it per client when a writable local path is available.",
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

type AuditPolicyFormState = {
	policyType: string;
	policyDays: number;
	policyCount: number;
};

function auditFormFromPolicyData(data: AuditPolicyData): AuditPolicyFormState & { sweepInterval: number } {
	const p = data.policy;
	if (p === "off") {
		return {
			policyType: "off",
			policyDays: 30,
			policyCount: 100_000,
			sweepInterval: data.sweep_interval_secs,
		};
	}
	if (typeof p === "object" && "keep_days" in p) {
		return {
			policyType: "keep_days",
			policyDays: p.keep_days.days,
			policyCount: 100_000,
			sweepInterval: data.sweep_interval_secs,
		};
	}
	if (typeof p === "object" && "keep_count" in p) {
		return {
			policyType: "keep_count",
			policyDays: 30,
			policyCount: p.keep_count.count,
			sweepInterval: data.sweep_interval_secs,
		};
	}
	if (typeof p === "object" && "combined" in p) {
		return {
			policyType: "combined",
			policyDays: p.combined.days,
			policyCount: p.combined.count,
			sweepInterval: data.sweep_interval_secs,
		};
	}
	return {
		policyType: "combined",
		policyDays: 30,
		policyCount: 100_000,
		sweepInterval: data.sweep_interval_secs,
	};
}

function buildAuditPolicy(type: string, days: number, count: number): AuditRetentionPolicy {
	switch (type) {
		case "off":
			return "off";
		case "keep_days":
			return { keep_days: { days } };
		case "keep_count":
			return { keep_count: { count } };
		default:
			return { combined: { days, count } };
	}
}

function auditFormMatchesSaved(form: AuditPolicyFormState, saved: AuditPolicyFormState): boolean {
	if (form.policyType !== saved.policyType) {
		return false;
	}
	switch (form.policyType) {
		case "off":
			return true;
		case "keep_days":
			return form.policyDays === saved.policyDays;
		case "keep_count":
			return form.policyCount === saved.policyCount;
		default:
			return form.policyDays === saved.policyDays && form.policyCount === saved.policyCount;
	}
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
		useState<LocalhostRuntimeMode>("desktop_managed");
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
	const [policyType, setPolicyType] = useState<string>("combined");
	const [policyDays, setPolicyDays] = useState<number>(30);
	const [policyCount, setPolicyCount] = useState<number>(100000);

	const policyQuery = useQuery({
		queryKey: ["audit", "policy"],
		queryFn: () => auditApi.getPolicy(),
	});

	const defaultClientPolicyQuery = useQuery({
		queryKey: ["client", "policy", "default"],
		queryFn: async () => {
			const settings = await systemApi.getSettings();
			return {
				config_mode: settings.default_config_mode,
				capability_source: "activated" as const,
				first_contact_behavior: settings.first_contact_behavior as
					| "deny"
					| "review"
					| "allow",
			};
		},
	});

	const systemSettingsQuery = useQuery({
		queryKey: ["system", "settings"],
		queryFn: () => systemApi.getSettings(),
	});
	const storeStatusQuery = useQuery({
		queryKey: ["secrets", "status"],
		queryFn: secretsApi.status,
	});
	const [selectedMode, setSelectedMode] = useState<string>("");
	const [passphraseInput, setPassphraseInput] = useState("");
	const [passphraseConfirmInput, setPassphraseConfirmInput] = useState("");
	const [passphraseSetupError, setPassphraseSetupError] = useState<string | null>(null);
	const [showSwitchConfirm, setShowSwitchConfirm] = useState(false);
	const [showPassphraseSetupDialog, setShowPassphraseSetupDialog] = useState(false);
	const [showCurrentPassphraseDialog, setShowCurrentPassphraseDialog] = useState(false);
	const [currentPassphraseInput, setCurrentPassphraseInput] = useState("");
	const [currentPassphraseError, setCurrentPassphraseError] = useState<string | null>(null);
	const [passphraseAction, setPassphraseAction] = useState<"switch" | "rotate" | null>(null);
	const passphraseSetupPasswordRef = useRef<HTMLInputElement>(null);
	const currentPassphraseInputRef = useRef<HTMLInputElement>(null);

	type ProviderSwitchMode = "operating_system" | "passphrase" | "local_file";
	type PendingProviderSwitch = {
		mode: ProviderSwitchMode;
		passphrase?: string;
		currentPassphrase?: string;
	};
	const pendingProviderSwitchRef = useRef<PendingProviderSwitch | null>(null);

	const resetPendingProviderSwitch = useCallback(() => {
		pendingProviderSwitchRef.current = null;
		setSelectedMode("");
		setPassphraseInput("");
		setPassphraseConfirmInput("");
		setPassphraseSetupError(null);
		setCurrentPassphraseInput("");
		setCurrentPassphraseError(null);
	}, []);

	const currentProviderMode =
		storeStatusQuery.data?.provider?.provider_mode ?? "operating_system";
	const isPassphraseModeConfigured =
		currentProviderMode === "passphrase" && storeStatusQuery.data?.status === "ready";
	const isPendingPassphraseSwitch =
		selectedMode === "passphrase" && currentProviderMode !== "passphrase";
	const effectiveEncryptionMode = selectedMode || currentProviderMode;

	const rotatePassphraseMutation = useMutation({
		mutationFn: () =>
			secretsApi.rotatePassphrase(
				currentPassphraseInput,
				passphraseInput,
				passphraseConfirmInput,
			),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["secrets", "status"] });
			resetPendingProviderSwitch();
			setShowPassphraseSetupDialog(false);
			setShowCurrentPassphraseDialog(false);
			setPassphraseAction(null);
			notifySuccess(
				t("settings:security.encryptionPasswordRotated", {
					defaultValue: "Encryption password updated successfully",
				}),
			);
		},
		onError: (error: unknown) => {
			resetPendingProviderSwitch();
			setShowPassphraseSetupDialog(false);
			setPassphraseAction(null);
			notifyError(
				t("settings:security.encryptionPasswordRotateError", {
					defaultValue: "Failed to update encryption password",
				}),
				stringifyError(error),
			);
		},
	});

	const switchProviderMutation = useMutation({
		mutationFn: (pending: PendingProviderSwitch) =>
			secretsApi.switchProvider(pending.mode, {
				passphrase: pending.passphrase,
				currentPassphrase: pending.currentPassphrase,
			}),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["secrets", "status"] });
			resetPendingProviderSwitch();
			setShowPassphraseSetupDialog(false);
			setShowSwitchConfirm(false);
			notifySuccess(
				t("settings:security.switchSuccess", {
					defaultValue: "Security mode updated successfully",
				}),
			);
		},
		onError: (error: unknown) => {
			resetPendingProviderSwitch();
			setShowSwitchConfirm(false);
			notifyError(
				t("settings:security.switchError", {
					defaultValue: "Failed to switch security mode",
				}),
				stringifyError(error),
			);
		},
	});

	const promptSecurityModeSwitchIfPending = useCallback(
		(modeOverride?: string) => {
			const mode = modeOverride ?? selectedMode;
			if (!mode) {
				return;
			}
			const isPassphraseRotation =
				mode === "passphrase" &&
				mode === currentProviderMode &&
				passphraseInput.trim().length > 0;
			if (mode === currentProviderMode && !isPassphraseRotation) {
				return;
			}
			if (mode === "passphrase" && !passphraseInput.trim()) {
				return;
			}
			if (switchProviderMutation.isPending || showSwitchConfirm) {
				return;
			}
			pendingProviderSwitchRef.current = {
				mode: mode as ProviderSwitchMode,
				passphrase: mode === "passphrase" ? passphraseInput : undefined,
			};
			setShowSwitchConfirm(true);
		},
		[
			selectedMode,
			passphraseInput,
			currentProviderMode,
			switchProviderMutation.isPending,
			showSwitchConfirm,
		],
	);

	const handleEncryptionModeChange = useCallback(
		(value: string) => {
			setSelectedMode(value);
			if (value === currentProviderMode) {
				return;
			}
			if (value === "passphrase") {
				setPassphraseInput("");
				setPassphraseConfirmInput("");
				setPassphraseSetupError(null);
				setShowPassphraseSetupDialog(true);
				return;
			}
			if (currentProviderMode === "passphrase") {
				setCurrentPassphraseInput("");
				setCurrentPassphraseError(null);
				setShowCurrentPassphraseDialog(true);
				return;
			}
			queueMicrotask(() => {
				promptSecurityModeSwitchIfPending(value);
			});
		},
		[currentProviderMode, promptSecurityModeSwitchIfPending],
	);

	const handlePassphraseSetupOpenChange = useCallback(
		(open: boolean) => {
			setShowPassphraseSetupDialog(open);
			if (!open) {
				setPassphraseInput("");
				setPassphraseConfirmInput("");
				setPassphraseSetupError(null);
				if (!showSwitchConfirm && !switchProviderMutation.isPending) {
					setSelectedMode("");
				}
			}
		},
		[showSwitchConfirm, switchProviderMutation.isPending],
	);

	const handlePassphraseSetupContinue = useCallback(() => {
		if (!passphraseInput.trim()) {
			setPassphraseSetupError(
				t("settings:security.passphraseRequired", {
					defaultValue: "Enter a master password to continue.",
				}),
			);
			return;
		}
		if (passphraseInput !== passphraseConfirmInput) {
			setPassphraseSetupError(
				t("settings:security.passphraseMismatch", {
					defaultValue: "Passwords do not match.",
				}),
			);
			return;
		}
		setPassphraseSetupError(null);
		setPassphraseConfirmInput("");
		setShowPassphraseSetupDialog(false);
		if (passphraseAction === "rotate") {
			rotatePassphraseMutation.mutate();
			return;
		}
		if (currentProviderMode === "passphrase") {
			setCurrentPassphraseInput("");
			setCurrentPassphraseError(null);
			setShowCurrentPassphraseDialog(true);
			return;
		}
		queueMicrotask(() => {
			promptSecurityModeSwitchIfPending("passphrase");
		});
	}, [
		passphraseInput,
		passphraseConfirmInput,
		currentProviderMode,
		passphraseAction,
		rotatePassphraseMutation,
		promptSecurityModeSwitchIfPending,
		t,
	]);

	const handleCurrentPassphraseOpenChange = useCallback(
		(open: boolean) => {
			setShowCurrentPassphraseDialog(open);
			if (!open && !switchProviderMutation.isPending) {
				setCurrentPassphraseInput("");
				setCurrentPassphraseError(null);
				if (!showSwitchConfirm) {
					resetPendingProviderSwitch();
				}
			}
		},
		[switchProviderMutation.isPending, showSwitchConfirm, resetPendingProviderSwitch],
	);

	const handleCurrentPassphraseContinue = useCallback(() => {
		if (!currentPassphraseInput.trim()) {
			setCurrentPassphraseError(
				t("settings:security.currentPassphraseRequired", {
					defaultValue: "Enter your current master password to continue.",
				}),
			);
			return;
		}
		setCurrentPassphraseError(null);
		if (passphraseAction === "rotate") {
			setShowCurrentPassphraseDialog(false);
			setShowPassphraseSetupDialog(true);
			return;
		}
		const mode = (selectedMode || currentProviderMode) as ProviderSwitchMode;
		pendingProviderSwitchRef.current = {
			mode,
			passphrase: mode === "passphrase" ? passphraseInput : undefined,
			currentPassphrase: currentPassphraseInput,
		};
		setShowCurrentPassphraseDialog(false);
		setShowSwitchConfirm(true);
	}, [
		currentPassphraseInput,
		selectedMode,
		currentProviderMode,
		passphraseInput,
		passphraseAction,
		t,
	]);

	const handleSwitchConfirmOpenChange = useCallback(
		(open: boolean) => {
			setShowSwitchConfirm(open);
			if (!open && !switchProviderMutation.isPending) {
				resetPendingProviderSwitch();
			}
		},
		[switchProviderMutation.isPending, resetPendingProviderSwitch],
	);

	const handleConfirmProviderSwitch = useCallback(() => {
		const pending = pendingProviderSwitchRef.current;
		if (!pending?.mode) {
			notifyError(
				t("settings:security.switchError", {
					defaultValue: "Failed to switch security mode",
				}),
				t("settings:security.switchMissingMode", {
					defaultValue: "No encryption mode selected for switching.",
				}),
			);
			setShowSwitchConfirm(false);
			resetPendingProviderSwitch();
			return;
		}
		switchProviderMutation.mutate(pending);
	}, [switchProviderMutation, resetPendingProviderSwitch, t]);

	// Password protection state
	const passwordQuery = useQuery({
		queryKey: ["password", "status"],
		queryFn: secretsApi.passwordStatus,
	});
	const [selectedProtectionLevel, setSelectedProtectionLevel] = useState<ProtectionLevel | "">("");
	const [pendingProtectionScope, setPendingProtectionScope] = useState<string[]>(["startup"]);
	const [settingsPasswordVerified, setSettingsPasswordVerified] = useState(
		() => sessionStorage.getItem("mcp_password_settings_verified") === "true",
	);
	const [protectionPasswordDialogOpen, setProtectionPasswordDialogOpen] = useState(false);
	const [protectionPasswordDialogMode, setProtectionPasswordDialogMode] =
		useState<ProtectionPasswordDialogMode>("set");

	const currentProtectionLevel = resolveProtectionLevel(passwordQuery.data);
	const effectiveProtectionLevel = selectedProtectionLevel || currentProtectionLevel;

	const needsSettingsPassword = useMemo(
		() => requiresSettingsPasswordGate(passwordQuery.data),
		[passwordQuery.data, settingsPasswordVerified],
	);

	const updatePasswordScopeMutation = useMutation({
		mutationFn: ({ level, currentPassword }: { level: Exclude<ProtectionLevel, "off">; currentPassword: string }) =>
			secretsApi.updatePasswordScope(protectionScopeForLevel(level), currentPassword),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["password", "status"] });
			setSelectedProtectionLevel("");
			notifySuccess(
				t("settings:security.protectionScopeUpdated", {
					defaultValue: "Protection mode updated",
				}),
			);
		},
		onError: (error: unknown) => {
			setSelectedProtectionLevel("");
			notifyError(
				t("settings:security.protectionScopeUpdateError", {
					defaultValue: "Failed to update protection mode",
				}),
				stringifyError(error),
			);
		},
	});

	const openProtectionPasswordDialog = useCallback((mode: ProtectionPasswordDialogMode) => {
		setProtectionPasswordDialogMode(mode);
		setProtectionPasswordDialogOpen(true);
	}, []);

	const openMasterPasswordDialog = useCallback(() => {
		setPassphraseInput("");
		setPassphraseConfirmInput("");
		setPassphraseSetupError(null);
		setCurrentPassphraseInput("");
		setCurrentPassphraseError(null);
		setSelectedMode("passphrase");
		if (isPassphraseModeConfigured) {
			setPassphraseAction("rotate");
			setShowCurrentPassphraseDialog(true);
			return;
		}
		setPassphraseAction("switch");
		setShowPassphraseSetupDialog(true);
	}, [isPassphraseModeConfigured]);

	const handleProtectionLevelChange = useCallback(
		(value: string) => {
			const level = value as ProtectionLevel;
			setSelectedProtectionLevel(level);

			if (level === "off") {
				if (passwordQuery.data?.has_password) {
					openProtectionPasswordDialog("clear");
				} else {
					setSelectedProtectionLevel("");
				}
				return;
			}

			if (!passwordQuery.data?.has_password) {
				setPendingProtectionScope(protectionScopeForLevel(level));
				openProtectionPasswordDialog("set");
				return;
			}

			if (level !== currentProtectionLevel) {
				// Verify current password before changing scope.
				openProtectionPasswordDialog("verify");
			}
		},
		[
			passwordQuery.data?.has_password,
			currentProtectionLevel,
			openProtectionPasswordDialog,
			updatePasswordScopeMutation,
		],
	);

	const handleProtectionPasswordDialogCancel = useCallback(() => {
		setSelectedProtectionLevel("");
		setPendingProtectionScope(["startup"]);
	}, []);

	const handleProtectionPasswordDialogSuccess = useCallback(
		(verifiedPassword?: string) => {
			if (protectionPasswordDialogMode === "verify" && verifiedPassword) {
				const level = selectedProtectionLevel as Exclude<ProtectionLevel, "off">;
				updatePasswordScopeMutation.mutate({ level, currentPassword: verifiedPassword });
			}
			setSelectedProtectionLevel("");
			setPendingProtectionScope(["startup"]);
		},
		[protectionPasswordDialogMode, selectedProtectionLevel, updatePasswordScopeMutation],
	);

	const handleSettingsPasswordUnlock = useCallback((_password: string) => {
		sessionStorage.setItem("mcp_password_settings_verified", "true");
		setSettingsPasswordVerified(true);
	}, []);

	const inspectorTimeoutMutation = useMutation({
		mutationFn: (timeout_ms: number) =>
			systemApi.setSettings({ inspector_timeout_ms: timeout_ms }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["system", "settings"] });
			notifySuccess(
				t("settings:developer.inspectorTimeoutSaved", {
					defaultValue: "Inspector timeout updated",
				}),
			);
		},
		onError: (_error: unknown) => {
			notifyError(
				t("settings:developer.inspectorTimeoutSaveError", {
					defaultValue: "Failed to save inspector timeout",
				}),
			);
		},
	});

	const [inspectorTimeoutInput, setInspectorTimeoutInput] = useState<number>(8000);

	useEffect(() => {
		if (systemSettingsQuery.data) {
			setInspectorTimeoutInput(systemSettingsQuery.data.inspector_timeout_ms);
		}
	}, [systemSettingsQuery.data]);

	const handleInspectorTimeoutChange = (ms: number) => {
		const clamped = Math.max(1000, Math.min(300000, ms));
		setInspectorTimeoutInput(clamped);
	};

	const handleInspectorTimeoutBlur = useCallback(() => {
		const saved = systemSettingsQuery.data?.inspector_timeout_ms ?? 8000;
		const clamped = Math.max(1000, Math.min(300000, inspectorTimeoutInput));
		if (clamped !== inspectorTimeoutInput) {
			setInspectorTimeoutInput(clamped);
		}
		if (clamped === saved || inspectorTimeoutMutation.isPending) {
			return;
		}
		inspectorTimeoutMutation.mutate(clamped);
	}, [
		inspectorTimeoutInput,
		inspectorTimeoutMutation,
		systemSettingsQuery.data?.inspector_timeout_ms,
	]);

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
		}
	}, [policyQuery.data]);

	useEffect(() => {
		if (defaultClientPolicyQuery.data?.config_mode) {
			setDashboardSetting(
				"clientDefaultMode",
				defaultClientPolicyQuery.data.config_mode as ClientDefaultMode,
			);
		}
	}, [defaultClientPolicyQuery.data, setDashboardSetting]);

	const policyMutation = useMutation({
		mutationFn: (data: { policy: AuditRetentionPolicy; sweep_interval_secs: number }) =>
			auditApi.setPolicy(data),
		onSuccess: () => {
			notifySuccess(t("settings:audit.saved", { defaultValue: "Log retention settings saved" }));
			policyQuery.refetch();
		},
		onError: (e) => {
			notifyError(t("settings:audit.saveFailed", { defaultValue: "Failed to save log retention settings" }), String(e));
		},
	});

	const defaultClientPolicyMutation = useMutation({
		mutationFn: async (payload: {
			config_mode: ClientDefaultMode;
			capability_source: CapabilitySource;
			first_contact_behavior: "deny" | "review" | "allow";
			policySnapshot?: { config_mode: string; first_contact_behavior: string } | null;
		}) => {
			const {
				config_mode,
				capability_source,
				first_contact_behavior,
				policySnapshot,
			} = payload;
			const modeChanged =
				!policySnapshot || policySnapshot.config_mode !== config_mode;
			const behaviorChanged =
				!policySnapshot ||
				policySnapshot.first_contact_behavior !== first_contact_behavior;

			if (!modeChanged && !behaviorChanged) {
				return {
					config_mode,
					capability_source,
					first_contact_behavior,
				};
			}

			const settings = await systemApi.setSettings({
				default_config_mode: config_mode,
				first_contact_behavior,
			});

			return {
				config_mode: settings.default_config_mode as ClientDefaultMode,
				capability_source,
				first_contact_behavior: settings.first_contact_behavior as
					| "deny"
					| "review"
					| "allow",
			};
		},
		onSuccess: (data) => {
			if (!data) {
				throw new Error("Missing default client policy response");
			}
			// Sync cache immediately so duplicate Radix `onValueChange` (same tick / refetch race)
			// sees the new policy and skips a second mutate + notify.
			queryClient.setQueryData(["client", "policy", "default"], {
				config_mode: data.config_mode,
				capability_source: data.capability_source,
				first_contact_behavior: data.first_contact_behavior,
			});
			setDashboardSetting("clientDefaultMode", data.config_mode as ClientDefaultMode);
			notifySuccess(
				t("settings:clients.modeTitle", {
					defaultValue: "Client Management Mode",
				}),
				t("settings:clients.policySaved", {
					defaultValue: "Default client policy updated.",
				}),
			);
			void defaultClientPolicyQuery.refetch();
		},
		onError: (error) => {
			notifyError(
				t("settings:clients.policySaveFailed", {
					defaultValue: "Failed to update default client policy",
				}),
				stringifyError(error),
			);
		},
	});

	const persistAuditPolicyIfChanged = useCallback(
		(next: AuditPolicyFormState) => {
			if (!policyQuery.data || policyMutation.isPending) {
				return;
			}

			const saved = auditFormFromPolicyData(policyQuery.data);
			const days = Math.max(
				1,
				Number.isFinite(next.policyDays) ? next.policyDays : saved.policyDays,
			);
			const count = Math.max(
				1,
				Number.isFinite(next.policyCount) ? next.policyCount : saved.policyCount,
			);
			const normalized: AuditPolicyFormState = {
				policyType: next.policyType,
				policyDays: days,
				policyCount: count,
			};

			if (auditFormMatchesSaved(normalized, saved)) {
				return;
			}

			policyMutation.mutate({
				policy: buildAuditPolicy(normalized.policyType, normalized.policyDays, normalized.policyCount),
				sweep_interval_secs: saved.sweepInterval,
			});
		},
		[policyQuery.data, policyMutation],
	);

	const handlePolicyTypeChange = useCallback(
		(value: string) => {
			setPolicyType(value);
			persistAuditPolicyIfChanged({ policyType: value, policyDays, policyCount });
		},
		[policyDays, policyCount, persistAuditPolicyIfChanged],
	);

	const handlePolicyDaysBlur = useCallback(() => {
		const saved = policyQuery.data ? auditFormFromPolicyData(policyQuery.data) : null;
		const clamped = Math.max(
			1,
			Number.isFinite(policyDays) ? policyDays : (saved?.policyDays ?? 30),
		);
		if (clamped !== policyDays) {
			setPolicyDays(clamped);
		}
		persistAuditPolicyIfChanged({ policyType, policyDays: clamped, policyCount });
	}, [policyType, policyDays, policyCount, policyQuery.data, persistAuditPolicyIfChanged]);

	const handlePolicyCountBlur = useCallback(() => {
		const saved = policyQuery.data ? auditFormFromPolicyData(policyQuery.data) : null;
		const clamped = Math.max(
			1,
			Number.isFinite(policyCount) ? policyCount : (saved?.policyCount ?? 100_000),
		);
		if (clamped !== policyCount) {
			setPolicyCount(clamped);
		}
		persistAuditPolicyIfChanged({ policyType, policyDays, policyCount: clamped });
	}, [policyType, policyDays, policyCount, policyQuery.data, persistAuditPolicyIfChanged]);

	const applyCoreSourceView = useCallback(
		(response: DesktopCoreSourceResponse) => {
			setCoreSource(response.selectedSource);
			setLocalhostRuntimeMode(response.localhostRuntimeMode);
			setRemoteBaseUrl(response.remoteBaseUrl || "");
			setLocalService(response.localService);
			setApiPort(response.localhostApiPort);
			setMcpPort(response.localhostMcpPort);
		},
		[],
	);

	const wireDashboardToCoreSource = useCallback(
		async (apiBaseUrl: string) => {
			setApiBaseUrl(apiBaseUrl);
			notificationsService.reconnectAfterApiBaseChanged();
			await queryClient.invalidateQueries({ predicate: () => true });
		},
		[queryClient],
	);

	const loadRuntimePorts = useCallback(async () => {
		setLoadingPorts(true);
		try {
			const applyAuthorityPorts = (api: number, mcp: number) => {
				setApiPort(api);
				setMcpPort(mcp);
				setCoreSource("localhost");
				setLocalhostRuntimeMode("desktop_managed");
				setRemoteBaseUrl("");
				setLocalService((current) => ({
					...current,
					status: "not_installed",
					label: "Not Installed",
					running: false,
					installed: false,
				}));
				syncApiBaseUrlForRuntimePort(api);
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
					notifyError(
						t("settings:system.portsReloadFailedTitle", {
							defaultValue: "Could not load ports from shell",
						}),
						t("settings:system.portsReloadFailedDescription", {
							defaultValue:
								"Check the desktop app is healthy and try Reload again.",
						}),
					);
				}
				return;
			}

			const settings = await systemApi.getSettings();
			if (
				typeof settings.api_port === "number" &&
				typeof settings.mcp_port === "number"
			) {
				applyAuthorityPorts(settings.api_port, settings.mcp_port);
				return;
			}

		} finally {
			setLoadingPorts(false);
		}
	}, [applyCoreSourceView, refreshCoreView, t, i18n.language]);

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
	const localServiceStatusLabel = t(
		`settings:system.localServiceStatus.${localService.status}`,
		{
			defaultValue: localService.status,
		},
	);
	const localServiceDetail = t(
		`settings:system.localServiceDetail.${localhostRuntimeMode}.${localService.status}`,
		{
			defaultValue: t("settings:system.serviceStatusFallback", {
				defaultValue:
					"The desktop will attach to the configured localhost core service when it is available.",
			}),
		},
	);

	const isSystemReadonlyInWeb = !isTauriShell;
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
		if (!isTauriShell || typeof apiPort !== "number" || typeof mcpPort !== "number") {
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
			await wireDashboardToCoreSource(response.apiBaseUrl);
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
	const generalSettingsRowClass =
		"flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4";
	const generalSettingsLabelClass = "min-w-0 space-y-0.5";
	const generalSettingsControlClass = "w-full shrink-0 sm:w-72";
	/** Clients tab: left column wraps without stealing flex space from controls; right keeps a floor width so Segments are not squeezed. */
	const clientsSettingsRowClass =
		"flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between sm:gap-6";
	const clientsSettingsLabelClass =
		"min-w-0 max-w-full flex-1 space-y-0.5 sm:pr-2 lg:max-w-lg xl:max-w-xl";
	const clientsSettingsControlClass =
		"w-full shrink-0 sm:w-auto sm:min-w-[16rem] md:min-w-[20rem] lg:min-w-[24rem]";

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
			DEFAULT_VIEW_CONFIG.map(({ value, icon: Icon, labelKey, fallback }) => ({
				value,
				label: t(labelKey, { defaultValue: fallback }),
				icon: <Icon className="h-4 w-4" />,
			})),
		[t, i18n.language],
	);

	const clientModeOptions = useMemo<SegmentOption[]>(
		() =>
			CLIENT_MODE_CONFIG.map(
				({ value, labelKey, fallback, disabled, tooltipKey, tooltipFallback }) => ({
					value,
					label: t(labelKey, { defaultValue: fallback }),
					...(disabled ? { disabled: true } : {}),
					...(tooltipKey && tooltipFallback
						? {
							tooltip: t(tooltipKey, { defaultValue: tooltipFallback }),
						}
						: {}),
				}),
			),
		[t, i18n.language],
	);

	const firstContactOptions = useMemo<SegmentOption[]>(
		() => [
			{
				value: "deny",
				label: t("settings:clients.firstContact.deny", { defaultValue: "Deny" }),
			},
			{
				value: "review",
				label: t("settings:clients.firstContact.review", { defaultValue: "Review" }),
			},
			{
				value: "allow",
				label: t("settings:clients.firstContact.allow", { defaultValue: "Allow" }),
			},
		],
		[t, i18n.language],
	);

	const currentFirstContactBehavior =
		(defaultClientPolicyQuery.data?.first_contact_behavior as
			| "deny"
			| "review"
			| "allow"
			| undefined) ?? "review";

	const handleClientDefaultModeSegmentChange = useCallback(
		(value: string) => {
			if (defaultClientPolicyMutation.isPending) {
				return;
			}
			const next = value as ClientDefaultMode;
			const currentMode =
				(defaultClientPolicyQuery.data?.config_mode as ClientDefaultMode | undefined) ??
				dashboardSettings.clientDefaultMode;
			if (next === currentMode) {
				return;
			}
			defaultClientPolicyMutation.mutate({
				config_mode: next,
				capability_source: "activated",
				first_contact_behavior: currentFirstContactBehavior,
				policySnapshot: defaultClientPolicyQuery.data
					? {
						config_mode: defaultClientPolicyQuery.data.config_mode,
						first_contact_behavior:
							defaultClientPolicyQuery.data.first_contact_behavior,
					}
					: null,
			});
		},
		[
			currentFirstContactBehavior,
			dashboardSettings.clientDefaultMode,
			defaultClientPolicyMutation,
			defaultClientPolicyQuery.data,
		],
	);

	const handleFirstContactSegmentChange = useCallback(
		(value: string) => {
			if (defaultClientPolicyMutation.isPending) {
				return;
			}
			const next = value as "deny" | "review" | "allow";
			if (next === currentFirstContactBehavior) {
				return;
			}
			defaultClientPolicyMutation.mutate({
				config_mode:
					(defaultClientPolicyQuery.data?.config_mode as ClientDefaultMode | undefined) ??
					dashboardSettings.clientDefaultMode,
				capability_source: "activated",
				first_contact_behavior: next,
				policySnapshot: defaultClientPolicyQuery.data
					? {
						config_mode: defaultClientPolicyQuery.data.config_mode,
						first_contact_behavior:
							defaultClientPolicyQuery.data.first_contact_behavior,
					}
					: null,
			});
		},
		[
			currentFirstContactBehavior,
			dashboardSettings.clientDefaultMode,
			defaultClientPolicyMutation,
			defaultClientPolicyQuery.data,
		],
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
	const settingsTabs = useMemo(
		() =>
			showLicenseTab
				? [
					"general",
					"servers",
					"clients",
					"profile",
					"market",
					"audit",
					"develop",
					"security",
					"system",
					"about",
				]
				: [
					"general",
					"servers",
					"clients",
					"profile",
					"market",
					"audit",
					"develop",
					"security",
					"system",
				],
		[showLicenseTab],
	);
	const { activeTab, setActiveTab } = useUrlTab({
		paramName: "tab",
		defaultTab: "general",
		validTabs: settingsTabs,
	});

	if (needsSettingsPassword) {
		return <LockScreen variant="login" onSuccess={handleSettingsPasswordUnlock} />;
	}

	return (
		<div className="space-y-4">
			<div className="flex items-center gap-2 min-w-0">
				<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
					{t("settings:title", { defaultValue: "Settings" })}
				</p>
			</div>

			<Tabs
				value={activeTab}
				onValueChange={setActiveTab}
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
					<TabsTrigger value="audit" className={tabTriggerClass}>
						<FileSearch className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.audit", { defaultValue: "Logs" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="develop" className={tabTriggerClass}>
						<Bug className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.developer", { defaultValue: "Developer" })}
						</span>
					</TabsTrigger>
					<TabsTrigger value="security" className={tabTriggerClass}>
						<ShieldCheck className="h-4 w-4 shrink-0" />
						<span className="hidden md:inline truncate">
							{t("settings:tabs.security", { defaultValue: "Security" })}
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
											"Baseline preferences for workspace layout, theme, language, and desktop shell options.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								{/* Default View */}
								<div className={generalSettingsRowClass}>
									<div className={generalSettingsLabelClass}>
										<h3 className={settingItemTitleClass}>
											{t("settings:general.defaultView", {
												defaultValue: "Default View",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:general.defaultViewDescription", {
												defaultValue:
													"Choose the default layout for displaying items.",
											})}
										</p>
									</div>
									<div className={generalSettingsControlClass}>
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

								{/* Theme */}
								<div className={generalSettingsRowClass}>
									<div className={generalSettingsLabelClass}>
										<h3 className={settingItemTitleClass}>
											{t("settings:general.themeTitle", {
												defaultValue: "Theme",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:general.themeDescription", {
												defaultValue: "Switch between light, dark, and system theme.",
											})}
										</p>
									</div>
									<div className={generalSettingsControlClass}>
										<Segment
											options={themeOptions}
											value={theme}
											onValueChange={(value) => setTheme(value as Theme)}
											showDots={false}
										/>
									</div>
								</div>

								{/* Language Selection */}
								<div className={generalSettingsRowClass}>
									<div className={generalSettingsLabelClass}>
										<h3 className={settingItemTitleClass}>
											{t("settings:general.language", {
												defaultValue: "Language",
											})}
										</h3>
										<p className={settingItemDescriptionClass}>
											{t("settings:general.languageDescription", {
												defaultValue: "Select the dashboard language.",
											})}
										</p>
									</div>
									<div className={generalSettingsControlClass}>
										<Select
											value={dashboardSettings.language}
											onValueChange={(value: DashboardLanguage) =>
												setDashboardSetting("language", value)
											}
										>
											<SelectTrigger id={languageId} className="w-full">
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
								</div>

								{isTauriShell && (
									<div className="space-y-5">
										<div className={generalSettingsRowClass}>
											<div className={generalSettingsLabelClass}>
												<h3 className={settingItemTitleClass}>
													{t("settings:general.menuBarTitle", {
														defaultValue: "Menu Bar Icon",
													})}
												</h3>
												<p className={settingItemDescriptionClass}>
													{t("settings:general.menuBarDescription", {
														defaultValue:
															"Choose when the desktop tray icon should appear.",
													})}
												</p>
											</div>
											<div className={generalSettingsControlClass}>
												<Select
													value={dashboardSettings.menuBarIconMode}
													onValueChange={(value: MenuBarIconMode) =>
														setDashboardSetting("menuBarIconMode", value)
													}
												>
													<SelectTrigger id={menuBarSelectId} className="w-full">
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
										</div>

										<div className={generalSettingsRowClass}>
											<div className={generalSettingsLabelClass}>
												<h3 className={settingItemTitleClass}>
													{t("settings:general.dockTitle", {
														defaultValue: "Dock / Taskbar Icon",
													})}
												</h3>
												<p className={settingItemDescriptionClass}>
													{t("settings:general.dockDescription", {
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
												{t("settings:general.dockHiddenNotice", {
													defaultValue:
														"The Dock or taskbar entry is hidden. The tray icon stays visible so you can reopen MCPMate.",
												})}
											</p>
										)}
									</div>
								)}
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
										checked={dashboardSettings.showServerLevelLogs}
										onCheckedChange={(checked) =>
											setDashboardSetting("showServerLevelLogs", checked)
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
								<div className={clientsSettingsRowClass}>
									<div className={clientsSettingsLabelClass}>
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
									<div className={clientsSettingsControlClass}>
										<Segment
											options={clientModeOptions}
											value={defaultClientPolicyQuery.data?.config_mode ?? dashboardSettings.clientDefaultMode}
											onValueChange={handleClientDefaultModeSegmentChange}
											disabled={defaultClientPolicyMutation.isPending}
											showDots={false}
										/>
									</div>
								</div>

								<div className={clientsSettingsRowClass}>
									<div className={clientsSettingsLabelClass}>
										<h3 className="text-base font-medium">
											{t("settings:clients.firstContactTitle", {
												defaultValue: "First-contact Behavior",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:clients.firstContactDescription", {
												defaultValue:
													"Control how new, unknown clients are handled when they first request an MCP connection.",
											})}
										</p>
									</div>
									<div className={clientsSettingsControlClass}>
										<Segment
											options={firstContactOptions}
											value={currentFirstContactBehavior}
											onValueChange={handleFirstContactSegmentChange}
											disabled={defaultClientPolicyMutation.isPending}
											showDots={false}
										/>
									</div>
								</div>

								<div className={clientsSettingsRowClass}>
									<div className={clientsSettingsLabelClass}>
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
									<div className={clientsSettingsControlClass}>
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

								<div className={clientsSettingsRowClass}>
									<div className={clientsSettingsLabelClass}>
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
									<div className={clientsSettingsControlClass}>
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

								<div className={clientsSettingsRowClass}>
									<div className={clientsSettingsLabelClass}>
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
										className={clientsSettingsControlClass}
									/>
								</div>
								<div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between sm:gap-6">
									<div className={clientsSettingsLabelClass}>
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
									<div className="shrink-0">
										<Switch
											checked={dashboardSettings.showClientLiveLogs}
											onCheckedChange={(checked) =>
												setDashboardSetting("showClientLiveLogs", checked)
											}
										/>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="security" className="mt-0 h-full">
						<Card className="h-full">
							<CardHeader>
								<CardTitle>
									{t("settings:security.title", { defaultValue: "Security" })}
								</CardTitle>
								<CardDescription>
									{t("settings:security.description", {
										defaultValue:
											"Login password and root key encryption settings.",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-6">
								{storeStatusQuery.isLoading ? (
									<p className="text-sm text-muted-foreground">
										{t("settings:security.loading", { defaultValue: "Checking store status..." })}
									</p>
								) : storeStatusQuery.isError ? (
									<Alert variant="destructive">
										<ShieldCheck className="h-4 w-4" />
										<AlertTitle>
											{t("settings:security.error.title", { defaultValue: "Status check failed" })}
										</AlertTitle>
										<AlertDescription>
											{storeStatusQuery.error instanceof Error
												? storeStatusQuery.error.message
												: t("settings:security.error.description", { defaultValue: "Could not retrieve store status." })}
										</AlertDescription>
									</Alert>
								) : storeStatusQuery.data ? (
									<>
										<div className="space-y-3">
											{/* Password Protection */}
											<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
												<div className="space-y-1.5">
													<div className="flex items-center gap-1.5">
														<h3 className="text-base font-medium">
															{t("settings:security.passwordProtection", { defaultValue: "Password Protection" })}
														</h3>
														<span
															className="inline-flex shrink-0"
															aria-label={
																effectiveProtectionLevel === "off"
																	? t("settings:security.protectionDisabled", {
																		defaultValue: "Protection disabled",
																	})
																	: t("settings:security.protectionEnabled", {
																		defaultValue: "Protection enabled",
																	})
															}
														>
															<ShieldCheck
																className={`h-4 w-4 ${effectiveProtectionLevel === "off"
																	? "text-red-500"
																	: "text-emerald-600"
																	}`}
															/>
														</span>
													</div>
													<p className="text-sm text-muted-foreground">
														{t("settings:security.passwordProtectionDescription", {
															defaultValue: "Require a login password before accessing MCPMate or Settings.",
														})}
													</p>
												</div>
												<div className="flex sm:justify-end">
													<Segment
														options={[
															{
																value: "startup",
																label: t("settings:security.protectionLevel.startup", {
																	defaultValue: "On entry",
																}),
															},
															{
																value: "settings",
																label: t("settings:security.protectionLevel.settings", {
																	defaultValue: "In Settings",
																}),
															},
															{
																value: "off",
																label: t("settings:security.protectionLevel.off", {
																	defaultValue: "None",
																}),
															},
														]}
														value={effectiveProtectionLevel}
														onValueChange={handleProtectionLevelChange}
														showDots={false}
														disabled={updatePasswordScopeMutation.isPending}
													/>
												</div>
											</div>
											{effectiveProtectionLevel !== "off" ? (
												<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
													<div className="space-y-1.5">
														<h3 className="text-base font-medium">
															{t("settings:security.loginPasswordRow", { defaultValue: "Password" })}
														</h3>
														<p className="text-sm text-muted-foreground">
															{t("settings:security.loginPasswordRowDescription", {
																defaultValue: "Login password used when protection is enabled.",
															})}
														</p>
													</div>
													<div className="flex sm:justify-end">
														<Button
															variant="outline"
															size="sm"
															onClick={() =>
																openProtectionPasswordDialog(
																	passwordQuery.data?.has_password ? "change" : "set",
																)
															}
														>
															{passwordQuery.data?.has_password
																? t("settings:security.changePassword", {
																	defaultValue: "Change Password",
																})
																: t("settings:security.setPassword", {
																	defaultValue: "Set Password",
																})}
														</Button>
													</div>
												</div>
											) : null}
										</div>

										<div className="space-y-3 border-t pt-6">
											{/* Encryption Mode */}
											<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
												<div className="space-y-1.5">
													<div className="flex items-center gap-1.5">
														<h3 className="text-base font-medium">
															{t("settings:security.encryptionMode", { defaultValue: "Encryption Mode" })}
														</h3>
														<span
															className="inline-flex shrink-0"
															aria-label={t("settings:security.encryptionModeStatus", {
																defaultValue: "Encryption mode security level",
															})}
														>
															<ShieldCheck
																className={`h-4 w-4 ${effectiveEncryptionMode === "passphrase"
																	? "text-orange-500"
																	: effectiveEncryptionMode === "local_file"
																		? "text-yellow-500"
																		: "text-emerald-600"
																	}`}
															/>
														</span>
													</div>
													<p className="text-sm text-muted-foreground">
														{t("settings:security.encryptionModeDescription", {
															defaultValue: "How the root encryption key is stored and protected.",
														})}
													</p>
												</div>
												<div className="flex sm:justify-end">
													<Segment
														options={[
															{ value: "operating_system", label: t("settings:security.mode.os", { defaultValue: "OS Keychain" }) },
															{ value: "passphrase", label: t("settings:security.mode.passphrase", { defaultValue: "Password" }) },
															{ value: "local_file", label: t("settings:security.mode.local", { defaultValue: "Local File" }) },
														]}
														value={selectedMode || currentProviderMode}
														onValueChange={handleEncryptionModeChange}
														showDots={false}
													/>
												</div>
											</div>
											{effectiveEncryptionMode === "passphrase" ? (
												<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center">
													<div className="space-y-1.5">
														<h3 className="text-base font-medium">
															{t("settings:security.encryptionPasswordRow", {
																defaultValue: "Password",
															})}
														</h3>
														<p className="text-sm text-muted-foreground">
															{t("settings:security.encryptionPasswordRowDescription", {
																defaultValue:
																	"Master password that wraps the root encryption key.",
															})}
														</p>
													</div>
													<div className="flex sm:justify-end">
														<Button
															variant="outline"
															size="sm"
															onClick={openMasterPasswordDialog}
														>
															{isPassphraseModeConfigured && !isPendingPassphraseSwitch
																? t("settings:security.changePassword", {
																	defaultValue: "Change Password",
																})
																: t("settings:security.setPassword", {
																	defaultValue: "Set Password",
																})}
														</Button>
													</div>
												</div>
											) : null}

											{storeStatusQuery.data.issue ? (
												<Alert variant="destructive">
													<ShieldCheck className="h-4 w-4" />
													<AlertTitle>
														{t("settings:security.issue.title", { defaultValue: "Store Issue" })}
													</AlertTitle>
													<AlertDescription>
														<strong>{storeStatusQuery.data.issue.reason_code}</strong>
														{" — "}
														{storeStatusQuery.data.issue.message}
													</AlertDescription>
												</Alert>
											) : null}

											{/* Mode description */}
											{effectiveEncryptionMode === "operating_system" && (
												<p className="text-xs text-muted-foreground">
													{t("settings:security.mode.osDetail", {
														defaultValue:
															"Root key stored in macOS Keychain, Windows Credential Manager, or Linux Secret Service. Best protection — no password needed.",
													})}
												</p>
											)}
											{effectiveEncryptionMode === "passphrase" && (
												<p className="text-xs text-muted-foreground">
													{t("settings:security.mode.passphraseDetail", {
														defaultValue:
															"Protect secrets with a password you set. Losing the password makes stored secrets unrecoverable.",
													})}
												</p>
											)}
											{effectiveEncryptionMode === "local_file" && (
												<p className="text-xs text-muted-foreground">
													{t("settings:security.mode.localDetail", {
														defaultValue:
															"Root key stored as a file in the app data directory. Protected by file permissions only — not recommended for sensitive environments.",
													})}
												</p>
											)}
										</div>

									</>
								) : null}
							</CardContent>
						</Card>

						<ProtectionPasswordDialog
							open={protectionPasswordDialogOpen}
							onOpenChange={setProtectionPasswordDialogOpen}
							mode={protectionPasswordDialogMode}
							scope={pendingProtectionScope}
							onSuccess={handleProtectionPasswordDialogSuccess}
							onCancel={handleProtectionPasswordDialogCancel}
						/>

						<AlertDialog open={showPassphraseSetupDialog} onOpenChange={handlePassphraseSetupOpenChange}>
							<AlertDialogContent
								onOpenAutoFocus={(event) => {
									event.preventDefault();
									passphraseSetupPasswordRef.current?.focus();
								}}
							>
								<AlertDialogHeader>
									<AlertDialogTitle>
										{t("settings:security.passphraseSetupTitle", {
											defaultValue: "Set Master Password",
										})}
									</AlertDialogTitle>
									<AlertDialogDescription>
										{t("settings:security.passphraseSetupDescription", {
											defaultValue:
												"This password wraps your root encryption key. It is not stored in plaintext. You will need it again only when switching away from Password encryption mode.",
										})}
									</AlertDialogDescription>
								</AlertDialogHeader>
								<div className="space-y-3">
									<div className="space-y-2">
										<Label htmlFor="passphrase-setup-password">
											{t("settings:security.passphraseLabel", { defaultValue: "Master Password" })}
										</Label>
										<Input
											ref={passphraseSetupPasswordRef}
											id="passphrase-setup-password"
											type="password"
											value={passphraseInput}
											onChange={(e) => {
												setPassphraseInput(e.target.value);
												setPassphraseSetupError(null);
											}}
											placeholder={t("settings:security.passphrasePlaceholder", {
												defaultValue: "Enter password...",
											})}
											className="h-9"
										/>
									</div>
									<div className="space-y-2">
										<Label htmlFor="passphrase-setup-confirm">
											{t("settings:security.confirmPassword", { defaultValue: "Confirm Password" })}
										</Label>
										<Input
											id="passphrase-setup-confirm"
											type="password"
											value={passphraseConfirmInput}
											onChange={(e) => {
												setPassphraseConfirmInput(e.target.value);
												setPassphraseSetupError(null);
											}}
											placeholder={t("settings:security.passphraseConfirmPlaceholder", {
												defaultValue: "Re-enter password...",
											})}
											className="h-9"
										/>
									</div>
									{passphraseSetupError ? (
										<p className="text-sm text-destructive">{passphraseSetupError}</p>
									) : null}
								</div>
								<AlertDialogFooter>
									<AlertDialogCancel>
										{t("settings:security.confirmCancel", { defaultValue: "Cancel" })}
									</AlertDialogCancel>
									<AlertDialogAction
										onClick={(event) => {
											event.preventDefault();
											handlePassphraseSetupContinue();
										}}
									>
										{t("settings:security.passphraseSetupContinue", { defaultValue: "Continue" })}
									</AlertDialogAction>
								</AlertDialogFooter>
							</AlertDialogContent>
						</AlertDialog>

						<AlertDialog
							open={showCurrentPassphraseDialog}
							onOpenChange={handleCurrentPassphraseOpenChange}
						>
							<AlertDialogContent
								onOpenAutoFocus={(event) => {
									event.preventDefault();
									currentPassphraseInputRef.current?.focus();
								}}
							>
								<AlertDialogHeader>
									<AlertDialogTitle>
										{t("settings:security.currentPassphraseTitle", {
											defaultValue: "Enter Current Master Password",
										})}
									</AlertDialogTitle>
									<AlertDialogDescription>
										{t("settings:security.currentPassphraseDescription", {
											defaultValue:
												"Your root key is wrapped with your current master password. Enter it to unlock the key before switching encryption mode.",
										})}
									</AlertDialogDescription>
								</AlertDialogHeader>
								<div className="space-y-3">
									<div className="space-y-2">
										<Label htmlFor="current-passphrase-input">
											{t("settings:security.passphraseLabel", { defaultValue: "Master Password" })}
										</Label>
										<Input
											ref={currentPassphraseInputRef}
											id="current-passphrase-input"
											type="password"
											value={currentPassphraseInput}
											onChange={(e) => {
												setCurrentPassphraseInput(e.target.value);
												setCurrentPassphraseError(null);
											}}
											placeholder={t("settings:security.passphrasePlaceholder", {
												defaultValue: "Enter password...",
											})}
											className="h-9"
										/>
									</div>
									{currentPassphraseError ? (
										<p className="text-sm text-destructive">{currentPassphraseError}</p>
									) : null}
								</div>
								<AlertDialogFooter>
									<AlertDialogCancel>
										{t("settings:security.confirmCancel", { defaultValue: "Cancel" })}
									</AlertDialogCancel>
									<AlertDialogAction
										onClick={(event) => {
											event.preventDefault();
											handleCurrentPassphraseContinue();
										}}
									>
										{t("settings:security.passphraseSetupContinue", { defaultValue: "Continue" })}
									</AlertDialogAction>
								</AlertDialogFooter>
							</AlertDialogContent>
						</AlertDialog>

						<AlertDialog open={showSwitchConfirm} onOpenChange={handleSwitchConfirmOpenChange}>
							<AlertDialogContent>
								<AlertDialogHeader>
									<AlertDialogTitle>
										{t("settings:security.confirmTitle", {
											defaultValue: "Switch Security Mode?",
										})}
									</AlertDialogTitle>
									<AlertDialogDescription>
										{t("settings:security.confirmDescription", {
											defaultValue:
												"This will migrate your root key to a new storage provider. Your encrypted secrets remain unchanged — only the key custody location changes. This operation is safe and reversible.",
										})}
									</AlertDialogDescription>
								</AlertDialogHeader>
								<AlertDialogFooter>
									<AlertDialogCancel>
										{t("settings:security.confirmCancel", { defaultValue: "Cancel" })}
									</AlertDialogCancel>
									<AlertDialogAction
										onClick={(event) => {
											event.preventDefault();
											handleConfirmProviderSwitch();
										}}
										disabled={switchProviderMutation.isPending}
									>
										{switchProviderMutation.isPending
											? t("settings:security.switching", { defaultValue: "Switching..." })
											: t("settings:security.confirmAction", { defaultValue: "Switch Mode" })}
									</AlertDialogAction>
								</AlertDialogFooter>
							</AlertDialogContent>
						</AlertDialog>
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
									{t("settings:audit.title", { defaultValue: "Log retention" })}
								</CardTitle>
								<CardDescription>
									{t("settings:audit.description", {
										defaultValue:
											"Control how long activity log events are kept in the local database.",
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
											{t("settings:audit.typeDescription", { defaultValue: "Choose how stored log events are pruned automatically." })}
										</p>
									</div>
									<div className="flex sm:justify-end">
										<Select value={policyType} onValueChange={handlePolicyTypeChange}>
											<SelectTrigger
												className="w-full sm:w-64"
												disabled={policyMutation.isPending}
											>
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
												onBlur={handlePolicyDaysBlur}
												disabled={policyMutation.isPending}
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
												onBlur={handlePolicyCountBlur}
												disabled={policyMutation.isPending}
												className="w-full sm:w-64"
											/>
										</div>
									</div>
								)}

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
													if (!isSystemReadonlyInWeb && value === "localhost") {
														setCoreSource("localhost");
													}
												}}
												options={sourceOptions}
												showDots={false}
												disabled={isSystemReadonlyInWeb}
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
													disabled={isSystemReadonlyInWeb}
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
													{localServiceStatusLabel}
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
													{localServiceDetail}
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
											readOnly={isSystemReadonlyInWeb}
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
											readOnly={isSystemReadonlyInWeb}
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
											{isTauriShell ? (
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
											) : null}
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

								<div className="flex items-center justify-between gap-4">
									<div>
										<h3 className="text-base font-medium">
											{t("settings:developer.inspectorTimeoutTitle", {
												defaultValue: "Inspector Timeout (ms)",
											})}
										</h3>
										<p className="text-sm text-muted-foreground">
											{t("settings:developer.inspectorTimeoutDescription", {
												defaultValue:
													"Default timeout for tool/resource/prompt calls in the Inspector drawer.",
											})}
										</p>
									</div>
									<div className="flex items-center gap-2">
										<Input
											type="number"
											min={1000}
											max={300000}
											step={500}
											value={inspectorTimeoutInput}
											disabled={inspectorTimeoutMutation.isPending}
											onChange={(e) =>
												handleInspectorTimeoutChange(
													parseInt(e.target.value, 10) || 8000,
												)
											}
											onBlur={handleInspectorTimeoutBlur}
											className="w-32"
										/>
									</div>
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
					<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between md:gap-4">
						<div className="min-w-0 space-y-0.5 md:flex-1">
							<h3 className="text-base font-medium">
								{t("settings:market.installChromeExtension", {
									defaultValue: "Install Chrome Extension",
								})}
							</h3>
							<p className="text-sm text-muted-foreground">
								{t("settings:market.installChromeExtensionDescription", {
									defaultValue:
										"Install the MCPMate browser extension from Chrome Web Store to detect importable MCP server snippets and send them to MCPMate.",
								})}
							</p>
						</div>
						<div className="w-full shrink-0 md:ml-auto md:w-52">
							<Button asChild variant="outline" size="sm" className="w-full">
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
					</div>

					<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between md:gap-4">
						<div className="min-w-0 space-y-0.5 md:flex-1">
							<h3 className="text-base font-medium">
								{t("settings:market.installEdgeExtension", {
									defaultValue: "Install Edge Extension",
								})}
							</h3>
							<p className="text-sm text-muted-foreground">
								{t("settings:market.installEdgeExtensionDescription", {
									defaultValue:
										"Install the MCPMate browser extension from Microsoft Edge Add-ons to discover importable MCP server configurations on web pages.",
								})}
							</p>
						</div>
						<div className="w-full shrink-0 md:ml-auto md:w-52">
							<Button asChild variant="outline" size="sm" className="w-full">
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
