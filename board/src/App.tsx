import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	BrowserRouter,
	Navigate,
	Route,
	Routes,
	useParams,
} from "react-router-dom";
import { Layout } from "./components/layout/layout";
import { LanguageSynchronizer } from "./components/language-synchronizer.ts";
import { ClientDetailPage } from "./pages/clients/client-detail-page";
import { ClientsPage } from "./pages/clients/clients-page";
import { DashboardPage } from "./pages/dashboard/dashboard-page";
import { MarketPage } from "./pages/market/market-page";
import { NotFoundPage } from "./pages/not-found-page";
import { ProfileDetailPage } from "./pages/profile/profile-detail-page";
import { ProfilePage } from "./pages/profile/profile-page";
import { ProfilePresetPage } from "./pages/profile/profile-preset-page";
import { RuntimePage } from "./pages/runtime/runtime-page";
import { InstanceDetailPage } from "./pages/servers/instance-detail-page";
import { ServerDetailPage } from "./pages/servers/server-detail-page";
import { ServerListPage } from "./pages/servers/server-list-page";
import { SettingsPage } from "./pages/settings/settings-page";

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
			<BrowserRouter>
				<Routes>
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
						<Route path="servers" element={<ServerListPage />} />
						<Route path="servers/:serverId" element={<ServerDetailPage />} />
						<Route
							path="servers/:serverId/instances/:instanceId"
							element={<InstanceDetailPage />}
						/>
						{/* Tools route removed */}
						<Route path="clients" element={<ClientsPage />} />
						<Route path="clients/:identifier" element={<ClientDetailPage />} />
						<Route path="runtime" element={<RuntimePage />} />
						<Route path="settings" element={<SettingsPage />} />

						<Route path="404" element={<NotFoundPage />} />
						<Route path="*" element={<Navigate to="/404" replace />} />
					</Route>
				</Routes>
			</BrowserRouter>
		</QueryClientProvider>
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
