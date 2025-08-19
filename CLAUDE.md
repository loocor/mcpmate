# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCPMate is a comprehensive Model Context Protocol (MCP) management center written in Rust. It consists of three main components:

1. **Proxy Server** (`mcpmate`): Core MCP proxy that aggregates multiple MCP servers into a unified interface
2. **Bridge** (`bridge`): Lightweight bridge converting stdio MCP clients to HTTP-based MCPMate proxy
3. **Runtime Manager**: Intelligent runtime environment management for JavaScript/Python/Bun MCP servers

## Development Commands

### Build & Compilation
```bash
# Build for development (debug mode)
cargo build

# Build for release
cargo build --release

# Cross-platform builds
./script/build-all.sh [debug|release]   # All platforms
./script/build-macos.sh [debug|release] [target]
./script/build-linux.sh [debug|release] [target]
./script/build-windows.sh [debug|release] [target]

# Universal macOS binary
./script/build-universal.sh [debug|release]
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Run tests with logging
RUST_LOG=debug cargo test

# Integration tests
cargo test --test integration

# Unit tests in specific module
cargo test --lib config::client
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint with Clippy
cargo clippy

# Clippy for all targets
cargo clippy --all-targets --all-features

# Check without building
cargo check
```

### Database & Migration
```bash
# Database is SQLite-based, migrations run automatically on startup
# Database file: config/mcpmate.db
# Schema files: src/config/database.rs
```

### Running the Application
```bash
# Start MCPMate proxy (default ports: API 8000, MCP 8080)
cargo run

# With custom ports
cargo run -- --api-port 9000 --mcp-port 9090

# Enable debug logging
RUST_LOG=debug cargo run

# Start bridge component
cargo run --bin bridge

# Runtime manager examples
cargo run -- runtime install node
cargo run -- runtime list
```

## Architecture Overview

### Module Structure
- **`src/main.rs`**: Main entry point for MCPMate proxy server
- **`src/lib.rs`**: Library exports for FFI integration with desktop applications
- **`src/core/`**: Core business logic with layered architecture:
  - `foundation/`: Infrastructure layer (errors, utils, types)
  - `events/`: Event system for inter-component communication
  - `connection/`: Individual connection management
  - `transport/`: Protocol transport layer (stdio, SSE, HTTP)
  - `pool/`: Connection pool management
  - `protocol/`: MCP protocol handling (tools, resources, prompts)
  - `proxy/`: Main proxy server implementation
  - `cache/`: High-performance Redb-based caching system
- **`src/config/`**: Database-driven configuration management
- **`src/api/`**: RESTful API server for external control
- **`src/runtime/`**: Runtime environment management
- **`src/system/`**: System detection and platform utilities
- **`src/interop/`**: FFI bridge for desktop app integration

### Key Design Patterns

#### Database-Driven Configuration
All configuration is stored in SQLite (`config/mcpmate.db`) with structured tables:
- `server_config`: MCP server definitions
- `config_suit`: Configuration suits for scenario-based management
- `config_suit_server`/`config_suit_tool`: Many-to-many relationships

#### Config Suits System
Central concept for managing MCP servers and tools:
- **Host App Suits**: Per-application configurations (Claude Desktop, Cursor, etc.)
- **Scenario Suits**: Task-specific tool collections (coding, research, etc.)
- **Shared Suits**: Common tool sets across applications
- Multi-activation support for complex workflows

#### Connection Pool Architecture
- Lazy connection initialization
- Health monitoring and auto-reconnection
- Parallel connection management
- Resource cleanup and lifecycle management

#### Transport Layer Abstraction
Supports multiple MCP transport protocols:
- **stdio**: Process-based communication
- **SSE**: Server-Sent Events over HTTP
- **streamable_http**: Bidirectional HTTP streaming

#### Dual-Layer Server State Management Architecture

MCPMate implements a sophisticated dual-layer architecture for managing MCP server states and capabilities:

**Layer 1: Global Servers Management (Instance Lifecycle Control)**
- **Direct Connection Pool Operations**: Creates, starts, stops, and terminates MCP server instances
- **Resource Allocation Control**: Determines which servers can obtain system resources
- **Instance Lifecycle Management**: Full control over server process creation and destruction
- **Primary Authority**: Overrides all other state management layers

**Layer 2: Config Suite Management (MCP Protocol Capability Filtering)**
- **Capability Filtering Role**: Acts as filter/reference table for MCP protocol capabilities
- **No Direct Instance Management**: Cannot directly create or terminate connection pool instances
- **Protocol-Level Control**: Determines which server capabilities are exposed to MCP protocol clients
- **Dual-Level Filtering System**:
  - **Server-Level Toggle**: Enable/disable all capabilities from a specific server
  - **Capability-Level Toggle**: Granular control over individual tools/prompts/resources/templates

**Multi-Suite Combination Network**:
- Multiple config suites can be simultaneously active
- Automatic deduplication based on unique capability names
- Combined capability sets are unified before transmission to downstream MCP clients
- Example: Suite A [Server1, Server2, Server3] + Suite B [Server3, Server4, Server5] → Unified capability network

**MCP Protocol vs HTTP Protocol Distinction**:
- Config Suite filtering affects **MCP protocol operations** (list_tools, call_tool, etc.)
- Downstream clients: Cursor, Winsurf, MCP Inspector, and other MCP-compatible clients
- Separate from HTTP/RESTful API management interface used by the Board web application

**Connection Pool Resource Management**:
- Idle timeout mechanism: Automatically releases instances after N minutes of inactivity
- Configurable timeout duration to balance resource usage vs startup latency
- Instance cleanup and lifecycle management independent of config suite states

**Architecture Constraint**:
- Config Suite operations must **NEVER** affect connection pool instance lifecycle
- Only Global Servers management layer should trigger instance creation/termination
- Config Suite changes only affect MCP protocol capability transmission filtering

### API Integration
RESTful API provides external control interface:
- Server management: `/api/mcp/servers/*`
- Config suit management: `/api/mcp/suits/*`
- Tool discovery: `/api/mcp/specs/tools/*`
- System monitoring: `/api/system/*`

## Testing Strategy

### Unit Tests
Located in `tests/unit/` with module-specific structure:
- Use `#[tokio::test]` for async tests
- Mock external dependencies with `mockall`
- Use `serial_test` for tests requiring exclusive access
- Test utilities in `tests/common/`

### Integration Tests
Located in `tests/integration/`:
- End-to-end workflow testing
- Database integration testing
- API endpoint testing with `axum-test`

### Test Configuration
- `clippy.toml`: Relaxed complexity thresholds for business logic
- `allow-unwrap-in-tests = true`: Permits unwrap in test code
- Use `test-case` for parameterized tests

## Development Notes

### Code Style
- Rust 2024 edition with 120-character line width
- Vertical function parameter layout for readability
- Group imports by std/external/crate
- Comprehensive error handling with `anyhow`/`thiserror`

### Feature Flags
- `interop`: Enables FFI bridge for desktop applications
- `standalone`: Standalone deployment mode

### Platform Support
Cross-compilation targets:
- Linux: x86_64, aarch64
- Windows: x86_64, aarch64 (gnullvm)
- macOS: x86_64, aarch64 (Intel/Apple Silicon)

### Performance Considerations
- Uses `lru` cache for frequently accessed data
- `dashmap` for concurrent hashmap operations
- Connection pooling with configurable limits
- Async/await throughout for non-blocking I/O

### Security
- Audit trail logging in `src/audit/`
- Policy-based access control
- Input validation for all API endpoints
- Secure handling of environment variables and credentials

# 🦀 Rust Best Practices Coding Standards

## Core Principles

1. **Use widely acclaimed idiomatic coding style from the Rust community**
2. **Prioritize ownership moves over unnecessary cloning and borrowing**
3. **Leverage the type system fully, let the compiler help catch errors**
4. **Embrace functional programming paradigms with iterator chaining**

## Syntax Guidelines

### Ownership Management
- ✅ Prefer `into_iter()` over `iter()`
- ✅ Move parameters when possible: `fn process(data: Vec<T>)` over `fn process(data: &Vec<T>)`
- ✅ Use `String` instead of `&str` as return type (when ownership is needed)

### Data Processing
- ✅ Use `map()` for data transformation
- ✅ Use `filter()` for data filtering
- ✅ Use `filter_map()` for simultaneous filtering and transformation
- ✅ Use `fold()` or `reduce()` for aggregation operations
- ✅ Use `collect()` to collect iterator results

### Error Handling
- ✅ Use `?` operator for early return style error handling
- ✅ Return `Result<T, E>` instead of `panic!`
- ✅ Use `unwrap_or()` or `unwrap_or_else()` to provide default values

### Pattern Matching
- ✅ Use `match` for business logic branching
- ✅ Leverage exhaustiveness checking in pattern matching
- ✅ Use `if let` for single pattern matching

### Chaining Style
- ✅ Adopt chained method calls for improved readability
- ✅ Each chain call on separate line for easier debugging and modification
- ✅ Use `collect()` or terminal operations at the end of chains

### Type Design
- ✅ Use `newtype` pattern to enhance type safety
- ✅ Implement necessary traits: `Debug`, `Clone`, `PartialEq`, etc.
- ✅ Use `enum` to represent states and variants

### Code Organization
- ✅ Use `impl` blocks to organize related methods
- ✅ Leverage `macro_rules!` to eliminate code duplication
- ✅ Use modules `mod` for proper code grouping

## Practice Checklist

```rust
// ✅ Example code template following standards
fn process_data(input: Vec<String>) -> Result<Vec<ProcessedItem>, ProcessError> {
    input
        .into_iter()                                    // Ownership move
        .filter(|item| !item.is_empty())                // Data filtering
        .map(|item| item.trim().to_string())            // Data transformation
        .map(ProcessedItem::try_from)                   // Type conversion
        .collect::<Result<Vec<_>, _>>()                 // Error aggregation
        .map_err(ProcessError::from)                    // Error conversion
}

// ✅ Error handling example
fn load_and_process_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")?;  // ? operator
    let raw_config: RawConfig = toml::from_str(&content)?;

    match raw_config.validate() {                           // match branching
        Ok(config) => Ok(config),
        Err(validation_errors) => Err(ConfigError::Invalid(validation_errors)),
    }
}

// ✅ Type design example
#[derive(Debug, Clone, PartialEq)]
pub struct UserId(u64);

#[derive(Debug)]
pub enum ProcessingState {
    Pending,
    InProgress { started_at: SystemTime },
    Completed { result: ProcessResult },
    Failed { error: ProcessError },
}
```

## Code Review Points

- [ ] Are unnecessary `.clone()` calls avoided?
- [ ] Is `?` used instead of `unwrap()`?
- [ ] Is chaining style adopted?
- [ ] Are appropriate iterator methods used?
- [ ] Are pattern matching advantages leveraged?
- [ ] Are ownership best practices followed?
- [ ] Is type-safe newtype pattern used?
- [ ] Are necessary traits implemented?
- [ ] Is `macro_rules!` used appropriately?
- [ ] Are excessive lifetime annotations avoided?

## Performance Optimization Points

- [ ] Avoid string allocation in loops
- [ ] Use `&str` for string slice operations
- [ ] Leverage lazy evaluation characteristics of iterators
- [ ] Use `Cow<str>` appropriately for potential string copying
- [ ] Use `Vec::with_capacity()` for capacity pre-allocation
- [ ] Avoid unnecessary intermediate collection allocations
- [ ] Use `Box<str>` instead of `String` (when string is no longer modified)
- [ ] Leverage `Arc<T>` and `Rc<T>` for sharing read-only data
- [ ] Use `lazy_static!` or `once_cell` for static data initialization
- [ ] Use `#[inline]` annotation for small functions appropriately

## Anti-patterns to Avoid

- ❌ Overusing `.clone()`
- ❌ Using `unwrap()` without error handling
- ❌ Passing `&Vec<T>` instead of `&[T]`
- ❌ Passing `&String` instead of `&str`
- ❌ Using `RefCell<T>` or `Mutex<T>` when unnecessary
- ❌ Ignoring borrow checker warnings from compiler
- ❌ Using too many lifetime parameters
- ❌ Using complex smart pointers in simple cases

## Memory Safety Guidelines

- [ ] Prefer stack allocation over heap allocation when possible
- [ ] Use `Box<T>` for single ownership of heap data
- [ ] Use `Rc<T>` for multiple ownership in single-threaded contexts
- [ ] Use `Arc<T>` for multiple ownership in multi-threaded contexts
- [ ] Avoid cyclic references with `Weak<T>`
- [ ] Use `Pin<T>` for self-referential structs when necessary

## Concurrency Best Practices

- [ ] Use `Send` and `Sync` traits appropriately
- [ ] Prefer message passing over shared state
- [ ] Use `tokio` for async programming
- [ ] Use `Mutex<T>` and `RwLock<T>` for shared mutable state
- [ ] Avoid blocking operations in async contexts
- [ ] Use `Arc<Mutex<T>>` for shared mutable data across threads

---

*Last Updated: August 2025*
*For Rust Development Projects*
