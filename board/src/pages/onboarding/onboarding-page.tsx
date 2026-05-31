import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ExternalLink,
  Github,
  Globe,
  MessagesSquare,
  Loader2,
  Rocket,
  Server,
  Terminal,
  Users,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useReducer, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Button } from "../../components/ui/button";
import { FeishuIcon } from "../../components/icons/feishu-icon";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Segment, type SegmentOption } from "../../components/ui/segment";
import { TooltipProvider } from "../../components/ui/tooltip";
import {
  clientsApi,
  extractImportStats,
  runtimeApi,
  serversApi,
} from "../../lib/api";
import {
  adminDiscoveryClientToUpdatePayload,
  clientTagsForDetected,
  enrichLocalServerCandidates,
  fetchAdminDiscoveryClientCatalog,
  fetchAdminDiscoveryServers,
  filterCatalogItemsByTag,
  type AdminDiscoveryClientCandidate,
  type AdminDiscoveryServerCandidate,
  type CatalogTagFilter,
} from "../../lib/admin-discovery";
import { readAdminDiscoveryPlatform } from "../../lib/desktop-platform";
import { websiteLangParam } from "../../lib/website-lang";
import {
  AdminDiscoveryPartialWarning,
  catalogTagLabel,
  OnboardingCatalogTabPanel,
  OnboardingCatalogToolbar,
  OnboardingClientCard,
  OnboardingEmptyCard,
  OnboardingScrollableGrid,
  OnboardingServerCard,
  POPULAR_CLIENT_TAG_FILTERS,
  POPULAR_SERVER_TAG_FILTERS,
} from "./onboarding-setup-ui";
import {
  buildOnboardingTabOptions,
  catalogFilterEmptyMessage,
  OnboardingDualTabStep,
  useLogoLoadFailures,
  useOnboardingDualTab,
} from "./onboarding-catalog-step";
import { applyManagedClientsForIdentifiers } from "../../lib/client-config-sync";
import { resolveActiveDefaultProfileId } from "../../lib/default-profile";
import { buildClientServersImportRequest } from "../../lib/server-import-payload";
import {
  groupSelectedDiscoveryServerConfigs,
  type OnboardingServerCandidateWithImport,
} from "../../lib/onboarding-server-selection";
import {
  onboardingApi,
  type OnboardingServerCandidate,
  type RuntimeEntry,
  type OnboardingStatusResp,
  type RuntimeCheckResp,
} from "../../lib/onboarding-api";
import {
  MCPMATE_DISCORD_COMMUNITY_HREF,
  MCPMATE_FEISHU_COMMUNITY_HREF,
  prefersFeishuCommunity,
} from "../../lib/mcpmate-community-urls";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { SUPPORTED_LANGUAGES } from "../../lib/i18n/index";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { cn } from "../../lib/utils";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import { isTauriEnvironmentSync } from "../../lib/platform";
import { showOperatorIntroOnce } from "../../lib/desktop-operator";
import type { ClientInfo } from "../../lib/types";

type WizardStep = "welcome" | "runtime" | "clients" | "servers" | "community";
type RuntimeKind = "node" | "bun" | "uv";
type WelcomeLanguage = "en" | "zh-cn" | "ja";

const ONBOARDING_CATALOG_LIMIT = 50;

const RUNTIME_NAME_PRIORITY: Record<RuntimeKind, string[]> = {
  node: ["node", "npx"],
  bun: ["bun", "bunx"],
  uv: ["uv", "uvx"],
};

type RuntimeState = {
  available: boolean;
  isManaged: boolean;
};

function computeRuntimeStateByKind(runtimes: RuntimeEntry[]): Map<RuntimeKind, RuntimeState> {
  const state = new Map<RuntimeKind, RuntimeState>();

  (Object.keys(RUNTIME_NAME_PRIORITY) as RuntimeKind[]).forEach((kind) => {
    const priorities = RUNTIME_NAME_PRIORITY[kind];
    const entries = priorities
      .map((runtimeName) =>
        runtimes.find((runtime) => runtime.name.toLowerCase() === runtimeName),
      )
      .filter((entry): entry is RuntimeEntry => Boolean(entry));

    const available = entries.some((entry) => entry.available);
    const isManaged = entries.some(
      (entry) => entry.available && entry.source === "mcpMate",
    );

    state.set(kind, { available, isManaged });
  });

  return state;
}

/** True when node, bun, and uv are all available (managed or system fallback) for onboarding gate. */
function areAllOnboardingRuntimeKindsAvailable(runtimes: RuntimeEntry[] | undefined): boolean {
  const runtimeStateByKind = computeRuntimeStateByKind(runtimes ?? []);
  return (Object.keys(RUNTIME_NAME_PRIORITY) as RuntimeKind[]).every((kind) => {
    const runtimeState = runtimeStateByKind.get(kind);
    return Boolean(runtimeState?.available);
  });
}

function normalizeWelcomeLanguage(language?: string): WelcomeLanguage {
  const lower = language?.toLowerCase() ?? "";
  if (lower.startsWith("zh")) return "zh-cn";
  if (lower.startsWith("ja")) return "ja";
  return "en";
}

function welcomeLanguageToI18nCode(language: WelcomeLanguage): string {
  switch (language) {
    case "zh-cn":
      return "zh-CN";
    case "ja":
      return "ja-JP";
    case "en":
      return "en";
  }
}

function getStepButtonClass(index: number, stepIndex: number): string {
  if (index < stepIndex) {
    return "text-emerald-600 dark:text-emerald-400";
  }

  if (index === stepIndex) {
    return "font-medium text-slate-900 dark:text-white";
  }

  return "text-slate-400";
}

function groupSelectedServerNamesByClient(
  candidates: OnboardingServerCandidateWithImport[],
  selectedKeys: Set<string>,
): Map<string, string[]> {
  const selectedByClient = new Map<string, string[]>();

  for (const candidate of candidates) {
    if (!selectedKeys.has(candidate.key)) continue;
    const clientIds = candidate.source_client_ids.filter(
      (id): id is string => typeof id === "string" && id.trim().length > 0,
    );
    if (clientIds.length === 0) continue;

    for (const sourceClientId of clientIds) {
      const names = selectedByClient.get(sourceClientId) ?? [];
      if (!names.includes(candidate.name)) {
        names.push(candidate.name);
      }
      selectedByClient.set(sourceClientId, names);
    }
  }

  return selectedByClient;
}

function serverCandidateDedupKey(candidate: OnboardingServerCandidateWithImport): string {
  const launchTarget =
    candidate.kind === "stdio"
      ? [candidate.command ?? "", ...candidate.args].join(" ")
      : candidate.url ?? "";
  return [
    candidate.name.trim().toLowerCase(),
    candidate.kind.trim().toLowerCase(),
    launchTarget.trim().toLowerCase(),
  ].join("::");
}

function mergeLocalAndAdminServerCandidates(
  localCandidates: OnboardingServerCandidate[],
  adminCandidates: AdminDiscoveryServerCandidate[],
): OnboardingServerCandidateWithImport[] {
  const seenKeys = new Set<string>();
  const seenCandidateShapes = new Set<string>();
  const merged: OnboardingServerCandidateWithImport[] = [];

  for (const candidate of localCandidates) {
    seenKeys.add(candidate.key);
    seenCandidateShapes.add(serverCandidateDedupKey(candidate));
    merged.push(candidate);
  }

  for (const candidate of adminCandidates) {
    const shapeKey = serverCandidateDedupKey(candidate);
    if (seenKeys.has(candidate.key) || seenCandidateShapes.has(shapeKey)) {
      continue;
    }
    seenKeys.add(candidate.key);
    seenCandidateShapes.add(shapeKey);
    merged.push(candidate);
  }

  return merged;
}

/** Detected clients that expose a local config path (eligible for onboarding server scan). */
function clientsWithScannableConfig(clients: ClientInfo[]): ClientInfo[] {
  return clients.filter(
    (client) =>
      client.detected && Boolean(client.config_path?.trim()),
  );
}

function RuntimeBrandIcon({ kind }: { kind: RuntimeKind }) {
  if (kind === "node") {
    return (
      <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-emerald-50 dark:bg-emerald-950/40">
        <Terminal className="h-7 w-7 text-emerald-600 dark:text-emerald-400" />
      </div>
    );
  }

  if (kind === "bun") {
    return (
      <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-amber-50 dark:bg-amber-950/40">
        <Rocket className="h-7 w-7 text-amber-600 dark:text-amber-300" />
      </div>
    );
  }

  return (
    <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-violet-50 dark:bg-violet-950/40">
      <Server className="h-7 w-7 text-violet-600 dark:text-violet-400" />
    </div>
  );
}

const STEP_ORDER: WizardStep[] = [
  "welcome",
  "runtime",
  "clients",
  "servers",
  "community",
];

const clientNameCollator = new Intl.Collator(undefined, {
  sensitivity: "base",
  numeric: true,
});

function clientSortName(client: ClientInfo): string {
  return (client.display_name || client.identifier).trim();
}

function compareClientsByName(left: ClientInfo, right: ClientInfo): number {
  const byName = clientNameCollator.compare(
    clientSortName(left),
    clientSortName(right),
  );
  if (byName !== 0) return byName;

  const byIdentifier = clientNameCollator.compare(
    left.identifier,
    right.identifier,
  );
  if (byIdentifier !== 0) return byIdentifier;

  return clientNameCollator.compare(left.config_path ?? "", right.config_path ?? "");
}

interface WizardState {
  step: WizardStep;
  selectedClients: Set<string>;
  selectedServers: Set<string>;
  welcomeConsent: boolean;
}

type WizardAction =
  | { type: "NEXT" }
  | { type: "PREV" }
  | { type: "TOGGLE_CLIENT"; identifier: string }
  | { type: "TOGGLE_SERVER"; name: string }
  | { type: "SET_STEP"; step: WizardStep }
  | { type: "SET_WELCOME_CONSENT"; consent: boolean };

function wizardReducer(state: WizardState, action: WizardAction): WizardState {
  switch (action.type) {
    case "NEXT": {
      const idx = STEP_ORDER.indexOf(state.step);
      const next = STEP_ORDER[Math.min(idx + 1, STEP_ORDER.length - 1)];
      return { ...state, step: next };
    }
    case "PREV": {
      const idx = STEP_ORDER.indexOf(state.step);
      const prev = STEP_ORDER[Math.max(idx - 1, 0)];
      return { ...state, step: prev };
    }
    case "TOGGLE_CLIENT": {
      const next = new Set(state.selectedClients);
      if (next.has(action.identifier)) {
        next.delete(action.identifier);
      } else {
        next.add(action.identifier);
      }
      return { ...state, selectedClients: next };
    }
    case "TOGGLE_SERVER": {
      const next = new Set(state.selectedServers);
      if (next.has(action.name)) {
        next.delete(action.name);
      } else {
        next.add(action.name);
      }
      return { ...state, selectedServers: next };
    }
    case "SET_STEP":
      return { ...state, step: action.step };
    case "SET_WELCOME_CONSENT":
      return { ...state, welcomeConsent: action.consent };
  }
}

function buildInitialWizardState(step: WizardStep): WizardState {
  return {
    step,
    selectedClients: new Set(),
    selectedServers: new Set(),
    welcomeConsent: false,
  };
}

// ── Main component ────────────────────────────────────────────────────────────

export function OnboardingPage() {
  const navigate = useNavigate();
  usePageTranslations("onboarding");
  const { t, i18n } = useTranslation(["onboarding", "translation"]);
  const qc = useQueryClient();
  const dashboardSettings = useAppStore((s) => s.dashboardSettings);
  const setDashboardSetting = useAppStore((s) => s.setDashboardSetting);
  const { activeTab: activeStep, setActiveTab: setActiveStep } = useUrlTab({
    paramName: "step",
    defaultTab: "welcome",
    validTabs: STEP_ORDER,
  });

  const [state, dispatch] = useReducer(
    wizardReducer,
    activeStep as WizardStep,
    buildInitialWizardState,
  );

  const [completing, setCompleting] = useState(false);
  const [serverCandidates, setServerCandidates] = useState<
    OnboardingServerCandidateWithImport[]
  >([]);
  const [adminClientCandidates, setAdminClientCandidates] = useState<Map<string, AdminDiscoveryClientCandidate>>(
    () => new Map(),
  );
  const [language, setLanguage] = useState<WelcomeLanguage>(() =>
    normalizeWelcomeLanguage(dashboardSettings.language),
  );
  const [welcomeConsentError, setWelcomeConsentError] = useState(false);
  const [installingKinds, setInstallingKinds] = useState<Set<RuntimeKind>>(new Set());

  useEffect(() => {
    if (activeStep !== state.step) {
      dispatch({ type: "SET_STEP", step: activeStep as WizardStep });
    }
  }, [activeStep, state.step]);

  const installRuntime = useCallback(
    async (kind: RuntimeKind) => {
      if (installingKinds.has(kind)) {
        return;
      }

      setInstallingKinds((prev) => new Set(prev).add(kind));

      try {
        await runtimeApi.install({ runtime_type: kind, verbose: true });
        await qc.refetchQueries({ queryKey: ["onboardingRuntimeCheck"], type: "active" });
        notifySuccess(
          t("runtime.install.successTitle", { defaultValue: "Install complete" }),
          t("runtime.install.successDescription", {
            defaultValue: "{{runtime}} installation finished.",
            runtime: kind.toUpperCase(),
          }),
        );
      } catch (error) {
        notifyError(
          t("runtime.install.errorTitle", { defaultValue: "Install failed" }),
          error instanceof Error ? error.message : String(error),
        );
      } finally {
        setInstallingKinds((prev) => {
          const next = new Set(prev);
          next.delete(kind);
          return next;
        });
      }
    },
    [installingKinds, qc, t],
  );

  const runtimeStepIndex = STEP_ORDER.indexOf("runtime");
  const currentStepIndex = STEP_ORDER.indexOf(state.step);
  const shouldFetchRuntimeCheck = currentStepIndex <= runtimeStepIndex;

  const { data: runtimeCheckForGate, isLoading: runtimeCheckForGateLoading } = useQuery<RuntimeCheckResp>({
    queryKey: ["onboardingRuntimeCheck"],
    queryFn: () => onboardingApi.runtimeCheck(),
    staleTime: 60_000,
    refetchInterval: state.step === "runtime" ? 3_000 : false,
    refetchIntervalInBackground: false,
    enabled: shouldFetchRuntimeCheck,
  });
  const runtimeCheckRuntimes = runtimeCheckForGate?.data?.runtimes;

  const goToStep = useCallback(
    (step: WizardStep) => {
      // Prevent navigating away from welcome step without consent
      if (state.step === "welcome" && !state.welcomeConsent && step !== "welcome") {
        setWelcomeConsentError(true);
        return;
      }
      // Gate: block forward navigation past runtime step when runtimes are unmet
      const runtimeIndex = STEP_ORDER.indexOf("runtime");
      const targetIndex = STEP_ORDER.indexOf(step);
      const currentIndex = STEP_ORDER.indexOf(state.step);
      if (currentIndex <= runtimeIndex && targetIndex > runtimeIndex) {
        if (runtimeCheckForGateLoading || !runtimeCheckRuntimes) {
          return;
        }
        if (!areAllOnboardingRuntimeKindsAvailable(runtimeCheckRuntimes)) {
          return;
        }
      }
      setActiveStep(step);
    },
    [setActiveStep, state.step, state.welcomeConsent, runtimeCheckForGateLoading, runtimeCheckRuntimes],
  );

  const goToNextStep = useCallback(() => {
    const idx = STEP_ORDER.indexOf(state.step);
    goToStep(STEP_ORDER[Math.min(idx + 1, STEP_ORDER.length - 1)]);
  }, [goToStep, state.step]);

  const goToPrevStep = useCallback(() => {
    const idx = STEP_ORDER.indexOf(state.step);
    goToStep(STEP_ORDER[Math.max(idx - 1, 0)]);
  }, [goToStep, state.step]);

  // Guard: if onboarding already completed, redirect to /
  const { data: statusResp } = useQuery<OnboardingStatusResp>({
    queryKey: ["onboardingStatus"],
    queryFn: () => onboardingApi.getStatus(),
    staleTime: 60_000,
  });

  useEffect(() => {
    if (statusResp?.data?.completed) {
      navigate("/", { replace: true });
    }
  }, [statusResp, navigate]);

  const handleComplete = useCallback(async () => {
    setCompleting(true);
    try {
      const clientsResp = await clientsApi.detect(false);
      const allListed = clientsResp?.client ?? [];
      const selectedServerNamesByClient = groupSelectedServerNamesByClient(
        serverCandidates,
        state.selectedServers,
      );
      const selectedDiscoveryServerConfigs = groupSelectedDiscoveryServerConfigs(
        serverCandidates,
        state.selectedServers,
      );
      const clientsToRegister = new Set([
        ...state.selectedClients,
        ...selectedServerNamesByClient.keys(),
      ]);
      const selectedClientList = allListed.filter((client) =>
        clientsToRegister.has(client.identifier),
      );
      const selectedAdminClientIds = [...clientsToRegister].filter((identifier) =>
        adminClientCandidates.has(identifier),
      );
      await Promise.all(
        selectedClientList.map(async (client) => {
          await clientsApi.update({
            identifier: client.identifier,
            display_name: client.display_name || client.identifier,
            config_file_state: "with_config_file",
            config_path: client.config_path,
            description: client.description ?? client.template?.description ?? undefined,
            homepage_url: client.homepage_url ?? client.template?.homepage_url ?? undefined,
            docs_url: client.docs_url ?? client.template?.docs_url ?? undefined,
            support_url: client.support_url ?? client.template?.support_url ?? undefined,
            logo_url: client.logo_url ?? undefined,
            clear_config_file_parse: true,
          });
          await clientsApi.approveRecord({ identifier: client.identifier });
        }),
      );
      if (selectedAdminClientIds.length > 0) {
        await Promise.all(
          selectedAdminClientIds.map(async (identifier) => {
            const candidate = adminClientCandidates.get(identifier);
            if (!candidate) {
              throw new Error(
                t("clients.presetMissing", {
                  defaultValue: "Client preset '{{identifier}}' was not found.",
                  identifier,
                }),
              );
            }
            await clientsApi.update(adminDiscoveryClientToUpdatePayload(candidate));
            await clientsApi.approveRecord({ identifier });
          }),
        );
      }

      const hasDiscoveryServerConfigs = Object.keys(selectedDiscoveryServerConfigs).length > 0;
      if (selectedServerNamesByClient.size > 0 || hasDiscoveryServerConfigs) {
        // In onboarding context, always link imported servers to the default-anchor profile
        // regardless of the autoAddServerToDefaultProfile setting (which defaults to false for new users).
        // The default anchor is seeded at app init, so it must already exist before onboarding runs.
        const targetProfileId = await resolveActiveDefaultProfileId();

        if (hasDiscoveryServerConfigs) {
          const response = await serversApi.importServers({
            mcpServers: selectedDiscoveryServerConfigs,
            target_profile_id: targetProfileId,
          });
          if (
            response &&
            typeof response === "object" &&
            "success" in response &&
            (response as { success?: boolean }).success === false
          ) {
            const err = (response as { error?: unknown }).error;
            throw new Error(err ? stringifyError(err) : "Server import failed");
          }
          const stats = extractImportStats(response);
          if (stats.failedCount > 0) {
            throw new Error(
              stats.errorDetails
                ? JSON.stringify(stats.errorDetails)
                : "Server import failed",
            );
          }
        }

        for (const [clientIdentifier, selectedServerNames] of selectedServerNamesByClient) {
          const response = await serversApi.importServers(
            buildClientServersImportRequest({
              clientIdentifier,
              selectedServerNames,
              targetProfileId,
            }),
          );
          if (
            response &&
            typeof response === "object" &&
            "success" in response &&
            (response as { success?: boolean }).success === false
          ) {
            const err = (response as { error?: unknown }).error;
            throw new Error(err ? stringifyError(err) : "Server import failed");
          }
          const stats = extractImportStats(response);
          if (stats.failedCount > 0) {
            throw new Error(
              stats.errorDetails
                ? JSON.stringify(stats.errorDetails)
                : "Server import failed",
            );
          }
        }
        await qc.invalidateQueries({ queryKey: ["servers"] });
        if (targetProfileId) {
          await qc.invalidateQueries({ queryKey: ["configSuits"] });
        }
      }

      try {
        const postRegister = await clientsApi.list(false, {
          persistDetected: false,
          includeDetected: true,
        });
        await applyManagedClientsForIdentifiers({
          clients: postRegister?.client ?? [],
          identifiers: clientsToRegister,
          dashboardSettings,
        });
      } catch (applyError) {
        notifyError(
          t("complete.applyClientsErrorTitle", {
            defaultValue: "Failed to apply MCP client configurations",
          }),
          applyError instanceof Error ? applyError.message : String(applyError),
        );
        setCompleting(false);
        return;
      }

      await qc.invalidateQueries({ queryKey: ["clients"] });

      await onboardingApi.complete(true);
      await qc.invalidateQueries({ queryKey: ["onboardingStatus"] });
      if (isTauriEnvironmentSync()) {
        try {
          await showOperatorIntroOnce();
        } catch (panelError) {
          notifyError(
            t("complete.operatorPanelErrorTitle", {
              defaultValue: "Could not open tray operator panel",
            }),
            panelError instanceof Error ? panelError.message : String(panelError),
          );
        }
      }
      navigate("/", { replace: true });
    } catch (error) {
      notifyError(
        t("servers.importErrorTitle", { defaultValue: "Server import failed" }),
        error instanceof Error ? error.message : String(error),
      );
      setCompleting(false);
    }
  }, [
    adminClientCandidates,
    dashboardSettings,
    navigate,
    qc,
    serverCandidates,
    state.selectedClients,
    state.selectedServers,
    t,
  ]);

  const handleLanguageChange = useCallback(
    (newLanguage: string) => {
      const storeLanguage = normalizeWelcomeLanguage(newLanguage);
      setLanguage(storeLanguage);
      const i18nCode =
        SUPPORTED_LANGUAGES.find((entry) => entry.store === storeLanguage)?.i18n ??
        storeLanguage;
      if (dashboardSettings.language !== storeLanguage) {
        setDashboardSetting("language", storeLanguage);
      }
      void i18n.changeLanguage(i18nCode);
    },
    [dashboardSettings.language, i18n, setDashboardSetting],
  );

  const handleGetStarted = useCallback(() => {
    setWelcomeConsentError(false);
    goToNextStep();
  }, [goToNextStep]);

  const stepIdx = STEP_ORDER.indexOf(state.step);
  const isFirst = stepIdx === 0;
  const isLast = stepIdx === STEP_ORDER.length - 1;

  const runtimeUnmet = useMemo(() => {
    if (state.step !== "runtime") {
      return false;
    }
    return !areAllOnboardingRuntimeKindsAvailable(runtimeCheckRuntimes);
  }, [runtimeCheckRuntimes, state.step]);

  const termsLabel = t("layout.terms", { defaultValue: "Terms" });
  const privacyLabel = t("layout.privacy", { defaultValue: "Privacy" });
  const langParam = websiteLangParam(i18n.language);
  const termsHref = `https://mcp.umate.ai/terms?lang=${langParam}`;
  const privacyHref = `https://mcp.umate.ai/privacy?lang=${langParam}`;

  const stepLabels = useMemo(
    () => [
      {
        key: "welcome" as WizardStep,
        label: t("steps.welcome", { defaultValue: "Welcome" }),
      },
      {
        key: "runtime" as WizardStep,
        label: t("steps.runtime", { defaultValue: "Runtime" }),
      },
      {
        key: "clients" as WizardStep,
        label: t("steps.clients", { defaultValue: "Clients" }),
      },
      {
        key: "servers" as WizardStep,
        label: t("steps.servers", { defaultValue: "Servers" }),
      },
      {
        key: "community" as WizardStep,
        label: t("steps.community", { defaultValue: "Community" }),
      },
    ],
    [t, i18n.language],
  );

  return (
    <div className="flex h-screen flex-col bg-gradient-to-b from-slate-50 to-white dark:from-slate-950 dark:to-slate-900">
      {/* Top bar with step indicator + language switcher */}
      <header className="shrink-0 border-b border-slate-200 px-6 py-4 dark:border-slate-800">
        <div className="mx-auto flex max-w-3xl items-center justify-between">
          <div className="flex items-center gap-2">
            <img
              src="/logo.svg"
              alt="MCPMate"
              className="h-6 w-6 object-contain dark:invert dark:brightness-0"
            />
            <span className="text-lg font-semibold tracking-tight">
              MCPMate
            </span>
          </div>
          <div className="flex items-center gap-3">
            <nav className="flex items-center gap-2 text-sm">
              {stepLabels.map((s, i) => (
                <button
                  key={s.key}
                  type="button"
                  onClick={() => goToStep(s.key)}
                  aria-current={i === stepIdx ? "step" : undefined}
                  className={`flex items-center gap-1 ${getStepButtonClass(i, stepIdx)} transition-colors hover:text-slate-700 dark:hover:text-slate-200`}
                >
                  {i < stepIdx ? (
                    <Check className="h-3.5 w-3.5" />
                  ) : (
                    <span className="flex h-5 w-5 items-center justify-center rounded-full border text-xs">
                      {i + 1}
                    </span>
                  )}
                  <span className="hidden sm:inline">{s.label}</span>
                  {i < stepLabels.length - 1 && (
                    <span className="mx-1 text-slate-300">/</span>
                  )}
                </button>
              ))}
            </nav>
          </div>
        </div>
      </header>

      {/* Step content — scrollable */}
      <main className="flex-1 overflow-y-auto px-6 py-10">
        <div className="mx-auto max-w-3xl">
          {state.step === "welcome" && (
            <WelcomeStep
              language={language}
              onLanguageChange={handleLanguageChange}
              consent={state.welcomeConsent}
              onConsentChange={(consent) => {
                dispatch({ type: "SET_WELCOME_CONSENT", consent });
                if (consent) setWelcomeConsentError(false);
              }}
              consentError={welcomeConsentError}
              onConsentErrorChange={setWelcomeConsentError}
              onGetStarted={handleGetStarted}
            />
          )}
          {state.step === "runtime" && (
            <RuntimeStep
              runtimes={runtimeCheckRuntimes}
              isLoading={runtimeCheckForGateLoading}
              installingKinds={installingKinds}
              onInstall={installRuntime}
            />
          )}
          {state.step === "clients" && (
            <ClientsStep
              selectedClients={state.selectedClients}
              onToggle={(id) =>
                dispatch({ type: "TOGGLE_CLIENT", identifier: id })
              }
              onAdminCandidatesChange={setAdminClientCandidates}
            />
          )}
          {state.step === "servers" && (
            <ServersStep
              selectedServers={state.selectedServers}
              onCandidatesChange={setServerCandidates}
              onToggle={(name) => dispatch({ type: "TOGGLE_SERVER", name })}
            />
          )}
          {state.step === "community" && <CommunityStep />}
        </div>
      </main>

      {/* Bottom navigation or welcome footer — fixed */}
      <footer className="shrink-0 border-t border-slate-200 px-6 py-4 dark:border-slate-800">
        <div className="mx-auto flex max-w-3xl items-center justify-between">
          {state.step === "welcome" ? (
            <>
              <div className="flex items-center gap-4 flex-wrap text-[11px] text-slate-500">
                <a
                  className="hover:underline"
                  href="https://mcp.umate.ai"
                  target="_blank"
                  rel="noreferrer"
                >
                  {t("layout.copyright", {
                    defaultValue: "© 2026 MCPMate",
                  })}
                </a>
              </div>
              <div className="flex items-center gap-3 text-[11px] text-slate-500">
                <a
                  className="hover:underline"
                  href={termsHref}
                  target="_blank"
                  rel="noreferrer"
                >
                  {termsLabel}
                </a>
                <span className="text-slate-300">•</span>
                <a
                  className="hover:underline"
                  href={privacyHref}
                  target="_blank"
                  rel="noreferrer"
                >
                  {privacyLabel}
                </a>
              </div>
            </>
          ) : (
            <>
              {isFirst ? (
                <div className="h-9 w-[88px]" aria-hidden="true" />
              ) : (
                <Button variant="ghost" onClick={goToPrevStep}>
                  <ArrowLeft className="mr-1.5 h-4 w-4" />
                  {t("nav.back", { defaultValue: "Back" })}
                </Button>
              )}
              <div className="flex items-center gap-2">
                {isLast ? (
                  <Button onClick={handleComplete} disabled={completing}>
                    {completing
                      ? t("nav.finishing", {
                        defaultValue: "Finishing...",
                      })
                      : t("nav.finish", {
                        defaultValue: "Finish Setup",
                      })}
                    <Check className="ml-1.5 h-4 w-4" />
                  </Button>
                ) : (
                  <Button onClick={goToNextStep} disabled={runtimeUnmet}>
                    {runtimeUnmet
                      ? t("runtime.install.required", {
                        defaultValue: "Install runtimes to continue",
                      })
                      : t("nav.next", { defaultValue: "Next" })}
                    {!runtimeUnmet && <ArrowRight className="ml-1.5 h-4 w-4" />}
                  </Button>
                )}
              </div>
            </>
          )}
        </div>
      </footer>
    </div>
  );
}

// ── Welcome step ──────────────────────────────────────────────────────────────

/** Endonyms for a11y + tooltips (always shown in that language, not UI locale). */
const WELCOME_LANGUAGE_SEGMENT_OPTIONS: SegmentOption[] = [
  { value: "en", label: "🇺🇸 English", ariaLabel: "English", tooltip: "English" },
  { value: "zh-cn", label: "🇨🇳 简体中文", ariaLabel: "简体中文", tooltip: "简体中文" },
  { value: "ja", label: "🇯🇵 日本語", ariaLabel: "日本語", tooltip: "日本語" },
];

function WelcomeStep({
  language,
  onLanguageChange,
  consent,
  onConsentChange,
  consentError,
  onConsentErrorChange,
  onGetStarted,
}: {
  language: WelcomeLanguage;
  onLanguageChange: (language: string) => void;
  consent: boolean;
  onConsentChange: (consent: boolean) => void;
  consentError: boolean;
  onConsentErrorChange: (visible: boolean) => void;
  onGetStarted: () => void;
}) {
  const { t, i18n } = useTranslation("onboarding");
  const consentId = useId();

  useEffect(() => {
    const i18nLang = welcomeLanguageToI18nCode(language);
    if (i18n.language !== i18nLang) {
      i18n.changeLanguage(i18nLang);
    }
  }, [language, i18n]);

  const handleGetStarted = () => {
    if (!consent) {
      onConsentErrorChange(true);
      return;
    }
    onConsentErrorChange(false);
    onGetStarted();
  };

  const handleConsentChange = (checked: boolean) => {
    onConsentChange(checked);
    if (checked) {
      onConsentErrorChange(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <Rocket className="mx-auto mb-6 h-16 w-16 text-emerald-500" />
      <h1 className="mb-3 text-3xl font-bold tracking-tight">
        {t("welcome.title", {
          defaultValue: "Welcome to MCPMate",
        })}
      </h1>

      {/* Language selector */}
      <div className="mb-8 w-full max-w-xs">
        <Segment
          showDots={false}
          className={cn(
            "[&_[role=tablist]]:min-h-11",
            "[&_[role=tab][data-state=active]]:bg-emerald-100 [&_[role=tab][data-state=active]]:text-emerald-900 [&_[role=tab][data-state=active]]:shadow-sm",
            "dark:[&_[role=tab][data-state=active]]:bg-emerald-900/50 dark:[&_[role=tab][data-state=active]]:text-emerald-50",
          )}
          options={WELCOME_LANGUAGE_SEGMENT_OPTIONS}
          value={language}
          onValueChange={onLanguageChange}
        />
      </div>

      <p className="mx-auto mb-8 max-w-lg text-lg text-slate-600 dark:text-slate-400">
        {t("welcome.description", {
          defaultValue:
            "Let's get you set up in a few quick steps. We'll check your environment, detect clients, and add some useful servers.",
        })}
      </p>

      {/* Consent checkbox */}
      <div className="mb-4 flex items-start gap-2">
        <input
          type="checkbox"
          id={consentId}
          checked={consent}
          onChange={(e) => handleConsentChange(e.target.checked)}
          className="mt-1 h-4 w-4 rounded border-slate-300 text-emerald-600 focus:ring-emerald-500"
        />
        <label htmlFor={consentId} className="text-sm text-slate-600 dark:text-slate-400">
          {t("welcome.consent", {
            defaultValue: "Allow scanning local runtimes and MCP server configurations",
          })}
        </label>
      </div>

      {/* Error alert */}
      {consentError && (
        <Alert variant="destructive" className="mb-4 max-w-md">
          <AlertDescription>
            {t("welcome.consentRequired", {
              defaultValue: "Please accept the scanning authorization to continue",
            })}
          </AlertDescription>
        </Alert>
      )}

      {/* Get Started button */}
      <Button size="lg" onClick={handleGetStarted}>
        {t("welcome.getStarted", { defaultValue: "Get Started" })}
        <ArrowRight className="ml-2 h-4 w-4" />
      </Button>
    </div>
  );
}

// ── Runtime step ──────────────────────────────────────────────────────────────

function RuntimeStep({
  runtimes: runtimeQueryRuntimes,
  isLoading: runtimeQueryLoading,
  installingKinds,
  onInstall,
}: {
  runtimes: RuntimeEntry[] | undefined;
  isLoading: boolean;
  installingKinds: Set<RuntimeKind>;
  onInstall: (kind: RuntimeKind) => Promise<void>;
}) {
  const { t, i18n } = useTranslation("onboarding");
  const [searchParams] = useSearchParams();

  const preview = searchParams.get("runtimePreview");
  const previewMode: "real" | "partial" | "none" =
    preview === "partial" || preview === "none" ? preview : "real";

  const baseRuntimes = runtimeQueryRuntimes ?? [];
  const runtimeSeeds: RuntimeEntry[] = useMemo(
    () => [
      { name: "node", available: true },
      { name: "bun", available: true },
      { name: "uv", available: true },
    ],
    [],
  );
  const sourceRuntimes = baseRuntimes.length > 0 ? baseRuntimes : runtimeSeeds;

  const runtimes = useMemo(() => {
    if (previewMode === "real") return sourceRuntimes;
    if (previewMode === "partial") {
      return sourceRuntimes.map((runtime) => {
        const name = runtime.name.toLowerCase();
        const missing = name.includes("python") || name === "uv" || name === "uvx";
        if (!missing) return runtime;
        return { ...runtime, available: false, version: undefined, path: undefined };
      });
    }
    return sourceRuntimes.map((runtime) => ({
      ...runtime,
      available: false,
      version: undefined,
      path: undefined,
    }));
  }, [previewMode, sourceRuntimes]);

  const showLoading = previewMode === "real" && runtimeQueryLoading;

  const getRuntimeKindFromEntry = useCallback(
    (name: string): RuntimeKind | null => {
      const n = name.toLowerCase();
      if (n === "node" || n === "npx") return "node";
      if (n === "bun" || n === "bunx") return "bun";
      if (n === "uv" || n === "uvx") return "uv";
      return null;
    },
    [],
  );

  const runtimeCardMeta = useMemo(
    () => [
      {
        key: "node" as RuntimeKind,
        title: t("runtime.install.nodeTitle", { defaultValue: "Node.js" }),
        website: "https://nodejs.org",
        description: t("runtime.install.nodeDescription", {
          defaultValue: "JavaScript runtime for npm-based MCP servers.",
        }),
      },
      {
        key: "bun" as RuntimeKind,
        title: t("runtime.install.bunTitle", { defaultValue: "Bun" }),
        website: "https://bun.sh",
        description: t("runtime.install.bunDescription", {
          defaultValue: "Fast JavaScript runtime and package manager.",
        }),
      },
      {
        key: "uv" as RuntimeKind,
        title: t("runtime.install.uvTitle", { defaultValue: "uv" }),
        website: "https://docs.astral.sh/uv/",
        description: t("runtime.install.uvDescription", {
          defaultValue: "Python runtime manager for Python-based MCP servers.",
        }),
      },
    ],
    [t, i18n.language],
  );

  const availableKinds = useMemo(() => {
    const set = new Set<RuntimeKind>();
    runtimes.forEach((runtime) => {
      if (!runtime.available) return;
      const kind = getRuntimeKindFromEntry(runtime.name);
      if (kind) set.add(kind);
    });
    return set;
  }, [getRuntimeKindFromEntry, runtimes]);

  const runtimeStateByKind = useMemo(
    () => computeRuntimeStateByKind(runtimes),
    [runtimes],
  );

  return (
    <div className="mx-auto flex min-h-[520px] w-full flex-col justify-center">
      <div className="mb-8 text-center">
        <Terminal className="mx-auto mb-3 h-10 w-10 text-cyan-500" />
        <h2 className="text-2xl font-bold tracking-tight">
          {t("runtime.title", {
            defaultValue: "Check Your Environment",
          })}
        </h2>
        <p className="mt-2 text-slate-600 dark:text-slate-400">
          {t("runtime.description", {
            defaultValue:
              "We'll check which runtimes on your system are usable by MCPMate.",
          })}
        </p>
      </div>

      {showLoading ? (
        <div className="flex justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-emerald-500" />
        </div>
      ) : (
        <>
          <div className="mb-6 grid gap-4 md:grid-cols-3">
            {runtimeCardMeta.map((card) => {
              const runtimeState = runtimeStateByKind.get(card.key) ?? {
                available: availableKinds.has(card.key),
                isManaged: false,
              };
              const { available, isManaged } = runtimeState;
              return (
                <div
                  key={card.key}
                  className="flex min-h-[190px] flex-col rounded-lg border-2 border-slate-200 bg-white p-4 text-left transition-all hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:hover:border-slate-600"
                >
                  <div className="mb-3 flex items-start justify-between gap-3">
                    <div className="flex items-center gap-3">
                      <RuntimeBrandIcon kind={card.key} />
                      <span className="text-base font-semibold">{card.title}</span>
                    </div>
                    <a
                      href={card.website}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-slate-400 transition-colors hover:bg-slate-100 hover:text-slate-600 dark:hover:bg-slate-800 dark:hover:text-slate-300"
                      aria-label={t("runtime.install.openOfficialSite", {
                        defaultValue: "Open official website",
                      })}
                      title={t("runtime.install.openOfficialSite", {
                        defaultValue: "Open official website",
                      })}
                    >
                      <Globe className="h-4 w-4" />
                    </a>
                  </div>
                  <p className="mb-4 flex-1 text-sm leading-6 text-slate-500">
                    {card.description}
                  </p>
                  <Button
                    size="sm"
                    variant={available && isManaged ? "secondary" : "default"}
                    className="w-fit"
                    disabled={(available && isManaged) || installingKinds.has(card.key)}
                    onClick={() => void onInstall(card.key)}
                  >
                    {(() => {
                      if (installingKinds.has(card.key)) {
                        return (
                          <>
                            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                            {t("runtime.install.installing", {
                              defaultValue: "Installing...",
                            })}
                          </>
                        );
                      }

                      if (available && isManaged) {
                        return (
                          <>
                            <Check className="mr-1 h-3.5 w-3.5" />
                            {t("runtime.install.ready", { defaultValue: "Ready" })}
                          </>
                        );
                      }
                      if (available && !isManaged) {
                        return t("runtime.install.replaceWithManaged", {
                          defaultValue: "Replace with managed",
                        });
                      }
                      return t("runtime.install.clickToInstall", {
                        defaultValue: "Click to install",
                      });
                    })()}
                  </Button>
                </div>
              );
            })}
          </div>

        </>
      )}
    </div>
  );
}

// ── Clients step ─────────────────────────────────────────────────────────────

type ClientsSetupTab = "detected" | "popular";

const CLIENTS_PRIMARY_TAB: ClientsSetupTab = "detected";
const CLIENTS_POPULAR_TAB: ClientsSetupTab = "popular";

function ClientsStep({
  selectedClients,
  onToggle,
  onAdminCandidatesChange,
}: {
  selectedClients: Set<string>;
  onToggle: (identifier: string) => void;
  onAdminCandidatesChange: (candidates: Map<string, AdminDiscoveryClientCandidate>) => void;
}) {
  const { t, i18n } = useTranslation("onboarding");
  const { markFailed: markLogoLoadFailed, hasFailed: logoLoadFailed } = useLogoLoadFailures();
  const { data, isLoading, isError, refetch, error, isFetching } = useQuery({
    queryKey: ["clients", "onboarding"],
    queryFn: () => clientsApi.detect(true),
    staleTime: 30_000,
  });

  const detectedClients = useMemo(
    () =>
      (data?.client ?? [])
        .filter((client) => client.detected)
        .sort(compareClientsByName),
    [data?.client],
  );
  const adminDiscoveryPlatformQuery = useQuery({
    queryKey: ["adminDiscoveryPlatform", "onboarding"],
    queryFn: () => readAdminDiscoveryPlatform(),
    staleTime: Infinity,
    retry: false,
  });
  const adminDiscoveryPlatform = adminDiscoveryPlatformQuery.data;
  const popularCatalogQuery = useQuery({
    queryKey: ["adminDiscoveryClients", "onboarding", adminDiscoveryPlatform ?? "web", i18n.language],
    queryFn: () =>
      fetchAdminDiscoveryClientCatalog({
        surface: "onboarding",
        limit: ONBOARDING_CATALOG_LIMIT,
        platform: adminDiscoveryPlatform,
        locale: i18n.language,
      }),
    enabled: adminDiscoveryPlatformQuery.isSuccess,
    staleTime: 60_000,
    retry: false,
  });
  const popularClients = useMemo(
    () => popularCatalogQuery.data?.clients ?? [],
    [popularCatalogQuery.data],
  );
  const popularCatalogDiagnostics = popularCatalogQuery.data?.diagnostics ?? [];
  const popularCatalogById = useMemo(
    () => new Map(popularClients.map((client) => [client.identifier, client])),
    [popularClients],
  );
  const detectedIdentifiers = useMemo(
    () => new Set(detectedClients.map((client) => client.identifier)),
    [detectedClients],
  );
  const {
    activeTab,
    setActiveTab,
    primaryTagFilter: detectedTagFilter,
    setPrimaryTagFilter: setDetectedTagFilter,
    popularTagFilter,
    setPopularTagFilter,
    isPrimaryTab,
  } = useOnboardingDualTab(
    CLIENTS_PRIMARY_TAB,
    CLIENTS_POPULAR_TAB,
    detectedClients.length === 0,
  );
  const detectedClientsWithTags = useMemo(
    () =>
      detectedClients.map((client) => ({
        ...client,
        tags: clientTagsForDetected(
          client.category,
          popularCatalogById.get(client.identifier)?.tags,
        ),
      })),
    [detectedClients, popularCatalogById],
  );
  const filteredDetectedClients = useMemo(
    () => filterCatalogItemsByTag(detectedClientsWithTags, detectedTagFilter),
    [detectedClientsWithTags, detectedTagFilter],
  );
  const filteredPopularClients = useMemo(
    () => filterCatalogItemsByTag(popularClients, popularTagFilter),
    [popularClients, popularTagFilter],
  );

  const clientTagLabel = useCallback(
    (tag: CatalogTagFilter) => catalogTagLabel(t, "clients", tag),
    [t, i18n.language],
  );

  useEffect(() => {
    onAdminCandidatesChange(
      new Map(popularClients.map((candidate) => [candidate.identifier, candidate])),
    );
  }, [onAdminCandidatesChange, popularClients]);

  const clientTabOptions = useMemo(
    () =>
      buildOnboardingTabOptions(
        {
          value: CLIENTS_PRIMARY_TAB,
          label: t("clients.tabs.detected", { defaultValue: "From your device" }),
          count: detectedClients.length,
        },
        {
          value: CLIENTS_POPULAR_TAB,
          label: t("clients.tabs.popular", { defaultValue: "Popular Clients" }),
          count: popularClients.length,
        },
      ),
    [detectedClients.length, popularClients.length, t, i18n.language],
  );

  const isDetectLoading = isLoading;
  const isPopularCatalogLoading =
    adminDiscoveryPlatformQuery.isLoading ||
    (popularCatalogQuery.isLoading && popularClients.length === 0);

  const recommendationErrorMessage = t("clients.recommendationError", {
    defaultValue: "MCPMate could not load the popular client catalog.",
  });
  const emptyFilteredMessage = t("clients.emptyFiltered", {
    defaultValue: "No clients match this category.",
  });

  return (
    <OnboardingDualTabStep
      icon={<Users className="mx-auto mb-3 h-10 w-10 text-blue-500" />}
      title={t("clients.title", { defaultValue: "Set Up MCP Clients" })}
      description={t("clients.description", {
        defaultValue:
          "Choose clients to manage now or pre-select popular ones to set up after installation.",
      })}
      tabOptions={clientTabOptions}
      activeTab={activeTab}
      onTabChange={(value) => setActiveTab(value as ClientsSetupTab)}
      isPrimaryTab={isPrimaryTab}
      primaryLoading={isDetectLoading}
      popularLoading={isPopularCatalogLoading}
      primaryPanel={
        <OnboardingCatalogTabPanel
          footnote={t("clients.detectedNotice", {
            defaultValue:
              "We scan your device automatically. Some clients may not appear due to version or compatibility—you can add them manually later.",
          })}
        >
          <OnboardingCatalogToolbar
            tagValue={detectedTagFilter}
            onTagChange={setDetectedTagFilter}
            tagOptions={POPULAR_CLIENT_TAG_FILTERS}
            tagLabel={clientTagLabel}
            refreshAriaLabel={t("clients.rescan", { defaultValue: "Rescan" })}
            onRefresh={() => void refetch()}
            refreshDisabled={isFetching}
          />
          {isError ? (
            <OnboardingEmptyCard
              message={t("clients.error", {
                defaultValue: "Failed to detect MCP clients. Please retry.",
              })}
            />
          ) : detectedClients.length === 0 ? (
            <OnboardingEmptyCard
              message={t("clients.empty", {
                defaultValue: "No MCP clients detected on this device yet.",
              })}
              actionLabel={t("clients.emptyAction", {
                defaultValue: "Browse popular clients",
              })}
              onAction={() => setActiveTab(CLIENTS_POPULAR_TAB)}
            />
          ) : filteredDetectedClients.length === 0 ? (
            <OnboardingEmptyCard message={emptyFilteredMessage} />
          ) : (
            <OnboardingScrollableGrid>
              {filteredDetectedClients.map((client) => {
                const isSelected = selectedClients.has(client.identifier);
                const catalogEntry = popularCatalogById.get(client.identifier);
                const displayName =
                  catalogEntry?.displayName || client.display_name || client.identifier;
                const localizedDescription =
                  catalogEntry?.description || client.description || undefined;
                const logoUrl = client.logo_url ?? catalogEntry?.logoUrl ?? undefined;
                const showLogo = Boolean(logoUrl) && !logoLoadFailed(client.identifier);
                return (
                  <OnboardingClientCard
                    key={client.identifier}
                    name={displayName}
                    description={localizedDescription}
                    logoUrl={logoUrl ?? undefined}
                    showLogo={showLogo}
                    isSelected={isSelected}
                    onToggle={() => onToggle(client.identifier)}
                    badgeLabel={t("clients.badges.detected", { defaultValue: "Detected" })}
                    onLogoError={() => markLogoLoadFailed(client.identifier)}
                  />
                );
              })}
            </OnboardingScrollableGrid>
          )}
          {isError ? (
            <div className="text-center">
              <p className="text-xs text-slate-400">
                {error instanceof Error ? error.message : ""}
              </p>
              <Button
                variant="outline"
                size="sm"
                className="mt-4"
                onClick={() => void refetch()}
              >
                {t("clients.retry", { defaultValue: "Retry detection" })}
              </Button>
            </div>
          ) : null}
        </OnboardingCatalogTabPanel>
      }
      popularPanel={
        <OnboardingCatalogTabPanel
          footnote={t("clients.recommendationNotice", {
            defaultValue:
              "Pre-selected clients stay pending until installed. After installation, return here to rescan or finish binding from the Clients page.",
          })}
        >
          {popularCatalogQuery.isError ? (
            <OnboardingEmptyCard message={recommendationErrorMessage} />
          ) : (
            <>
              {popularCatalogDiagnostics.length > 0 ? (
                <AdminDiscoveryPartialWarning
                  message={t("clients.recommendationPartialWarning", {
                    count: popularCatalogDiagnostics.length,
                    defaultValue:
                      "Some client presets were skipped because their discovery data is invalid.",
                  })}
                />
              ) : null}
              <OnboardingCatalogToolbar
                tagValue={popularTagFilter}
                onTagChange={setPopularTagFilter}
                tagOptions={POPULAR_CLIENT_TAG_FILTERS}
                tagLabel={clientTagLabel}
                refreshAriaLabel={t("clients.refresh", { defaultValue: "Refresh" })}
                onRefresh={() => void popularCatalogQuery.refetch()}
                refreshDisabled={popularCatalogQuery.isFetching}
              />
              {filteredPopularClients.length === 0 ? (
                <OnboardingEmptyCard
                  message={catalogFilterEmptyMessage(
                    popularClients.length,
                    recommendationErrorMessage,
                    emptyFilteredMessage,
                  )}
                />
              ) : (
                <TooltipProvider delayDuration={200}>
                  <OnboardingScrollableGrid>
                    {filteredPopularClients.map((client) => {
                      const isSelected = selectedClients.has(client.identifier);
                      const isDetected = detectedIdentifiers.has(client.identifier);
                      const showLogo =
                        Boolean(client.logoUrl) && !logoLoadFailed(client.identifier);
                      const displayName = client.displayName || client.identifier;
                      return (
                        <OnboardingClientCard
                          key={client.identifier}
                          name={displayName}
                          description={client.description || undefined}
                          logoUrl={client.logoUrl || undefined}
                          showLogo={showLogo}
                          isSelected={isSelected}
                          isDetected={isDetected}
                          homepageUrl={client.homepageUrl || client.docsUrl || undefined}
                          onToggle={() => onToggle(client.identifier)}
                          onInstall={() => {
                            const target = client.homepageUrl || client.docsUrl;
                            if (target) window.open(target, "_blank", "noopener,noreferrer");
                          }}
                          badgeLabel={
                            isDetected
                              ? t("clients.badges.detected", { defaultValue: "Detected" })
                              : t("clients.badges.installable", {
                                  defaultValue: "Installable",
                                })
                          }
                          badgeVariant={isDetected ? "success" : "warning"}
                          installAriaLabel={t("clients.installAria", {
                            defaultValue: "Open {{name}} official site to install",
                            name: displayName,
                          })}
                          installTooltip={t("clients.installTooltip", {
                            defaultValue: "Open the official site to download and install",
                          })}
                          selectedAriaLabel={t("clients.selectedAria", {
                            defaultValue: "{{name}} pre-selected for setup",
                            name: displayName,
                          })}
                          unselectedAriaLabel={t("clients.unselectedAria", {
                            defaultValue: "{{name}} not pre-selected",
                            name: displayName,
                          })}
                          onLogoError={() => markLogoLoadFailed(client.identifier)}
                        />
                      );
                    })}
                  </OnboardingScrollableGrid>
                </TooltipProvider>
              )}
            </>
          )}
        </OnboardingCatalogTabPanel>
      }
    />
  );
}

type ServersSetupTab = "local" | "popular";

const SERVERS_PRIMARY_TAB: ServersSetupTab = "local";
const SERVERS_POPULAR_TAB: ServersSetupTab = "popular";

function ServersStep({
  selectedServers,
  onCandidatesChange,
  onToggle,
}: {
  selectedServers: Set<string>;
  onCandidatesChange: (candidates: OnboardingServerCandidateWithImport[]) => void;
  onToggle: (name: string) => void;
}) {
  const { t, i18n } = useTranslation("onboarding");
  const { markFailed: markLogoLoadFailed, hasFailed: logoLoadFailed } = useLogoLoadFailures();
  const clientsQuery = useQuery({
    queryKey: ["clients", "onboarding"],
    queryFn: () => clientsApi.detect(true),
    staleTime: 30_000,
  });
  const allClients: ClientInfo[] = useMemo(
    () => clientsQuery.data?.client ?? [],
    [clientsQuery.data?.client],
  );
  const scannableClients = useMemo(
    () => clientsWithScannableConfig(allClients),
    [allClients],
  );
  const scannableClientKey = useMemo(
    () =>
      scannableClients
        .map((c) => c.identifier)
        .sort()
        .join("|"),
    [scannableClients],
  );
  const scanQuery = useQuery({
    queryKey: ["onboardingServerScan", scannableClientKey],
    enabled: scannableClients.length > 0,
    queryFn: async () => {
      const response = await onboardingApi.scanServers(
        scannableClients.map((client) => ({
          identifier: client.identifier,
          display_name: client.display_name || client.identifier,
          config_path: client.config_path!,
          config_file_parse:
            client.config_file_parse_override ??
            client.config_file_parse_effective ??
            null,
        })),
      );
      if (!response.success || !response.data) {
        throw new Error(String(response.error?.message ?? "Server scan failed"));
      }
      return response.data;
    },
    staleTime: 30_000,
    refetchOnWindowFocus: false,
  });
  const localServerCandidates = useMemo(
    () => scanQuery.data?.candidates ?? [],
    [scanQuery.data?.candidates],
  );
  const {
    activeTab,
    setActiveTab,
    primaryTagFilter: localTagFilter,
    setPrimaryTagFilter: setLocalTagFilter,
    popularTagFilter,
    setPopularTagFilter,
    isPrimaryTab,
  } = useOnboardingDualTab(
    SERVERS_PRIMARY_TAB,
    SERVERS_POPULAR_TAB,
    localServerCandidates.length === 0,
  );
  const popularServersQuery = useQuery({
    queryKey: ["adminDiscoveryServers", "onboarding", i18n.language],
    queryFn: () =>
      fetchAdminDiscoveryServers({
        surface: "onboarding",
        limit: ONBOARDING_CATALOG_LIMIT,
        locale: i18n.language,
      }),
    staleTime: 60_000,
  });
  const popularServerCandidates = useMemo(
    () => popularServersQuery.data ?? [],
    [popularServersQuery.data],
  );
  const candidates = useMemo(
    () =>
      mergeLocalAndAdminServerCandidates(
        localServerCandidates,
        popularServerCandidates,
      ),
    [localServerCandidates, popularServerCandidates],
  );
  const enrichedLocalServerCandidates = useMemo(
    () => enrichLocalServerCandidates(localServerCandidates, popularServerCandidates),
    [localServerCandidates, popularServerCandidates],
  );
  const filteredLocalServerCandidates = useMemo(
    () => filterCatalogItemsByTag(enrichedLocalServerCandidates, localTagFilter),
    [enrichedLocalServerCandidates, localTagFilter],
  );
  const filteredPopularServerCandidates = useMemo(
    () => filterCatalogItemsByTag(popularServerCandidates, popularTagFilter),
    [popularServerCandidates, popularTagFilter],
  );

  const serverTagLabel = useCallback(
    (tag: CatalogTagFilter) => catalogTagLabel(t, "servers", tag),
    [t, i18n.language],
  );

  useEffect(() => {
    onCandidatesChange(candidates);
  }, [candidates, onCandidatesChange]);

  const serverTabOptions = useMemo(
    () =>
      buildOnboardingTabOptions(
        {
          value: SERVERS_PRIMARY_TAB,
          label: t("servers.tabs.local", { defaultValue: "From your device" }),
          count: localServerCandidates.length,
        },
        {
          value: SERVERS_POPULAR_TAB,
          label: t("servers.tabs.popular", { defaultValue: "Popular Servers" }),
          count: popularServerCandidates.length,
        },
      ),
    [localServerCandidates.length, popularServerCandidates.length, t, i18n.language],
  );

  const isLocalLoading =
    clientsQuery.isLoading ||
    (scannableClients.length > 0 && scanQuery.isLoading && localServerCandidates.length === 0);
  const isPopularServersLoading =
    popularServersQuery.isLoading && popularServerCandidates.length === 0;

  const rescanServers = useCallback(() => {
    void clientsQuery.refetch();
    if (scannableClients.length > 0) {
      void scanQuery.refetch();
    }
  }, [clientsQuery, scanQuery, scannableClients.length]);

  const recommendationErrorMessage = t("servers.recommendationError", {
    defaultValue: "MCPMate could not load preset server data.",
  });
  const emptyFilteredMessage = t("servers.emptyFiltered", {
    defaultValue: "No servers match this category.",
  });

  return (
    <OnboardingDualTabStep
      icon={<Server className="mx-auto mb-3 h-10 w-10 text-violet-500" />}
      title={t("servers.title", { defaultValue: "Set Up MCP Servers" })}
      description={t("servers.description", {
        defaultValue:
          "Import servers found in local client configs or add MCPMate presets directly.",
      })}
      tabOptions={serverTabOptions}
      activeTab={activeTab}
      onTabChange={(value) => setActiveTab(value as ServersSetupTab)}
      isPrimaryTab={isPrimaryTab}
      primaryLoading={isLocalLoading}
      popularLoading={isPopularServersLoading}
      spinnerAccent="violet"
      primaryPanel={
        <OnboardingCatalogTabPanel
          footnote={t("servers.localNotice", {
            defaultValue:
              "We scan your device automatically. Some servers may not appear due to version or compatibility—you can add them manually later.",
          })}
        >
          <OnboardingCatalogToolbar
            tagValue={localTagFilter}
            onTagChange={setLocalTagFilter}
            tagOptions={POPULAR_SERVER_TAG_FILTERS}
            tagLabel={serverTagLabel}
            refreshAriaLabel={t("servers.rescan", { defaultValue: "Rescan" })}
            onRefresh={rescanServers}
            refreshDisabled={clientsQuery.isFetching || scanQuery.isFetching}
          />
          {scannableClients.length === 0 ? (
            <OnboardingEmptyCard
              message={t("servers.noScannableClients", {
                defaultValue:
                  "No detected MCP clients have a local configuration path to scan yet.",
              })}
              actionLabel={t("servers.emptyAction", {
                defaultValue: "Browse popular servers",
              })}
              onAction={() => setActiveTab(SERVERS_POPULAR_TAB)}
            />
          ) : scanQuery.isError ? (
            <OnboardingEmptyCard
              message={String(
                scanQuery.error instanceof Error ? scanQuery.error.message : "Server scan failed",
              )}
            />
          ) : localServerCandidates.length === 0 ? (
            <OnboardingEmptyCard
              message={t("servers.empty", {
                defaultValue:
                  "No importable MCP servers were found in your local client configs.",
              })}
              actionLabel={t("servers.emptyAction", {
                defaultValue: "Browse popular servers",
              })}
              onAction={() => setActiveTab(SERVERS_POPULAR_TAB)}
            />
          ) : filteredLocalServerCandidates.length === 0 ? (
            <OnboardingEmptyCard message={emptyFilteredMessage} />
          ) : (
            <OnboardingScrollableGrid>
              {filteredLocalServerCandidates.map((server) => {
                const isSelected = selectedServers.has(server.key);
                const detail = server.kind === "stdio" ? server.command : server.url;
                const logoUrl = server.logoUrl || undefined;
                const showLogo = Boolean(logoUrl) && !logoLoadFailed(server.key);
                return (
                  <OnboardingServerCard
                    key={server.key}
                    name={server.name}
                    kind={server.kind}
                    detail={detail ?? undefined}
                    logoUrl={logoUrl}
                    showLogo={showLogo}
                    onLogoError={() => markLogoLoadFailed(server.key)}
                    isSelected={isSelected}
                    onToggle={() => onToggle(server.key)}
                    sourceLabel={`${t("servers.sources", { defaultValue: "Found in" })}: ${server.source_clients.join(", ")}`}
                    selectedAriaLabel={t("servers.selectedAria", {
                      defaultValue: "{{name}} server selected",
                      name: server.name,
                    })}
                    unselectedAriaLabel={t("servers.unselectedAria", {
                      defaultValue: "{{name}} server not selected",
                      name: server.name,
                    })}
                  />
                );
              })}
            </OnboardingScrollableGrid>
          )}
        </OnboardingCatalogTabPanel>
      }
      popularPanel={
        <OnboardingCatalogTabPanel
          footnote={t("servers.recommendationNotice", {
            defaultValue:
              "These presets can be imported directly without an existing local client config.",
          })}
        >
          {popularServersQuery.isError || popularServerCandidates.length === 0 ? (
            <OnboardingEmptyCard message={recommendationErrorMessage} />
          ) : (
            <>
              <OnboardingCatalogToolbar
                tagValue={popularTagFilter}
                onTagChange={setPopularTagFilter}
                tagOptions={POPULAR_SERVER_TAG_FILTERS}
                tagLabel={serverTagLabel}
                refreshAriaLabel={t("servers.refresh", { defaultValue: "Refresh" })}
                onRefresh={() => void popularServersQuery.refetch()}
                refreshDisabled={popularServersQuery.isFetching}
              />
              {filteredPopularServerCandidates.length === 0 ? (
                <OnboardingEmptyCard message={emptyFilteredMessage} />
              ) : (
                <OnboardingScrollableGrid>
                  {filteredPopularServerCandidates.map((server) => {
                    const isSelected = selectedServers.has(server.key);
                    const logoUrl = server.logoUrl || undefined;
                    const showLogo = Boolean(logoUrl) && !logoLoadFailed(server.key);
                    return (
                      <OnboardingServerCard
                        key={server.key}
                        name={server.name}
                        kind={server.kind}
                        description={server.description || undefined}
                        logoUrl={logoUrl}
                        showLogo={showLogo}
                        onLogoError={() => markLogoLoadFailed(server.key)}
                        isSelected={isSelected}
                        onToggle={() => onToggle(server.key)}
                        selectedAriaLabel={t("servers.selectedAria", {
                          defaultValue: "{{name}} server selected",
                          name: server.name,
                        })}
                        unselectedAriaLabel={t("servers.unselectedAria", {
                          defaultValue: "{{name}} server not selected",
                          name: server.name,
                        })}
                      />
                    );
                  })}
                </OnboardingScrollableGrid>
              )}
            </>
          )}
        </OnboardingCatalogTabPanel>
      }
    />
  );
}

// ── Community step ────────────────────────────────────────────────────────────

const COMMUNITY_LINKS = [
  {
    key: "github",
    titleKey: "community.github.title",
    defaultTitle: "GitHub Issues",
    descriptionKey: "community.github.description",
    defaultDescription: "Report bugs, request features, and browse open issues.",
    href: "https://github.com/loocor/MCPMate/issues",
  },
  {
    key: "discussions",
    titleKey: "community.discussions.title",
    defaultTitle: "GitHub Discussions",
    descriptionKey: "community.discussions.description",
    defaultDescription:
      "Ask questions, share ideas, and discuss MCPMate with maintainers and users.",
    href: "https://github.com/loocor/MCPMate/discussions",
  },
] as const;

type CommunityChatLinkKey = "discord" | "feishu";

type CommunityLink =
  | {
      key: CommunityChatLinkKey;
      titleKey: string;
      defaultTitle: string;
      descriptionKey: string;
      defaultDescription: string;
      href: string;
    }
  | (typeof COMMUNITY_LINKS)[number];

function communityChatLink(language: string): Extract<CommunityLink, { key: CommunityChatLinkKey }> {
  if (prefersFeishuCommunity(language)) {
    return {
      key: "feishu",
      titleKey: "community.feishu.title",
      defaultTitle: "Feishu Community",
      descriptionKey: "community.feishu.description",
      defaultDescription:
        "Join the Chinese user community for support, tips, and product updates.",
      href: MCPMATE_FEISHU_COMMUNITY_HREF,
    };
  }
  return {
    key: "discord",
    titleKey: "community.discord.title",
    defaultTitle: "Discord",
    descriptionKey: "community.discord.description",
    defaultDescription: "Chat with the community, get support, and follow product updates.",
    href: MCPMATE_DISCORD_COMMUNITY_HREF,
  };
}

function communityLinksForLanguage(language: string): CommunityLink[] {
  return [communityChatLink(language), ...COMMUNITY_LINKS];
}

function CommunityLinkBrandIcon({ linkKey }: { linkKey: CommunityLink["key"] }) {
  if (linkKey === "feishu") {
    return (
      <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-white ring-1 ring-slate-200/80 dark:bg-slate-900 dark:ring-slate-700">
        <FeishuIcon className="h-8 w-8" />
      </div>
    );
  }
  if (linkKey === "discord") {
    return (
      <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-indigo-50 dark:bg-indigo-950/40">
        <MessagesSquare className="h-7 w-7 text-[#5865F2]" />
      </div>
    );
  }
  if (linkKey === "github") {
    return (
      <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-slate-100 dark:bg-slate-800">
        <Github className="h-7 w-7 text-slate-800 dark:text-slate-200" />
      </div>
    );
  }
  return (
    <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-violet-50 dark:bg-violet-950/40">
      <MessagesSquare className="h-7 w-7 text-violet-600 dark:text-violet-400" />
    </div>
  );
}

function CommunityStep() {
  const { t, i18n } = useTranslation("onboarding");
  const communityLinks = useMemo(
    () => communityLinksForLanguage(i18n.language),
    [i18n.language],
  );
  const showDiscordFallback = prefersFeishuCommunity(i18n.language);
  return (
    <div className="mx-auto flex min-h-[520px] w-full flex-col justify-center">
      <div className="mb-8 text-center">
        <Users className="mx-auto mb-3 h-10 w-10 text-amber-500" />
        <h2 className="text-2xl font-bold tracking-tight">
          {t("community.title", {
            defaultValue: "Join the Community",
          })}
        </h2>
        <p className="mt-2 text-slate-600 dark:text-slate-400">
          {t("community.description", {
            defaultValue:
              "Connect with other MCPMate users, get help, and stay up to date.",
          })}
        </p>
      </div>

      <div className="mb-6 grid gap-4 md:grid-cols-3">
        {communityLinks.map((link) => {
          const cardTitle = t(link.titleKey, { defaultValue: link.defaultTitle });
          return (
            <a
              key={link.key}
              href={link.href}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={t("community.openExternalAria", {
                defaultValue: "Open {{title}} in a new tab",
                title: cardTitle,
              })}
              className="group flex min-h-[190px] flex-col rounded-lg border-2 border-slate-200 bg-white p-4 text-left transition-all hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:hover:border-slate-600"
            >
              <div className="mb-3 flex items-start justify-between gap-3">
                <div className="flex min-w-0 items-center gap-3">
                  <CommunityLinkBrandIcon linkKey={link.key} />
                  <span className="text-base font-semibold">{cardTitle}</span>
                </div>
                <span
                  className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-slate-400 transition-colors group-hover:bg-slate-100 group-hover:text-slate-600 dark:group-hover:bg-slate-800 dark:text-slate-500 dark:group-hover:text-slate-300"
                  aria-hidden={true}
                >
                  <ExternalLink className="h-4 w-4" />
                </span>
              </div>
              <p className="flex-1 text-sm leading-6 text-slate-500 dark:text-slate-400">
                {t(link.descriptionKey, {
                  defaultValue: link.defaultDescription,
                })}
              </p>
            </a>
          );
        })}
      </div>
      {showDiscordFallback ? (
        <p className="text-center text-sm text-slate-500 dark:text-slate-400">
          {t("community.discordFallback", {
            defaultValue: "International users can join our",
          })}{" "}
          <a
            href={MCPMATE_DISCORD_COMMUNITY_HREF}
            target="_blank"
            rel="noopener noreferrer"
            className="font-medium text-slate-700 underline-offset-2 hover:underline dark:text-slate-300"
          >
            {t("community.discord.title", { defaultValue: "Discord" })}
          </a>
          {t("community.discordFallbackSuffix", { defaultValue: " community." })}
        </p>
      ) : null}
    </div>
  );
}

export default OnboardingPage;
