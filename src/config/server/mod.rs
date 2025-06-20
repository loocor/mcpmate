// Server operations module
// Contains all server-related database operations split by functionality

pub mod args;
pub mod crud;
pub mod enabled;
pub mod env;
pub mod init;
pub mod meta;
pub mod tools;

pub use crud::{
    delete_server, get_all_servers, get_server, get_server_by_id, upsert_server, upsert_server_tx,
};

pub use args::{get_server_args, upsert_server_args};

pub use env::{get_server_env, upsert_server_env};

pub use meta::{get_server_meta, upsert_server_meta};

pub use enabled::{
    get_enabled_servers, get_enabled_servers_by_suites, get_server_global_status,
    is_server_enabled_in_any_suit, is_server_in_suit, update_server_global_status,
};
