import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { serversApi, systemApi, toolsApi, configApi } from '../../lib/api';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { Activity, Server, Wrench, Sliders } from 'lucide-react';
import { StatusBadge } from '../../components/status-badge';
import { formatUptime } from '../../lib/utils';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

const mockMetricsHistory = Array.from({ length: 12 }, (_, i) => ({
  time: new Date(Date.now() - (11 - i) * 5 * 60 * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
  cpu: Math.random() * 30 + 5,
  memory: Math.random() * 40 + 20,
  connections: Math.floor(Math.random() * 20),
}));

export function DashboardPage() {
  const { data: systemStatus, isLoading: isLoadingSystem } = useQuery({
    queryKey: ['systemStatus'],
    queryFn: systemApi.getStatus,
    refetchInterval: 30000,
  });

  const { data: servers, isLoading: isLoadingServers } = useQuery({
    queryKey: ['servers'],
    queryFn: serversApi.getAll,
    refetchInterval: 30000,
  });

  const { data: tools, isLoading: isLoadingTools } = useQuery({
    queryKey: ['tools'],
    queryFn: toolsApi.getAll,
    refetchInterval: 30000,
  });

  const {
    data: currentConfig,
    isLoading: isLoadingConfig,
    isError: isConfigError
  } = useQuery({
    queryKey: ['currentConfig'],
    queryFn: configApi.getCurrentConfig,
    refetchInterval: 30000,
    // Don't retry too many times for config API since we have fallback
    retry: 1,
    // Don't show error in React Query devtools
    useErrorBoundary: false,
  });

  const connectedServers = servers?.servers?.filter(server => server.status === 'connected').length || 0;
  const enabledTools = tools?.tools?.filter(tool => tool.is_enabled).length || 0;

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">System Status</CardTitle>
            <Activity className="h-4 w-4 text-slate-500" />
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <CardDescription>Status</CardDescription>
                {isLoadingSystem ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <StatusBadge status={systemStatus?.status || 'unknown'} />
                )}
              </div>
              <div className="flex items-center justify-between">
                <CardDescription>Uptime</CardDescription>
                {isLoadingSystem ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <span className="text-sm font-medium">{formatUptime(systemStatus?.uptime || 0)}</span>
                )}
              </div>
              <div className="flex items-center justify-between">
                <CardDescription>Version</CardDescription>
                {isLoadingSystem ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <span className="text-sm font-medium">{systemStatus?.version || 'Unknown'}</span>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Current Suit</CardTitle>
            <Sliders className="h-4 w-4 text-slate-500" />
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {isLoadingConfig ? (
                <div className="space-y-2">
                  <div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                  <div className="h-5 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                </div>
              ) : currentConfig ? (
                <>
                  <div className="flex items-center justify-between">
                    <CardDescription>Max Connections</CardDescription>
                    <span className="text-sm font-medium">
                      {currentConfig.global_settings.max_concurrent_connections}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <CardDescription>Log Level</CardDescription>
                    <span className="text-sm font-medium">
                      {currentConfig.global_settings.log_level}
                    </span>
                  </div>
                </>
              ) : (
                <p className="text-sm text-slate-500">No configuration loaded</p>
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Servers</CardTitle>
            <Server className="h-4 w-4 text-slate-500" />
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <CardDescription>Total Servers</CardDescription>
                {isLoadingServers ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <span className="text-2xl font-bold">{servers?.servers?.length || 0}</span>
                )}
              </div>
              <div className="flex items-center justify-between">
                <CardDescription>Connected</CardDescription>
                {isLoadingServers ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <span className="text-2xl font-bold">{connectedServers}</span>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Tools</CardTitle>
            <Wrench className="h-4 w-4 text-slate-500" />
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <CardDescription>Total Tools</CardDescription>
                {isLoadingTools ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <span className="text-2xl font-bold">{tools?.tools?.length || 0}</span>
                )}
              </div>
              <div className="flex items-center justify-between">
                <CardDescription>Enabled</CardDescription>
                {isLoadingTools ? (
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                ) : (
                  <span className="text-2xl font-bold">{enabledTools}</span>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card className="col-span-1 md:col-span-2">
          <CardHeader>
            <CardTitle>System Performance</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart
                  data={mockMetricsHistory}
                  margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
                >
                  <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                  <XAxis
                    dataKey="time"
                    stroke="#9ca3af"
                    fontSize={12}
                  />
                  <YAxis stroke="#9ca3af" fontSize={12} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#1f2937',
                      border: '1px solid #4b5563',
                      borderRadius: '6px',
                      color: '#e5e7eb'
                    }}
                  />
                  <Line
                    type="monotone"
                    dataKey="cpu"
                    stroke="#3b82f6"
                    name="CPU (%)"
                    strokeWidth={2}
                    dot={false}
                    activeDot={{ r: 6 }}
                  />
                  <Line
                    type="monotone"
                    dataKey="memory"
                    stroke="#10b981"
                    name="Memory (%)"
                    strokeWidth={2}
                    dot={false}
                    activeDot={{ r: 6 }}
                  />
                  <Line
                    type="monotone"
                    dataKey="connections"
                    stroke="#f59e0b"
                    name="Connections"
                    strokeWidth={2}
                    dot={false}
                    activeDot={{ r: 6 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Recent Servers</CardTitle>
          </CardHeader>
          <CardContent>
            {isLoadingServers ? (
              <div className="space-y-2">
                {Array.from({ length: 3 }).map((_, i) => (
                  <div key={i} className="flex items-center justify-between rounded-md border p-3">
                    <div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                    <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                  </div>
                ))}
              </div>
            ) : servers?.servers?.length ? (
              <div className="space-y-2">
                {servers.servers.slice(0, 5).map((server) => (
                  <div key={server.name} className="flex items-center justify-between rounded-md border p-3">
                    <span className="font-medium">{server.name}</span>
                    <StatusBadge status={server.status} />
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-center text-slate-500">No servers available</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent Tools</CardTitle>
          </CardHeader>
          <CardContent>
            {isLoadingTools ? (
              <div className="space-y-2">
                {Array.from({ length: 3 }).map((_, i) => (
                  <div key={i} className="flex items-center justify-between rounded-md border p-3">
                    <div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                    <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                  </div>
                ))}
              </div>
            ) : tools?.tools?.length ? (
              <div className="space-y-2">
                {tools.tools.slice(0, 5).map((tool) => (
                  <div key={`${tool.server_name}-${tool.tool_name}`} className="flex items-center justify-between rounded-md border p-3">
                    <div>
                      <span className="font-medium">{tool.tool_name}</span>
                      <span className="ml-2 text-xs text-slate-500">{tool.server_name}</span>
                    </div>
                    <StatusBadge status={tool.is_enabled ? "enabled" : "disabled"} showLabel={true} />
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-center text-slate-500">No tools available</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}