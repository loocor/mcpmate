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

use anyhow::{Context, Result, anyhow};
use once_cell::sync::Lazy;
use sqlx::{Pool, Sqlite};
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

/// Get global database pool
fn db_pool() -> Result<&'static Pool<Sqlite>> {
    Ok(&GLOBAL_DB.get().ok_or_else(|| anyhow!("Resolver not initialized"))?.pool)
}

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

async fn get_cached_id(server_name: &str) -> Option<String> {
    CACHE.read().await.name_to_id.get(server_name).cloned()
}

async fn get_cached_name(server_id: &str) -> Option<String> {
    CACHE.read().await.id_to_name.get(server_id).cloned()
}

async fn cache_mapping(
    server_name: &str,
    server_id: &str,
) {
    let mut cache = CACHE.write().await;
    cache.name_to_id.insert(server_name.to_string(), server_id.to_string());
    cache.id_to_name.insert(server_id.to_string(), server_name.to_string());
}

// ================================
// Public API - Strict Mode Only
// ================================

/// Convert server_name to server_id
///
/// Returns Ok(Some(server_id)) if found, Ok(None) if not found, Err(_) if database error.
/// Uses cache to optimize performance.
pub async fn to_id(server_name: &str) -> Result<Option<String>> {
    // Try cache first
    if let Some(cached) = get_cached_id(server_name).await {
        return Ok(Some(cached));
    }

    // Query database
    let pool = db_pool()?;
    match crate::config::server::get_server(pool, server_name).await {
        Ok(Some(server)) => {
            if let Some(server_id) = server.id {
                // Cache the result
                cache_mapping(server_name, &server_id).await;
                Ok(Some(server_id))
            } else {
                tracing::warn!("Server '{}' found but has no ID", server_name);
                Ok(None)
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.context(format!("Failed to resolve server_name '{}'", server_name))),
    }
}

/// Convert server_id to server_name
///
/// Returns Ok(Some(server_name)) if found, Ok(None) if not found, Err(_) if database error.
/// Uses cache to optimize performance.
pub async fn to_name(server_id: &str) -> Result<Option<String>> {
    // Try cache first
    if let Some(cached) = get_cached_name(server_id).await {
        return Ok(Some(cached));
    }

    // Query database
    let pool = db_pool()?;
    let name = sqlx::query_scalar::<_, String>("SELECT name FROM server_config WHERE id = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await
        .context(format!("Failed to resolve server_id '{}'", server_id))?;

    if let Some(server_name) = name {
        let normalized_name = server_name.replace(' ', "_");
        // Cache the result
        cache_mapping(&normalized_name, server_id).await;
        Ok(Some(normalized_name))
    } else {
        Ok(None)
    }
}

/// Clear cache (for testing or configuration changes)
pub async fn clear_cache() {
    let mut cache = CACHE.write().await;
    cache.name_to_id.clear();
    cache.id_to_name.clear();
}
