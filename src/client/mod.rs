pub mod sse;
pub mod stdio;
pub mod utils;

// Re-export commonly used items
pub use sse::handle_sse_server;
pub use stdio::handle_stdio_server;
pub use utils::{prepare_command_env, schema_formater};

pub struct CallToolInput<'a> {
    pub server_name: String,
    pub server_config: &'a crate::config::ServerConfig,
    pub tool_name: String,
    pub arguments: Option<serde_json::Value>,
}

pub use sse::call_tool_sse;
pub use stdio::call_tool_stdio;
