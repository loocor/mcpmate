// Tool models for MCPMate
// Contains data models for tool configuration

use serde::{Deserialize, Serialize};

/// Tool configuration update model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUpdate {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Alias name for the tool (user-defined)
    pub alias_name: Option<String>,
}
