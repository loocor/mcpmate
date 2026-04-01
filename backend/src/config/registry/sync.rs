// Registry sync service for background synchronization

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::Pool;
use sqlx::Sqlite;
use tracing;

use super::RegistryCacheService;
use super::cache::RegistryCacheEntry;

const REGISTRY_API_URL: &str = "https://registry.modelcontextprotocol.io/v0.1/servers";
const SYNC_INTERVAL_SECS: u64 = 3600; // 1 hour
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Registry server from API response
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryServer {
    pub name: String,
    pub version: String,
    #[serde(rename = "$schema")]
    pub schema_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    pub repository: Option<RegistryRepository>,
    #[serde(default)]
    pub packages: Vec<RegistryPackage>,
    #[serde(default)]
    pub remotes: Vec<RegistryRemote>,
    #[serde(default)]
    pub icons: Vec<RegistryIcon>,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
    pub status: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryPackage {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryRemote {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryIcon {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryRepository {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub subfolder: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

/// Registry API response
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryResponse {
    pub servers: Vec<RegistryServerEnvelope>,
    pub metadata: RegistryResponseMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryServerEnvelope {
    pub server: RegistryServer,
    #[serde(rename = "_meta")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryResponseMetadata {
    pub next_cursor: Option<String>,
    pub count: usize,
}

/// Registry sync service
pub struct RegistrySyncService {
    cache_service: RegistryCacheService,
    client: Client,
}

impl RegistrySyncService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent("MCPMate/0.1.0 (+https://mcp.umate.ai)")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            cache_service: RegistryCacheService::new(pool),
            client,
        }
    }

    /// Perform initial sync on startup
    pub async fn initial_sync(&self) -> Result<()> {
        tracing::info!("Starting initial registry sync");

        match self.sync_all().await {
            Ok(count) => {
                tracing::info!("Initial registry sync completed: {} servers cached", count);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Initial registry sync failed: {}", e);
                Err(e)
            }
        }
    }

    /// Sync all servers from registry
    pub async fn sync_all(&self) -> Result<usize> {
        let mut all_servers = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let response = self.fetch_servers(cursor.as_deref()).await?;
            let count = response.servers.len();
            all_servers.extend(response.servers);

            cursor = response.metadata.next_cursor;
            if cursor.is_none() {
                break;
            }

            tracing::debug!("Fetched {} servers, continuing with cursor", count);
        }

        // Convert to cache entries and sync
        let entries: Vec<RegistryCacheEntry> = all_servers.iter().map(|s| self.server_to_entry(s)).collect();

        let total_synced = self.cache_service.sync_incremental(&entries).await?;

        // Mark deleted servers
        let active_names: Vec<&str> = all_servers.iter().map(|s| s.server.name.as_str()).collect();
        let deleted_count = self.cache_service.mark_deleted(&active_names).await?;

        if deleted_count > 0 {
            tracing::info!("Marked {} servers as deleted", deleted_count);
        }

        Ok(total_synced)
    }

    /// Perform incremental sync using updated_since
    pub async fn sync_incremental(&self) -> Result<usize> {
        let last_sync = self.cache_service.last_sync_time().await?;

        let updated_since = last_sync.map(|t| t.to_rfc3339());

        if updated_since.is_none() {
            return self.sync_all().await;
        }

        tracing::info!("Starting incremental registry sync since {:?}", updated_since);

        let mut all_servers = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let response = self
                .fetch_servers_with_updated_since(cursor.as_deref(), updated_since.as_deref())
                .await?;
            all_servers.extend(response.servers);

            cursor = response.metadata.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        let entries: Vec<RegistryCacheEntry> = all_servers.iter().map(|s| self.server_to_entry(s)).collect();

        let count = self.cache_service.sync_incremental(&entries).await?;
        tracing::info!("Incremental sync completed: {} servers updated", count);

        Ok(count)
    }

    /// Fetch servers from registry API
    async fn fetch_servers(
        &self,
        cursor: Option<&str>,
    ) -> Result<RegistryResponse> {
        self.fetch_servers_with_params(cursor, None, None).await
    }

    /// Fetch servers with updated_since filter
    async fn fetch_servers_with_updated_since(
        &self,
        cursor: Option<&str>,
        updated_since: Option<&str>,
    ) -> Result<RegistryResponse> {
        self.fetch_servers_with_params(cursor, updated_since, None).await
    }

    /// Fetch servers with all parameters
    async fn fetch_servers_with_params(
        &self,
        cursor: Option<&str>,
        updated_since: Option<&str>,
        include_deleted: Option<bool>,
    ) -> Result<RegistryResponse> {
        let mut request = self.client.get(REGISTRY_API_URL).query(&[("limit", "100")]);

        if let Some(c) = cursor {
            request = request.query(&[("cursor", c)]);
        }

        if let Some(since) = updated_since {
            request = request.query(&[("updated_since", since)]);
        }

        if include_deleted.unwrap_or(false) {
            request = request.query(&[("include_deleted", "true")]);
        }

        let response = request
            .send()
            .await
            .with_context(|| "Failed to fetch from registry API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Registry API returned status {}", response.status()));
        }

        let data: RegistryResponse = response
            .json()
            .await
            .with_context(|| "Failed to parse registry response")?;

        Ok(data)
    }

    /// Convert registry server to cache entry
    fn server_to_entry(
        &self,
        envelope: &RegistryServerEnvelope,
    ) -> RegistryCacheEntry {
        let server = &envelope.server;
        RegistryCacheEntry {
            server_name: server.name.clone(),
            version: server.version.clone(),
            schema_url: server.schema_url.clone(),
            title: server.title.clone(),
            description: server.description.clone(),
            packages_json: serde_json::to_string(&server.packages).ok(),
            remotes_json: serde_json::to_string(&server.remotes).ok(),
            icons_json: serde_json::to_string(&server.icons).ok(),
            meta_json: envelope
                .meta
                .as_ref()
                .or(server.meta.as_ref())
                .and_then(|m| serde_json::to_string(m).ok()),
            website_url: server.website_url.clone(),
            repository_json: server
                .repository
                .as_ref()
                .and_then(|repo| serde_json::to_string(repo).ok()),
            status: server.status.clone().unwrap_or_else(|| "active".to_string()),
            published_at: server.published_at,
            updated_at: server.updated_at,
            synced_at: Utc::now(),
        }
    }

    /// Start background sync task
    pub fn start_background_sync(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(SYNC_INTERVAL_SECS));

            loop {
                interval.tick().await;

                tracing::debug!("Starting scheduled registry sync");

                if let Err(e) = self.sync_incremental().await {
                    tracing::warn!("Scheduled registry sync failed: {}", e);
                }
            }
        });
    }
}

/// Start registry sync service
pub fn start_registry_sync_service(pool: Pool<Sqlite>) -> Arc<RegistrySyncService> {
    let service = Arc::new(RegistrySyncService::new(pool));

    // Spawn initial sync task
    let service_clone = Arc::clone(&service);
    tokio::spawn(async move {
        if let Err(e) = service_clone.initial_sync().await {
            tracing::warn!("Initial registry sync failed: {}", e);
        }
    });

    // Start background sync
    let service_clone = Arc::clone(&service);
    service_clone.start_background_sync();

    service
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_server_deserialization() {
        let json = r#"{
            "server": {
                "name": "test-server",
                "version": "1.0.0",
                "title": "Test Server",
                "description": "A test server",
                "websiteUrl": "https://example.com/server",
                "repository": {
                    "url": "https://github.com/example/test-server",
                    "source": "github"
                },
                "packages": [{"name": "test-pkg", "version": "1.0.0"}],
                "remotes": [{"url": "https://example.com", "type": "http"}],
                "icons": [{"url": "https://example.com/icon.png"}],
                "status": "active"
            },
            "_meta": {
                "io.modelcontextprotocol.registry/official": {
                    "status": "active"
                }
            }
        }"#;

        let server: RegistryServerEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(server.server.name, "test-server");
        assert_eq!(server.server.version, "1.0.0");
        assert_eq!(server.server.title, Some("Test Server".to_string()));
        assert_eq!(server.server.packages.len(), 1);
        assert_eq!(server.server.remotes.len(), 1);
        assert_eq!(server.server.icons.len(), 1);
        assert!(server.meta.is_some());
    }

    #[tokio::test]
    async fn test_server_to_entry_conversion() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let service = RegistrySyncService::new(pool);

        let server = RegistryServerEnvelope {
            server: RegistryServer {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
                schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
                title: Some("Test Server".to_string()),
                description: Some("A test server".to_string()),
                website_url: Some("https://example.com/server".to_string()),
                repository: Some(RegistryRepository {
                    url: Some("https://github.com/example/test-server".to_string()),
                    source: Some("github".to_string()),
                    subfolder: None,
                    id: None,
                }),
                packages: vec![RegistryPackage {
                    name: Some("test-pkg".to_string()),
                    version: Some("1.0.0".to_string()),
                }],
                remotes: vec![],
                icons: vec![],
                meta: None,
                status: Some("active".to_string()),
                published_at: None,
                updated_at: None,
            },
            meta: Some(serde_json::json!({
                "io.modelcontextprotocol.registry/official": {
                    "status": "active"
                }
            })),
        };

        let entry = service.server_to_entry(&server);
        assert_eq!(entry.server_name, "test-server");
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.title, Some("Test Server".to_string()));
        assert!(entry.packages_json.is_some());
        assert_eq!(entry.website_url.as_deref(), Some("https://example.com/server"));
        assert!(entry.repository_json.is_some());
    }
}
