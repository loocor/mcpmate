import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useRef, useState, type ReactNode } from "react";
import {
	BrowserRouter,
	Navigate,
	Route,
	Routes,
	useLocation,
	useNavigate,
	useParams,
} from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Layout } from "./components/layout/layout";
import { LanguageSynchronizer } from "./components/language-synchronizer.ts";
import { ThemeSynchronizer } from "./components/theme-synchronizer";
import { ApiDocsPage } from "./pages/api-docs/api-docs-page";
import { AuditPage } from "./pages/audit/audit-page";
import { ClientDirectCapabilitiesPage } from "./pages/clients/client-direct-page.tsx";
import { ClientsPage } from "./pages/clients/clients-page";
import { DashboardPage } from "./pages/dashboard/dashboard-page";
import { MarketDetailPage } from "./pages/market/market-detail-page";
import { MarketPage } from "./pages/market/market-page";
import { NotFoundPage } from "./pages/not-found-page";
import { ProfileDetailPage } from "./pages/profile/profile-detail-page";
import { ProfilePage } from "./pages/profile/profile-page";
import { ProfilePresetPage } from "./pages/profile/profile-preset-page";
import { RuntimePage } from "./pages/runtime/runtime-page";
import { SecretsPage } from "./pages/secrets/secrets-page";
import { InstanceDetailPage } from "./pages/servers/instance-detail-page";
import { ServerDetailPage } from "./pages/servers/server-detail-page";
import { OAuthCallbackPage } from "./pages/servers/oauth-callback-page";
import { ServerListPage } from "./pages/servers/server-list-page";
import { SettingsPage } from "./pages/settings/settings-page";
import { OnboardingPage } from "./pages/onboarding/onboarding-page";
import { TrayOperatorPanelPage } from "./pages/operator/tray-operator-panel-page";
import { OperatorBackendWaitingPage } from "./pages/operator/operator-backend-waiting";
import {
	notificationsService,
	setApiBaseUrl,
	systemApi,
} from "./lib/api";
import {
	backendReadinessStatusKey,
	shouldReportBackendReadinessAttempt,
} from "./lib/backend-readiness-diagnostics";
import { recordDesktopDiagnosticEvent } from "./lib/desktop-diagnostics";
import { isTauriEnvironmentSync } from "./lib/platform";

const ClientDetailPage = lazy(() =>
	import("./pages/clients/client-detail-page").then((module) => ({
		default: module.ClientDetailPage,
	})),
);

const OPEN_FULL_BOARD_PATH_EVENT = "mcpmate://open-full-board-path";

let pendingFullBoardNavigation:
	| {
			path: string;
			pathname: string;
	  }
	| null = null;
let pendingFullBoardNavigationTake:
	| Promise<{
			path: string;
			pathname: string;
	  } | null>
	| null = null;

// Initialize the query client
const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 30 * 1000, // 30 seconds
			retry: 1,
			refetchOnWindowFocus: true,
		},
	},
});

function App() {
	return (
		<QueryClientProvider client={queryClient}>
			<LanguageSynchronizer />
			<ThemeSynchronizer />
			<BackendReadinessGate>
				<BrowserRouter
					future={{
						v7_startTransition: true,
						v7_relativeSplatPath: true,
					}}
				>
					<DesktopFullBoardPathBridge>
						<Routes>
							<Route path="oauth/callback" element={<OAuthCallbackPage />} />
							<Route path="onboarding" element={<OnboardingPage />} />
							<Route path="operator" element={<TrayOperatorPanelPage />} />
							<Route path="/" element={<Layout />}>
								<Route index element={<DashboardPage />} />
								{/* New canonical routes */}
								<Route path="profiles" element={<ProfilePage />} />
								<Route
									path="profiles/presets/:presetId"
									element={<ProfilePresetPage />}
								/>
								<Route path="profiles/:profileId" element={<ProfileDetailPage />} />
								{/* Back-compat: redirect old routes */}
								<Route
									path="config"
									element={<Navigate to="/profiles" replace />}
								/>
								<Route
									path="config/presets/:presetId"
									element={<LegacyPresetRedirect />}
								/>
								<Route
									path="config/suits/:suitId"
									element={<LegacySuitRedirect />}
								/>
								<Route
									path="config/profiles/:suitId"
									element={<LegacySuitRedirect />}
								/>
								<Route path="market" element={<MarketPage />} />
								<Route path="market/:registryKey" element={<MarketDetailPage />} />
								<Route path="servers" element={<ServerListPage />} />
								<Route path="servers/:serverId" element={<ServerDetailPage />} />
								<Route
									path="servers/:serverId/instances/:instanceId"
									element={<InstanceDetailPage />}
								/>
								{/* Tools route removed */}
								<Route path="clients" element={<ClientsPage />} />
								<Route
									path="clients/:identifier/direct/:serverId"
									element={<ClientDirectCapabilitiesPage />}
								/>
								<Route
									path="clients/:identifier"
									element={
										<Suspense fallback={null}>
											<ClientDetailPage />
										</Suspense>
									}
								/>
								<Route path="runtime" element={<RuntimePage />} />
								<Route path="secrets" element={<SecretsPage />} />
								<Route path="audit" element={<AuditPage />} />
								<Route path="api-docs" element={<ApiDocsPage />} />
								<Route path="account" element={<Navigate to="/" replace />} />
								<Route path="settings" element={<SettingsPage />} />

								<Route path="404" element={<NotFoundPage />} />
								<Route path="*" element={<Navigate to="/404" replace />} />
							</Route>
						</Routes>
					</DesktopFullBoardPathBridge>
				</BrowserRouter>
			</BackendReadinessGate>
		</QueryClientProvider>
	);
}

function toFullBoardPath(raw: unknown): string | undefined {
	if (typeof raw !== "string") {
		return undefined;
	}
	if (!raw.startsWith("/") || raw.startsWith("//")) {
		return undefined;
	}
	const routePath = raw.split(/[?#]/, 1)[0].replace(/\/+$/, "");
	if (routePath === "/operator" || routePath.startsWith("/operator/")) {
		return undefined;
	}
	return raw;
}

function DesktopFullBoardPathBridge({ children }: { children: ReactNode }) {
	const navigate = useNavigate();
	const location = useLocation();
	const isOperatorRoute = location.pathname === "/operator";
	const [ready, setReady] = useState(false);
	const [pendingNavigationPath, setPendingNavigationPath] = useState<string | null>(null);

	useEffect(() => {
		if (!pendingNavigationPath) {
			return;
		}
		if (location.pathname === pendingNavigationPath) {
			if (pendingFullBoardNavigation?.pathname === pendingNavigationPath) {
				pendingFullBoardNavigation = null;
				pendingFullBoardNavigationTake = null;
			}
			setPendingNavigationPath(null);
			setReady(true);
		}
	}, [location.pathname, pendingNavigationPath]);

	useEffect(() => {
		if (!isTauriEnvironmentSync() || isOperatorRoute) {
			setReady(true);
			return;
		}

		if (
			pendingFullBoardNavigation &&
			location.pathname !== pendingFullBoardNavigation.pathname
		) {
			setPendingNavigationPath(pendingFullBoardNavigation.pathname);
			navigate(pendingFullBoardNavigation.path, { replace: true });
			return;
		}

		let cancelled = false;
		let unlisten: (() => void) | undefined;

		async function takeAndNavigate(): Promise<boolean> {
			try {
				if (!pendingFullBoardNavigationTake) {
					pendingFullBoardNavigationTake = (async () => {
						const { invoke } = await import("@tauri-apps/api/core");
						const rawPath = await invoke<unknown>("mcp_shell_take_pending_full_board_path");
						const path = toFullBoardPath(rawPath);
						if (!path) {
							if (rawPath !== null && rawPath !== undefined && import.meta.env.DEV) {
								console.warn("[App] Ignoring invalid full board path", rawPath);
							}
							return null;
						}
						const pathname = path.split(/[?#]/, 1)[0].replace(/\/+$/, "") || "/";
						return { path, pathname };
					})();
				}
				const navigation = await pendingFullBoardNavigationTake;
				if (cancelled) {
					return false;
				}
				if (!navigation) {
					pendingFullBoardNavigationTake = null;
					return false;
				}
				if (navigation) {
					pendingFullBoardNavigation = navigation;
					setPendingNavigationPath(navigation.pathname);
					navigate(navigation.path, { replace: true });
					return true;
				}
			} catch (error) {
				pendingFullBoardNavigationTake = null;
				if (import.meta.env.DEV) {
					console.warn("[App] Failed to take pending full board path", error);
				}
			}
			return false;
		}

		async function bind(): Promise<void> {
			let waitingForNavigation = false;
			try {
				const { listen } = await import("@tauri-apps/api/event");
				unlisten = await listen(OPEN_FULL_BOARD_PATH_EVENT, () => {
					void takeAndNavigate();
				});
				const isNavigating = await takeAndNavigate();
				if (isNavigating) {
					waitingForNavigation = true;
					return;
				}
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn("[App] Failed to bind full board path delivery", error);
				}
			} finally {
				if (!cancelled && !waitingForNavigation) {
					setReady(true);
				}
			}
		}

		void bind();
		return () => {
			cancelled = true;
			if (unlisten) {
				void unlisten();
			}
		};
	}, [isOperatorRoute, location.pathname, navigate]);

	return ready ? <>{children}</> : null;
}

type BackendReadinessMessageKey = "starting" | "waitingForBackend" | "confirmingReadiness";

const backendReadinessFallbacks: Record<BackendReadinessMessageKey, string> = {
	starting: "Starting MCPMate Core...",
	waitingForBackend: "Waiting for MCPMate backend",
	confirmingReadiness: "Confirming backend readiness...",
};

function BackendReadinessGate({ children }: { children: ReactNode }) {
	const { t } = useTranslation();
	const [desktopSourceReady, setDesktopSourceReady] = useState(
		() => !isTauriEnvironmentSync(),
	);
	const [backendReady, setBackendReady] = useState(false);
	const [attempt, setAttempt] = useState(0);
	const [messageKey, setMessageKey] = useState<BackendReadinessMessageKey>("starting");
	const [readinessStartedAtMs] = useState(() => Date.now());
	const lastDiagnosticRef = useRef<{
		reportedAtMs: number;
		statusKey: string;
	} | null>(null);

	useEffect(() => {
		if (!isTauriEnvironmentSync()) {
			setDesktopSourceReady(true);
			return;
		}

		let cancelled = false;
		const syncDesktopCoreSource = async () => {
			try {
				const { invoke } = await import("@tauri-apps/api/core");
				const source = (await invoke("mcp_shell_read_core_source")) as {
					apiBaseUrl?: string;
				};
				if (!cancelled && typeof source.apiBaseUrl === "string") {
					setApiBaseUrl(source.apiBaseUrl);
					notificationsService.reconnectAfterApiBaseChanged();
				}
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn("[App] Failed to resolve desktop core source", error);
				}
			} finally {
				if (!cancelled) {
					setDesktopSourceReady(true);
				}
			}
		};

		void syncDesktopCoreSource();

		return () => {
			cancelled = true;
		};
	}, []);

	useEffect(() => {
		if (!desktopSourceReady || backendReady) {
			return;
		}

		let cancelled = false;
		let retryTimer: number | undefined;

		function scheduleRetry(): void {
			if (cancelled) {
				return;
			}
			retryTimer = window.setTimeout(() => setAttempt((value) => value + 1), 1_000);
		}

		function reportReadinessWait(payload: unknown, error: unknown): void {
			const nowMs = Date.now();
			const reportAttempt = attempt + 1;
			const elapsedMs = nowMs - readinessStartedAtMs;
			const statusKey = backendReadinessStatusKey(
				isReadinessPayload(payload) ? payload : null,
				error,
			);
			if (
				!shouldReportBackendReadinessAttempt({
					attempt: reportAttempt,
					lastReportedAtMs: lastDiagnosticRef.current?.reportedAtMs ?? null,
					lastStatusKey: lastDiagnosticRef.current?.statusKey ?? null,
					nowMs,
					statusKey,
				})
			) {
				return;
			}

			lastDiagnosticRef.current = { reportedAtMs: nowMs, statusKey };
			const data = { attempt: reportAttempt, elapsedMs, statusKey };
			console.info("[MCPMate] waiting for backend readiness", data);
			void recordDesktopDiagnosticEvent({
				level: "info",
				source: "backend-readiness",
				message: "waiting for backend readiness",
				data,
			}).catch((error) => {
				if (import.meta.env.DEV) {
					console.warn("[MCPMate] failed to persist readiness diagnostic", error);
				}
			});
		}

		async function checkReadiness(): Promise<void> {
			setMessageKey("waitingForBackend");
			let payload: unknown = null;
			let readinessError: unknown = null;
			try {
				setMessageKey("confirmingReadiness");
				payload = await systemApi.getReadiness();
				if (cancelled) {
					return;
				}
				if (isReadinessPayload(payload) && payload.type === "ready" && payload.status === "ok") {
					setBackendReady(true);
					return;
				}
			} catch (error) {
				readinessError = error;
				// Keep retrying until the backend API is reachable.
			}
			reportReadinessWait(payload, readinessError);
			scheduleRetry();
		}

		void checkReadiness();

		return () => {
			cancelled = true;
			if (retryTimer !== undefined) {
				window.clearTimeout(retryTimer);
			}
		};
	}, [
		attempt,
		backendReady,
		desktopSourceReady,
		readinessStartedAtMs,
	]);

	if (!backendReady) {
		if (isOperatorSurfacePath()) {
			return <OperatorBackendWaitingPage messageKey={messageKey} />;
		}
		return (
			<BackendWaitingPage
				message={t(`backendReadiness.${messageKey}`, {
					defaultValue: backendReadinessFallbacks[messageKey],
				})}
			/>
		);
	}

	return <>{children}</>;
}

function isReadinessPayload(value: unknown): value is {
	reason?: string;
	status?: string;
	type?: string;
} {
	return typeof value === "object" && value !== null;
}

function isOperatorSurfacePath(): boolean {
	if (typeof window === "undefined") {
		return false;
	}
	const normalized = window.location.pathname.replace(/\/+$/, "") || "/";
	return normalized === "/operator";
}

function BackendWaitingPage({ message }: { message: string }) {
	const { t } = useTranslation();

	return (
		<div className="flex min-h-screen items-center justify-center bg-slate-50 px-6 text-slate-900 dark:bg-slate-950 dark:text-white">
			<div className="flex max-w-sm flex-col items-center text-center">
				<img
					src="/logo.svg"
					alt="MCPMate"
					className="mb-6 h-12 w-12 object-contain dark:invert dark:brightness-0"
				/>
				<div className="mb-4 h-8 w-8 animate-spin rounded-full border-2 border-slate-300 border-t-emerald-500" />
				<h1 className="text-xl font-semibold">
					{t("backendReadiness.title", { defaultValue: "MCPMate is starting" })}
				</h1>
				<p className="mt-2 text-sm text-slate-500 dark:text-slate-400">{message}</p>
			</div>
		</div>
	);
}

function LegacyPresetRedirect() {
	const { presetId } = useParams();
	return <Navigate to={`/profiles/presets/${presetId ?? ""}`} replace />;
}

function LegacySuitRedirect() {
	const { suitId } = useParams();
	return <Navigate to={`/profiles/${suitId ?? ""}`} replace />;
}

export default App;
