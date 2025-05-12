import React from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Layout } from './components/layout/layout';
import { DashboardPage } from './pages/dashboard/dashboard-page';
import { ConfigPage } from './pages/config/config-page';
import { ConfigPresetPage } from './pages/config/config-preset-page';
import { ServerListPage } from './pages/servers/server-list-page';
import { ServerDetailPage } from './pages/servers/server-detail-page';
import { InstanceDetailPage } from './pages/servers/instance-detail-page';
import { ToolsPage } from './pages/tools/tools-page';
import { SystemPage } from './pages/system/system-page';
import { SettingsPage } from './pages/settings/settings-page';
import { NotFoundPage } from './pages/not-found-page';
import { ApiTestPage } from './pages/debug/api-test-page';
import { Toaster } from './components/ui/toaster';

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
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<DashboardPage />} />
            <Route path="config" element={<ConfigPage />} />
            <Route path="config/presets/:presetId" element={<ConfigPresetPage />} />
            <Route path="servers" element={<ServerListPage />} />
            <Route path="servers/:serverName" element={<ServerDetailPage />} />
            <Route path="servers/:serverName/instances/:instanceId" element={<InstanceDetailPage />} />
            <Route path="tools" element={<ToolsPage />} />
            <Route path="system" element={<SystemPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="debug/api-test" element={<ApiTestPage />} />
            <Route path="404" element={<NotFoundPage />} />
            <Route path="*" element={<Navigate to="/404" replace />} />
          </Route>
        </Routes>
        <Toaster />
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;