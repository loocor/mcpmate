//! Built-in MCP Services - Protocol Converter
//!
//! This module provides MCP services that act as protocol converters,
//! transforming existing API capabilities into MCP tool interfaces.

mod registry;
mod suits;

pub use registry::BuiltinServiceRegistry;
pub use suits::SuitsService;