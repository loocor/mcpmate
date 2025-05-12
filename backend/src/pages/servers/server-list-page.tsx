import React, { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { serversApi } from '../../lib/api';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { Eye, RefreshCw, AlertCircle, Edit, Trash, Power, PowerOff, Plus } from 'lucide-react';
import { StatusBadge } from '../../components/status-badge';
import { ErrorDisplay } from '../../components/error-display';
import { ServerForm } from '../../components/server-form';
import { ConfirmDialog } from '../../components/confirm-dialog';
import { useToast } from '/src/components/ui/use-toast';
import { MCPServerConfig, ServerDetail } from '../../lib/types';

export function ServerListPage() {
  const [debugInfo, setDebugInfo] = useState<string | null>(null);
  const [isAddServerOpen, setIsAddServerOpen] = useState(false);
  const [editingServer, setEditingServer] = useState<ServerDetail | null>(null);
  const [deletingServer, setDeletingServer] = useState<string | null>(null);
  const [isDeleteConfirmOpen, setIsDeleteConfirmOpen] = useState(false);
  const [isDeleteLoading, setIsDeleteLoading] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const { toast } = useToast();
  const queryClient = useQueryClient();

  const {
    data: servers,
    isLoading,
    refetch,
    isRefetching,
    error,
    isError
  } = useQuery({
    queryKey: ['servers'],
    queryFn: async () => {
      try {
        // 添加调试信息
        console.log('Fetching servers...');
        const result = await serversApi.getAll();
        console.log('Servers fetched:', result);
        return result;
      } catch (err) {
        console.error('Error fetching servers:', err);
        // 捕获错误信息用于显示
        setDebugInfo(
          err instanceof Error
            ? `${err.message}\n\n${err.stack}`
            : String(err)
        );
        throw err;
      }
    },
    refetchInterval: 30000,
    retry: 1, // 减少重试次数，以便更快地显示错误
  });

  // 服务器详情查询
  const getServerDetails = async (serverName: string) => {
    try {
      return await serversApi.getServer(serverName);
    } catch (error) {
      console.error(`Error fetching server details for ${serverName}:`, error);
      return null;
    }
  };

  // 启用/禁用服务器
  const toggleServerMutation = useMutation({
    mutationFn: async ({ serverName, enable }: { serverName: string; enable: boolean }) => {
      if (enable) {
        return await serversApi.enableServer(serverName);
      } else {
        return await serversApi.disableServer(serverName);
      }
    },
    onSuccess: (_, variables) => {
      toast({
        title: variables.enable ? "服务器已启用" : "服务器已禁用",
        description: `服务器 ${variables.serverName} ${variables.enable ? "已成功启用" : "已成功禁用"}`,
      });
      queryClient.invalidateQueries({ queryKey: ['servers'] });
    },
    onError: (error, variables) => {
      toast({
        title: "操作失败",
        description: `无法${variables.enable ? "启用" : "禁用"}服务器: ${error instanceof Error ? error.message : String(error)}`,
        variant: "destructive",
      });
    },
  });

  // 重新连接服务器
  const reconnectServerMutation = useMutation({
    mutationFn: async (serverName: string) => {
      return await serversApi.reconnectAllInstances(serverName);
    },
    onSuccess: (_, serverName) => {
      toast({
        title: "服务器已重新连接",
        description: `服务器 ${serverName} 的所有实例已成功重新连接`,
      });
      queryClient.invalidateQueries({ queryKey: ['servers'] });
    },
    onError: (error, serverName) => {
      toast({
        title: "重新连接失败",
        description: `无法重新连接服务器 ${serverName}: ${error instanceof Error ? error.message : String(error)}`,
        variant: "destructive",
      });
    },
  });

  // 创建服务器
  const createServerMutation = useMutation({
    mutationFn: async (serverConfig: Partial<MCPServerConfig>) => {
      return await serversApi.createServer(serverConfig);
    },
    onSuccess: () => {
      toast({
        title: "服务器已创建",
        description: "新服务器已成功创建",
      });
      queryClient.invalidateQueries({ queryKey: ['servers'] });
    },
    onError: (error) => {
      toast({
        title: "创建失败",
        description: `无法创建服务器: ${error instanceof Error ? error.message : String(error)}`,
        variant: "destructive",
      });
    },
  });

  // 更新服务器
  const updateServerMutation = useMutation({
    mutationFn: async ({ serverName, config }: { serverName: string; config: Partial<MCPServerConfig> }) => {
      return await serversApi.updateServer(serverName, config);
    },
    onSuccess: (_, variables) => {
      toast({
        title: "服务器已更新",
        description: `服务器 ${variables.serverName} 已成功更新`,
      });
      queryClient.invalidateQueries({ queryKey: ['servers'] });
    },
    onError: (error, variables) => {
      toast({
        title: "更新失败",
        description: `无法更新服务器 ${variables.serverName}: ${error instanceof Error ? error.message : String(error)}`,
        variant: "destructive",
      });
    },
  });

  // 处理添加服务器
  const handleAddServer = async (serverConfig: Partial<MCPServerConfig>) => {
    await createServerMutation.mutateAsync(serverConfig);
    setIsAddServerOpen(false);
  };

  // 处理编辑服务器
  const handleEditServer = async (serverName: string) => {
    const serverDetails = await getServerDetails(serverName);
    if (serverDetails) {
      setEditingServer(serverDetails);
    } else {
      toast({
        title: "获取服务器详情失败",
        description: `无法获取服务器 ${serverName} 的详情`,
        variant: "destructive",
      });
    }
  };

  // 处理更新服务器
  const handleUpdateServer = async (config: Partial<MCPServerConfig>) => {
    if (editingServer) {
      await updateServerMutation.mutateAsync({
        serverName: editingServer.name,
        config,
      });
      setEditingServer(null);
    }
  };

  // 处理删除服务器
  const handleDeleteServer = async () => {
    if (!deletingServer) return;

    setIsDeleteLoading(true);
    setDeleteError(null);

    try {
      await serversApi.deleteServer(deletingServer);
      toast({
        title: "服务器已删除",
        description: `服务器 ${deletingServer} 已成功删除`,
      });
      queryClient.invalidateQueries({ queryKey: ['servers'] });
      setIsDeleteConfirmOpen(false);
      setDeletingServer(null);
    } catch (error) {
      setDeleteError(error instanceof Error ? error.message : "删除服务器时出错");
    } finally {
      setIsDeleteLoading(false);
    }
  };

  // 添加调试按钮处理函数
  const toggleDebugInfo = () => {
    if (debugInfo) {
      setDebugInfo(null);
    } else {
      setDebugInfo(
        `API Base URL: ${window.location.origin}\n` +
        `Current Time: ${new Date().toISOString()}\n` +
        `Error: ${error instanceof Error ? error.message : String(error)}\n` +
        `Servers Data: ${JSON.stringify(servers, null, 2)}`
      );
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold tracking-tight">服务器</h2>
        <div className="flex gap-2">
          {isError && (
            <Button
              onClick={toggleDebugInfo}
              variant="outline"
              size="sm"
            >
              <AlertCircle className="mr-2 h-4 w-4" />
              {debugInfo ? "隐藏调试" : "调试"}
            </Button>
          )}
          <Button
            onClick={() => refetch()}
            disabled={isRefetching}
            variant="outline"
            size="sm"
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${isRefetching ? 'animate-spin' : ''}`} />
            刷新
          </Button>
          <Button
            onClick={() => setIsAddServerOpen(true)}
            size="sm"
          >
            <Plus className="mr-2 h-4 w-4" />
            新增服务器
          </Button>
        </div>
      </div>

      {/* 显示错误信息 */}
      {isError && (
        <ErrorDisplay
          title="加载服务器失败"
          error={error as Error}
          onRetry={() => refetch()}
        />
      )}

      {/* 显示调试信息 */}
      {debugInfo && (
        <Card className="overflow-hidden">
          <CardHeader className="bg-slate-100 dark:bg-slate-800 p-4">
            <CardTitle className="text-lg flex justify-between">
              调试信息
              <Button
                onClick={() => setDebugInfo(null)}
                variant="ghost"
                size="sm"
              >
                关闭
              </Button>
            </CardTitle>
          </CardHeader>
          <CardContent className="p-4">
            <pre className="whitespace-pre-wrap text-xs overflow-auto max-h-96">
              {debugInfo}
            </pre>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {isLoading ? (
          // Loading skeleton
          Array.from({ length: 6 }).map((_, i) => (
            <Card key={i} className="overflow-hidden">
              <CardHeader className="p-4">
                <div className="h-6 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                <div className="h-4 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
              </CardHeader>
              <CardContent className="p-4 pt-0">
                <div className="mt-2 flex justify-between">
                  <div className="h-5 w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                  <div className="h-9 w-20 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                </div>
              </CardContent>
            </Card>
          ))
        ) : servers?.servers?.length ? (
          servers.servers.map((server) => (
            <Card key={server.name} className="overflow-hidden">
              <CardHeader className="p-4">
                <CardTitle className="text-xl">{server.name}</CardTitle>
                <CardDescription>类型: {server.kind}</CardDescription>
              </CardHeader>
              <CardContent className="p-4 pt-0">
                <div className="flex flex-col gap-3">
                  <div className="flex justify-between items-center">
                    <div className="flex flex-col">
                      {/* 使用增强的 StatusBadge 组件 */}
                      <StatusBadge status={server.status} blinkOnError={server.status === 'error'} />
                      {server.instance_count > 0 && (
                        <span className="mt-1 text-xs text-slate-500">
                          {server.instance_count} 个实例
                        </span>
                      )}
                    </div>
                    <Link to={`/servers/${server.name}`}>
                      <Button size="sm">
                        <Eye className="mr-2 h-4 w-4" />
                        详情
                      </Button>
                    </Link>
                  </div>

                  {/* 服务器操作按钮 */}
                  <div className="flex gap-2 justify-end">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => toggleServerMutation.mutate({
                        serverName: server.name,
                        enable: server.status === 'disconnected' || server.status === 'error'
                      })}
                      disabled={toggleServerMutation.isPending}
                      title={server.status === 'connected' ? "禁用服务器" : "启用服务器"}
                    >
                      {server.status === 'connected' ? (
                        <PowerOff className="h-4 w-4" />
                      ) : (
                        <Power className="h-4 w-4" />
                      )}
                    </Button>

                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => reconnectServerMutation.mutate(server.name)}
                      disabled={reconnectServerMutation.isPending}
                      title="重新连接所有实例"
                    >
                      <RefreshCw className="h-4 w-4" />
                    </Button>

                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => handleEditServer(server.name)}
                      title="编辑服务器配置"
                    >
                      <Edit className="h-4 w-4" />
                    </Button>

                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => {
                        setDeletingServer(server.name);
                        setIsDeleteConfirmOpen(true);
                      }}
                      title="删除服务器"
                    >
                      <Trash className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))
        ) : (
          <div className="col-span-full">
            <Card>
              <CardContent className="flex flex-col items-center justify-center p-6">
                <p className="mb-2 text-center text-slate-500">
                  {isError
                    ? "加载服务器失败，请查看上方错误信息。"
                    : "未找到服务器。请确保后端服务正在运行并且已配置服务器。"}
                </p>
                <Button
                  onClick={() => setIsAddServerOpen(true)}
                  size="sm"
                  className="mt-4"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  添加第一个服务器
                </Button>
              </CardContent>
            </Card>
          </div>
        )}
      </div>

      {/* 添加服务器表单 */}
      <ServerForm
        isOpen={isAddServerOpen}
        onClose={() => setIsAddServerOpen(false)}
        onSubmit={handleAddServer}
        title="添加服务器"
        submitLabel="创建"
      />

      {/* 编辑服务器表单 */}
      {editingServer && (
        <ServerForm
          isOpen={!!editingServer}
          onClose={() => setEditingServer(null)}
          onSubmit={handleUpdateServer}
          initialData={editingServer}
          title={`编辑服务器: ${editingServer.name}`}
          submitLabel="更新"
        />
      )}

      {/* 删除确认对话框 */}
      <ConfirmDialog
        isOpen={isDeleteConfirmOpen}
        onClose={() => {
          setIsDeleteConfirmOpen(false);
          setDeleteError(null);
        }}
        onConfirm={handleDeleteServer}
        title="删除服务器"
        description={`确定要删除服务器 "${deletingServer}" 吗？此操作不可撤销。`}
        confirmLabel="删除"
        cancelLabel="取消"
        variant="destructive"
        isLoading={isDeleteLoading}
        error={deleteError}
      />
    </div>
  );
}