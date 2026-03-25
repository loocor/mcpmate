// Client handlers module
// Provides HTTP API handlers for client management functionality

pub mod backups;
pub mod config;
pub mod handlers;
pub mod import;
pub mod manage;

// Re-export the main handler functions for use in routes
pub use backups::{delete_backup, get_backup_policy, list_backups, set_backup_policy};
pub use handlers::{
    config_apply, config_details, config_import, config_restore, get_capability_config, list,
    update_capability_config, update_settings,
};
pub use manage::manage;
