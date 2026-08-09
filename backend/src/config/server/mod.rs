// Server operations module
// Contains all server-related database operations split by functionality

pub mod args;
pub mod capabilities;
pub mod crud;
pub mod definition;
pub mod enabled;
pub mod env;
pub mod fingerprint;
pub mod headers;
pub mod import;
pub mod init;
pub mod meta;
pub mod namespace;
pub mod namespace_repair;
pub mod oauth;
pub mod preview;
pub mod tools;
pub mod transport;

pub use args::{get_server_args, upsert_server_args};
pub use crud::{delete_server, get_all_servers, get_server, get_server_by_id, upsert_server, upsert_server_tx};
pub use definition::{
    clear_persisted_http_authorization_headers, ensure_persisted_http_authorization_headers_clearable,
    load_validated_server_transport, upsert_server_definition,
};
pub use env::{get_server_env, upsert_server_env};
pub use headers::{
    get_server_headers, merge_env_for_update, merge_headers_for_update, remove_authorization_headers,
    replace_server_headers, upsert_server_headers,
};
pub use meta::{get_server_meta, upsert_server_meta};
pub use namespace::{NamespaceValidationError, suggest_server_namespace, validate_server_namespace};
pub use oauth::{
    delete_server_oauth_config, delete_server_oauth_token, get_all_oauth_configs, get_all_oauth_tokens,
    get_effective_server_headers, get_server_oauth_config, get_server_oauth_token, has_manual_authorization_header,
    upsert_server_oauth_config, upsert_server_oauth_token,
};
pub use transport::{
    ServerTransportDraftLoad, get_server_transport_draft, get_server_transport_drafts, upsert_server_transport_draft_tx,
};

pub use capabilities::{CapabilitySnapshot, discover_from_config, discover_from_connection};
pub use enabled::{
    ServerEnabledService, get_enabled_servers, get_enabled_servers_by_profile, get_server_global_status,
    is_server_enabled_in_any_active_profile, is_server_enabled_in_any_profile, is_server_in_profile,
};
pub use import::{
    ConflictPolicy, ImportOptions, ImportOutcome, SkipReason, SkippedServer, import_batch,
    plan_import_from_client_inspection,
};
