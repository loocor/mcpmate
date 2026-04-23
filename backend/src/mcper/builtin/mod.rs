//! Built-in MCP Services - Protocol Converter
//!
//! This module provides MCP services that act as protocol converters,
//! transforming existing API capabilities into MCP tool interfaces.

mod broker;
mod client;
mod helpers;
pub mod names;
mod profile;
mod registry;
mod types;

pub use broker::BrokerService;
pub use client::{ClientBuiltinContext, ClientService};
pub use names::*;
pub use profile::ProfileService;
pub use registry::BuiltinServiceRegistry;
pub use types::{PromptDetail, ResourceDetail, ServerDetail, ToolDetail};
