// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use serde::{Deserialize, Serialize};

//
// API 模型
//

/// Instance status information
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceStatus {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
}

/// Server status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatusResponse {
    /// Server name
    pub name: String,
    /// Server status summary
    pub status: String,
    /// List of instances
    pub instances: Vec<InstanceStatus>,
}

/// 实例摘要
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceSummary {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
    /// Connected at time (ISO 8601 format)
    pub connected_at: Option<String>,
}

/// 服务响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    /// Server name
    pub name: String,
    /// Number of instances
    pub instances_count: usize,
    /// Is enabled in configuration
    pub enabled: bool,
}

/// 实例列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstancesResponse {
    /// Server name
    pub name: String,
    /// List of instances
    pub instances: Vec<ServerInstanceSummary>,
}

/// 实例详情
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceDetails {
    /// Connection attempts
    pub connection_attempts: u32,
    /// Last connected time (seconds since connection)
    pub last_connected_seconds: Option<u64>,
    /// Number of tools available
    pub tools_count: usize,
    /// Error message if status is Error
    pub error_message: Option<String>,
    /// Server type (stdio, sse, etc.)
    pub server_type: String,
}

/// 实例响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceResponse {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Instance status
    pub status: String,
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Instance details
    pub details: ServerInstanceDetails,
}

/// 实例健康状态响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceHealthResponse {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Is instance healthy
    pub healthy: bool,
    /// Health check message
    pub message: String,
    /// Current status
    pub status: String,
    /// Last health check time (ISO 8601 format)
    pub checked_at: String,
}

/// 操作请求
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationRequest {
    /// Force the operation (optional, for disconnect)
    pub force: Option<bool>,
    /// Reset the connection (optional, for reconnect)
    pub reset: Option<bool>,
}

/// 操作响应
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationResponse {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Operation result (success or error message)
    pub result: String,
    /// New status after operation
    pub status: String,
    /// Allowed operations after this operation
    pub allowed_operations: Vec<String>,
}

/// Server list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerListResponse {
    /// List of servers
    pub servers: Vec<ServerResponse>,
}

/// Instance details
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceDetails {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
    /// Connection attempts
    pub connection_attempts: u32,
    /// Last connected time (seconds since connection)
    pub last_connected_seconds: Option<u64>,
    /// Number of tools available
    pub tools_count: usize,
    /// Error message if status is Error
    pub error_message: Option<String>,
}

/// Detailed server response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerDetailsResponse {
    /// Server name
    pub name: String,
    /// Server status summary
    pub status: String,
    /// Server type (stdio, sse, etc.)
    pub server_type: String,
    /// Is enabled in configuration
    pub is_enabled: bool,
    /// List of instances
    pub instances: Vec<InstanceDetails>,
}

//
// 系统相关模型
//

/// 系统状态响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    /// System version
    pub version: String,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Number of connected servers
    pub connected_servers: usize,
    /// Number of total instances
    pub total_instances: usize,
    /// Number of ready instances
    pub ready_instances: usize,
}

/// 系统指标响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage in MB
    pub memory_usage_mb: f32,
    /// Number of requests processed
    pub requests_processed: u64,
    /// Average response time in ms
    pub avg_response_time_ms: f32,
}
