//! Migration utility from JSON files to Redb format

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use super::{
    manager::RedbCacheManager,
    types::*,
};

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub json_cache_dir: PathBuf,
    pub backup_original: bool,
    pub validate_after_migration: bool,
    pub batch_size: usize,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            json_cache_dir: PathBuf::from("cache"),
            backup_original: true,
            validate_after_migration: true,
            batch_size: 100,
        }
    }
}

/// Migration report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_files_found: usize,
    pub files_migrated: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub total_servers: usize,
    pub total_tools: usize,
    pub total_resources: usize,
    pub total_prompts: usize,
    pub errors: Vec<MigrationError>,
    pub warnings: Vec<String>,
    pub validation_results: Option<ValidationResults>,
}

/// Migration validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub servers_validated: usize,
    pub servers_failed: usize,
    pub data_integrity_ok: bool,
    pub performance_improvement: Option<PerformanceComparison>,
}

/// Performance comparison between JSON and Redb
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub json_read_time_ms: f64,
    pub redb_read_time_ms: f64,
    pub performance_improvement_factor: f64,
    pub storage_size_reduction_percent: f64,
}

/// Migration error types
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum MigrationError {
    #[error("Failed to read JSON file {file}: {error}")]
    JsonReadError { file: String, error: String },
    
    #[error("Failed to parse JSON file {file}: {error}")]
    JsonParseError { file: String, error: String },
    
    #[error("Failed to store data in Redb: {error}")]
    RedbStoreError { error: String },
    
    #[error("Data validation failed for server {server_id}: {error}")]
    ValidationError { server_id: String, error: String },
    
    #[error("IO error: {error}")]
    IoError { error: String },
}

/// Legacy JSON cache file structure (from inspect module)
#[derive(Debug, Clone, Deserialize)]
struct LegacyServerCapabilities {
    pub server_name: String,
    pub server_version: Option<String>,
    pub protocol_version: String,
    pub tools: Vec<LegacyToolInfo>,
    pub resources: Vec<LegacyResourceInfo>,
    pub prompts: Vec<LegacyPromptInfo>,
    pub resource_templates: Option<Vec<LegacyResourceTemplateInfo>>,
    #[serde(default = "Utc::now")]
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyResourceInfo {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyPromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<LegacyPromptArgument>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyResourceTemplateInfo {
    pub uri_template: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Cache migrator for JSON to Redb conversion
pub struct CacheMigrator {
    config: MigrationConfig,
    cache_manager: RedbCacheManager,
}

impl CacheMigrator {
    /// Create a new cache migrator
    pub fn new(cache_manager: RedbCacheManager, config: MigrationConfig) -> Self {
        Self {
            config,
            cache_manager,
        }
    }
    
    /// Migrate all JSON cache files to Redb format
    pub async fn migrate_all(&self) -> Result<MigrationReport> {
        let started_at = Utc::now();
        info!("Starting cache migration from JSON to Redb format");
        
        // Create backup if requested
        if self.config.backup_original {
            self.create_backup().await?;
        }
        
        // Discover JSON cache files
        let json_files = self.discover_json_files()?;
        info!("Found {} JSON cache files to migrate", json_files.len());
        
        let mut report = MigrationReport {
            started_at,
            completed_at: Utc::now(), // Will be updated at the end
            total_files_found: json_files.len(),
            files_migrated: 0,
            files_skipped: 0,
            files_failed: 0,
            total_servers: 0,
            total_tools: 0,
            total_resources: 0,
            total_prompts: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            validation_results: None,
        };
        
        // Process files in batches
        for batch in json_files.chunks(self.config.batch_size) {
            for json_file in batch {
                match self.migrate_single_file(json_file).await {
                    Ok(server_data) => {
                        report.files_migrated += 1;
                        report.total_servers += 1;
                        report.total_tools += server_data.tools.len();
                        report.total_resources += server_data.resources.len();
                        report.total_prompts += server_data.prompts.len();
                        
                        debug!("Successfully migrated: {:?}", json_file);
                    }
                    Err(e) => {
                        report.files_failed += 1;
                        report.errors.push(e);
                        error!("Failed to migrate file: {:?}", json_file);
                    }
                }
            }
        }
        
        // Validate migration if requested
        if self.config.validate_after_migration {
            info!("Running post-migration validation");
            match self.validate_migration(&json_files).await {
                Ok(validation_results) => {
                    report.validation_results = Some(validation_results);
                }
                Err(e) => {
                    warn!("Migration validation failed: {}", e);
                    report.warnings.push(format!("Validation failed: {}", e));
                }
            }
        }
        
        report.completed_at = Utc::now();
        
        info!(
            "Migration completed: {}/{} files migrated successfully",
            report.files_migrated, report.total_files_found
        );
        
        Ok(report)
    }
    
    /// Discover all JSON cache files in the cache directory
    fn discover_json_files(&self) -> Result<Vec<PathBuf>> {
        let mut json_files = Vec::new();
        
        if !self.config.json_cache_dir.exists() {
            warn!("JSON cache directory does not exist: {:?}", self.config.json_cache_dir);
            return Ok(json_files);
        }
        
        for entry in WalkDir::new(&self.config.json_cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                // Skip non-cache files (look for server capability cache pattern)
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.contains("capabilities") || file_name.starts_with("server_") {
                        json_files.push(path.to_path_buf());
                    }
                }
            }
        }
        
        Ok(json_files)
    }
    
    /// Migrate a single JSON file to Redb format
    async fn migrate_single_file(&self, json_file: &Path) -> Result<CachedServerData, MigrationError> {
        // Read JSON file
        let json_content = fs::read_to_string(json_file)
            .map_err(|e| MigrationError::JsonReadError {
                file: json_file.display().to_string(),
                error: e.to_string(),
            })?;
        
        // Parse JSON
        let legacy_data: LegacyServerCapabilities = serde_json::from_str(&json_content)
            .map_err(|e| MigrationError::JsonParseError {
                file: json_file.display().to_string(),
                error: e.to_string(),
            })?;
        
        // Extract server ID from filename or generate one
        let server_id = self.extract_server_id_from_filename(json_file)
            .unwrap_or_else(|| {
                use nanoid::nanoid;
                format!("migrated_{}", nanoid!(12))
            });
        
        // Convert to new format
        let server_data = self.convert_legacy_to_cached_data(server_id, legacy_data)?;
        
        // Store in Redb
        self.cache_manager.store_server_data(&server_data).await
            .map_err(|e| MigrationError::RedbStoreError {
                error: e.to_string(),
            })?;
        
        Ok(server_data)
    }
    
    /// Extract server ID from JSON filename
    fn extract_server_id_from_filename(&self, json_file: &Path) -> Option<String> {
        json_file.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| {
                // Handle common patterns:
                // - server_<id>_capabilities.json -> <id>
                // - <id>_capabilities.json -> <id>
                // - capabilities_<id>.json -> <id>
                if stem.starts_with("server_") && stem.ends_with("_capabilities") {
                    stem.strip_prefix("server_")
                        .and_then(|s| s.strip_suffix("_capabilities"))
                        .unwrap_or(stem)
                        .to_string()
                } else if stem.ends_with("_capabilities") {
                    stem.strip_suffix("_capabilities")
                        .unwrap_or(stem)
                        .to_string()
                } else if stem.starts_with("capabilities_") {
                    stem.strip_prefix("capabilities_")
                        .unwrap_or(stem)
                        .to_string()
                } else {
                    stem.to_string()
                }
            })
    }
    
    /// Convert legacy data structure to new cached data format
    fn convert_legacy_to_cached_data(
        &self,
        server_id: String,
        legacy: LegacyServerCapabilities,
    ) -> Result<CachedServerData, MigrationError> {
        let now = Utc::now();
        
        // Convert tools
        let tools = legacy.tools.into_iter().map(|tool| CachedToolInfo {
            name: tool.name.clone(),
            description: tool.description,
            input_schema_json: serde_json::to_string(&tool.input_schema).unwrap_or_default(),
            unique_name: Some(format!("{}_{}", server_id, tool.name)),
            enabled: true, // Default to enabled during migration
            cached_at: now,
        }).collect();
        
        // Convert resources
        let resources = legacy.resources.into_iter().map(|resource| CachedResourceInfo {
            uri: resource.uri,
            name: resource.name,
            description: resource.description,
            mime_type: resource.mime_type,
            enabled: true, // Default to enabled during migration
            cached_at: now,
        }).collect();
        
        // Convert prompts
        let prompts = legacy.prompts.into_iter().map(|prompt| CachedPromptInfo {
            name: prompt.name,
            description: prompt.description,
            arguments: prompt.arguments.into_iter().map(|arg| PromptArgument {
                name: arg.name,
                description: arg.description,
                required: arg.required,
            }).collect(),
            enabled: true, // Default to enabled during migration
            cached_at: now,
        }).collect();
        
        // Convert resource templates
        let resource_templates = legacy.resource_templates
            .unwrap_or_default()
            .into_iter()
            .map(|template| CachedResourceTemplateInfo {
                uri_template: template.uri_template,
                name: template.name,
                description: template.description,
                mime_type: template.mime_type,
                enabled: true, // Default to enabled during migration
                cached_at: now,
            })
            .collect();
        
        Ok(CachedServerData {
            server_id: server_id.clone(),
            server_name: legacy.server_name,
            server_version: legacy.server_version,
            protocol_version: legacy.protocol_version,
            tools,
            resources,
            prompts,
            resource_templates,
            cached_at: legacy.cached_at,
            fingerprint: format!("migrated_{}", server_id),
        })
    }
    
    /// Create backup of original JSON files
    async fn create_backup(&self) -> Result<()> {
        let backup_dir = self.config.json_cache_dir.with_extension("backup");
        
        if backup_dir.exists() {
            warn!("Backup directory already exists: {:?}", backup_dir);
            return Ok(());
        }
        
        info!("Creating backup at: {:?}", backup_dir);
        
        // Copy entire cache directory to backup location
        Self::copy_dir_recursive(&self.config.json_cache_dir, &backup_dir)
            .context("Failed to create backup")?;
        
        info!("Backup created successfully");
        Ok(())
    }
    
    /// Recursively copy directory
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate migration by comparing JSON and Redb data
    async fn validate_migration(&self, json_files: &[PathBuf]) -> Result<ValidationResults> {
        let mut servers_validated = 0;
        let mut servers_failed = 0;
        let mut json_read_times = Vec::new();
        let mut redb_read_times = Vec::new();
        
        for json_file in json_files.iter().take(10) { // Sample validation
            match self.validate_single_file(json_file).await {
                Ok((json_time, redb_time)) => {
                    servers_validated += 1;
                    json_read_times.push(json_time);
                    redb_read_times.push(redb_time);
                }
                Err(_) => {
                    servers_failed += 1;
                }
            }
        }
        
        let data_integrity_ok = servers_failed == 0;
        
        let performance_improvement = if !json_read_times.is_empty() && !redb_read_times.is_empty() {
            let avg_json_time = json_read_times.iter().sum::<f64>() / json_read_times.len() as f64;
            let avg_redb_time = redb_read_times.iter().sum::<f64>() / redb_read_times.len() as f64;
            let improvement_factor = avg_json_time / avg_redb_time;
            
            // Estimate storage size reduction
            let json_size = self.estimate_json_cache_size()?;
            let redb_size = self.cache_manager.get_stats().await.cache_size_bytes;
            let size_reduction = if json_size > 0 {
                ((json_size as f64 - redb_size as f64) / json_size as f64) * 100.0
            } else {
                0.0
            };
            
            Some(PerformanceComparison {
                json_read_time_ms: avg_json_time,
                redb_read_time_ms: avg_redb_time,
                performance_improvement_factor: improvement_factor,
                storage_size_reduction_percent: size_reduction,
            })
        } else {
            None
        };
        
        Ok(ValidationResults {
            servers_validated,
            servers_failed,
            data_integrity_ok,
            performance_improvement,
        })
    }
    
    /// Validate a single file by comparing JSON and Redb data
    async fn validate_single_file(&self, json_file: &Path) -> Result<(f64, f64)> {
        let server_id = self.extract_server_id_from_filename(json_file)
            .ok_or_else(|| anyhow::anyhow!("Could not extract server ID from filename"))?;
        
        // Time JSON read
        let json_start = std::time::Instant::now();
        let json_content = fs::read_to_string(json_file)?;
        let _legacy_data: LegacyServerCapabilities = serde_json::from_str(&json_content)?;
        let json_time = json_start.elapsed().as_secs_f64() * 1000.0;
        
        // Time Redb read
        let redb_start = std::time::Instant::now();
        let query = CacheQuery {
            server_id,
            freshness_level: FreshnessLevel::Cached,
            include_disabled: true,
        };
        let _redb_data = self.cache_manager.get_server_data(&query).await?;
        let redb_time = redb_start.elapsed().as_secs_f64() * 1000.0;
        
        Ok((json_time, redb_time))
    }
    
    /// Estimate total JSON cache size
    fn estimate_json_cache_size(&self) -> Result<u64> {
        let mut total_size = 0;
        
        for entry in WalkDir::new(&self.config.json_cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file() {
                total_size += entry.metadata()?.len();
            }
        }
        
        Ok(total_size)
    }
}

/// Print migration report in a formatted way
pub fn print_migration_report(report: &MigrationReport) {
    println!("\n=== Cache Migration Report ===");
    println!("Started: {}", report.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Completed: {}", report.completed_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Duration: {:.2} seconds", 
        report.completed_at.signed_duration_since(report.started_at).num_seconds());
    
    println!("\n--- Migration Summary ---");
    println!("Files found: {}", report.total_files_found);
    println!("Files migrated: {}", report.files_migrated);
    println!("Files skipped: {}", report.files_skipped);
    println!("Files failed: {}", report.files_failed);
    
    println!("\n--- Data Summary ---");
    println!("Servers: {}", report.total_servers);
    println!("Tools: {}", report.total_tools);
    println!("Resources: {}", report.total_resources);
    println!("Prompts: {}", report.total_prompts);
    
    if !report.errors.is_empty() {
        println!("\n--- Errors ---");
        for error in &report.errors {
            println!("  - {}", error);
        }
    }
    
    if !report.warnings.is_empty() {
        println!("\n--- Warnings ---");
        for warning in &report.warnings {
            println!("  - {}", warning);
        }
    }
    
    if let Some(validation) = &report.validation_results {
        println!("\n--- Validation Results ---");
        println!("Servers validated: {}", validation.servers_validated);
        println!("Servers failed: {}", validation.servers_failed);
        println!("Data integrity: {}", if validation.data_integrity_ok { "OK" } else { "FAILED" });
        
        if let Some(perf) = &validation.performance_improvement {
            println!("\n--- Performance Improvement ---");
            println!("JSON read time: {:.2}ms", perf.json_read_time_ms);
            println!("Redb read time: {:.2}ms", perf.redb_read_time_ms);
            println!("Performance improvement: {:.2}x", perf.performance_improvement_factor);
            println!("Storage reduction: {:.2}%", perf.storage_size_reduction_percent);
        }
    }
    
    println!("\n=== Migration Complete ===");
}