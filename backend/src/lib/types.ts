// API Response Types
export interface ServerSummary {
  name: string;
  kind: "stdio" | "sse" | "streamable_http";
  status: "connected" | "disconnected" | "error" | "initializing";
  instance_count: number;
}

export interface InstanceSummary {
  id: string;
  status: "running" | "initializing" | "error" | "stopped";
  startTime?: string;
}

export interface ServerDetail extends ServerSummary {
  command?: string;
  commandPath?: string;
  args?: string[];
  env?: Record<string, string>;
  instances: InstanceSummary[];
}

export interface InstanceDetail extends InstanceSummary {
  server_name: string;
  health_status: "healthy" | "unhealthy" | "unknown";
}

export interface InstanceHealth {
  instance_id: string;
  server_name: string;
  is_healthy: boolean;
  details?: string;
}

export interface Tool {
  tool_name: string;
  server_name: string;
  is_enabled: boolean;
  description?: string;
}

export interface ToolDetail extends Tool {
  configuration: Record<string, any>;
}

export interface SystemStatus {
  status: "running" | "degraded" | "stopped" | "error";
  version: string;
  uptime: number;
  active_mcp_servers: number;
  aggregated_tools_count: number;
}

export interface SystemMetrics {
  cpu_usage_percent?: number;
  memory_usage_bytes?: number;
  active_connections?: number;
  total_requests_mcp?: number;
  error_rate_mcp?: number;
  uptime_seconds?: number;
  timestamp?: string;
  connected_servers_count?: number;
  total_instances_count?: number;
  ready_instances_count?: number;
}

// API Response Status
export type ApiResponse<T> = {
  status: string;
  message?: string;
  data?: T;
};

// Notification Types
export interface ToolsChangedNotification {
  event: "tools_list_changed";
}

// UI Mode Type
export type UIMode = "wizard" | "expert";

// Theme Type
export type Theme = "light" | "dark" | "system";

// Configuration Types
export interface ConfigPreset {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
  is_active: boolean;
  config: MCPConfig;
}

export interface MCPConfig {
  servers: MCPServerConfig[];
  tools: MCPToolConfig[];
  global_settings: GlobalSettings;
}

export interface MCPServerConfig {
  name: string;
  kind: "stdio" | "sse" | "streamable_http";
  command?: string;
  command_path?: string;
  args?: string[];
  env?: Record<string, string>;
  max_instances?: number;
  retry_policy?: RetryPolicy;
}

export interface MCPToolConfig {
  name: string;
  server_name: string;
  is_enabled: boolean; // Keep this field name for consistency with API
  settings?: Record<string, any>;
}

export interface GlobalSettings {
  max_concurrent_connections: number;
  request_timeout_ms: number;
  enable_metrics: boolean;
  log_level: "debug" | "info" | "warn" | "error";
}

export interface RetryPolicy {
  max_attempts: number;
  initial_delay_ms: number;
  max_delay_ms: number;
  backoff_multiplier: number;
}