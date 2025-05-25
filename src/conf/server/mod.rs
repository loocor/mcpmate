// Server operations module
// Contains all server-related database operations split by functionality

pub mod crud;
pub mod args;
pub mod env;
pub mod meta;
pub mod enabled;

// Re-export all public functions for backward compatibility
pub use crud::{
    get_all_servers,
    get_server,
    get_server_by_id,
    upsert_server,
    upsert_server_tx,
    delete_server,
};

pub use args::{
    get_server_args,
    upsert_server_args,
};

pub use env::{
    get_server_env,
    upsert_server_env,
};

pub use meta::{
    get_server_meta,
    upsert_server_meta,
};

pub use enabled::{
    get_enabled_servers,
    is_server_enabled_in_any_suit,
    is_server_in_suit,
    update_server_global_status,
    get_server_global_status,
};
