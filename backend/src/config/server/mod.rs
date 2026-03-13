// Server operations module
// Contains all server-related database operations split by functionality

pub mod args;
pub mod capabilities;
pub mod crud;
pub mod enabled;
pub mod env;
pub mod fingerprint;
pub mod headers;
pub mod import;
pub mod init;
pub mod meta;
pub mod preview;
pub mod tools;

pub use args::{get_server_args, upsert_server_args};
pub use crud::{delete_server, get_all_servers, get_server, get_server_by_id, upsert_server, upsert_server_tx};
pub use env::{get_server_env, upsert_server_env};
pub use headers::{get_server_headers, replace_server_headers, upsert_server_headers};
pub use meta::{get_server_meta, upsert_server_meta};

pub use capabilities::{
    CapabilityManager, CapabilitySnapshot, CapabilitySync, SyncStrategy, discover_from_config,
    discover_from_connection, overwrite_capabilities, store_dual_write, store_redb_snapshot, sync_via_connection_pool,
    upsert_shadow_prompt, upsert_shadow_resource, upsert_shadow_resource_template,
};
pub use enabled::{
    ServerEnabledService, get_enabled_servers, get_enabled_servers_by_profile, get_server_global_status,
    is_server_enabled_in_any_active_profile, is_server_enabled_in_any_profile, is_server_in_profile,
    update_server_global_status,
};
pub use import::{ConflictPolicy, ImportOptions, ImportOutcome, SkipReason, SkippedServer, import_batch};
