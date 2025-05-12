import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { configApi } from '../../lib/api';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { ArrowLeft, Save, Trash2, Check } from 'lucide-react';

export function ConfigPresetPage() {
  const { presetId } = useParams<{ presetId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const isNew = presetId === 'new';
  
  const { data: preset, isLoading } = useQuery({
    queryKey: ['configPreset', presetId],
    queryFn: () => configApi.getPreset(presetId!),
    enabled: !isNew && !!presetId,
  });
  
  const createMutation = useMutation({
    mutationFn: configApi.createPreset,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['configPresets'] });
      navigate('/config');
    },
  });
  
  const updateMutation = useMutation({
    mutationFn: ({ id, preset }: { id: string; preset: any }) => 
      configApi.updatePreset(id, preset),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['configPresets', 'configPreset'] });
    },
  });
  
  const deleteMutation = useMutation({
    mutationFn: configApi.deletePreset,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['configPresets'] });
      navigate('/config');
    },
  });
  
  const applyMutation = useMutation({
    mutationFn: configApi.applyPreset,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['currentConfig', 'configPresets', 'configPreset'] });
    },
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center">
          <Link to="/config">
            <Button variant="ghost" size="sm" className="mr-4">
              <ArrowLeft className="mr-2 h-4 w-4" />
              Back to Configuration
            </Button>
          </Link>
          <h2 className="text-2xl font-bold tracking-tight">
            {isNew ? 'New Configuration Preset' : preset?.name}
          </h2>
          {!isNew && preset?.is_active && (
            <span className="ml-3 flex items-center rounded-full bg-emerald-50 px-3 py-1 text-sm font-medium text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400">
              <Check className="mr-2 h-4 w-4" />
              Active
            </span>
          )}
        </div>
        {!isNew && preset && (
          <div className="flex gap-2">
            <Button
              variant="destructive"
              size="sm"
              disabled={deleteMutation.isPending}
              onClick={() => {
                if (confirm('Are you sure you want to delete this preset?')) {
                  deleteMutation.mutate(preset.id);
                }
              }}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={preset.is_active || applyMutation.isPending}
              onClick={() => applyMutation.mutate(preset.id)}
            >
              Apply Preset
            </Button>
          </div>
        )}
      </div>
      
      {isLoading ? (
        <Card>
          <CardContent className="p-6">
            <div className="h-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-6">
          <Card>
            <CardHeader>
              <CardTitle>Basic Information</CardTitle>
              <CardDescription>
                General information about this configuration preset
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {/* Form fields for name, description, etc. */}
                <p className="text-center text-slate-500">Form implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
          
          <Card>
            <CardHeader>
              <CardTitle>Global Settings</CardTitle>
              <CardDescription>
                System-wide configuration parameters
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {/* Form fields for global settings */}
                <p className="text-center text-slate-500">Form implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
          
          <Card>
            <CardHeader>
              <CardTitle>Server Configurations</CardTitle>
              <CardDescription>
                MCP server definitions and their settings
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {/* Server configuration form/list */}
                <p className="text-center text-slate-500">Form implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
          
          <Card>
            <CardHeader>
              <CardTitle>Tool Configurations</CardTitle>
              <CardDescription>
                Tool-specific settings and parameters
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {/* Tool configuration form/list */}
                <p className="text-center text-slate-500">Form implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
          
          <div className="flex justify-end gap-4">
            <Button
              variant="outline"
              onClick={() => navigate('/config')}
            >
              Cancel
            </Button>
            <Button
              onClick={() => {
                // Handle save
              }}
              disabled={createMutation.isPending || updateMutation.isPending}
            >
              <Save className="mr-2 h-4 w-4" />
              {isNew ? 'Create Preset' : 'Save Changes'}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}