//! Built-in MCP Services - Protocol Converter
//!
//! This module provides MCP services that act as protocol converters,
//! transforming existing API capabilities into MCP tool interfaces.

mod profile;
mod registry;

pub use profile::ProfileService;
pub use registry::BuiltinServiceRegistry;
