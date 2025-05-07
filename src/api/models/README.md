# MCPMate API Models

This directory contains data models for the MCPMate Proxy API server.

## Purpose

Models define the structure of request and response data for API endpoints. They provide type safety and serialization/deserialization capabilities.

## Files

- `mod.rs` - Models module entry point
- `mcp.rs` - Models for MCP server management
- `system.rs` - Models for system operations

## MCP Models

The `mcp.rs` file contains models for MCP server and instance management:

- Server models:
  - `ServerResponse` - Basic server information
  - `ServerListResponse` - List of servers
  - `ServerDetailsResponse` - Detailed server information
  - `ServerStatusResponse` - Server status information

- Instance models:
  - `InstanceStatus` - Basic instance status
  - `InstanceDetails` - Detailed instance information
  - `ServerInstanceSummary` - Instance summary information
  - `ServerInstanceResponse` - Instance response information
  - `ServerInstancesResponse` - List of instances for a server
  - `InstanceHealthResponse` - Instance health information

- Operation models:
  - `OperationRequest` - Request for an operation
  - `OperationResponse` - Response from an operation

## System Models

The `system.rs` file contains models for system operations:

- `SystemStatusResponse` - System status information
- `SystemMetricsResponse` - System metrics information

## Model Types

Models typically include:

- **Request Models**: Structures for incoming request data
- **Response Models**: Structures for outgoing response data
- **Shared Models**: Structures used in both requests and responses

## Validation

Models can include validation logic to ensure that incoming data meets the required constraints. This is typically done using Serde's validation features or custom validation methods.

## Adding New Models

To add new models:

1. Create a new model module or extend an existing one
2. Define the model structures with appropriate Serde attributes
3. Implement any necessary validation logic
4. Import and expose the models in `mod.rs`
