import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toolsApi } from '../../lib/api';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card';
import { RefreshCw, Search, Wrench } from 'lucide-react';
import { Switch } from '../../components/ui/switch';

export function ToolsPage() {
  const [searchTerm, setSearchTerm] = React.useState('');
  const queryClient = useQueryClient();

  const {
    data: tools,
    isLoading,
    refetch,
    isRefetching
  } = useQuery({
    queryKey: ['tools'],
    queryFn: toolsApi.getAll,
    refetchInterval: 30000,
  });

  const enableMutation = useMutation({
    mutationFn: ({ serverName, toolName }: { serverName: string; toolName: string }) =>
      toolsApi.enableTool(serverName, toolName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tools'] });
    },
  });

  const disableMutation = useMutation({
    mutationFn: ({ serverName, toolName }: { serverName: string; toolName: string }) =>
      toolsApi.disableTool(serverName, toolName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tools'] });
    },
  });

  const handleToggleTool = (tool: { server_name: string; tool_name: string; is_enabled: boolean }) => {
    if (tool.is_enabled) {
      disableMutation.mutate({ serverName: tool.server_name, toolName: tool.tool_name });
    } else {
      enableMutation.mutate({ serverName: tool.server_name, toolName: tool.tool_name });
    }
  };

  const filteredTools = tools?.tools?.filter(tool =>
    tool.tool_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    tool.server_name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold tracking-tight">Tools</h2>
        <Button
          onClick={() => refetch()}
          disabled={isRefetching}
          variant="outline"
          size="sm"
        >
          <RefreshCw className={`mr-2 h-4 w-4 ${isRefetching ? 'animate-spin' : ''}`} />
          Refresh
        </Button>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
        <input
          type="text"
          placeholder="Search tools..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="w-full rounded-md border border-slate-200 bg-white py-2 pl-10 pr-4 text-sm placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-slate-300 dark:border-slate-800 dark:bg-slate-950 dark:placeholder:text-slate-400 dark:focus:ring-slate-600"
        />
      </div>

      <div className="grid gap-4">
        {isLoading ? (
          <Card>
            <CardContent className="p-6">
              <div className="space-y-4">
                {Array.from({ length: 5 }).map((_, i) => (
                  <div key={i} className="flex items-center justify-between rounded-md border p-4">
                    <div className="space-y-1">
                      <div className="h-5 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                      <div className="h-4 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                    </div>
                    <div className="h-6 w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        ) : filteredTools?.length ? (
          <Card>
            <CardHeader>
              <CardTitle>
                <div className="flex items-center">
                  <Wrench className="mr-2 h-5 w-5" />
                  Available Tools ({filteredTools.length})
                </div>
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {filteredTools.map((tool) => (
                  <div
                    key={`${tool.server_name}-${tool.tool_name}`}
                    className="flex items-center justify-between rounded-md border p-4"
                  >
                    <div className="space-y-1">
                      <div className="flex items-center">
                        <h3 className="font-medium">{tool.tool_name}</h3>
                        <span className="ml-2 rounded bg-slate-100 px-2 py-0.5 text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-300">
                          {tool.server_name}
                        </span>
                      </div>
                      {tool.description && (
                        <p className="text-sm text-slate-500">{tool.description}</p>
                      )}
                    </div>
                    <div className="flex items-center">
                      <span className="mr-2 text-sm">
                        {tool.is_enabled ? 'Enabled' : 'Disabled'}
                      </span>
                      <Switch
                        checked={tool.is_enabled}
                        onCheckedChange={() => handleToggleTool(tool)}
                        disabled={enableMutation.isPending || disableMutation.isPending}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        ) : (
          <Card>
            <CardContent className="flex flex-col items-center justify-center p-12">
              <p className="mb-1 text-center text-lg font-medium">No tools found</p>
              {searchTerm ? (
                <p className="text-center text-slate-500">
                  No tools match your search criteria. Try searching for something else.
                </p>
              ) : (
                <p className="text-center text-slate-500">
                  No tools are currently available from connected servers.
                </p>
              )}
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}