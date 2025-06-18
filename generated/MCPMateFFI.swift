//
//  MCPMateFFI.swift
//  MCPMate FFI Bindings
//
//  Generated Swift bindings for MCPMate Rust FFI
//

import Foundation

// MARK: - MCPMateEngine Class

/// MCPMate FFI Engine for managing backend service lifecycle
public class MCPMateEngine {
    private var rustEngine: OpaquePointer?
    
    /// Initialize a new MCPMate engine instance
    public init() {
        self.rustEngine = mcpmate_engine_new()
    }
    
    deinit {
        if let engine = rustEngine {
            mcpmate_engine_free(engine)
        }
    }
    
    /// Start the MCPMate service with specified ports
    /// - Parameters:
    ///   - apiPort: Port for REST API server (default: 8080)
    ///   - mcpPort: Port for MCP proxy server (default: 3000)
    /// - Returns: true if startup initiated successfully, false otherwise
    public func start(apiPort: UInt16 = 8080, mcpPort: UInt16 = 3000) -> Bool {
        guard let engine = rustEngine else { return false }
        return mcpmate_engine_start(engine, apiPort, mcpPort)
    }
    
    /// Stop the MCPMate service
    public func stop() {
        guard let engine = rustEngine else { return }
        mcpmate_engine_stop(engine)
    }
    
    /// Check if the service is currently running
    /// - Returns: true if service is running, false otherwise
    public func isRunning() -> Bool {
        guard let engine = rustEngine else { return false }
        return mcpmate_engine_is_running(engine)
    }
    
    /// Get current startup progress as JSON string
    /// - Returns: JSON string containing startup progress information
    public func getStartupProgressJSON() -> String {
        guard let engine = rustEngine else { return "{}" }
        let cString = mcpmate_engine_get_startup_progress_json(engine)
        let result = String(cString: cString)
        mcpmate_string_free(cString)
        return result
    }
    
    /// Get service information as JSON string
    /// - Returns: JSON string containing service information
    public func getServiceInfoJSON() -> String {
        guard let engine = rustEngine else { return "{}" }
        let cString = mcpmate_engine_get_service_info_json(engine)
        let result = String(cString: cString)
        mcpmate_string_free(cString)
        return result
    }
}

// MARK: - Convenience Data Models

/// Startup progress information
public struct StartupProgress: Codable {
    public let percentage: Float
    public let currentStep: String
    public let connectedServers: UInt32
    public let totalServers: UInt32
    public let isComplete: Bool
    public let errorMessage: String?
    
    enum CodingKeys: String, CodingKey {
        case percentage
        case currentStep = "current_step"
        case connectedServers = "connected_servers"
        case totalServers = "total_servers"
        case isComplete = "is_complete"
        case errorMessage = "error_message"
    }
}

/// Service information
public struct ServiceInfo: Codable {
    public let version: String
    public let apiPort: UInt16
    public let mcpPort: UInt16
    public let uptimeSeconds: UInt64
    public let isRunning: Bool
    public let activeConnections: UInt32
    
    enum CodingKeys: String, CodingKey {
        case version
        case apiPort = "api_port"
        case mcpPort = "mcp_port"
        case uptimeSeconds = "uptime_seconds"
        case isRunning = "is_running"
        case activeConnections = "active_connections"
    }
}

// MARK: - Convenience Extensions

extension MCPMateEngine {
    /// Get startup progress as structured data
    /// - Returns: StartupProgress struct or nil if parsing fails
    public func getStartupProgress() -> StartupProgress? {
        let jsonString = getStartupProgressJSON()
        guard let data = jsonString.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(StartupProgress.self, from: data)
    }
    
    /// Get service info as structured data
    /// - Returns: ServiceInfo struct or nil if parsing fails
    public func getServiceInfo() -> ServiceInfo? {
        let jsonString = getServiceInfoJSON()
        guard let data = jsonString.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(ServiceInfo.self, from: data)
    }
}

// MARK: - C FFI Function Declarations

/// Create a new MCPMate engine instance
@_silgen_name("mcpmate_engine_new")
func mcpmate_engine_new() -> OpaquePointer

/// Free MCPMate engine instance
@_silgen_name("mcpmate_engine_free")
func mcpmate_engine_free(_ engine: OpaquePointer)

/// Start MCPMate service
@_silgen_name("mcpmate_engine_start")
func mcpmate_engine_start(_ engine: OpaquePointer, _ apiPort: UInt16, _ mcpPort: UInt16) -> Bool

/// Stop MCPMate service
@_silgen_name("mcpmate_engine_stop")
func mcpmate_engine_stop(_ engine: OpaquePointer)

/// Check if service is running
@_silgen_name("mcpmate_engine_is_running")
func mcpmate_engine_is_running(_ engine: OpaquePointer) -> Bool

/// Get startup progress as JSON
@_silgen_name("mcpmate_engine_get_startup_progress_json")
func mcpmate_engine_get_startup_progress_json(_ engine: OpaquePointer) -> UnsafePointer<CChar>

/// Get service info as JSON
@_silgen_name("mcpmate_engine_get_service_info_json")
func mcpmate_engine_get_service_info_json(_ engine: OpaquePointer) -> UnsafePointer<CChar>

/// Free string allocated by Rust
@_silgen_name("mcpmate_string_free")
func mcpmate_string_free(_ string: UnsafePointer<CChar>)
