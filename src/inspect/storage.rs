// Temporary File Storage for Inspect System
// Manages capability information cache in system temporary directory

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs as async_fs;
use tokio::sync::Mutex;
use tokio::time::{Interval, interval};

use super::types::{
    CacheConfig, CacheEntryMetadata, InspectError, InspectResult, FileCacheManifest,
    ServerCapabilities,
};

/// Temporary file storage for capabilities cache
pub struct TempFileStorage {
    /// Cache directory path
    cache_dir: PathBuf,
    /// Configuration
    config: CacheConfig,
    /// Cleanup scheduler
    cleanup_interval: Option<Interval>,
    /// Current process ID for session isolation
    #[allow(dead_code)]
    process_id: u32,
    /// Mutex to protect manifest file operations
    manifest_mutex: Arc<Mutex<()>>,
}

impl Clone for TempFileStorage {
    fn clone(&self) -> Self {
        Self {
            cache_dir: self.cache_dir.clone(),
            config: self.config.clone(),
            cleanup_interval: None, // Don't clone interval, each instance manages its own
            process_id: self.process_id,
            manifest_mutex: Arc::clone(&self.manifest_mutex),
        }
    }
}

impl TempFileStorage {
    /// Create new temporary file storage
    pub fn new(config: CacheConfig) -> InspectResult<Self> {
        let process_id = std::process::id();
        let cache_dir = Self::create_cache_directory(process_id)?;

        Ok(Self {
            cache_dir,
            config,
            cleanup_interval: None,
            process_id,
            manifest_mutex: Arc::new(Mutex::new(())),
        })
    }

    /// Create cache directory in system temp
    fn create_cache_directory(process_id: u32) -> InspectResult<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let cache_dir = temp_dir.join(format!("mcpmate-capabilities/session-{}", process_id));

        fs::create_dir_all(&cache_dir).map_err(|e| {
            InspectError::CacheError(format!("Failed to create cache directory: {}", e))
        })?;

        tracing::info!("Created inspect cache directory: {:?}", cache_dir);
        Ok(cache_dir)
    }

    /// Start cleanup scheduler
    pub fn start_cleanup_scheduler(&mut self) {
        let cleanup_timer = interval(self.config.cleanup_interval);
        self.cleanup_interval = Some(cleanup_timer);

        let cache_dir = self.cache_dir.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut cleanup_interval = interval(config.cleanup_interval);
            loop {
                cleanup_interval.tick().await;
                if let Err(e) = Self::cleanup_expired_files(&cache_dir, &config).await {
                    tracing::warn!("Cache cleanup failed: {}", e);
                }
            }
        });
    }

    /// Store server capabilities to file
    pub async fn store_capabilities(
        &self,
        server_id: &str,
        capabilities: &ServerCapabilities,
    ) -> InspectResult<()> {
        let server_dir = self.cache_dir.join(server_id);
        async_fs::create_dir_all(&server_dir).await.map_err(|e| {
            InspectError::CacheError(format!("Failed to create server directory: {}", e))
        })?;

        // Store capabilities
        let capabilities_file = server_dir.join("capabilities.json");
        let capabilities_json = serde_json::to_string_pretty(capabilities)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        async_fs::write(&capabilities_file, capabilities_json)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to write capabilities file: {}", e))
            })?;

        // Store individual components for faster access
        self.store_tools(server_id, &capabilities.tools).await?;
        self.store_resources(server_id, &capabilities.resources)
            .await?;
        self.store_prompts(server_id, &capabilities.prompts).await?;

        // Update manifest
        self.update_manifest(server_id, &capabilities_file).await?;

        tracing::debug!("Stored capabilities for server '{}'", server_id);
        Ok(())
    }

    /// Load server capabilities from file
    pub async fn load_capabilities(
        &self,
        server_id: &str,
    ) -> InspectResult<Option<ServerCapabilities>> {
        let capabilities_file = self.cache_dir.join(server_id).join("capabilities.json");

        if !capabilities_file.exists() {
            return Ok(None);
        }

        // Check if file is expired
        if self.is_file_expired(&capabilities_file).await? {
            tracing::debug!("Capabilities file for server '{}' is expired", server_id);
            return Ok(None);
        }

        let content = async_fs::read_to_string(&capabilities_file)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to read capabilities file: {}", e))
            })?;

        let capabilities: ServerCapabilities = serde_json::from_str(&content)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        // Update last accessed time
        self.touch_file(&capabilities_file).await?;

        tracing::debug!("Loaded capabilities for server '{}'", server_id);
        Ok(Some(capabilities))
    }

    /// Store tools separately for faster access
    async fn store_tools(
        &self,
        server_id: &str,
        tools: &[super::types::ToolInfo],
    ) -> InspectResult<()> {
        let tools_file = self.cache_dir.join(server_id).join("tools.json");
        let tools_json = serde_json::to_string_pretty(tools)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        async_fs::write(&tools_file, tools_json)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to write tools file: {}", e))
            })?;

        Ok(())
    }

    /// Store resources separately for faster access
    async fn store_resources(
        &self,
        server_id: &str,
        resources: &[super::types::ResourceInfo],
    ) -> InspectResult<()> {
        let resources_file = self.cache_dir.join(server_id).join("resources.json");
        let resources_json = serde_json::to_string_pretty(resources)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        async_fs::write(&resources_file, resources_json)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to write resources file: {}", e))
            })?;

        Ok(())
    }

    /// Store prompts separately for faster access
    async fn store_prompts(
        &self,
        server_id: &str,
        prompts: &[super::types::PromptInfo],
    ) -> InspectResult<()> {
        let prompts_file = self.cache_dir.join(server_id).join("prompts.json");
        let prompts_json = serde_json::to_string_pretty(prompts)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        async_fs::write(&prompts_file, prompts_json)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to write prompts file: {}", e))
            })?;

        Ok(())
    }

    /// Update cache manifest with concurrent access protection
    async fn update_manifest(
        &self,
        server_id: &str,
        capabilities_file: &Path,
    ) -> InspectResult<()> {
        // Acquire manifest lock to prevent concurrent modifications
        let _lock = self.manifest_mutex.lock().await;

        let manifest_file = self.cache_dir.join("manifest.json");

        // Load existing manifest or create new one
        let mut manifest = if manifest_file.exists() {
            let content = async_fs::read_to_string(&manifest_file)
                .await
                .map_err(|e| {
                    InspectError::CacheError(format!("Failed to read manifest: {}", e))
                })?;
            serde_json::from_str::<FileCacheManifest>(&content).unwrap_or_else(|_| {
                tracing::warn!("Corrupted manifest file, creating new one");
                FileCacheManifest {
                    entries: HashMap::new(),
                    total_size: 0,
                    last_cleanup: SystemTime::now(),
                }
            })
        } else {
            FileCacheManifest {
                entries: HashMap::new(),
                total_size: 0,
                last_cleanup: SystemTime::now(),
            }
        };

        // Get file metadata
        let metadata = async_fs::metadata(capabilities_file).await.map_err(|e| {
            InspectError::CacheError(format!("Failed to get file metadata: {}", e))
        })?;

        let entry_metadata = CacheEntryMetadata {
            created_at: metadata.created().unwrap_or(SystemTime::now()),
            last_accessed: SystemTime::now(),
            size: metadata.len(),
            version: "1.0".to_string(),
        };

        // Update manifest atomically
        if let Some(old_entry) = manifest.entries.get(server_id) {
            manifest.total_size = manifest.total_size.saturating_sub(old_entry.size);
        }
        manifest.total_size += entry_metadata.size;
        manifest
            .entries
            .insert(server_id.to_string(), entry_metadata);

        // Save manifest atomically
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        // Write to temporary file first, then rename for atomic operation
        let temp_manifest_file = manifest_file.with_extension("tmp");
        async_fs::write(&temp_manifest_file, &manifest_json)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to write temp manifest: {}", e))
            })?;

        async_fs::rename(&temp_manifest_file, &manifest_file)
            .await
            .map_err(|e| InspectError::CacheError(format!("Failed to rename manifest: {}", e)))?;

        tracing::debug!(
            "Updated manifest for server '{}' with atomic operation",
            server_id
        );
        Ok(())
    }

    /// Check if file is expired based on TTL
    async fn is_file_expired(
        &self,
        file_path: &Path,
    ) -> InspectResult<bool> {
        let metadata = async_fs::metadata(file_path).await.map_err(|e| {
            InspectError::CacheError(format!("Failed to get file metadata: {}", e))
        })?;

        let modified = metadata.modified().map_err(|e| {
            InspectError::CacheError(format!("Failed to get file modification time: {}", e))
        })?;

        let elapsed = SystemTime::now().duration_since(modified).map_err(|e| {
            InspectError::CacheError(format!("Failed to calculate file age: {}", e))
        })?;

        Ok(elapsed > self.config.max_file_age)
    }

    /// Touch file to update access time
    async fn touch_file(
        &self,
        file_path: &Path,
    ) -> InspectResult<()> {
        // Simple way to update access time - read and write back
        if file_path.exists() {
            let content = async_fs::read(file_path).await.map_err(|e| {
                InspectError::CacheError(format!("Failed to read file for touch: {}", e))
            })?;
            async_fs::write(file_path, content)
                .await
                .map_err(|e| InspectError::CacheError(format!("Failed to touch file: {}", e)))?;
        }
        Ok(())
    }

    /// Cleanup expired files and update manifest accordingly
    async fn cleanup_expired_files(
        cache_dir: &Path,
        config: &CacheConfig,
    ) -> InspectResult<()> {
        if !cache_dir.exists() {
            return Ok(());
        }

        let mut total_size = 0u64;
        let mut entries_to_remove = Vec::new();
        let mut server_ids_to_remove = Vec::new();

        // Read directory entries
        let mut dir_entries = async_fs::read_dir(cache_dir).await.map_err(|e| {
            InspectError::CacheError(format!("Failed to read cache directory: {}", e))
        })?;

        while let Some(entry) = dir_entries.next_entry().await.map_err(|e| {
            InspectError::CacheError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_dir() {
                // Check server directory
                if let Err(e) = Self::cleanup_server_directory(
                    &path,
                    config,
                    &mut total_size,
                    &mut entries_to_remove,
                    &mut server_ids_to_remove,
                )
                .await
                {
                    tracing::warn!("Failed to cleanup server directory {:?}: {}", path, e);
                }
            }
        }

        // Remove expired entries
        for path in &entries_to_remove {
            if let Err(e) = async_fs::remove_dir_all(path).await {
                tracing::warn!("Failed to remove expired directory {:?}: {}", path, e);
            } else {
                tracing::debug!("Removed expired cache directory: {:?}", path);
            }
        }

        // Update manifest to remove deleted entries
        if !server_ids_to_remove.is_empty() {
            if let Err(e) =
                Self::update_manifest_after_cleanup(cache_dir, &server_ids_to_remove).await
            {
                tracing::warn!("Failed to update manifest after cleanup: {}", e);
            }
        }

        // Check total size limit
        if total_size > config.max_file_cache_size {
            tracing::warn!(
                "Cache size ({} bytes) exceeds limit ({} bytes), consider reducing cache size",
                total_size,
                config.max_file_cache_size
            );
        }

        tracing::debug!(
            "Cache cleanup completed, total size: {} bytes, removed {} entries",
            total_size,
            entries_to_remove.len()
        );
        Ok(())
    }

    /// Cleanup individual server directory
    async fn cleanup_server_directory(
        server_dir: &Path,
        config: &CacheConfig,
        total_size: &mut u64,
        entries_to_remove: &mut Vec<PathBuf>,
        server_ids_to_remove: &mut Vec<String>,
    ) -> InspectResult<()> {
        let capabilities_file = server_dir.join("capabilities.json");

        if capabilities_file.exists() {
            let metadata = async_fs::metadata(&capabilities_file).await.map_err(|e| {
                InspectError::CacheError(format!(
                    "Failed to get capabilities file metadata: {}",
                    e
                ))
            })?;

            let modified = metadata.modified().map_err(|e| {
                InspectError::CacheError(format!("Failed to get file modification time: {}", e))
            })?;

            let elapsed = SystemTime::now().duration_since(modified).map_err(|e| {
                InspectError::CacheError(format!("Failed to calculate file age: {}", e))
            })?;

            if elapsed > config.max_file_age {
                entries_to_remove.push(server_dir.to_path_buf());
                server_ids_to_remove.push(
                    server_dir
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                );
            } else {
                *total_size += metadata.len();
            }
        }

        Ok(())
    }

    /// Update manifest after cleanup
    async fn update_manifest_after_cleanup(
        cache_dir: &Path,
        server_ids_to_remove: &[String],
    ) -> InspectResult<()> {
        let manifest_file = cache_dir.join("manifest.json");

        if !manifest_file.exists() {
            return Ok(());
        }

        let content = async_fs::read_to_string(&manifest_file)
            .await
            .map_err(|e| InspectError::CacheError(format!("Failed to read manifest: {}", e)))?;

        let mut manifest: FileCacheManifest = serde_json::from_str(&content)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        // Remove entries from manifest
        for server_id in server_ids_to_remove {
            if let Some(removed_entry) = manifest.entries.remove(server_id) {
                manifest.total_size = manifest.total_size.saturating_sub(removed_entry.size);
            }
        }

        // Update last cleanup time
        manifest.last_cleanup = SystemTime::now();

        // Save manifest atomically
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        // Write to temporary file first, then rename for atomic operation
        let temp_manifest_file = manifest_file.with_extension("tmp");
        async_fs::write(&temp_manifest_file, &manifest_json)
            .await
            .map_err(|e| {
                InspectError::CacheError(format!("Failed to write temp manifest: {}", e))
            })?;

        async_fs::rename(&temp_manifest_file, &manifest_file)
            .await
            .map_err(|e| InspectError::CacheError(format!("Failed to rename manifest: {}", e)))?;

        tracing::debug!(
            "Updated manifest after cleanup, removed {} entries",
            server_ids_to_remove.len()
        );
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> InspectResult<CacheStats> {
        let manifest_file = self.cache_dir.join("manifest.json");

        if !manifest_file.exists() {
            return Ok(CacheStats {
                total_entries: 0,
                total_size: 0,
                last_cleanup: SystemTime::now(),
            });
        }

        let content = async_fs::read_to_string(&manifest_file)
            .await
            .map_err(|e| InspectError::CacheError(format!("Failed to read manifest: {}", e)))?;

        let manifest: FileCacheManifest = serde_json::from_str(&content)
            .map_err(|e| InspectError::SerializationError(e.to_string()))?;

        Ok(CacheStats {
            total_entries: manifest.entries.len(),
            total_size: manifest.total_size,
            last_cleanup: manifest.last_cleanup,
        })
    }

    /// Clear all cache files
    pub async fn clear_cache(&self) -> InspectResult<()> {
        if self.cache_dir.exists() {
            async_fs::remove_dir_all(&self.cache_dir)
                .await
                .map_err(|e| InspectError::CacheError(format!("Failed to clear cache: {}", e)))?;

            // Recreate cache directory
            async_fs::create_dir_all(&self.cache_dir)
                .await
                .map_err(|e| {
                    InspectError::CacheError(format!("Failed to recreate cache directory: {}", e))
                })?;
        }

        tracing::info!("Cleared inspect cache directory: {:?}", self.cache_dir);
        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cached entries
    pub total_entries: usize,
    /// Total cache size in bytes
    pub total_size: u64,
    /// Last cleanup time
    pub last_cleanup: SystemTime,
}

impl Drop for TempFileStorage {
    fn drop(&mut self) {
        // Cleanup on drop - best effort
        if let Err(e) = std::fs::remove_dir_all(&self.cache_dir) {
            tracing::warn!("Failed to cleanup cache directory on drop: {}", e);
        } else {
            tracing::debug!("Cleaned up cache directory on drop: {:?}", self.cache_dir);
        }
    }
}
