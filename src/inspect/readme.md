# MCPMate Inspect Module

## Overview

The Inspect module provides independent MCP server capability inspect and caching functionality for MCPMate. It enables querying server capabilities (tools, resources, prompts) without requiring servers to be actively connected to the production connection pool.

## Purpose

The Inspect system solves a critical configuration management challenge: **allowing users to explore server capabilities before enabling them**. This enables informed configuration decisions without the overhead of maintaining active connections for every registered server.

### Key Benefits

- **Independent Operation**: Capability inspect is completely isolated from production traffic
- **Flexible Caching**: Dual-level caching (memory + file) optimizes performance
- **Configuration Support**: Enables capability-based configuration without server activation
- **Performance Optimized**: Avoids cold-start penalties through intelligent caching strategies

## Architecture

### Module Structure

```
src/inspect/
├── mod.rs           # Main service interface and builder
├── client.rs        # MCP server connection client for capability fetching
├── manager.rs       # Dual-level cache management (memory + file)
├── storage.rs       # Temporary file storage implementation
├── capabilities.rs  # Capability processing and format conversion
└── types.rs         # Data types, errors, and configuration
```

### Core Components

#### InspectService
The main service interface that coordinates all inspect operations:
- Manages capability retrieval with configurable refresh strategies
- Integrates caching, processing, and client components
- Provides unified API for server handlers

#### McpInspectClient
Independent MCP client for temporary server connections:
- Creates isolated connections for capability inspect
- Handles timeouts and connection lifecycle
- Converts RMCP types to inspect format

#### CapabilitiesCache
Dual-level caching system for performance optimization:
- **L1 Cache**: In-memory LRU cache for hot data (recently accessed servers)
- **L2 Cache**: File-based cache in system temp directory for warm data
- Configurable TTL, size limits, and cleanup policies

#### TempFileStorage
File-based caching implementation:
- Uses system temp directory with process isolation
- Automatic cleanup on process exit
- Size and age-based eviction policies
- Atomic operations with manifest tracking

### Integration with Server Handlers

The inspect module is integrated into the server API handlers to provide capability endpoints:

```
/api/mcp/servers/{identifier}/capabilities     # Complete capability overview
/api/mcp/servers/{identifier}/tools            # Tool listing
/api/mcp/servers/{identifier}/tools/{name}     # Tool details
/api/mcp/servers/{identifier}/resources        # Resource listing
/api/mcp/servers/{identifier}/prompts          # Prompt listing
/api/mcp/servers/{identifier}/resource-templates # Resource template listing
```

#### Handler Integration Pattern

Server handlers access the inspect service through the application state:

```rust
// Get inspect service from app state
let inspect_service = get_inspect_service(&state).await?;

// Resolve server identifier (supports both name and ID)
let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

// Query capabilities with timeout and caching
let capabilities = inspect_service
    .get_server_capabilities(&server_info.server_id, refresh_strategy)
    .await?;
```

## Key Features

### Refresh Strategies

- **CacheFirst**: Use cached data if available, don't refresh
- **RefreshIfStale**: Refresh only if cache is expired (default)
- **Force**: Always fetch fresh data from server

### Response Formats

- **Json**: Standard format with all essential fields (default)
- **Compact**: Minimal format for bandwidth optimization
- **Detailed**: Complete format including all metadata and annotations

### Caching Configuration

```rust
pub struct CacheConfig {
    memory_cache_size: usize,          // Default: 10 entries
    default_ttl: Duration,             // Default: 5 minutes
    max_file_cache_size: u64,          // Default: 50MB
    max_file_age: Duration,            // Default: 7 days
    cleanup_interval: Duration,        // Default: 1 hour
}
```

## Usage Examples

### Basic Capability Query

```rust
let inspect_service = InspectService::new(database, event_bus)?;
let capabilities = inspect_service
    .get_server_capabilities("server_id", RefreshStrategy::RefreshIfStale)
    .await?;
```

### Tool Inspect

```rust
let tools = inspect_service
    .get_server_tools("server_id", InspectParams::default())
    .await?;
```

### Cache Management

```rust
// Invalidate specific server cache
inspect_service.invalidate_server_cache("server_id").await?;

// Clear all caches
inspect_service.clear_all_cache().await?;

// Get cache statistics
let stats = inspect_service.get_cache_stats().await?;
```

## Design Decisions

### Why File-Based L2 Cache?

1. **Complex Data Structures**: JSON naturally handles nested capability schemas
2. **Simple Access Patterns**: Primarily key-based lookups by server_id
3. **Development Friendly**: Cache contents are human-readable for debugging
4. **No Schema Migration**: Avoids database migration complexity

### Why Independent Client?

1. **Isolation**: Inspect operations don't affect production connections
2. **Timeout Control**: Independent timeout management for inspect operations
3. **Simplified Lifecycle**: Temporary connections with automatic cleanup
4. **Reduced Complexity**: No need to coordinate with connection pool state

## Error Handling

The module provides comprehensive error handling through the `InspectError` enum:

- `ServerNotFound`: Server not registered in database
- `ConnectionFailed`: Unable to establish MCP connection
- `CacheError`: File system or cache operation failures
- `Timeout`: Operation exceeded configured timeout
- `SerializationError`: JSON parsing or formatting errors

## Performance Characteristics

- **Cache Hit (L1)**: ~1ms response time
- **Cache Hit (L2)**: ~10-50ms response time (file I/O)
- **Cache Miss**: 1-30s depending on server response time
- **Memory Usage**: ~10MB for typical workloads
- **Disk Usage**: Up to 50MB configurable limit

## Future Enhancements

- Resource template support expansion
- Capability diff tracking for change detection
- Metrics and monitoring integration
- Advanced cache warming strategies
- Capability-based server recommendations