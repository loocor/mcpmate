// HTTP module for MCPMate
// Contains HTTP server implementations for different transport types

pub mod pool;
pub mod proxy;
pub mod unified;

// Re-exports
pub use proxy::HttpProxyServer;
