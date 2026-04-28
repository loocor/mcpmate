// Client handlers module
// Provides HTTP API handlers for client management functionality

pub mod approval;
pub mod backups;
pub mod config;
pub mod handlers;
pub mod import;
pub mod manage;

// Re-export the main handler functions for use in routes
pub use approval::{approve_client, suspend_client};
pub use backups::{delete_backup, get_backup_policy, list_backups, set_backup_policy};
pub use handlers::{
    client_attach, client_detach, config_apply, config_details, config_file_parse_inspect,
    config_file_parse_inspect_existing, config_import, config_restore, get_capability_config, list,
    update_capability_config, update_settings,
};
pub use manage::delete_client;
