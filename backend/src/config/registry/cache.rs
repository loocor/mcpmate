// Registry cache service for local caching of MCP registry data

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use tracing;

/// Registry server cache entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegistryCacheEntry {
    /// Server name (primary key)
    pub server_name: String,
    /// Server version from registry
    pub version: String,
    pub schema_url: Option<String>,
    /// Display title
    pub title: Option<String>,
    /// Server description
    pub description: Option<String>,
    /// JSON-serialized packages array
    pub packages_json: Option<String>,
    /// JSON-serialized remotes array
    pub remotes_json: Option<String>,
    /// JSON-serialized icons array
    pub icons_json: Option<String>,
    /// JSON-serialized metadata object
    pub meta_json: Option<String>,
    pub website_url: Option<String>,
    pub repository_json: Option<String>,
    /// Server status (active, deprecated, deleted)
    pub status: String,
    /// When the server was published
    pub published_at: Option<DateTime<Utc>>,
    /// When the server was last updated in registry
    pub updated_at: Option<DateTime<Utc>>,
    /// When this cache entry was last synced
    pub synced_at: DateTime<Utc>,
}

/// Search result with pagination cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub servers: Vec<RegistryCacheEntry>,
    pub next_cursor: Option<String>,
    pub total: i64,
}

/// Registry cache service for CRUD operations and sync
pub struct RegistryCacheService {
    pool: Pool<Sqlite>,
}

impl RegistryCacheService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Upsert a cache entry (insert or update)
    pub async fn upsert(
        &self,
        entry: &RegistryCacheEntry,
    ) -> Result<()> {
        tracing::trace!("Upserting registry cache entry: {}", entry.server_name);

        sqlx::query(
            r#"
            INSERT INTO registry_cache (
                server_name, version, schema_url, title, description,
                packages_json, remotes_json, icons_json, meta_json,
                website_url, repository_json,
                status, published_at, updated_at, synced_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(server_name) DO UPDATE SET
                version = excluded.version,
                schema_url = excluded.schema_url,
                title = excluded.title,
                description = excluded.description,
                packages_json = excluded.packages_json,
                remotes_json = excluded.remotes_json,
                icons_json = excluded.icons_json,
                meta_json = excluded.meta_json,
                website_url = excluded.website_url,
                repository_json = excluded.repository_json,
                status = excluded.status,
                published_at = excluded.published_at,
                updated_at = excluded.updated_at,
                synced_at = excluded.synced_at
            "#,
        )
        .bind(&entry.server_name)
        .bind(&entry.version)
        .bind(&entry.schema_url)
        .bind(&entry.title)
        .bind(&entry.description)
        .bind(&entry.packages_json)
        .bind(&entry.remotes_json)
        .bind(&entry.icons_json)
        .bind(&entry.meta_json)
        .bind(&entry.website_url)
        .bind(&entry.repository_json)
        .bind(&entry.status)
        .bind(entry.published_at)
        .bind(entry.updated_at)
        .bind(entry.synced_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to upsert registry cache entry: {}", entry.server_name))?;

        tracing::trace!("Successfully upserted registry cache entry: {}", entry.server_name);
        Ok(())
    }

    /// Get a cache entry by server name
    pub async fn get_by_name(
        &self,
        name: &str,
    ) -> Result<Option<RegistryCacheEntry>> {
        tracing::debug!("Getting registry cache entry by name: {}", name);

        let entry = sqlx::query_as::<_, RegistryCacheEntry>("SELECT * FROM registry_cache WHERE server_name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to get registry cache entry: {}", name))?;

        Ok(entry)
    }

    /// Get all cache entries with optional status filter
    pub async fn list_all(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<RegistryCacheEntry>> {
        tracing::debug!("Listing all registry cache entries with status: {:?}", status);

        let entries = match status {
            Some(s) => {
                sqlx::query_as::<_, RegistryCacheEntry>(
                    "SELECT * FROM registry_cache WHERE status = ? ORDER BY server_name",
                )
                .bind(s)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, RegistryCacheEntry>("SELECT * FROM registry_cache ORDER BY server_name")
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .with_context(|| "Failed to list registry cache entries")?;

        Ok(entries)
    }

    /// Search cache entries by name or description
    pub async fn search_local(
        &self,
        query: &str,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<SearchResult> {
        tracing::debug!(
            "Searching registry cache: query='{}', limit={}, cursor={:?}",
            query,
            limit,
            cursor
        );

        let search_pattern = format!("%{}%", query.to_lowercase());

        // Count total matching entries
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM registry_cache
            WHERE status = 'active'
            AND (LOWER(server_name) LIKE ? OR LOWER(COALESCE(description, '')) LIKE ?)
            "#,
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_one(&self.pool)
        .await
        .with_context(|| "Failed to count search results")?;

        // Build query with optional cursor
        let entries = match cursor {
            Some(c) => {
                sqlx::query_as::<_, RegistryCacheEntry>(
                    r#"
                    SELECT * FROM registry_cache
                    WHERE status = 'active'
                    AND (LOWER(server_name) LIKE ? OR LOWER(COALESCE(description, '')) LIKE ?)
                    AND server_name > ?
                    ORDER BY server_name
                    LIMIT ?
                    "#,
                )
                .bind(&search_pattern)
                .bind(&search_pattern)
                .bind(c)
                .bind(limit as i32)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, RegistryCacheEntry>(
                    r#"
                    SELECT * FROM registry_cache
                    WHERE status = 'active'
                    AND (LOWER(server_name) LIKE ? OR LOWER(COALESCE(description, '')) LIKE ?)
                    ORDER BY server_name
                    LIMIT ?
                    "#,
                )
                .bind(&search_pattern)
                .bind(&search_pattern)
                .bind(limit as i32)
                .fetch_all(&self.pool)
                .await
            }
        }
        .with_context(|| "Failed to search registry cache")?;

        // Determine next cursor
        let next_cursor = if entries.len() == limit as usize {
            entries.last().map(|e| e.server_name.clone())
        } else {
            None
        };

        Ok(SearchResult {
            servers: entries,
            next_cursor,
            total,
        })
    }

    /// Get the last sync time
    pub async fn last_sync_time(&self) -> Result<Option<DateTime<Utc>>> {
        tracing::debug!("Getting last sync time");

        let time: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT MAX(synced_at) FROM registry_cache")
            .fetch_one(&self.pool)
            .await
            .with_context(|| "Failed to get last sync time")?;

        Ok(time)
    }

    /// Sync entries from registry data (incremental update)
    pub async fn sync_incremental(
        &self,
        entries: &[RegistryCacheEntry],
    ) -> Result<usize> {
        tracing::info!("Syncing {} registry cache entries", entries.len());

        let mut count = 0;
        for entry in entries {
            self.upsert(entry).await?;
            count += 1;
        }

        tracing::info!("Successfully synced {} registry cache entries", count);
        Ok(count)
    }

    /// Mark entries as deleted that are no longer in the registry
    pub async fn mark_deleted(
        &self,
        active_names: &[&str],
    ) -> Result<usize> {
        tracing::debug!("Marking deleted entries, active names: {}", active_names.len());

        if active_names.is_empty() {
            return Ok(0);
        }

        // Build placeholders for IN clause
        let placeholders: Vec<String> = active_names.iter().map(|_| "?".to_string()).collect();
        let placeholders_str = placeholders.join(",");

        let query_str = format!(
            "UPDATE registry_cache SET status = 'deleted', synced_at = ? WHERE server_name NOT IN ({}) AND status = 'active'",
            placeholders_str
        );

        let mut query = sqlx::query(&query_str).bind(Utc::now());
        for name in active_names {
            query = query.bind(name);
        }

        let result = query
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to mark deleted entries")?;

        let rows_affected = result.rows_affected() as usize;
        if rows_affected > 0 {
            tracing::info!("Marked {} entries as deleted", rows_affected);
        }

        Ok(rows_affected)
    }

    /// Delete all cache entries
    pub async fn clear(&self) -> Result<()> {
        tracing::info!("Clearing registry cache");

        sqlx::query("DELETE FROM registry_cache")
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to clear registry cache")?;

        tracing::info!("Registry cache cleared");
        Ok(())
    }

    /// Get cache entry count
    pub async fn count(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM registry_cache")
            .fetch_one(&self.pool)
            .await
            .with_context(|| "Failed to count registry cache entries")?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory SQLite pool");

        // Initialize table
        crate::config::registry::init::initialize_registry_cache_table(&pool)
            .await
            .expect("Failed to initialize registry cache table");

        pool
    }

    fn create_test_entry(
        name: &str,
        version: &str,
    ) -> RegistryCacheEntry {
        RegistryCacheEntry {
            server_name: name.to_string(),
            version: version.to_string(),
            schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
            title: Some(format!("{} Server", name)),
            description: Some(format!("Description for {}", name)),
            packages_json: Some("[]".to_string()),
            remotes_json: Some("[]".to_string()),
            icons_json: Some("[]".to_string()),
            meta_json: Some("{}".to_string()),
            website_url: Some(format!("https://{}.example.com", name)),
            repository_json: Some(format!(
                r#"{{"url":"https://github.com/example/{name}","source":"github"}}"#
            )),
            status: "active".to_string(),
            published_at: None,
            updated_at: None,
            synced_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_cache_upsert_and_retrieve() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        let entry = create_test_entry("test-server", "1.0.0");
        service.upsert(&entry).await.unwrap();

        let retrieved = service.get_by_name("test-server").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.server_name, "test-server");
        assert_eq!(retrieved.version, "1.0.0");
        assert_eq!(retrieved.title, Some("test-server Server".to_string()));
    }

    #[tokio::test]
    async fn test_cache_incremental_sync_updates_existing() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        // Insert initial entry
        let entry1 = create_test_entry("sync-server", "1.0.0");
        service.upsert(&entry1).await.unwrap();

        // Sync with updated version
        let entry2 = create_test_entry("sync-server", "2.0.0");
        service.sync_incremental(&[entry2]).await.unwrap();

        let retrieved = service.get_by_name("sync-server").await.unwrap().unwrap();
        assert_eq!(retrieved.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_cache_search_matches_name_and_description() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        // Insert test entries
        let entry1 = create_test_entry("filesystem-server", "1.0.0");
        let entry2 = RegistryCacheEntry {
            server_name: "other-server".to_string(),
            version: "1.0.0".to_string(),
            schema_url: None,
            title: Some("Other Server".to_string()),
            description: Some("A filesystem tool for MCP".to_string()),
            packages_json: None,
            remotes_json: None,
            icons_json: None,
            meta_json: None,
            website_url: None,
            repository_json: None,
            status: "active".to_string(),
            published_at: None,
            updated_at: None,
            synced_at: Utc::now(),
        };
        let entry3 = create_test_entry("unrelated-server", "1.0.0");

        service.sync_incremental(&[entry1, entry2, entry3]).await.unwrap();

        // Search for "filesystem"
        let result = service.search_local("filesystem", 10, None).await.unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.servers.len(), 2);

        // Verify both matches are present
        let names: Vec<&str> = result.servers.iter().map(|s| s.server_name.as_str()).collect();
        assert!(names.contains(&"filesystem-server"));
        assert!(names.contains(&"other-server"));
    }

    #[tokio::test]
    async fn test_cache_handles_deleted_servers() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        // Insert test entries
        let entry1 = create_test_entry("active-server", "1.0.0");
        let entry2 = create_test_entry("deleted-server", "1.0.0");
        service.sync_incremental(&[entry1, entry2]).await.unwrap();

        // Mark deleted-server as deleted
        service.mark_deleted(&["active-server"]).await.unwrap();

        // Verify deleted server is marked
        let deleted = service.get_by_name("deleted-server").await.unwrap().unwrap();
        assert_eq!(deleted.status, "deleted");

        // Verify active server is still active
        let active = service.get_by_name("active-server").await.unwrap().unwrap();
        assert_eq!(active.status, "active");

        // Verify list_all with status filter
        let active_only = service.list_all(Some("active")).await.unwrap();
        assert_eq!(active_only.len(), 1);
        assert_eq!(active_only[0].server_name, "active-server");
    }

    #[tokio::test]
    async fn test_cache_search_pagination() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        // Insert multiple entries
        for i in 0..5 {
            let entry = create_test_entry(&format!("server-{}", i), "1.0.0");
            service.upsert(&entry).await.unwrap();
        }

        // Search with limit
        let result = service.search_local("server", 2, None).await.unwrap();
        assert_eq!(result.servers.len(), 2);
        assert_eq!(result.total, 5);
        assert!(result.next_cursor.is_some());

        // Search with cursor
        let result2 = service
            .search_local("server", 2, result.next_cursor.as_deref())
            .await
            .unwrap();
        assert_eq!(result2.servers.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_last_sync_time() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        // Initially no sync time
        let time = service.last_sync_time().await.unwrap();
        assert!(time.is_none());

        // After sync, should have a time
        let entry = create_test_entry("synced-server", "1.0.0");
        service.upsert(&entry).await.unwrap();

        let time = service.last_sync_time().await.unwrap();
        assert!(time.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let pool = setup_test_db().await;
        let service = RegistryCacheService::new(pool);

        // Insert entries
        let entry = create_test_entry("clear-test", "1.0.0");
        service.upsert(&entry).await.unwrap();

        assert_eq!(service.count().await.unwrap(), 1);

        // Clear
        service.clear().await.unwrap();

        assert_eq!(service.count().await.unwrap(), 0);
    }
}
