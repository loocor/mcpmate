# Technical Stack

> Last Updated: 2025-08-02
> Version: 1.0.0

## Application Framework

- **Framework:** Rust with Axum
- **Version:** Latest stable
- **Architecture:** Event-driven proxy-based system

## Database

- **Primary Database:** SQLite with migrations support
- **ORM:** Custom SQL with prepared statements
- **Features:** Multi-tenant isolation, audit logging

## MCP Integration

- **SDK:** Custom rmcp (Rust MCP) SDK v0.3.1
- **Protocols:** stdio, SSE, HTTP support
- **Runtime Support:** Node.js, Python (uv), Bun.js

## Desktop Applications

- **macOS:** SwiftUI native application
- **Windows:** Planned WinUI3 implementation
- **Linux:** Planned GTK4 implementation

## Backend Services

- **Web Server:** Axum with RESTful API
- **Async Runtime:** Tokio for high-performance concurrency
- **Connection Management:** Custom connection pooling with health monitoring
- **Process Management:** Cross-platform runtime managers

## Development Tools

- **Build System:** Cargo with workspace configuration
- **Testing:** Built-in Rust testing framework
- **Documentation:** Comprehensive API documentation
- **Debugging:** Integrated MCP service debugging tools