import React, { useState } from 'react';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card';
import { Input } from '../../components/ui/input';
import { Label } from '../../components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../components/ui/tabs';
import { serversApi, toolsApi, systemApi, configApi } from '../../lib/api';

type ApiEndpoint = {
  name: string;
  description: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  path: string;
  handler: () => Promise<any>;
  requiresParams?: boolean;
};

export function ApiTestPage() {
  const [results, setResults] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [serverName, setServerName] = useState<string>('');
  const [toolName, setToolName] = useState<string>('');
  const [instanceId, setInstanceId] = useState<string>('');
  const [presetId, setPresetId] = useState<string>('');

  const serverEndpoints: ApiEndpoint[] = [
    {
      name: 'Get All Servers',
      description: 'Retrieve all servers',
      method: 'GET',
      path: '/api/mcp/servers',
      handler: () => serversApi.getAll(),
    },
    {
      name: 'Get Server Details',
      description: 'Get details for a specific server',
      method: 'GET',
      path: '/api/mcp/servers/{serverName}',
      handler: () => serversApi.getServer(serverName),
      requiresParams: true,
    },
    {
      name: 'Get Server Instances',
      description: 'Get all instances for a server',
      method: 'GET',
      path: '/api/mcp/servers/{serverName}/instances',
      handler: () => serversApi.getInstances(serverName),
      requiresParams: true,
    },
  ];

  const toolEndpoints: ApiEndpoint[] = [
    {
      name: 'Get All Tools',
      description: 'Retrieve all tools',
      method: 'GET',
      path: '/api/mcp/tools',
      handler: () => toolsApi.getAll(),
    },
    {
      name: 'Get Tool Details',
      description: 'Get details for a specific tool',
      method: 'GET',
      path: '/api/mcp/tools/{serverName}/{toolName}',
      handler: () => toolsApi.getTool(serverName, toolName),
      requiresParams: true,
    },
  ];

  const systemEndpoints: ApiEndpoint[] = [
    {
      name: 'Get System Status',
      description: 'Get current system status',
      method: 'GET',
      path: '/api/system/status',
      handler: () => systemApi.getStatus(),
    },
    {
      name: 'Get System Metrics',
      description: 'Get system metrics',
      method: 'GET',
      path: '/api/system/metrics',
      handler: () => systemApi.getMetrics(),
    },
  ];

  const configEndpoints: ApiEndpoint[] = [
    {
      name: 'Get Current Config',
      description: 'Get current active configuration',
      method: 'GET',
      path: '/api/config/current',
      handler: () => configApi.getCurrentConfig(),
    },
    {
      name: 'Get Config Presets',
      description: 'Get all configuration presets',
      method: 'GET',
      path: '/api/config/presets',
      handler: () => configApi.getPresets(),
    },
    {
      name: 'Get Config Preset',
      description: 'Get a specific configuration preset',
      method: 'GET',
      path: '/api/config/presets/{presetId}',
      handler: () => configApi.getPreset(presetId),
      requiresParams: true,
    },
  ];

  const testEndpoint = async (endpoint: ApiEndpoint) => {
    setLoading(true);
    setError(null);
    
    try {
      const result = await endpoint.handler();
      setResults(JSON.stringify(result, null, 2));
    } catch (err) {
      console.error('API test error:', err);
      setError(err instanceof Error ? err.message : String(err));
      setResults('');
    } finally {
      setLoading(false);
    }
  };

  const renderEndpointButtons = (endpoints: ApiEndpoint[]) => {
    return endpoints.map((endpoint) => {
      const isDisabled = endpoint.requiresParams && (
        (endpoint.path.includes('{serverName}') && !serverName) ||
        (endpoint.path.includes('{toolName}') && !toolName) ||
        (endpoint.path.includes('{instanceId}') && !instanceId) ||
        (endpoint.path.includes('{presetId}') && !presetId)
      );
      
      return (
        <div key={endpoint.name} className="mb-4 p-4 border rounded-md">
          <div className="flex justify-between items-start mb-2">
            <div>
              <h3 className="font-medium">{endpoint.name}</h3>
              <p className="text-sm text-slate-500">{endpoint.description}</p>
              <div className="flex items-center mt-1">
                <span className={`text-xs px-2 py-1 rounded-md mr-2 ${
                  endpoint.method === 'GET' ? 'bg-blue-100 text-blue-800' :
                  endpoint.method === 'POST' ? 'bg-green-100 text-green-800' :
                  endpoint.method === 'PUT' ? 'bg-yellow-100 text-yellow-800' :
                  'bg-red-100 text-red-800'
                }`}>
                  {endpoint.method}
                </span>
                <code className="text-xs bg-slate-100 p-1 rounded">{endpoint.path}</code>
              </div>
            </div>
            <Button 
              onClick={() => testEndpoint(endpoint)} 
              disabled={isDisabled || loading}
              size="sm"
            >
              Test
            </Button>
          </div>
          
          {endpoint.requiresParams && (
            <div className="mt-2 p-2 bg-slate-50 rounded-md">
              <p className="text-xs text-slate-500 mb-2">Required parameters:</p>
              <div className="grid gap-2">
                {endpoint.path.includes('{serverName}') && (
                  <div className="grid grid-cols-3 gap-2 items-center">
                    <Label htmlFor="serverName" className="text-xs">Server Name:</Label>
                    <Input 
                      id="serverName"
                      value={serverName}
                      onChange={(e) => setServerName(e.target.value)}
                      className="col-span-2 h-8 text-sm"
                      placeholder="Enter server name"
                    />
                  </div>
                )}
                {endpoint.path.includes('{toolName}') && (
                  <div className="grid grid-cols-3 gap-2 items-center">
                    <Label htmlFor="toolName" className="text-xs">Tool Name:</Label>
                    <Input 
                      id="toolName"
                      value={toolName}
                      onChange={(e) => setToolName(e.target.value)}
                      className="col-span-2 h-8 text-sm"
                      placeholder="Enter tool name"
                    />
                  </div>
                )}
                {endpoint.path.includes('{instanceId}') && (
                  <div className="grid grid-cols-3 gap-2 items-center">
                    <Label htmlFor="instanceId" className="text-xs">Instance ID:</Label>
                    <Input 
                      id="instanceId"
                      value={instanceId}
                      onChange={(e) => setInstanceId(e.target.value)}
                      className="col-span-2 h-8 text-sm"
                      placeholder="Enter instance ID"
                    />
                  </div>
                )}
                {endpoint.path.includes('{presetId}') && (
                  <div className="grid grid-cols-3 gap-2 items-center">
                    <Label htmlFor="presetId" className="text-xs">Preset ID:</Label>
                    <Input 
                      id="presetId"
                      value={presetId}
                      onChange={(e) => setPresetId(e.target.value)}
                      className="col-span-2 h-8 text-sm"
                      placeholder="Enter preset ID"
                    />
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      );
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold tracking-tight">API Test</h2>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div>
          <Tabs defaultValue="servers">
            <TabsList className="mb-4">
              <TabsTrigger value="servers">Servers</TabsTrigger>
              <TabsTrigger value="tools">Tools</TabsTrigger>
              <TabsTrigger value="system">System</TabsTrigger>
              <TabsTrigger value="config">Config</TabsTrigger>
            </TabsList>
            
            <TabsContent value="servers" className="mt-0">
              {renderEndpointButtons(serverEndpoints)}
            </TabsContent>
            
            <TabsContent value="tools" className="mt-0">
              {renderEndpointButtons(toolEndpoints)}
            </TabsContent>
            
            <TabsContent value="system" className="mt-0">
              {renderEndpointButtons(systemEndpoints)}
            </TabsContent>
            
            <TabsContent value="config" className="mt-0">
              {renderEndpointButtons(configEndpoints)}
            </TabsContent>
          </Tabs>
        </div>
        
        <div>
          <Card>
            <CardHeader>
              <CardTitle>Response</CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <div className="flex justify-center items-center h-64">
                  <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500"></div>
                </div>
              ) : error ? (
                <div className="bg-red-50 border border-red-200 text-red-800 p-4 rounded-md">
                  <h4 className="font-medium mb-2">Error</h4>
                  <p className="text-sm">{error}</p>
                </div>
              ) : results ? (
                <pre className="bg-slate-50 p-4 rounded-md overflow-auto max-h-[500px] text-xs">
                  {results}
                </pre>
              ) : (
                <div className="flex justify-center items-center h-64 text-slate-400">
                  Select an API endpoint to test
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
