// SSE module for MCPMate
// Contains SSE server implementation

pub mod pool;
pub mod server;

// Re-exports
pub use server::SseProxyServer;
