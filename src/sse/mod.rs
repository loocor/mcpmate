// SSE module for MCPMate
// Contains SSE server implementation

pub mod pool;
pub mod server;
pub mod unified;

// Re-exports
pub use server::SseProxyServer;
