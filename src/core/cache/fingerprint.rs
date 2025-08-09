//! Unified fingerprinting system for change detection

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::debug;

use super::{
    manager::RedbCacheManager,
    types::CacheError,
};

/// Unified fingerprint for MCP server change detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MCPServerFingerprint {
    pub code_fingerprint: CodeFingerprint,
    pub dependency_fingerprint: DependencyFingerprint,
    pub capability_fingerprint: CapabilityFingerprint,
    pub config_fingerprint: ConfigFingerprint,
    pub combined_hash: String,
    pub generated_at: DateTime<Utc>,
}

/// Code-level fingerprint for file changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeFingerprint {
    pub file_hashes: HashMap<PathBuf, String>,
    pub total_files: usize,
    pub total_size: u64,
    pub last_modified: DateTime<Utc>,
}

/// Dependency fingerprint for package changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyFingerprint {
    pub package_lock_hash: Option<String>,
    pub manifest_hash: String,
    pub resolved_versions: HashMap<String, String>,
}

/// Capability fingerprint for MCP server capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityFingerprint {
    pub tools_hash: String,
    pub resources_hash: String,
    pub prompts_hash: String,
    pub server_info_hash: String,
}

/// Configuration fingerprint for server config changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFingerprint {
    pub server_config_hash: String,
    pub environment_hash: String,
    pub arguments_hash: String,
}

/// Change detection result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    CodeChange { files_changed: Vec<PathBuf> },
    DependencyChange { packages_changed: Vec<String> },
    CapabilityChange { capabilities_diff: CapabilityDiff },
    ConfigChange { config_diff: ConfigDiff },
    NoChange,
}

/// Capability differences
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDiff {
    pub tools_changed: bool,
    pub resources_changed: bool,
    pub prompts_changed: bool,
    pub server_info_changed: bool,
}

/// Configuration differences
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiff {
    pub server_config_changed: bool,
    pub environment_changed: bool,
    pub arguments_changed: bool,
}

/// Fingerprint comparison result
#[derive(Debug, Clone)]
pub struct FingerprintDiff {
    pub has_changes: bool,
    pub change_types: Vec<ChangeType>,
    pub old_fingerprint: MCPServerFingerprint,
    pub new_fingerprint: MCPServerFingerprint,
}

/// Fingerprint generation errors
#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),
    
    #[error("Failed to parse manifest file: {0}")]
    ManifestParseError(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Walk directory error: {0}")]
    WalkDir(#[from] walkdir::Error),
}

/// Trait for fingerprint generation
#[async_trait::async_trait]
pub trait FingerprintGenerator {
    async fn generate_fingerprint(&self, server_path: &Path) -> Result<MCPServerFingerprint, FingerprintError>;
    async fn compare_fingerprints(&self, old: &MCPServerFingerprint, new: &MCPServerFingerprint) -> FingerprintDiff;
}

/// Default fingerprint generator implementation
pub struct DefaultFingerprintGenerator {
    cache_manager: RedbCacheManager,
}

impl DefaultFingerprintGenerator {
    pub fn new(cache_manager: RedbCacheManager) -> Self {
        Self { cache_manager }
    }
    
    /// Generate code fingerprint by scanning files
    async fn generate_code_fingerprint(&self, server_path: &Path) -> Result<CodeFingerprint, FingerprintError> {
        if !server_path.exists() {
            return Err(FingerprintError::PathNotFound(server_path.to_path_buf()));
        }
        
        let mut file_hashes = HashMap::new();
        let mut total_files = 0;
        let mut total_size = 0;
        let mut last_modified = SystemTime::UNIX_EPOCH;
        
        // Scan for relevant source files
        let extensions = ["js", "ts", "py", "rs", "go", "java", "cpp", "c", "h"];
        
        for entry in walkdir::WalkDir::new(server_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        let content = fs::read(path).await?;
                        let hash = format!("{:x}", Sha256::digest(&content));
                        
                        let metadata = entry.metadata()?;
                        let modified = metadata.modified()?;
                        
                        if modified > last_modified {
                            last_modified = modified;
                        }
                        
                        file_hashes.insert(path.to_path_buf(), hash);
                        total_files += 1;
                        total_size += metadata.len();
                    }
                }
            }
        }
        
        let last_modified_dt = DateTime::<Utc>::from(last_modified);
        
        Ok(CodeFingerprint {
            file_hashes,
            total_files,
            total_size,
            last_modified: last_modified_dt,
        })
    }
    
    /// Generate dependency fingerprint from package files
    async fn generate_dependency_fingerprint(&self, server_path: &Path) -> Result<DependencyFingerprint, FingerprintError> {
        let mut package_lock_hash = None;
        let mut manifest_hash = String::new();
        let mut resolved_versions = HashMap::new();
        
        // Check for different package manager files
        let package_files = [
            ("package.json", "package-lock.json"),
            ("Cargo.toml", "Cargo.lock"),
            ("pyproject.toml", "poetry.lock"),
            ("requirements.txt", "requirements.txt"),
            ("go.mod", "go.sum"),
        ];
        
        for (manifest_file, lock_file) in &package_files {
            let manifest_path = server_path.join(manifest_file);
            let lock_path = server_path.join(lock_file);
            
            if manifest_path.exists() {
                let manifest_content = fs::read(&manifest_path).await?;
                manifest_hash = format!("{:x}", Sha256::digest(&manifest_content));
                
                if lock_path.exists() {
                    let lock_content = fs::read(&lock_path).await?;
                    package_lock_hash = Some(format!("{:x}", Sha256::digest(&lock_content)));
                }
                
                // Parse manifest for version information
                resolved_versions = self.parse_manifest_versions(&manifest_path, manifest_file).await
                    .unwrap_or_default();
                
                break;
            }
        }
        
        Ok(DependencyFingerprint {
            package_lock_hash,
            manifest_hash,
            resolved_versions,
        })
    }
    
    /// Parse manifest file for version information
    async fn parse_manifest_versions(&self, manifest_path: &Path, file_type: &str) -> Result<HashMap<String, String>, FingerprintError> {
        let content = fs::read_to_string(manifest_path).await?;
        let mut versions = HashMap::new();
        
        match file_type {
            "package.json" => {
                if let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(deps) = package_json.get("dependencies").and_then(|d| d.as_object()) {
                        for (name, version) in deps {
                            if let Some(version_str) = version.as_str() {
                                versions.insert(name.clone(), version_str.to_string());
                            }
                        }
                    }
                    if let Some(dev_deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
                        for (name, version) in dev_deps {
                            if let Some(version_str) = version.as_str() {
                                versions.insert(format!("dev:{}", name), version_str.to_string());
                            }
                        }
                    }
                }
            }
            "Cargo.toml" => {
                // Simple TOML parsing for dependencies
                for line in content.lines() {
                    if line.contains("=") && !line.trim_start().starts_with('#') {
                        if let Some(deps_section) = line.split('=').next() {
                            let dep_name = deps_section.trim().trim_matches('"');
                            if let Some(version_part) = line.split('=').nth(1) {
                                let version = version_part.trim().trim_matches('"').trim_matches('\'')
                                    .split_whitespace().next().unwrap_or("").to_string();
                                if !version.is_empty() && !dep_name.is_empty() {
                                    versions.insert(dep_name.to_string(), version);
                                }
                            }
                        }
                    }
                }
            }
            "requirements.txt" => {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        if let Some((name, version)) = line.split_once("==") {
                            versions.insert(name.trim().to_string(), version.trim().to_string());
                        } else if let Some((name, version)) = line.split_once(">=") {
                            versions.insert(name.trim().to_string(), format!(">={}", version.trim()));
                        }
                    }
                }
            }
            _ => {
                debug!("Unsupported manifest file type: {}", file_type);
            }
        }
        
        Ok(versions)
    }
    
    /// Generate capability fingerprint from cached server data
    async fn generate_capability_fingerprint(&self, server_id: &str) -> Result<CapabilityFingerprint, FingerprintError> {
        let query = super::types::CacheQuery {
            server_id: server_id.to_string(),
            instance_type: super::types::InstanceType::Production,
            freshness_level: super::types::FreshnessLevel::Cached,
            include_disabled: true,
        };
        
        let server_data = match self.cache_manager.get_server_data(&query).await {
            Ok(result) => result.data,
            Err(_) => {
                // If no cached data, return empty fingerprint
                return Ok(CapabilityFingerprint {
                    tools_hash: String::new(),
                    resources_hash: String::new(),
                    prompts_hash: String::new(),
                    server_info_hash: String::new(),
                });
            }
        };
        
        if let Some(data) = server_data {
            let tools_json = serde_json::to_string(&data.tools)?;
            let resources_json = serde_json::to_string(&data.resources)?;
            let prompts_json = serde_json::to_string(&data.prompts)?;
            let server_info = format!("{}-{}-{}", data.server_name, data.server_version.unwrap_or_default(), data.protocol_version);
            
            Ok(CapabilityFingerprint {
                tools_hash: format!("{:x}", Sha256::digest(tools_json.as_bytes())),
                resources_hash: format!("{:x}", Sha256::digest(resources_json.as_bytes())),
                prompts_hash: format!("{:x}", Sha256::digest(prompts_json.as_bytes())),
                server_info_hash: format!("{:x}", Sha256::digest(server_info.as_bytes())),
            })
        } else {
            Ok(CapabilityFingerprint {
                tools_hash: String::new(),
                resources_hash: String::new(),
                prompts_hash: String::new(),
                server_info_hash: String::new(),
            })
        }
    }
    
    /// Generate configuration fingerprint from server config
    /// TODO: Will be used in Phase 2 for config change detection
    #[allow(dead_code)]
    async fn generate_config_fingerprint(&self, server_config: &serde_json::Value) -> Result<ConfigFingerprint, FingerprintError> {
        let config_json = serde_json::to_string(server_config)?;
        let server_config_hash = format!("{:x}", Sha256::digest(config_json.as_bytes()));
        
        // Extract environment and arguments if present
        let environment_hash = if let Some(env) = server_config.get("env") {
            let env_json = serde_json::to_string(env)?;
            format!("{:x}", Sha256::digest(env_json.as_bytes()))
        } else {
            String::new()
        };
        
        let arguments_hash = if let Some(args) = server_config.get("args") {
            let args_json = serde_json::to_string(args)?;
            format!("{:x}", Sha256::digest(args_json.as_bytes()))
        } else {
            String::new()
        };
        
        Ok(ConfigFingerprint {
            server_config_hash,
            environment_hash,
            arguments_hash,
        })
    }
    
    /// Generate combined hash from all fingerprints
    fn generate_combined_hash(&self, fingerprint: &MCPServerFingerprint) -> String {
        let combined = format!(
            "{}-{}-{}-{}",
            serde_json::to_string(&fingerprint.code_fingerprint).unwrap_or_default(),
            serde_json::to_string(&fingerprint.dependency_fingerprint).unwrap_or_default(),
            serde_json::to_string(&fingerprint.capability_fingerprint).unwrap_or_default(),
            serde_json::to_string(&fingerprint.config_fingerprint).unwrap_or_default(),
        );
        
        format!("{:x}", Sha256::digest(combined.as_bytes()))
    }
}

#[async_trait::async_trait]
impl FingerprintGenerator for DefaultFingerprintGenerator {
    async fn generate_fingerprint(&self, server_path: &Path) -> Result<MCPServerFingerprint, FingerprintError> {
        debug!("Generating fingerprint for server at: {:?}", server_path);
        
        let code_fingerprint = self.generate_code_fingerprint(server_path).await?;
        let dependency_fingerprint = self.generate_dependency_fingerprint(server_path).await?;
        
        // For capability and config fingerprints, we need server ID
        // This is a simplified implementation - in practice, you'd pass server_id and config
        let server_id = server_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let capability_fingerprint = self.generate_capability_fingerprint(&server_id).await?;
        
        // Default empty config fingerprint - would be populated with actual server config
        let config_fingerprint = ConfigFingerprint {
            server_config_hash: String::new(),
            environment_hash: String::new(),
            arguments_hash: String::new(),
        };
        
        let mut fingerprint = MCPServerFingerprint {
            code_fingerprint,
            dependency_fingerprint,
            capability_fingerprint,
            config_fingerprint,
            combined_hash: String::new(),
            generated_at: Utc::now(),
        };
        
        fingerprint.combined_hash = self.generate_combined_hash(&fingerprint);
        
        debug!("Generated fingerprint with hash: {}", fingerprint.combined_hash);
        Ok(fingerprint)
    }
    
    async fn compare_fingerprints(&self, old: &MCPServerFingerprint, new: &MCPServerFingerprint) -> FingerprintDiff {
        let mut change_types = Vec::new();
        
        // Check for code changes
        if old.code_fingerprint != new.code_fingerprint {
            let mut files_changed = Vec::new();
            
            // Find changed files
            for (path, new_hash) in &new.code_fingerprint.file_hashes {
                if let Some(old_hash) = old.code_fingerprint.file_hashes.get(path) {
                    if old_hash != new_hash {
                        files_changed.push(path.clone());
                    }
                } else {
                    // New file
                    files_changed.push(path.clone());
                }
            }
            
            // Find deleted files
            for path in old.code_fingerprint.file_hashes.keys() {
                if !new.code_fingerprint.file_hashes.contains_key(path) {
                    files_changed.push(path.clone());
                }
            }
            
            if !files_changed.is_empty() {
                change_types.push(ChangeType::CodeChange { files_changed });
            }
        }
        
        // Check for dependency changes
        if old.dependency_fingerprint != new.dependency_fingerprint {
            let mut packages_changed = Vec::new();
            
            // Find changed packages
            for (package, new_version) in &new.dependency_fingerprint.resolved_versions {
                if let Some(old_version) = old.dependency_fingerprint.resolved_versions.get(package) {
                    if old_version != new_version {
                        packages_changed.push(package.clone());
                    }
                } else {
                    packages_changed.push(package.clone());
                }
            }
            
            // Find removed packages
            for package in old.dependency_fingerprint.resolved_versions.keys() {
                if !new.dependency_fingerprint.resolved_versions.contains_key(package) {
                    packages_changed.push(package.clone());
                }
            }
            
            if !packages_changed.is_empty() {
                change_types.push(ChangeType::DependencyChange { packages_changed });
            }
        }
        
        // Check for capability changes
        if old.capability_fingerprint != new.capability_fingerprint {
            let capabilities_diff = CapabilityDiff {
                tools_changed: old.capability_fingerprint.tools_hash != new.capability_fingerprint.tools_hash,
                resources_changed: old.capability_fingerprint.resources_hash != new.capability_fingerprint.resources_hash,
                prompts_changed: old.capability_fingerprint.prompts_hash != new.capability_fingerprint.prompts_hash,
                server_info_changed: old.capability_fingerprint.server_info_hash != new.capability_fingerprint.server_info_hash,
            };
            
            change_types.push(ChangeType::CapabilityChange { capabilities_diff });
        }
        
        // Check for config changes
        if old.config_fingerprint != new.config_fingerprint {
            let config_diff = ConfigDiff {
                server_config_changed: old.config_fingerprint.server_config_hash != new.config_fingerprint.server_config_hash,
                environment_changed: old.config_fingerprint.environment_hash != new.config_fingerprint.environment_hash,
                arguments_changed: old.config_fingerprint.arguments_hash != new.config_fingerprint.arguments_hash,
            };
            
            change_types.push(ChangeType::ConfigChange { config_diff });
        }
        
        let has_changes = !change_types.is_empty();
        
        if change_types.is_empty() {
            change_types.push(ChangeType::NoChange);
        }
        
        FingerprintDiff {
            has_changes,
            change_types,
            old_fingerprint: old.clone(),
            new_fingerprint: new.clone(),
        }
    }
}

/// Change detector for monitoring server changes
pub struct ChangeDetector {
    fingerprint_generator: Box<dyn FingerprintGenerator + Send + Sync>,
    cache_manager: RedbCacheManager,
}

impl ChangeDetector {
    pub fn new(cache_manager: RedbCacheManager) -> Self {
        let fingerprint_generator = Box::new(DefaultFingerprintGenerator::new(cache_manager.clone()));
        
        Self {
            fingerprint_generator,
            cache_manager,
        }
    }
    
    /// Detect changes for a server
    pub async fn detect_changes(&self, server_id: &str, server_path: &Path) -> Result<FingerprintDiff, FingerprintError> {
        // Get stored fingerprint
        let old_fingerprint = self.get_stored_fingerprint(server_id).await?;
        
        // Generate new fingerprint
        let new_fingerprint = self.fingerprint_generator.generate_fingerprint(server_path).await?;
        
        // Compare fingerprints
        let diff = self.fingerprint_generator.compare_fingerprints(&old_fingerprint, &new_fingerprint).await;
        
        // Store new fingerprint if changes detected
        if diff.has_changes {
            self.store_fingerprint(server_id, &new_fingerprint).await?;
        }
        
        Ok(diff)
    }
    
    /// Get stored fingerprint for a server
    async fn get_stored_fingerprint(&self, _server_id: &str) -> Result<MCPServerFingerprint, FingerprintError> {
        // This would read from the FINGERPRINTS_TABLE in Redb
        // For now, return a default fingerprint
        Ok(MCPServerFingerprint {
            code_fingerprint: CodeFingerprint {
                file_hashes: HashMap::new(),
                total_files: 0,
                total_size: 0,
                last_modified: Utc::now(),
            },
            dependency_fingerprint: DependencyFingerprint {
                package_lock_hash: None,
                manifest_hash: String::new(),
                resolved_versions: HashMap::new(),
            },
            capability_fingerprint: CapabilityFingerprint {
                tools_hash: String::new(),
                resources_hash: String::new(),
                prompts_hash: String::new(),
                server_info_hash: String::new(),
            },
            config_fingerprint: ConfigFingerprint {
                server_config_hash: String::new(),
                environment_hash: String::new(),
                arguments_hash: String::new(),
            },
            combined_hash: String::new(),
            generated_at: Utc::now(),
        })
    }
    
    /// Store fingerprint for a server
    async fn store_fingerprint(&self, server_id: &str, _fingerprint: &MCPServerFingerprint) -> Result<(), FingerprintError> {
        // This would store in the FINGERPRINTS_TABLE in Redb
        debug!("Stored fingerprint for server: {}", server_id);
        Ok(())
    }
    
    /// Invalidate cache based on detected changes
    pub async fn invalidate_cache_if_changed(&self, server_id: &str, diff: &FingerprintDiff) -> Result<(), CacheError> {
        if diff.has_changes {
            debug!("Changes detected for server {}, invalidating cache", server_id);
            self.cache_manager.remove_server_data(server_id).await?;
        }
        
        Ok(())
    }
}