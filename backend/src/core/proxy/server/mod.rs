//! Proxy server module index: minimal declarations and re-exports

mod common;
mod gateway;
mod prompts;
mod resources;
mod tools;

pub use common::{
    ClientContext, ClientIdentitySource, ClientTransport, ManagedClientContextResolver, ObservedClientInfo,
    SessionBinding, SessionBoundClientContextResolver, UnifiedHttpServer, UnifiedHttpServerConfig,
    load_unify_direct_exposure_eligible_server_ids, resolve_initialize_context_parts, resolve_request_context_parts,
    unify_directly_exposed_prompt_allowed, unify_directly_exposed_resource_allowed,
    unify_directly_exposed_template_allowed, unify_directly_exposed_tool_allowed, unify_route_mode,
};
pub(crate) use common::{resolve_direct_surface_value, unify_directly_exposed_resource_route_allowed};
pub use gateway::ProxyServer;
