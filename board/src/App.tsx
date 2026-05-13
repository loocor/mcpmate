import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useState, type ReactNode } from "react";
import {
	BrowserRouter,
	Navigate,
	Route,
	Routes,
	useParams,
} from "react-router-dom";
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
import { InstanceDetailPage } from "./pages/servers/instance-detail-page";
import { ServerDetailPage } from "./pages/servers/server-detail-page";
import { OAuthCallbackPage } from "./pages/servers/oauth-callback-page";
import { ServerListPage } from "./pages/servers/server-list-page";
import { SettingsPage } from "./pages/settings/settings-page";
import { OnboardingPage } from "./pages/onboarding/onboarding-page";
import {
	notificationsService,
	setApiBaseUrl,
	systemApi,
} from "./lib/api";
import { isTauriEnvironmentSync } from "./lib/platform";

const ClientDetailPage = lazy(() =>
	import("./pages/clients/client-detail-page").then((module) => ({
		default: module.ClientDetailPage,
	})),
);

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
					<Routes>
						<Route path="oauth/callback" element={<OAuthCallbackPage />} />
						<Route path="onboarding" element={<OnboardingPage />} />
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
							<Route path="audit" element={<AuditPage />} />
							<Route path="api-docs" element={<ApiDocsPage />} />
							<Route path="account" element={<Navigate to="/" replace />} />
							<Route path="settings" element={<SettingsPage />} />

							<Route path="404" element={<NotFoundPage />} />
							<Route path="*" element={<Navigate to="/404" replace />} />
						</Route>
					</Routes>
				</BrowserRouter>
			</BackendReadinessGate>
		</QueryClientProvider>
	);
}

function BackendReadinessGate({ children }: { children: ReactNode }) {
	const [desktopSourceReady, setDesktopSourceReady] = useState(
		() => !isTauriEnvironmentSync(),
	);
	const [backendReady, setBackendReady] = useState(false);
	const [attempt, setAttempt] = useState(0);
	const [message, setMessage] = useState("Starting MCPMate Core...");

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

		async function checkReadiness(): Promise<void> {
			setMessage("Waiting for MCPMate backend");
			try {
				setMessage("Confirming backend readiness...");
				const payload = await systemApi.getReadiness();
				if (cancelled) {
					return;
				}
				if (payload.type === "ready" && payload.status === "ok") {
					setBackendReady(true);
					return;
				}
			} catch {
				// Keep retrying until the backend API is reachable.
			}
			scheduleRetry();
		}

		void checkReadiness();

		return () => {
			cancelled = true;
			if (retryTimer !== undefined) {
				window.clearTimeout(retryTimer);
			}
		};
	}, [desktopSourceReady, backendReady, attempt]);

	if (!backendReady) {
		return <BackendWaitingPage message={message} />;
	}

	return <>{children}</>;
}

function BackendWaitingPage({ message }: { message: string }) {
	return (
		<div className="flex min-h-screen items-center justify-center bg-slate-50 px-6 text-slate-900 dark:bg-slate-950 dark:text-white">
			<div className="flex max-w-sm flex-col items-center text-center">
				<img
					src="/logo.svg"
					alt="MCPMate"
					className="mb-6 h-12 w-12 object-contain dark:invert dark:brightness-0"
				/>
				<div className="mb-4 h-8 w-8 animate-spin rounded-full border-2 border-slate-300 border-t-emerald-500" />
				<h1 className="text-xl font-semibold">MCPMate is starting</h1>
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
