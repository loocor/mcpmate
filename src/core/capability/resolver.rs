//! Server identifier resolver for MCPMate
//!
//! Provides unified server_name ↔ server_id conversion with global database access.
//! Uses strict error handling to ensure configuration problems are visible.
//!
//! ## Usage Guidelines
//!
//! ### MCP Protocol Layer (should use server_name)
//! - All MCP protocol implementations (tools, prompts, resources)
//! - Protocol-level logging and error messages
//! - Any code that interfaces directly with MCP specification requirements
//!
//! ### MCPMate Internal Management Layer (should use server_id)
//! - Connection pool operations (`pool.connections.get()` calls)
//! - Database operations and queries
//! - Configuration management and profile operations
//!
//! ## API
//!
//! ```rust
//! // Convert server_name to server_id
//! let server_id = resolver::to_id("playwright-server").await?;
//!
//! // Convert server_id to server_name
//! let server_name = resolver::to_name("SERVabc123").await?;
//! ```

use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
// removed direct sqlx usage after delegating to ServerMappingManager
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

use crate::config::database::Database;

// ================================
// Global Database Access
// ================================

/// Global database instance for server resolution
static GLOBAL_DB: OnceLock<Arc<Database>> = OnceLock::new();

/// Initialize global resolver (called once during startup)
pub fn init(database: Arc<Database>) -> Result<()> {
    GLOBAL_DB
        .set(database)
        .map_err(|_| anyhow!("Resolver already initialized"))
}

// Removed db_pool; resolver now delegates to in-memory mapping manager

/// Check if resolver is initialized
pub fn is_initialized() -> bool {
    GLOBAL_DB.get().is_some()
}

// ================================
// In-Memory Cache
// ================================

static CACHE: Lazy<Arc<RwLock<ResolverCache>>> = Lazy::new(|| Arc::new(RwLock::new(ResolverCache::default())));

#[derive(Default)]
struct ResolverCache {
    name_to_id: HashMap<String, String>,
    id_to_name: HashMap<String, String>,
}

// ================================
// Public API - Strict Mode Only
// ================================

/// Convert server_name to server_id
///
/// Returns Ok(Some(server_id)) if found, Ok(None) if not found, Err(_) if database error.
/// Now delegates to ServerMappingManager for better performance.
pub async fn to_id(server_name: &str) -> Result<Option<String>> {
    // Delegate to the global server mapping manager for zero-I/O lookup
    Ok(crate::core::capability::global_server_mapping_manager()
        .get_id_by_name(server_name)
        .await)
}

/// Convert server_id to server_name
///
/// Returns Ok(Some(server_name)) if found, Ok(None) if not found, Err(_) if database error.
/// Now delegates to ServerMappingManager for better performance.
pub async fn to_name(server_id: &str) -> Result<Option<String>> {
    // Delegate to the global server mapping manager for zero-I/O lookup
    Ok(crate::core::capability::global_server_mapping_manager()
        .get_name_by_id(server_id)
        .await)
}

/// Clear cache (for testing or configuration changes)
pub async fn clear_cache() {
    let mut cache = CACHE.write().await;
    cache.name_to_id.clear();
    cache.id_to_name.clear();
}
