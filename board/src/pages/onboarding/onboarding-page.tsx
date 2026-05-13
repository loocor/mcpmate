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
import { Card, CardContent } from "../../components/ui/card";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Segment, type SegmentOption } from "../../components/ui/segment";
import {
  clientsApi,
  extractImportStats,
  runtimeApi,
  serversApi,
} from "../../lib/api";
import { websiteLangParam } from "../../lib/website-lang";
import { applyManagedClientsForIdentifiers } from "../../lib/client-config-sync";
import { resolveActiveDefaultProfileId } from "../../lib/default-profile";
import { buildClientServersImportRequest } from "../../lib/server-import-payload";
import {
  onboardingApi,
  type OnboardingServerCandidate,
  type RuntimeEntry,
  type OnboardingStatusResp,
  type RuntimeCheckResp,
} from "../../lib/onboarding-api";
import { MCPMATE_DISCORD_COMMUNITY_HREF } from "../../lib/mcpmate-community-urls";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { SUPPORTED_LANGUAGES } from "../../lib/i18n/index";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { cn } from "../../lib/utils";
import { notifyError, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { ClientInfo } from "../../lib/types";

type WizardStep = "welcome" | "runtime" | "clients" | "servers" | "community";
type RuntimeKind = "node" | "bun" | "uv";
type WelcomeLanguage = "en" | "zh-cn" | "ja";

function normalizeWelcomeLanguage(language?: string): WelcomeLanguage {
  const lower = language?.toLowerCase() ?? "";
  if (lower.startsWith("zh")) return "zh-cn";
  if (lower.startsWith("ja")) return "ja";
  return "en";
}

function groupSelectedServerNamesByClient(
  candidates: OnboardingServerCandidate[],
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
const ONBOARDING_SELECTIONS_STORAGE_KEY = "mcpmate_onboarding_selections";

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

interface PersistedOnboardingSelections {
  selectedClients: string[];
  selectedServers: string[];
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

function readPersistedSelections(): PersistedOnboardingSelections {
  if (typeof window === "undefined") {
    return { selectedClients: [], selectedServers: [] };
  }

  try {
    const raw = window.localStorage.getItem(ONBOARDING_SELECTIONS_STORAGE_KEY);
    if (!raw) {
      return { selectedClients: [], selectedServers: [] };
    }

    const parsed = JSON.parse(raw) as Partial<PersistedOnboardingSelections>;
    const selectedClients = Array.isArray(parsed.selectedClients)
      ? parsed.selectedClients.filter(
        (value): value is string => typeof value === "string" && value.trim().length > 0,
      )
      : [];
    const selectedServers = Array.isArray(parsed.selectedServers)
      ? parsed.selectedServers.filter(
        (value): value is string => typeof value === "string" && value.trim().length > 0,
      )
      : [];

    return { selectedClients, selectedServers };
  } catch {
    return { selectedClients: [], selectedServers: [] };
  }
}

function buildInitialWizardState(step: WizardStep): WizardState {
  const persisted = readPersistedSelections();
  return {
    step,
    selectedClients: new Set(persisted.selectedClients),
    selectedServers: new Set(persisted.selectedServers),
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
    OnboardingServerCandidate[]
  >([]);
  const [language, setLanguage] = useState<WelcomeLanguage>(() =>
    normalizeWelcomeLanguage(dashboardSettings.language),
  );
  const [welcomeConsentError, setWelcomeConsentError] = useState(false);

  useEffect(() => {
    if (activeStep !== state.step) {
      dispatch({ type: "SET_STEP", step: activeStep as WizardStep });
    }
  }, [activeStep, state.step]);

  const goToStep = useCallback(
    (step: WizardStep) => {
      // Prevent navigating away from welcome step without consent
      if (state.step === "welcome" && !state.welcomeConsent && step !== "welcome") {
        setWelcomeConsentError(true);
        return;
      }
      setActiveStep(step);
    },
    [setActiveStep, state.step, state.welcomeConsent],
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
      window.localStorage.removeItem(ONBOARDING_SELECTIONS_STORAGE_KEY);
      navigate("/", { replace: true });
    }
  }, [statusResp, navigate]);

  useEffect(() => {
    if (statusResp?.data?.completed) {
      return;
    }

    const payload: PersistedOnboardingSelections = {
      selectedClients: Array.from(state.selectedClients),
      selectedServers: Array.from(state.selectedServers),
    };
    window.localStorage.setItem(
      ONBOARDING_SELECTIONS_STORAGE_KEY,
      JSON.stringify(payload),
    );
  }, [state.selectedClients, state.selectedServers, statusResp?.data?.completed]);

  const handleComplete = useCallback(async () => {
    setCompleting(true);
    try {
      const clientsResp = await clientsApi.detect(false);
      const allListed = clientsResp?.client ?? [];
      const selectedServerNamesByClient = groupSelectedServerNamesByClient(
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
      await Promise.all(
        selectedClientList.map(async (client) => {
          await clientsApi.update({
            identifier: client.identifier,
            display_name: client.display_name || client.identifier,
            connection_mode: "local_config_detected",
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

      if (selectedServerNamesByClient.size > 0) {
        // In onboarding context, always link imported servers to the default-anchor profile
        // regardless of the autoAddServerToDefaultProfile setting (which defaults to false for new users).
        // The default anchor is seeded at app init, so it must already exist before onboarding runs.
        const targetProfileId = await resolveActiveDefaultProfileId();

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
            throw new Error(err ? String(err) : "Server import failed");
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
      window.localStorage.removeItem(ONBOARDING_SELECTIONS_STORAGE_KEY);
      navigate("/", { replace: true });
    } catch (error) {
      notifyError(
        t("servers.importErrorTitle", { defaultValue: "Server import failed" }),
        error instanceof Error ? error.message : String(error),
      );
      setCompleting(false);
    }
  }, [
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
                  className={`flex items-center gap-1 ${i < stepIdx
                    ? "text-emerald-600 dark:text-emerald-400"
                    : i === stepIdx
                      ? "font-medium text-slate-900 dark:text-white"
                      : "text-slate-400"
                    } transition-colors hover:text-slate-700 dark:hover:text-slate-200`}
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
          {state.step === "runtime" && <RuntimeStep />}
          {state.step === "clients" && (
            <ClientsStep
              selectedClients={state.selectedClients}
              onToggle={(id) =>
                dispatch({ type: "TOGGLE_CLIENT", identifier: id })
              }
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
                  <Button onClick={goToNextStep}>
                    {t("nav.next", { defaultValue: "Next" })}
                    <ArrowRight className="ml-1.5 h-4 w-4" />
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

  // Sync language with i18n
  useEffect(() => {
    const i18nLang = language === "zh-cn" ? "zh-CN" : language === "ja" ? "ja-JP" : "en";
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

function RuntimeStep() {
  const { t, i18n } = useTranslation("onboarding");
  const [searchParams] = useSearchParams();
  const qc = useQueryClient();
  const [installingKinds, setInstallingKinds] = useState<Set<RuntimeKind>>(
    new Set(),
  );
  const { data, isLoading } = useQuery<RuntimeCheckResp>({
    queryKey: ["onboardingRuntimeCheck"],
    queryFn: () => onboardingApi.runtimeCheck(),
    staleTime: 60_000,
  });
  const installRuntime = useCallback(
    async (kind: RuntimeKind) => {
      if (installingKinds.has(kind)) return;
      setInstallingKinds((prev) => new Set(prev).add(kind));
      try {
        await runtimeApi.install({ runtime_type: kind, verbose: true });
        await qc.invalidateQueries({ queryKey: ["onboardingRuntimeCheck"] });
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

  const preview = searchParams.get("runtimePreview");
  const previewMode: "real" | "partial" | "none" =
    preview === "partial" || preview === "none" ? preview : "real";

  const baseRuntimes = data?.data?.runtimes ?? [];
  const runtimeSeeds: RuntimeEntry[] = useMemo(
    () => [
      { name: "node", available: true },
      { name: "bun", available: true },
      { name: "python3", available: true },
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

  const showLoading = previewMode === "real" && isLoading;

  const getRuntimeKindFromEntry = useCallback(
    (name: string): RuntimeKind | null => {
      const n = name.toLowerCase();
      if (n === "node" || n === "npx") return "node";
      if (n === "bun" || n === "bunx") return "bun";
      if (n === "python3" || n === "uv" || n === "uvx") return "uv";
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
              "MCP servers need a JavaScript or Python runtime. We'll check what's available on your system.",
          })}
        </p>
      </div>

      {showLoading ? (
        <div className="flex justify-center py-12">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-slate-300 border-t-emerald-500" />
        </div>
      ) : (
        <>
          <div className="mb-6 grid gap-4 md:grid-cols-3">
            {runtimeCardMeta.map((card) => {
              const available = availableKinds.has(card.key);
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
                    variant={available ? "secondary" : "default"}
                    className="w-fit"
                    disabled={available || installingKinds.has(card.key)}
                    onClick={() => void installRuntime(card.key)}
                  >
                    {available ? (
                      t("runtime.install.ready", { defaultValue: "Ready" })
                    ) : installingKinds.has(card.key) ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        {t("runtime.install.installing", {
                          defaultValue: "Installing...",
                        })}
                      </>
                    ) : (
                      t("runtime.install.clickToInstall", {
                        defaultValue: "Click to install",
                      })
                    )}
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

// ── Clients step (auto-detect only) ──────────────────────────────────────────

function ClientsStep({
  selectedClients,
  onToggle,
}: {
  selectedClients: Set<string>;
  onToggle: (identifier: string) => void;
}) {
  const { t } = useTranslation("onboarding");
  const [logoLoadFailedClients, setLogoLoadFailedClients] = useState<Set<string>>(
    () => new Set(),
  );
  const { data, isLoading, isError, refetch, error } = useQuery({
    queryKey: ["clients", "onboarding"],
    queryFn: () => clientsApi.detect(true),
    staleTime: 30_000,
    refetchOnMount: "always",
  });

  const detectedClients = useMemo(
    () =>
      (data?.client ?? [])
        .filter((client) => client.detected)
        .sort(compareClientsByName),
    [data?.client],
  );

  return (
    <div>
      <div className="mb-6 text-center">
        <Users className="mx-auto mb-3 h-10 w-10 text-blue-500" />
        <h2 className="text-2xl font-bold tracking-tight">
          {t("clients.title", {
            defaultValue: "Detected MCP Clients",
          })}
        </h2>
        <p className="mt-2 text-slate-600 dark:text-slate-400">
          {t("clients.description", {
            defaultValue:
              "We found these MCP clients on your system. Select the ones you'd like MCPMate to manage.",
          })}
        </p>
      </div>

      {isLoading ? (
        <div className="flex justify-center py-12">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-slate-300 border-t-emerald-500" />
        </div>
      ) : isError ? (
        <Card>
          <CardContent className="py-8 text-center text-slate-500">
            <p>
              {t("clients.error", {
                defaultValue:
                  "Failed to detect MCP clients. Please retry.",
              })}
            </p>
            <p className="mt-1 text-xs text-slate-400">
              {(error as Error)?.message ?? ""}
            </p>
            <Button
              variant="outline"
              size="sm"
              className="mt-4"
              onClick={() => void refetch()}
            >
              {t("clients.retry", { defaultValue: "Retry detection" })}
            </Button>
          </CardContent>
        </Card>
      ) : detectedClients.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-slate-500">
            {t("clients.empty", {
              defaultValue:
                "No MCP clients detected on this system. You can add clients manually later from the Clients page.",
            })}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {detectedClients.map((client) => {
            const isSelected = selectedClients.has(client.identifier);
            const showLogo =
              Boolean(client.logo_url) &&
              !logoLoadFailedClients.has(client.identifier);
            return (
              <button
                key={client.identifier}
                type="button"
                onClick={() => onToggle(client.identifier)}
                className={`flex items-center gap-3 rounded-lg border-2 p-4 text-left transition-all ${isSelected
                  ? "border-emerald-500 bg-emerald-50 dark:border-emerald-400 dark:bg-emerald-950/30"
                  : "border-slate-200 bg-white hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:hover:border-slate-600"
                  }`}
              >
                <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-slate-100 text-sm font-semibold dark:bg-slate-800">
                  {showLogo ? (
                    <img
                      src={client.logo_url ?? undefined}
                      alt={client.display_name || client.identifier}
                      className="h-full w-full object-cover"
                      loading="lazy"
                      onError={() => {
                        setLogoLoadFailedClients((prev) => {
                          if (prev.has(client.identifier)) return prev;
                          const next = new Set(prev);
                          next.add(client.identifier);
                          return next;
                        });
                      }}
                    />
                  ) : (
                    (client.display_name || client.identifier)
                      .charAt(0)
                      .toUpperCase()
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="font-medium">
                    {client.display_name || client.identifier}
                  </div>
                  {client.description && (
                    <div className="mt-0.5 truncate text-xs text-slate-500">
                      {client.description}
                    </div>
                  )}
                  {client.config_path && (
                    <div className="mt-1 truncate font-mono text-xs text-slate-400">
                      {client.config_path}
                    </div>
                  )}
                </div>
                {isSelected && (
                  <Check className="h-5 w-5 shrink-0 text-emerald-500" />
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ServersStep({
  selectedServers,
  onCandidatesChange,
  onToggle,
}: {
  selectedServers: Set<string>;
  onCandidatesChange: (candidates: OnboardingServerCandidate[]) => void;
  onToggle: (name: string) => void;
}) {
  const { t } = useTranslation("onboarding");
  const clientsQuery = useQuery({
    queryKey: ["clients", "onboarding"],
    queryFn: () => clientsApi.detect(true),
    staleTime: 30_000,
  });
  const allClients: ClientInfo[] = clientsQuery.data?.client ?? [];
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
    staleTime: 0,
  });

  useEffect(() => {
    onCandidatesChange(scanQuery.data?.candidates ?? []);
  }, [onCandidatesChange, scanQuery.data?.candidates, scannableClientKey]);

  const candidates = scanQuery.data?.candidates ?? [];

  return (
    <div>
      <div className="mb-6 text-center">
        <Server className="mx-auto mb-3 h-10 w-10 text-violet-500" />
        <h2 className="text-2xl font-bold tracking-tight">
          {t("servers.title", {
            defaultValue: "Import Existing Servers",
          })}
        </h2>
        <p className="mt-2 text-slate-600 dark:text-slate-400">
          {t("servers.description", {
            defaultValue:
              "We scanned every detected MCP client that has a local config file. Choose the servers you'd like MCPMate to import.",
          })}
        </p>
      </div>

      {scannableClients.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-slate-500">
            {t("servers.noScannableClients", {
              defaultValue:
                "No detected MCP clients have a local configuration path to scan yet. You can skip this step or finish client setup first.",
            })}
          </CardContent>
        </Card>
      ) : clientsQuery.isLoading || scanQuery.isLoading ? (
        <div className="flex justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-violet-500" />
        </div>
      ) : candidates.length === 0 ? (
        <Card>
          <CardContent className="space-y-3 py-8 text-center text-slate-500">
            <p>
              {t("servers.empty", {
                defaultValue:
                  "No importable MCP servers were found across your detected clients.",
              })}
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          <div className="grid gap-3">
            {candidates.map((server) => {
              const isSelected = selectedServers.has(server.key);
              const detail =
                server.kind === "stdio" ? server.command : server.url;
              return (
                <button
                  key={server.key}
                  type="button"
                  onClick={() => onToggle(server.key)}
                  className={`flex items-center gap-4 rounded-lg border-2 p-4 text-left transition-all ${isSelected
                    ? "border-emerald-500 bg-emerald-50 dark:border-emerald-400 dark:bg-emerald-950/30"
                    : "border-slate-200 bg-white hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:hover:border-slate-600"
                    }`}
                >
                  <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-violet-100 dark:bg-violet-900/30">
                    <Server className="h-5 w-5 text-violet-600 dark:text-violet-400" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium">{server.name}</span>
                      <span className="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-500 dark:bg-slate-800 dark:text-slate-400">
                        {server.kind}
                      </span>
                    </div>
                    {detail && (
                      <div className="mt-0.5 truncate font-mono text-xs text-slate-500">
                        {detail}
                      </div>
                    )}
                    <div className="mt-1.5 text-xs text-slate-400">
                      {t("servers.sources", { defaultValue: "Found in" })}: {server.source_clients.join(", ")}
                    </div>
                  </div>
                  {isSelected && (
                    <Check className="h-5 w-5 shrink-0 text-emerald-500" />
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Community step ────────────────────────────────────────────────────────────

const COMMUNITY_LINKS = [
  {
    key: "discord",
    titleKey: "community.discord.title",
    defaultTitle: "Discord",
    descriptionKey: "community.discord.description",
    defaultDescription:
      "Chat with the community, get support, and follow product updates.",
    href: MCPMATE_DISCORD_COMMUNITY_HREF,
  },
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

function CommunityLinkBrandIcon({ linkKey }: { linkKey: (typeof COMMUNITY_LINKS)[number]["key"] }) {
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
  const { t } = useTranslation("onboarding");
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
        {COMMUNITY_LINKS.map((link) => {
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
    </div>
  );
}

export default OnboardingPage;
