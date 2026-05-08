import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ExternalLink,
  Github,
  Globe,
  MessageCircle,
  Loader2,
  Rocket,
  Server,
  Terminal,
  Users,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { SiBun, SiNodedotjs, SiUv } from "@icons-pack/react-simple-icons";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { clientsApi, runtimeApi, serversApi } from "../../lib/api";
import {
  onboardingApi,
  type OnboardingServerCandidate,
  type RuntimeEntry,
  type OnboardingStatusResp,
  type RuntimeCheckResp,
} from "../../lib/onboarding-api";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { SUPPORTED_LANGUAGES } from "../../lib/i18n/index";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { ClientInfo } from "../../lib/types";

type WizardStep = "welcome" | "runtime" | "clients" | "servers" | "community";
type RuntimeKind = "node" | "bun" | "uv";

function buildServerImportPayload(
  candidates: OnboardingServerCandidate[],
  selectedKeys: Set<string>,
) {
  const mcpServers: Record<
    string,
    {
      type: string;
      command?: string | null;
      args?: string[];
      env?: Record<string, string>;
      url?: string | null;
    }
  > = {};

  for (const candidate of candidates) {
    if (!selectedKeys.has(candidate.key)) continue;
    const kind = candidate.kind.toLowerCase() === "sse" ? "streamable_http" : candidate.kind;
    mcpServers[candidate.name] = {
      type: kind,
      ...(kind === "streamable_http"
        ? { url: candidate.url }
        : { command: candidate.command }),
      args: candidate.args,
      env: candidate.env,
    };
  }

  return mcpServers;
}

function RuntimeBrandIcon({ kind }: { kind: RuntimeKind }) {
  if (kind === "node") {
    return (
      <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-emerald-50 dark:bg-emerald-950/40">
        <SiNodedotjs className="h-7 w-7 text-[#339933]" />
      </div>
    );
  }

  if (kind === "bun") {
    return (
      <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-amber-50 dark:bg-amber-950/40">
        <SiBun className="h-7 w-7 text-[#FBF0DF]" />
      </div>
    );
  }

  return (
    <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-violet-50 dark:bg-violet-950/40">
      <SiUv className="h-7 w-7 text-[#DE5FE9]" />
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
}

type WizardAction =
  | { type: "NEXT" }
  | { type: "PREV" }
  | { type: "TOGGLE_CLIENT"; identifier: string }
  | { type: "TOGGLE_SERVER"; name: string }
  | { type: "SET_STEP"; step: WizardStep };

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
  }
}

// ── Main component ────────────────────────────────────────────────────────────

export function OnboardingPage() {
  const navigate = useNavigate();
  usePageTranslations("onboarding");
  const { t, i18n } = useTranslation("onboarding");
  const qc = useQueryClient();
  const dashboardSettings = useAppStore((s) => s.dashboardSettings);
  const setDashboardSetting = useAppStore((s) => s.setDashboardSetting);
  const { activeTab: activeStep, setActiveTab: setActiveStep } = useUrlTab({
    paramName: "step",
    defaultTab: "welcome",
    validTabs: STEP_ORDER,
  });

  const [state, dispatch] = useReducer(wizardReducer, {
    step: activeStep as WizardStep,
    selectedClients: new Set<string>(),
    selectedServers: new Set<string>(),
  });

  const [completing, setCompleting] = useState(false);
  const [serverCandidates, setServerCandidates] = useState<
    OnboardingServerCandidate[]
  >([]);

  useEffect(() => {
    if (activeStep !== state.step) {
      dispatch({ type: "SET_STEP", step: activeStep as WizardStep });
    }
  }, [activeStep, state.step]);

  const goToStep = useCallback(
    (step: WizardStep) => {
      setActiveStep(step);
    },
    [setActiveStep],
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
      const clientsResp = await clientsApi.list(false, {
        persistDetected: false,
        includeDetected: true,
      });
      const selectedClientList = (clientsResp?.client ?? []).filter((client) =>
        state.selectedClients.has(client.identifier),
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

      const mcpServers = buildServerImportPayload(
        serverCandidates,
        state.selectedServers,
      );
      if (Object.keys(mcpServers).length > 0) {
        const response = await serversApi.importServers({ mcpServers });
        if (response.success === false) {
          throw new Error(String(response.error ?? "Server import failed"));
        }
        await qc.invalidateQueries({ queryKey: ["servers"] });
      }
      await qc.invalidateQueries({ queryKey: ["clients"] });

      await onboardingApi.complete(true);
      await qc.invalidateQueries({ queryKey: ["onboardingStatus"] });
      navigate("/", { replace: true });
    } catch (error) {
      notifyError(
        t("servers.importErrorTitle", { defaultValue: "Server import failed" }),
        error instanceof Error ? error.message : String(error),
      );
      setCompleting(false);
    }
  }, [navigate, qc, serverCandidates, state.selectedClients, state.selectedServers, t]);

  const handleStartWithLanguage = useCallback(
    (storeLanguage: "en" | "zh-cn" | "ja") => {
      const i18nCode =
        SUPPORTED_LANGUAGES.find((entry) => entry.store === storeLanguage)?.i18n ??
        storeLanguage;
      if (dashboardSettings.language !== storeLanguage) {
        setDashboardSetting("language", storeLanguage);
      }
      void i18n.changeLanguage(i18nCode);
      goToNextStep();
    },
    [dashboardSettings.language, goToNextStep, i18n, setDashboardSetting],
  );

  const stepIdx = STEP_ORDER.indexOf(state.step);
  const isFirst = stepIdx === 0;
  const isLast = stepIdx === STEP_ORDER.length - 1;

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
            <WelcomeStep onStartWithLanguage={handleStartWithLanguage} />
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
              selectedClients={state.selectedClients}
              selectedServers={state.selectedServers}
              onCandidatesChange={setServerCandidates}
              onToggle={(name) => dispatch({ type: "TOGGLE_SERVER", name })}
            />
          )}
          {state.step === "community" && <CommunityStep />}
        </div>
      </main>

      {/* Bottom navigation — fixed */}
      {state.step !== "welcome" && (
        <footer className="shrink-0 border-t border-slate-200 px-6 py-4 dark:border-slate-800">
          <div className="mx-auto flex max-w-3xl items-center justify-between">
            {isFirst ? (
              <div className="h-9 w-[88px]" aria-hidden="true" />
            ) : (
              <Button variant="ghost" onClick={goToPrevStep}>
                <ArrowLeft className="mr-1.5 h-4 w-4" />
                {t("nav.back", { defaultValue: "Back" })}
              </Button>
            )}
            <div className="flex items-center gap-2">
              {!isLast && (
                <Button
                  variant="ghost"
                  onClick={goToNextStep}
                >
                  {t("nav.skip", { defaultValue: "Skip" })}
                </Button>
              )}
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
          </div>
        </footer>
      )}
    </div>
  );
}

// ── Welcome step ──────────────────────────────────────────────────────────────

function WelcomeStep({
  onStartWithLanguage,
}: {
  onStartWithLanguage: (language: "en" | "zh-cn" | "ja") => void;
}) {
  const { t } = useTranslation("onboarding");
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <Rocket className="mx-auto mb-6 h-16 w-16 text-emerald-500" />
      <h1 className="mb-3 text-3xl font-bold tracking-tight">
        {t("welcome.title", {
          defaultValue: "Welcome to MCPMate",
        })}
      </h1>
      <p className="mx-auto mb-8 max-w-lg text-lg text-slate-600 dark:text-slate-400">
        {t("welcome.description", {
          defaultValue:
            "Let's get you set up in a few quick steps. We'll check your environment, detect clients, and add some useful servers.",
        })}
      </p>
      <p className="mb-4 text-sm text-slate-500 dark:text-slate-400">
        {t("welcome.chooseLanguage", {
          defaultValue: "Choose your language to continue",
        })}
      </p>
      <div className="flex flex-wrap items-center justify-center gap-2">
        <Button size="lg" onClick={() => onStartWithLanguage("en")}>
          🇺🇸 English
        </Button>
        <Button size="lg" onClick={() => onStartWithLanguage("zh-cn")}>
          🇨🇳 简体中文
        </Button>
        <Button size="lg" onClick={() => onStartWithLanguage("ja")}>
          🇯🇵 日本語
        </Button>
      </div>
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
  const { data, isLoading, isError, refetch, error } = useQuery({
    queryKey: ["clients", "onboarding"],
    queryFn: () => clientsApi.list(true, { persistDetected: false, includeDetected: true }),
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
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-slate-100 text-sm font-semibold dark:bg-slate-800">
                  {(client.display_name || client.identifier)
                    .charAt(0)
                    .toUpperCase()}
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
  selectedClients,
  selectedServers,
  onCandidatesChange,
  onToggle,
}: {
  selectedClients: Set<string>;
  selectedServers: Set<string>;
  onCandidatesChange: (candidates: OnboardingServerCandidate[]) => void;
  onToggle: (name: string) => void;
}) {
  const { t } = useTranslation("onboarding");
  const selectedClientIds = useMemo(
    () => Array.from(selectedClients).sort(),
    [selectedClients],
  );
  const selectedClientKey = selectedClientIds.join("|");
  const clientsQuery = useQuery({
    queryKey: ["clients", "onboarding"],
    queryFn: () => clientsApi.list(true, { persistDetected: false, includeDetected: true }),
    staleTime: 30_000,
  });
  const allClients: ClientInfo[] = clientsQuery.data?.client ?? [];
  const selectedClientList = useMemo(
    () => allClients.filter((client) => selectedClients.has(client.identifier)),
    [allClients, selectedClients],
  );
  const scanQuery = useQuery({
    queryKey: ["onboardingServerScan", selectedClientKey],
    enabled: selectedClientList.length > 0,
    queryFn: async () => {
      const response = await onboardingApi.scanServers(
        selectedClientList
          .filter((client) => client.config_path)
          .map((client) => ({
            identifier: client.identifier,
            display_name: client.display_name || client.identifier,
            config_path: client.config_path,
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
  }, [onCandidatesChange, scanQuery.data?.candidates, selectedClientKey]);

  const candidates = scanQuery.data?.candidates ?? [];
  const scanErrors = scanQuery.data?.errors ?? [];

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
              "We scanned the MCP clients you selected. Choose the servers you'd like MCPMate to import.",
          })}
        </p>
      </div>

      {selectedClientIds.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-slate-500">
            {t("servers.selectClientsFirst", {
              defaultValue:
                "Select at least one detected client first, or skip this step.",
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
                  "No importable MCP servers were found in the selected clients.",
              })}
            </p>
            {scanErrors.length > 0 && (
              <div className="mx-auto max-w-lg rounded-lg border border-amber-200 bg-amber-50 p-3 text-left text-xs text-amber-700 dark:border-amber-800 dark:bg-amber-950/30 dark:text-amber-300">
                {scanErrors.map((error) => (
                  <div key={error.client_name}>
                    {error.client_name}: {error.message}
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {scanErrors.length > 0 && (
            <div className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-xs text-amber-700 dark:border-amber-800 dark:bg-amber-950/30 dark:text-amber-300">
              {scanErrors.map((error) => (
                <div key={error.client_name}>
                  {error.client_name}: {error.message}
                </div>
              ))}
            </div>
          )}
          <div className="grid gap-3">
            {candidates.map((server) => {
              const isSelected = selectedServers.has(server.key);
              const detail =
                server.kind === "streamable_http" ? server.url : server.command;
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
    key: "github",
    icon: Github,
    titleKey: "community.github.title",
    defaultTitle: "GitHub Issues",
    descriptionKey: "community.github.description",
    defaultDescription: "Report bugs, request features, and browse open issues.",
    href: "https://github.com/loocor/MCPMate/issues",
  },
  {
    key: "docs",
    icon: MessageCircle,
    titleKey: "community.docs.title",
    defaultTitle: "Documentation",
    descriptionKey: "community.docs.description",
    defaultDescription: "Guides, tutorials, and API references.",
    href: "https://mcp.umate.ai/docs",
  },
  {
    key: "chrome",
    icon: ExternalLink,
    titleKey: "community.chrome.title",
    defaultTitle: "Chrome Extension",
    descriptionKey: "community.chrome.description",
    defaultDescription: "Detect and import MCP server snippets from web pages.",
    href: "https://chromewebstore.google.com/detail/mcpmate-server-import/jngogcgclencgillbmeeimkcjjnobidf",
  },
  {
    key: "edge",
    icon: ExternalLink,
    titleKey: "community.edge.title",
    defaultTitle: "Edge Extension",
    descriptionKey: "community.edge.description",
    defaultDescription: "Import MCP server configurations directly from Edge.",
    href: "https://microsoftedge.microsoft.com/addons/detail/mcpmate-server-import/nbpdfanhajcjghegoocfmjkpaklidckn",
  },
] as const;

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

      <div className="grid gap-4 sm:grid-cols-2">
        {COMMUNITY_LINKS.map((link) => {
          const Icon = link.icon;
          return (
            <a
              key={link.key}
              href={link.href}
              target="_blank"
              rel="noopener noreferrer"
              className="flex min-h-[120px] items-start gap-3 rounded-lg border-2 border-slate-200 bg-white p-4 text-left transition-all hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:hover:border-slate-600"
            >
              <Icon className="mt-0.5 h-5 w-5 shrink-0 text-slate-500" />
              <div>
                <div className="font-medium">
                  {t(link.titleKey, { defaultValue: link.defaultTitle })}
                </div>
                <div className="mt-0.5 text-sm text-slate-500">
                  {t(link.descriptionKey, {
                    defaultValue: link.defaultDescription,
                  })}
                </div>
              </div>
            </a>
          );
        })}
      </div>
    </div>
  );
}

export default OnboardingPage;
