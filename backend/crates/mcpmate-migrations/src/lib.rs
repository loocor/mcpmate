//! The sole owner of durable SQLite schema evolution in MCPMate.

mod migrations;
mod runner;

pub use runner::{
    DatabaseSource, prepare_audit_database, prepare_config_database, verify_audit_database, verify_config_database,
};
