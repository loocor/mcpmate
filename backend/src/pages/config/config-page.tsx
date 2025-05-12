import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { configApi } from '../../lib/api';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { RefreshCw, Plus, Save, Clock, Check } from 'lucide-react';
import { formatRelativeTime } from '../../lib/utils';

export function ConfigPage() {
  const queryClient = useQueryClient();

  const {
    data: currentConfig,
    isLoading: isLoadingConfig,
    refetch,
    isRefetching,
    isError: isConfigError
  } = useQuery({
    queryKey: ['currentConfig'],
    queryFn: configApi.getCurrentConfig,
    retry: 1,
    useErrorBoundary: false,
  });

  const {
    data: presets,
    isLoading: isLoadingPresets,
    isError: isPresetsError
  } = useQuery({
    queryKey: ['configPresets'],
    queryFn: configApi.getPresets,
    retry: 1,
    useErrorBoundary: false,
  });

  const updateMutation = useMutation({
    mutationFn: configApi.updateConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['currentConfig'] });
    },
  });

  const applyPresetMutation = useMutation({
    mutationFn: configApi.applyPreset,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['currentConfig', 'configPresets'] });
    },
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold tracking-tight">Configuration</h2>
        <div className="flex gap-2">
          <Button
            onClick={() => refetch()}
            disabled={isRefetching}
            variant="outline"
            size="sm"
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${isRefetching ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
          <Link to="/config/presets/new">
            <Button size="sm">
              <Plus className="mr-2 h-4 w-4" />
              New Preset
            </Button>
          </Link>
        </div>
      </div>

      <div className="grid gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Current Configuration</CardTitle>
            <CardDescription>
              Active configuration settings for MCPMate
            </CardDescription>
          </CardHeader>
          <CardContent>
            {isLoadingConfig ? (
              <div className="space-y-4">
                <div className="h-8 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                <div className="h-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
              </div>
            ) : currentConfig ? (
              <div className="space-y-6">
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  <div>
                    <h3 className="mb-2 text-sm font-medium text-slate-500">Global Settings</h3>
                    <dl className="space-y-1">
                      <div className="flex justify-between">
                        <dt className="text-sm">Max Connections:</dt>
                        <dd className="font-medium">{currentConfig.global_settings.max_concurrent_connections}</dd>
                      </div>
                      <div className="flex justify-between">
                        <dt className="text-sm">Request Timeout:</dt>
                        <dd className="font-medium">{currentConfig.global_settings.request_timeout_ms}ms</dd>
                      </div>
                      <div className="flex justify-between">
                        <dt className="text-sm">Metrics Enabled:</dt>
                        <dd className="font-medium">{currentConfig.global_settings.enable_metrics ? 'Yes' : 'No'}</dd>
                      </div>
                      <div className="flex justify-between">
                        <dt className="text-sm">Log Level:</dt>
                        <dd className="font-medium">{currentConfig.global_settings.log_level}</dd>
                      </div>
                    </dl>
                  </div>

                  <div>
                    <h3 className="mb-2 text-sm font-medium text-slate-500">Servers</h3>
                    <p className="text-sm text-slate-600 dark:text-slate-400">
                      {currentConfig.servers.length} configured server{currentConfig.servers.length !== 1 ? 's' : ''}
                    </p>
                  </div>

                  <div>
                    <h3 className="mb-2 text-sm font-medium text-slate-500">Tools</h3>
                    <p className="text-sm text-slate-600 dark:text-slate-400">
                      {currentConfig.tools.length} configured tool{currentConfig.tools.length !== 1 ? 's' : ''}
                    </p>
                  </div>
                </div>

                <div className="flex justify-end">
                  <Button
                    onClick={() => {/* Open edit modal */ }}
                    disabled={updateMutation.isPending}
                  >
                    <Save className="mr-2 h-4 w-4" />
                    Edit Configuration
                  </Button>
                </div>
              </div>
            ) : (
              <p className="text-center text-slate-500">Error loading current configuration</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Configuration Presets</CardTitle>
            <CardDescription>
              Saved configuration presets that can be quickly applied
            </CardDescription>
          </CardHeader>
          <CardContent>
            {isLoadingPresets ? (
              <div className="space-y-4">
                {Array.from({ length: 3 }).map((_, i) => (
                  <div key={i} className="flex items-center justify-between rounded-lg border p-4">
                    <div className="space-y-1">
                      <div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                      <div className="h-4 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                    </div>
                    <div className="h-9 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                  </div>
                ))}
              </div>
            ) : presets?.length ? (
              <div className="space-y-4">
                {presets.map((preset) => (
                  <div key={preset.id} className="flex items-center justify-between rounded-lg border p-4">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <h3 className="font-medium">{preset.name}</h3>
                        {preset.is_active && (
                          <span className="flex items-center rounded-full bg-emerald-50 px-2 py-1 text-xs font-medium text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400">
                            <Check className="mr-1 h-3 w-3" />
                            Active
                          </span>
                        )}
                      </div>
                      {preset.description && (
                        <p className="text-sm text-slate-500">{preset.description}</p>
                      )}
                      <p className="text-xs text-slate-400">
                        <Clock className="mr-1 inline-block h-3 w-3" />
                        Last updated {formatRelativeTime(preset.updated_at)}
                      </p>
                    </div>
                    <div className="flex gap-2">
                      <Link to={`/config/presets/${preset.id}`}>
                        <Button variant="outline" size="sm">View</Button>
                      </Link>
                      <Button
                        variant="secondary"
                        size="sm"
                        disabled={preset.is_active || applyPresetMutation.isPending}
                        onClick={() => applyPresetMutation.mutate(preset.id)}
                      >
                        Apply
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-center text-slate-500">No configuration presets available</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}