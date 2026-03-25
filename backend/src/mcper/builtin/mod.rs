//! Built-in MCP Services - Protocol Converter
//!
//! This module provides MCP services that act as protocol converters,
//! transforming existing API capabilities into MCP tool interfaces.

mod client;
mod helpers;
mod profile;
mod registry;
mod types;

pub use client::{ClientBuiltinContext, ClientService};
pub use profile::ProfileService;
pub use registry::BuiltinServiceRegistry;
pub use types::{PromptDetail, ResourceDetail, ServerDetail, ToolDetail};
