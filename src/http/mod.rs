// HTTP module for MCPMate
// Contains HTTP server implementations for different transport types

pub mod pool;
pub mod server;
pub mod unified;

// Re-exports
pub use server::HttpProxyServer;
