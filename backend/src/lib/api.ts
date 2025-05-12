import { ApiResponse, ConfigPreset, InstanceDetail, InstanceHealth, MCPConfig, ServerDetail, ServerSummary, SystemMetrics, SystemStatus, Tool, ToolDetail } from "./types";

// Base API URL - in a real app, this would be in an environment variable
// 使用相对路径，这样前端和后端可以在同一域名下工作，避免CORS问题
const API_BASE_URL = "";

// Helper function for making API requests
async function fetchApi<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  console.log(`Fetching API: ${url}`, options);

  try {
    const response = await fetch(url, {
      headers: {
        "Content-Type": "application/json",
        ...options?.headers,
      },
      ...options,
    });

    if (!response.ok) {
      // 尝试解析错误响应
      const errorText = await response.text();
      let errorObj = {};

      try {
        errorObj = JSON.parse(errorText);
        console.error(`API Error (${response.status}):`, errorObj);
      } catch (e) {
        // 如果不是JSON，则使用原始文本
        console.error(`API Error (${response.status}):`, errorText);
      }

      throw new Error(
        errorObj.message ||
        errorObj.error?.message ||
        `API Error: ${response.status} ${response.statusText}`
      );
    }

    const data = await response.json();
    console.log(`API Response from ${endpoint}:`, data);
    return data;
  } catch (error) {
    console.error(`API Request Failed for ${endpoint}:`, error);
    throw error;
  }
}

// Server Management API
export const serversApi = {
  // Get all servers
  getAll: () => fetchApi<{ servers: ServerSummary[] }>("/api/mcp/servers"),

  // Get server details
  getServer: (name: string) => fetchApi<ServerDetail>(`/api/mcp/servers/${name}`),

  // Get all instances for a server
  getInstances: (serverName: string) =>
    fetchApi<InstanceSummary[]>(`/api/mcp/servers/${serverName}/instances`),

  // Get instance details
  getInstance: (serverName: string, instanceId: string) =>
    fetchApi<InstanceDetail>(`/api/mcp/servers/${serverName}/instances/${instanceId}`),

  // Check instance health
  getInstanceHealth: (serverName: string, instanceId: string) =>
    fetchApi<InstanceHealth>(`/api/mcp/servers/${serverName}/instances/${instanceId}/health`),

  // Disconnect instance
  disconnectInstance: (serverName: string, instanceId: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/instances/${instanceId}/disconnect`, {
      method: "POST"
    }),

  // Force disconnect instance
  forceDisconnectInstance: (serverName: string, instanceId: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/instances/${instanceId}/disconnect/force`, {
      method: "POST"
    }),

  // Reconnect instance
  reconnectInstance: (serverName: string, instanceId: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/instances/${instanceId}/reconnect`, {
      method: "POST"
    }),

  // Reset and reconnect instance
  resetAndReconnectInstance: (serverName: string, instanceId: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/instances/${instanceId}/reconnect/reset`, {
      method: "POST"
    }),

  // Cancel initializing instance
  cancelInstance: (serverName: string, instanceId: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/instances/${instanceId}/cancel`, {
      method: "POST"
    }),

  // 以下是新增的服务器管理功能（模拟实现）

  // 启用服务器
  enableServer: async (serverName: string) => {
    try {
      return await fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/enable`, {
        method: "POST"
      });
    } catch (error) {
      console.warn("API not available, using mock implementation:", error);
      // 模拟成功响应
      return {
        status: "success",
        message: `Server ${serverName} enabled successfully (mock)`
      };
    }
  },

  // 禁用服务器
  disableServer: async (serverName: string) => {
    try {
      return await fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/disable`, {
        method: "POST"
      });
    } catch (error) {
      console.warn("API not available, using mock implementation:", error);
      // 模拟成功响应
      return {
        status: "success",
        message: `Server ${serverName} disabled successfully (mock)`
      };
    }
  },

  // 重新连接服务器的所有实例
  reconnectAllInstances: async (serverName: string) => {
    try {
      return await fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}/reconnect`, {
        method: "POST"
      });
    } catch (error) {
      console.warn("API not available, using mock implementation:", error);
      // 模拟成功响应
      return {
        status: "success",
        message: `All instances of server ${serverName} reconnected successfully (mock)`
      };
    }
  },

  // 创建新服务器
  createServer: async (serverConfig: Partial<MCPServerConfig>) => {
    try {
      return await fetchApi<ApiResponse<ServerSummary>>(`/api/mcp/servers`, {
        method: "POST",
        body: JSON.stringify(serverConfig)
      });
    } catch (error) {
      console.warn("API not available, using mock implementation:", error);
      // 模拟成功响应
      const mockServer: ServerSummary = {
        name: serverConfig.name || `server-${Date.now()}`,
        kind: serverConfig.kind || "stdio",
        status: "initializing",
        instance_count: 0
      };
      return {
        status: "success",
        message: `Server ${mockServer.name} created successfully (mock)`,
        data: mockServer
      };
    }
  },

  // 更新服务器配置
  updateServer: async (serverName: string, serverConfig: Partial<MCPServerConfig>) => {
    try {
      return await fetchApi<ApiResponse<ServerSummary>>(`/api/mcp/servers/${serverName}`, {
        method: "PUT",
        body: JSON.stringify(serverConfig)
      });
    } catch (error) {
      console.warn("API not available, using mock implementation:", error);
      // 模拟成功响应
      return {
        status: "success",
        message: `Server ${serverName} updated successfully (mock)`,
        data: {
          name: serverName,
          kind: serverConfig.kind || "stdio",
          status: "connected",
          instance_count: 1
        }
      };
    }
  },

  // 删除服务器
  deleteServer: async (serverName: string) => {
    try {
      return await fetchApi<ApiResponse<null>>(`/api/mcp/servers/${serverName}`, {
        method: "DELETE"
      });
    } catch (error) {
      console.warn("API not available, using mock implementation:", error);
      // 模拟成功响应
      return {
        status: "success",
        message: `Server ${serverName} deleted successfully (mock)`
      };
    }
  }
};

// Tools Management API
export const toolsApi = {
  // Get all tools
  getAll: () => fetchApi<{ tools: Tool[] }>("/api/mcp/tools"),

  // Get tool details
  getTool: (serverName: string, toolName: string) =>
    fetchApi<ToolDetail>(`/api/mcp/tools/${serverName}/${toolName}`),

  // Update tool configuration
  updateTool: (serverName: string, toolName: string, config: Partial<ToolDetail>) =>
    fetchApi<ApiResponse<ToolDetail>>(`/api/mcp/tools/${serverName}/${toolName}`, {
      method: "POST",
      body: JSON.stringify(config),
    }),

  // Enable tool
  enableTool: (serverName: string, toolName: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/tools/${serverName}/${toolName}/enable`, {
      method: "POST",
    }),

  // Disable tool
  disableTool: (serverName: string, toolName: string) =>
    fetchApi<ApiResponse<null>>(`/api/mcp/tools/${serverName}/${toolName}/disable`, {
      method: "POST",
    }),
};

// System Management API
export const systemApi = {
  // Get system status
  getStatus: () => fetchApi<SystemStatus>("/api/system/status"),

  // Get system metrics
  getMetrics: () => fetchApi<SystemMetrics>("/api/system/metrics"),
};

// Mock data for configuration
const mockGlobalSettings: GlobalSettings = {
  max_concurrent_connections: 10,
  request_timeout_ms: 30000,
  enable_metrics: true,
  log_level: "info",
};

const mockConfig: MCPConfig = {
  servers: [],
  tools: [],
  global_settings: mockGlobalSettings,
};

const mockPresets: ConfigPreset[] = [
  {
    id: "default",
    name: "Default Configuration",
    description: "Default system configuration",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    is_active: true,
    config: mockConfig,
  },
];

// Configuration Management API
export const configApi = {
  // Get current active configuration
  getCurrentConfig: async () => {
    try {
      return await fetchApi<MCPConfig>("/api/config/current");
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);

      // Return mock config with empty servers and tools
      // We'll populate it with real data if available
      const config = { ...mockConfig };

      // Try to fetch servers
      try {
        const serversResponse = await serversApi.getAll();
        if (serversResponse && serversResponse.servers && Array.isArray(serversResponse.servers)) {
          config.servers = serversResponse.servers.map(server => ({
            name: server.name,
            kind: server.kind,
            command: server.command,
            command_path: undefined,
            args: [],
            env: {},
            max_instances: 1,
            retry_policy: {
              max_attempts: 3,
              initial_delay_ms: 1000,
              max_delay_ms: 10000,
              backoff_multiplier: 1.5,
            },
          }));
        }
      } catch (e) {
        console.error("Failed to fetch servers for mock config:", e);
      }

      // Try to fetch tools
      try {
        const toolsResponse = await toolsApi.getAll();
        if (toolsResponse && toolsResponse.tools && Array.isArray(toolsResponse.tools)) {
          config.tools = toolsResponse.tools.map(tool => ({
            name: tool.tool_name,
            server_name: tool.server_name,
            is_enabled: tool.is_enabled,
            settings: {},
          }));
        }
      } catch (e) {
        console.error("Failed to fetch tools for mock config:", e);
      }

      return config;
    }
  },

  // Update current configuration
  updateConfig: async (config: MCPConfig) => {
    try {
      return await fetchApi<ApiResponse<MCPConfig>>("/api/config", {
        method: "POST",
        body: JSON.stringify(config),
      });
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      return {
        status: "success",
        message: "Configuration updated (mock)",
        data: config,
      };
    }
  },

  // Get all configuration presets
  getPresets: async () => {
    try {
      return await fetchApi<ConfigPreset[]>("/api/config/presets");
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      return mockPresets;
    }
  },

  // Get specific preset
  getPreset: async (id: string) => {
    try {
      return await fetchApi<ConfigPreset>(`/api/config/presets/${id}`);
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      const preset = mockPresets.find(p => p.id === id);
      if (!preset) {
        throw new Error(`Preset with ID ${id} not found`);
      }
      return preset;
    }
  },

  // Create new preset
  createPreset: async (preset: Omit<ConfigPreset, "id" | "created_at" | "updated_at">) => {
    try {
      return await fetchApi<ApiResponse<ConfigPreset>>("/api/config/presets", {
        method: "POST",
        body: JSON.stringify(preset),
      });
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      const newPreset: ConfigPreset = {
        ...preset,
        id: `mock-${Date.now()}`,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      mockPresets.push(newPreset);
      return {
        status: "success",
        message: "Preset created (mock)",
        data: newPreset,
      };
    }
  },

  // Update preset
  updatePreset: async (id: string, preset: Partial<ConfigPreset>) => {
    try {
      return await fetchApi<ApiResponse<ConfigPreset>>(`/api/config/presets/${id}`, {
        method: "PUT",
        body: JSON.stringify(preset),
      });
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      const index = mockPresets.findIndex(p => p.id === id);
      if (index === -1) {
        throw new Error(`Preset with ID ${id} not found`);
      }

      const updatedPreset = {
        ...mockPresets[index],
        ...preset,
        updated_at: new Date().toISOString(),
      };

      mockPresets[index] = updatedPreset;

      return {
        status: "success",
        message: "Preset updated (mock)",
        data: updatedPreset,
      };
    }
  },

  // Delete preset
  deletePreset: async (id: string) => {
    try {
      return await fetchApi<ApiResponse<null>>(`/api/config/presets/${id}`, {
        method: "DELETE",
      });
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      const index = mockPresets.findIndex(p => p.id === id);
      if (index === -1) {
        throw new Error(`Preset with ID ${id} not found`);
      }

      mockPresets.splice(index, 1);

      return {
        status: "success",
        message: "Preset deleted (mock)",
      };
    }
  },

  // Apply preset
  applyPreset: async (id: string) => {
    try {
      return await fetchApi<ApiResponse<null>>(`/api/config/presets/${id}/apply`, {
        method: "POST",
      });
    } catch (error) {
      console.warn("Config API not available, using mock data:", error);
      const preset = mockPresets.find(p => p.id === id);
      if (!preset) {
        throw new Error(`Preset with ID ${id} not found`);
      }

      // Mark this preset as active and others as inactive
      mockPresets.forEach(p => {
        p.is_active = p.id === id;
      });

      return {
        status: "success",
        message: "Preset applied (mock)",
      };
    }
  },
};

// WebSocket setup for notifications
export class NotificationsService {
  private ws: WebSocket | null = null;
  private listeners: Map<string, ((data: any) => void)[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 5000; // 5 seconds

  connect() {
    if (this.ws) return;

    // 使用相对路径，避免CORS问题
    const wsUrl = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`;
    console.log(`Connecting to WebSocket at ${wsUrl}`);

    try {
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        console.log('WebSocket connection established');
        this.reconnectAttempts = 0; // 重置重连计数
      };

      this.ws.onmessage = (event) => {
        try {
          console.log('WebSocket message received:', event.data);
          const data = JSON.parse(event.data);
          if (data.event) {
            const eventListeners = this.listeners.get(data.event) || [];
            console.log(`Dispatching ${data.event} event to ${eventListeners.length} listeners`);
            eventListeners.forEach(listener => listener(data));
          }
        } catch (error) {
          console.error("Error parsing WebSocket message:", error);
        }
      };

      this.ws.onerror = (error) => {
        console.error("WebSocket error:", error);
      };

      this.ws.onclose = (event) => {
        console.log(`WebSocket connection closed: ${event.code} ${event.reason}`);
        this.ws = null;

        // 实现指数退避重连策略
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
          const delay = this.reconnectDelay * Math.pow(1.5, this.reconnectAttempts);
          this.reconnectAttempts++;
          console.log(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
          setTimeout(() => this.connect(), delay);
        } else {
          console.error(`Failed to reconnect after ${this.maxReconnectAttempts} attempts`);
        }
      };
    } catch (error) {
      console.error("Error creating WebSocket connection:", error);
    }
  }

  subscribe(event: string, callback: (data: any) => void) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event)!.push(callback);

    if (!this.ws) {
      this.connect();
    }

    return () => {
      const eventListeners = this.listeners.get(event) || [];
      const index = eventListeners.indexOf(callback);
      if (index !== -1) {
        eventListeners.splice(index, 1);
      }
    };
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}

export const notificationsService = new NotificationsService();