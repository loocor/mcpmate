import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useParams, Link } from 'react-router-dom';
import { serversApi } from '../../lib/api';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { ArrowLeft, RefreshCw, Monitor, PlayCircle, StopCircle, RotateCw, XCircle } from 'lucide-react';
import { StatusBadge } from '../../components/status-badge';
import { formatRelativeTime } from '../../lib/utils';
import { useToast } from '/src/components/ui/use-toast';

export function ServerDetailPage() {
  const { serverName } = useParams<{ serverName: string }>();
  const { toast } = useToast();
  const queryClient = useQueryClient();

  const { data: server, isLoading, refetch, isRefetching } = useQuery({
    queryKey: ['server', serverName],
    queryFn: () => serversApi.getServer(serverName || ''),
    enabled: !!serverName,
    refetchInterval: 15000,
  });

  // 实例操作 mutation
  const instanceMutation = useMutation({
    mutationFn: async ({
      action,
      instanceId
    }: {
      action: 'disconnect' | 'reconnect' | 'reset' | 'cancel';
      instanceId: string;
    }) => {
      if (!serverName) throw new Error('Server name is required');

      switch (action) {
        case 'disconnect':
          return await serversApi.disconnectInstance(serverName, instanceId);
        case 'reconnect':
          return await serversApi.reconnectInstance(serverName, instanceId);
        case 'reset':
          return await serversApi.resetAndReconnectInstance(serverName, instanceId);
        case 'cancel':
          return await serversApi.cancelInstance(serverName, instanceId);
        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
    onSuccess: (_, variables) => {
      const actionMap = {
        disconnect: '已断开连接',
        reconnect: '已重新连接',
        reset: '已重置并重新连接',
        cancel: '已取消',
      };

      toast({
        title: `实例${actionMap[variables.action]}`,
        description: `实例 ${variables.instanceId.substring(0, 8)}... ${actionMap[variables.action]}成功`,
      });

      queryClient.invalidateQueries({ queryKey: ['server', serverName] });
    },
    onError: (error, variables) => {
      toast({
        title: '操作失败',
        description: `无法${variables.action === 'disconnect' ? '断开' :
          variables.action === 'reconnect' ? '重新连接' :
            variables.action === 'reset' ? '重置' : '取消'}实例: ${error instanceof Error ? error.message : String(error)}`,
        variant: 'destructive',
      });
    },
  });

  // 处理实例操作
  const handleInstanceAction = (action: 'disconnect' | 'reconnect' | 'reset' | 'cancel', instanceId: string) => {
    instanceMutation.mutate({ action, instanceId });
  };

  if (!serverName) {
    return <div>未提供服务器名称</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center">
          <Link to="/servers">
            <Button variant="ghost" size="sm" className="mr-4">
              <ArrowLeft className="mr-2 h-4 w-4" />
              返回服务器列表
            </Button>
          </Link>
          <h2 className="text-3xl font-bold tracking-tight">{serverName}</h2>
          {!isLoading && server && (
            <StatusBadge
              status={server.status}
              instances={server.instances}
              className="ml-3"
              blinkOnError={true}
            />
          )}
        </div>
        <Button
          onClick={() => refetch()}
          disabled={isRefetching}
          variant="outline"
          size="sm"
        >
          <RefreshCw className={`mr-2 h-4 w-4 ${isRefetching ? 'animate-spin' : ''}`} />
          刷新
        </Button>
      </div>

      {isLoading ? (
        <Card>
          <CardContent className="p-6">
            <div className="h-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
          </CardContent>
        </Card>
      ) : server ? (
        <>
          <Card>
            <CardHeader>
              <CardTitle>服务器配置</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <h3 className="mb-2 text-sm font-medium text-slate-500">基本信息</h3>
                  <dl className="space-y-2">
                    <div className="flex justify-between">
                      <dt className="font-medium">名称:</dt>
                      <dd>{server.name}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="font-medium">类型:</dt>
                      <dd>{server.kind}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="font-medium">状态:</dt>
                      <dd>
                        <StatusBadge
                          status={server.status}
                          instances={server.instances}
                          blinkOnError={true}
                        />
                      </dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="font-medium">活动实例:</dt>
                      <dd>{server.instances.length}</dd>
                    </div>
                  </dl>
                </div>

                {server.command && (
                  <div>
                    <h3 className="mb-2 text-sm font-medium text-slate-500">命令配置</h3>
                    <dl className="space-y-2">
                      <div className="flex justify-between">
                        <dt className="font-medium">命令:</dt>
                        <dd className="font-mono text-sm">{server.command}</dd>
                      </div>
                      {server.commandPath && (
                        <div className="flex justify-between">
                          <dt className="font-medium">路径:</dt>
                          <dd className="font-mono text-sm">{server.commandPath}</dd>
                        </div>
                      )}
                      {server.args && server.args.length > 0 && (
                        <div className="flex justify-between">
                          <dt className="font-medium">参数:</dt>
                          <dd className="font-mono text-sm">{server.args.join(' ')}</dd>
                        </div>
                      )}
                    </dl>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>实例 ({server.instances.length})</CardTitle>
              <CardDescription>
                此服务器的所有实例列表
              </CardDescription>
            </CardHeader>
            <CardContent>
              {server.instances.length > 0 ? (
                <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                  {server.instances.map((instance) => (
                    <Card key={instance.id} className="overflow-hidden">
                      <CardHeader className="p-4">
                        <div className="flex items-center justify-between">
                          <CardTitle className="text-sm font-medium truncate" title={instance.id}>
                            {instance.id.substring(0, 8)}...
                          </CardTitle>
                          <StatusBadge status={instance.status} blinkOnError={instance.status === 'error'} />
                        </div>
                        {instance.startTime && (
                          <CardDescription>
                            启动于 {formatRelativeTime(instance.startTime)}
                          </CardDescription>
                        )}
                      </CardHeader>
                      <CardContent className="p-4 pt-0">
                        <div className="mt-2 flex flex-wrap gap-2">
                          <Link to={`/servers/${serverName}/instances/${instance.id}`}>
                            <Button size="sm" variant="outline">
                              <Monitor className="mr-2 h-4 w-4" />
                              详情
                            </Button>
                          </Link>

                          {instance.status === 'initializing' ? (
                            <Button
                              size="sm"
                              variant="destructive"
                              onClick={() => handleInstanceAction('cancel', instance.id)}
                              disabled={instanceMutation.isPending}
                            >
                              <XCircle className="mr-2 h-4 w-4" />
                              取消
                            </Button>
                          ) : instance.status === 'running' ? (
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => handleInstanceAction('disconnect', instance.id)}
                              disabled={instanceMutation.isPending}
                            >
                              <StopCircle className="mr-2 h-4 w-4" />
                              断开连接
                            </Button>
                          ) : (
                            <>
                              <Button
                                size="sm"
                                variant="secondary"
                                onClick={() => handleInstanceAction('reconnect', instance.id)}
                                disabled={instanceMutation.isPending}
                              >
                                <PlayCircle className="mr-2 h-4 w-4" />
                                重新连接
                              </Button>
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => handleInstanceAction('reset', instance.id)}
                                disabled={instanceMutation.isPending}
                              >
                                <RotateCw className="mr-2 h-4 w-4" />
                                重置并重连
                              </Button>
                            </>
                          )}
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              ) : (
                <p className="text-center text-slate-500">此服务器没有可用的实例。</p>
              )}
            </CardContent>
          </Card>
        </>
      ) : (
        <Card>
          <CardContent className="p-6">
            <p className="text-center text-slate-500">未找到服务器或加载服务器详情时出错。</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}