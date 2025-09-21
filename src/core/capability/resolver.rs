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
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

use crate::config::database::Database;

// ================================
// Global Database Access
// ================================

/// Global database instance for server resolution
static GLOBAL_DB: OnceLock<Arc<Database>> = OnceLock::new();

/// Initialize resolver and build in-memory mappings (id<->name)
pub async fn init(database: Arc<Database>) -> Result<()> {
    GLOBAL_DB
        .set(database.clone())
        .map_err(|_| anyhow!("Resolver already initialized"))?;
    refresh_from_database(&database).await
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

/// Convert server_name to server_id (from in-memory cache)
pub async fn to_id(server_name: &str) -> Result<Option<String>> {
    Ok(id_by_name(server_name).await)
}

/// Convert server_id to server_name (from in-memory cache)
pub async fn to_name(server_id: &str) -> Result<Option<String>> {
    Ok(name_by_id(server_id).await)
}

/// Clear cache (for testing or configuration changes)
pub async fn clear_cache() {
    let mut cache = CACHE.write().await;
    cache.name_to_id.clear();
    cache.id_to_name.clear();
}

/// Get server_id by server_name (pure cache, non-fallible)
pub async fn id_by_name(server_name: &str) -> Option<String> {
    let cache = CACHE.read().await;
    cache.name_to_id.get(server_name).cloned()
}

/// Get server_name by server_id (pure cache, non-fallible)
pub async fn name_by_id(server_id: &str) -> Option<String> {
    let cache = CACHE.read().await;
    cache.id_to_name.get(server_id).cloned()
}

/// Refresh mappings from database; rebuilds both directions atomically
pub async fn refresh_from_database(database: &Database) -> Result<()> {
    let rows = sqlx::query("SELECT id, name FROM server_config WHERE id IS NOT NULL AND name IS NOT NULL")
        .fetch_all(&database.pool)
        .await?;

    let mut name_to_id: HashMap<String, String> = HashMap::new();
    let mut id_to_name: HashMap<String, String> = HashMap::new();

    for row in rows {
        let id: String = row.try_get(0)?;
        let name: String = row.try_get(1)?;
        id_to_name.insert(id.clone(), name.clone());
        name_to_id.insert(name, id);
    }

    let mut cache = CACHE.write().await;
    cache.name_to_id = name_to_id;
    cache.id_to_name = id_to_name;
    Ok(())
}

/// Upsert a mapping entry into cache
pub async fn upsert(
    server_id: &str,
    server_name: &str,
) {
    let mut cache = CACHE.write().await;
    cache.id_to_name.insert(server_id.to_string(), server_name.to_string());
    cache.name_to_id.insert(server_name.to_string(), server_id.to_string());
}

/// Remove a mapping by server_id
pub async fn remove_by_id(server_id: &str) {
    let mut cache = CACHE.write().await;
    if let Some(name) = cache.id_to_name.remove(server_id) {
        cache.name_to_id.remove(&name);
    }
}
