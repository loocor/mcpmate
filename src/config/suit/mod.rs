// Config Suit operations module for MCPMate
// Contains CRUD operations for configuration suits, organized by functional domains

pub mod basic;
pub mod init;
pub mod mgmt;
pub mod server;
pub mod tool;

// Basic query operations
pub use basic::{
    get_active_config_suits, get_all_config_suits, get_config_suit, get_config_suit_by_name,
    get_config_suits_by_type, get_default_config_suit,
};

// Management operations
pub use mgmt::{
    delete_config_suit, set_config_suit_active, set_config_suit_default, upsert_config_suit,
    upsert_config_suit_tx,
};

// Server association operations
pub use server::{
    add_server_to_config_suit, get_config_suit_servers, remove_server_from_config_suit,
};

// Tool association operations
pub use tool::{add_tool_to_config_suit, get_config_suit_tools, remove_tool_from_config_suit};
