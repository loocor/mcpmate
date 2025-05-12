import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { useParams, Link } from 'react-router-dom';
import { serversApi } from '../../lib/api';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { ArrowLeft, RefreshCw, StopCircle, PlayCircle, RotateCw, Shield } from 'lucide-react';
import { StatusBadge } from '../../components/status-badge';
import { formatRelativeTime } from '../../lib/utils';

export function InstanceDetailPage() {
  const { serverName, instanceId } = useParams<{ serverName: string; instanceId: string }>();
  
  const { 
    data: instance, 
    isLoading, 
    refetch, 
    isRefetching 
  } = useQuery({
    queryKey: ['instance', serverName, instanceId],
    queryFn: () => serversApi.getInstance(serverName || '', instanceId || ''),
    enabled: !!serverName && !!instanceId,
    refetchInterval: 10000,
  });
  
  const { 
    data: health, 
    isLoading: isLoadingHealth 
  } = useQuery({
    queryKey: ['instanceHealth', serverName, instanceId],
    queryFn: () => serversApi.getInstanceHealth(serverName || '', instanceId || ''),
    enabled: !!serverName && !!instanceId,
    refetchInterval: 10000,
  });
  
  if (!serverName || !instanceId) {
    return <div>Server name or instance ID not provided</div>;
  }
  
  const handleDisconnect = () => {
    serversApi.disconnectInstance(serverName, instanceId)
      .then(() => refetch());
  };
  
  const handleForceDisconnect = () => {
    serversApi.forceDisconnectInstance(serverName, instanceId)
      .then(() => refetch());
  };
  
  const handleReconnect = () => {
    serversApi.reconnectInstance(serverName, instanceId)
      .then(() => refetch());
  };
  
  const handleResetAndReconnect = () => {
    serversApi.resetAndReconnectInstance(serverName, instanceId)
      .then(() => refetch());
  };
  
  const handleCancel = () => {
    serversApi.cancelInstance(serverName, instanceId)
      .then(() => refetch());
  };
  
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center">
          <Link to={`/servers/${serverName}`}>
            <Button variant="ghost" size="sm" className="mr-4">
              <ArrowLeft className="mr-2 h-4 w-4" />
              Back to Server
            </Button>
          </Link>
          <h2 className="text-2xl font-bold tracking-tight truncate" title={instanceId}>
            {instanceId.substring(0, 12)}...
          </h2>
          {!isLoading && instance && <StatusBadge status={instance.status} className="ml-3" />}
        </div>
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
      
      {isLoading ? (
        <Card>
          <CardContent className="p-6">
            <div className="h-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
          </CardContent>
        </Card>
      ) : instance ? (
        <>
          <div className="grid gap-4 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Instance Details</CardTitle>
              </CardHeader>
              <CardContent>
                <dl className="space-y-2">
                  <div className="flex justify-between">
                    <dt className="font-medium">Instance ID:</dt>
                    <dd className="font-mono text-sm">{instance.id}</dd>
                  </div>
                  <div className="flex justify-between">
                    <dt className="font-medium">Server:</dt>
                    <dd>{instance.server_name}</dd>
                  </div>
                  <div className="flex justify-between">
                    <dt className="font-medium">Status:</dt>
                    <dd><StatusBadge status={instance.status} /></dd>
                  </div>
                  {instance.startTime && (
                    <div className="flex justify-between">
                      <dt className="font-medium">Started:</dt>
                      <dd>{formatRelativeTime(instance.startTime)}</dd>
                    </div>
                  )}
                  <div className="flex justify-between">
                    <dt className="font-medium">Health:</dt>
                    <dd>
                      {isLoadingHealth ? (
                        <div className="h-5 w-20 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
                      ) : (
                        <StatusBadge 
                          status={health?.is_healthy ? 'healthy' : 'unhealthy'} 
                        />
                      )}
                    </dd>
                  </div>
                </dl>
              </CardContent>
            </Card>
            
            <Card>
              <CardHeader>
                <CardTitle>Instance Controls</CardTitle>
                <CardDescription>Manage the instance connection state</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex flex-wrap gap-3">
                  {instance.status === 'initializing' ? (
                    <Button variant="destructive" onClick={handleCancel}>
                      <Shield className="mr-2 h-4 w-4" />
                      Cancel Initialization
                    </Button>
                  ) : instance.status === 'running' ? (
                    <>
                      <Button variant="secondary" onClick={handleDisconnect}>
                        <StopCircle className="mr-2 h-4 w-4" />
                        Disconnect
                      </Button>
                      <Button variant="destructive" onClick={handleForceDisconnect}>
                        <Shield className="mr-2 h-4 w-4" />
                        Force Disconnect
                      </Button>
                    </>
                  ) : (
                    <>
                      <Button onClick={handleReconnect}>
                        <PlayCircle className="mr-2 h-4 w-4" />
                        Reconnect
                      </Button>
                      <Button variant="outline" onClick={handleResetAndReconnect}>
                        <RotateCw className="mr-2 h-4 w-4" />
                        Reset & Reconnect
                      </Button>
                    </>
                  )}
                </div>
                
                {health && !health.is_healthy && health.details && (
                  <div className="mt-4">
                    <p className="text-sm font-medium text-red-500">Health issue detected:</p>
                    <p className="mt-1 rounded bg-red-50 p-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-200">
                      {health.details}
                    </p>
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
          
          <Card>
            <CardHeader>
              <CardTitle>Instance Metrics</CardTitle>
              <CardDescription>
                Performance metrics and statistics for this instance
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-center text-slate-500 py-6">
                Instance metrics visualization to be implemented
              </p>
            </CardContent>
          </Card>
        </>
      ) : (
        <Card>
          <CardContent className="p-6">
            <p className="text-center text-slate-500">Instance not found or error loading instance details.</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}