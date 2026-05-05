//! Simplified Runtime Downloader
//!
//! Provides basic download functionality for runtime binaries.
//! Replaces the complex download/ directory with a simple, focused implementation.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::common::RuntimeType;
use crate::common::env::{Architecture, OperatingSystem, detect_environment};

const NODE_DIST_INDEX_URL: &str = "https://nodejs.org/dist/index.json";

#[derive(Debug, Clone)]
struct ResolvedRuntimeDownload {
    file_name: String,
    resolved_version: String,
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct NodeDistIndexEntry {
    files: Vec<String>,
    lts: serde_json::Value,
    version: String,
}

impl NodeDistIndexEntry {
    fn is_lts(&self) -> bool {
        !matches!(self.lts, serde_json::Value::Bool(false))
    }

    fn parsed_version(&self) -> Option<semver::Version> {
        semver::Version::parse(self.version.trim_start_matches('v')).ok()
    }
}

/// Simple runtime downloader
pub struct RuntimeDownloader {
    client: Client,
}

impl RuntimeDownloader {
    /// Create a new downloader
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    /// Download a runtime to the specified directory
    pub async fn download_runtime(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
        target_dir: &PathBuf,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(target_dir)
            .await
            .context("Failed to create target directory")?;

        let resolved = self.resolve_download(runtime_type, version).await?;

        tracing::info!(
            "Downloading {} {} from {}",
            runtime_type.as_str(),
            resolved.resolved_version,
            resolved.url
        );

        let response = self
            .client
            .get(&resolved.url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
        }

        let file_path = target_dir.join(&resolved.file_name);
        let mut file = File::create(&file_path)
            .await
            .context("Failed to create download file")?;

        let content = response.bytes().await.context("Failed to read download content")?;

        file.write_all(&content)
            .await
            .context("Failed to write download file")?;

        tracing::info!(
            "Downloaded {} {} to {}",
            runtime_type.as_str(),
            resolved.resolved_version,
            file_path.display()
        );
        Ok(file_path)
    }

    async fn resolve_download(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<ResolvedRuntimeDownload> {
        let env = detect_environment()?;

        match runtime_type {
            RuntimeType::Bun => self.resolve_bun_download(&env),
            RuntimeType::Uv => self.resolve_uv_download(&env),
            RuntimeType::Node => self.resolve_node_download(&env, version).await,
        }
    }

    fn resolve_bun_download(
        &self,
        env: &crate::common::env::Environment,
    ) -> Result<ResolvedRuntimeDownload> {
        let platform = match env.os {
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
        };

        let arch = match env.arch {
            Architecture::X86_64 => "x64",
            Architecture::Aarch64 => "aarch64",
        };

        let file_name = format!("bun-{}-{}.zip", platform, arch);
        Ok(ResolvedRuntimeDownload {
            url: format!("https://github.com/oven-sh/bun/releases/latest/download/{file_name}"),
            file_name,
            resolved_version: RuntimeType::Bun.default_version().to_string(),
        })
    }

    fn resolve_uv_download(
        &self,
        env: &crate::common::env::Environment,
    ) -> Result<ResolvedRuntimeDownload> {
        let platform = match env.os {
            OperatingSystem::MacOS => "apple-darwin",
            OperatingSystem::Linux => "unknown-linux-gnu",
            OperatingSystem::Windows => "pc-windows-msvc",
        };

        let arch = match env.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        };

        let ext = match env.os {
            OperatingSystem::Windows => "zip",
            _ => "tar.gz",
        };

        let file_name = format!("uv-{}-{}.{}", arch, platform, ext);
        Ok(ResolvedRuntimeDownload {
            url: format!("https://github.com/astral-sh/uv/releases/latest/download/{file_name}"),
            file_name,
            resolved_version: RuntimeType::Uv.default_version().to_string(),
        })
    }

    async fn resolve_node_download(
        &self,
        env: &crate::common::env::Environment,
        version: Option<&str>,
    ) -> Result<ResolvedRuntimeDownload> {
        let requested = version
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(RuntimeType::Node.default_version());

        let index = self.fetch_node_dist_index().await?;
        let entry = self.resolve_node_index_entry(&index, requested)?;
        let version_tag = entry.version.clone();
        let archive_platform = Self::node_archive_platform(env);
        let index_platform = Self::node_index_platform(env);
        let arch = env.arch.node_arch();
        let ext = match env.os {
            OperatingSystem::Windows => "zip",
            _ => "tar.gz",
        };
        let file_name = format!("node-{}-{}-{}.{}", version_tag, archive_platform, arch, ext);
        let file_key = match env.os {
            OperatingSystem::Windows => format!("{}-{}-zip", index_platform, arch),
            OperatingSystem::MacOS => format!("{}-{}-tar", index_platform, arch),
            OperatingSystem::Linux => format!("{}-{}", index_platform, arch),
        };

        if !entry.files.iter().any(|value| value == &file_key) {
            return Err(anyhow::anyhow!(
                "Node.js {} does not publish {} for this platform",
                version_tag,
                file_key
            ));
        }

        Ok(ResolvedRuntimeDownload {
            url: format!("https://nodejs.org/dist/{version_tag}/{file_name}"),
            file_name,
            resolved_version: version_tag,
        })
    }

    async fn fetch_node_dist_index(&self) -> Result<Vec<NodeDistIndexEntry>> {
        let response = self
            .client
            .get(NODE_DIST_INDEX_URL)
            .send()
            .await
            .context("Failed to fetch Node.js dist index")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Node.js dist index request failed with status: {}",
                response.status()
            ));
        }

        response
            .json::<Vec<NodeDistIndexEntry>>()
            .await
            .context("Failed to decode Node.js dist index")
    }

    fn resolve_node_index_entry<'a>(
        &self,
        index: &'a [NodeDistIndexEntry],
        requested: &str,
    ) -> Result<&'a NodeDistIndexEntry> {
        let normalized = requested.trim().trim_start_matches('v').to_ascii_lowercase();

        if normalized == "latest" {
            return index
                .first()
                .ok_or_else(|| anyhow::anyhow!("Node.js dist index is empty"));
        }

        if normalized == "lts" {
            return index
                .iter()
                .find(|entry| entry.is_lts())
                .ok_or_else(|| anyhow::anyhow!("No active LTS release found in Node.js dist index"));
        }

        let requested_parts = normalized.split('.').collect::<Vec<_>>();

        if requested_parts.len() == 1 && requested_parts[0].chars().all(|ch| ch.is_ascii_digit()) {
            let requested_major = requested_parts[0]
                .parse::<u64>()
                .context("Invalid Node.js major version")?;

            return index
                .iter()
                .find(|entry| {
                    entry
                        .parsed_version()
                        .is_some_and(|version| version.major == requested_major)
                })
                .ok_or_else(|| anyhow::anyhow!("Node.js major version {} not found", requested_major));
        }

        if let Ok(requested_version) = semver::Version::parse(&normalized) {
            return index
                .iter()
                .find(|entry| entry.parsed_version().as_ref() == Some(&requested_version))
                .ok_or_else(|| anyhow::anyhow!("Node.js version v{} not found", requested_version));
        }

        if requested_parts.len() >= 2
            && requested_parts
                .iter()
                .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
        {
            return index
                .iter()
                .find(|entry| {
                    let Some(version) = entry.parsed_version() else {
                        return false;
                    };

                    version.major.to_string() == requested_parts[0]
                        && version.minor.to_string() == requested_parts[1]
                        && requested_parts
                            .get(2)
                            .is_none_or(|patch| version.patch.to_string() == *patch)
                })
                .ok_or_else(|| anyhow::anyhow!("Node.js version prefix {} not found", requested));
        }

        Err(anyhow::anyhow!(
            "Unsupported Node.js version spec '{}'. Use lts, latest, a major version, or a full semver.",
            requested
        ))
    }

    fn node_archive_platform(env: &crate::common::env::Environment) -> &'static str {
        match env.os {
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "win",
        }
    }

    fn node_index_platform(env: &crate::common::env::Environment) -> &'static str {
        match env.os {
            OperatingSystem::MacOS => "osx",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "win",
        }
    }
}

impl Default for RuntimeDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::env::{Architecture, Environment, OperatingSystem};
    use serde_json::json;

    fn entry(
        version: &str,
        lts: serde_json::Value,
    ) -> NodeDistIndexEntry {
        NodeDistIndexEntry {
            version: version.to_string(),
            lts,
            files: vec![
                "linux-x64".to_string(),
                "linux-arm64".to_string(),
                "osx-arm64-tar".to_string(),
                "osx-x64-tar".to_string(),
                "win-x64-zip".to_string(),
            ],
        }
    }

    #[test]
    fn resolves_node_lts_and_latest() {
        let downloader = RuntimeDownloader::new();
        let index = vec![
            entry("v25.1.0", json!(false)),
            entry("v24.15.0", json!("Krypton")),
            entry("v22.20.0", json!("Jod")),
        ];

        assert_eq!(
            downloader.resolve_node_index_entry(&index, "latest").unwrap().version,
            "v25.1.0"
        );
        assert_eq!(
            downloader.resolve_node_index_entry(&index, "lts").unwrap().version,
            "v24.15.0"
        );
    }

    #[test]
    fn resolves_node_major_and_exact_versions() {
        let downloader = RuntimeDownloader::new();
        let index = vec![
            entry("v25.1.0", json!(false)),
            entry("v24.15.0", json!("Krypton")),
            entry("v24.14.3", json!("Krypton")),
        ];

        assert_eq!(
            downloader.resolve_node_index_entry(&index, "24").unwrap().version,
            "v24.15.0"
        );
        assert_eq!(
            downloader.resolve_node_index_entry(&index, "24.14").unwrap().version,
            "v24.14.3"
        );
        assert_eq!(
            downloader.resolve_node_index_entry(&index, "v24.15.0").unwrap().version,
            "v24.15.0"
        );
    }

    #[test]
    fn node_platform_uses_official_dist_labels() {
        let mac = Environment {
            os: OperatingSystem::MacOS,
            arch: Architecture::Aarch64,
        };
        let linux = Environment {
            os: OperatingSystem::Linux,
            arch: Architecture::X86_64,
        };
        let windows = Environment {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
        };

        assert_eq!(RuntimeDownloader::node_archive_platform(&mac), "darwin");
        assert_eq!(RuntimeDownloader::node_index_platform(&mac), "osx");
        assert_eq!(RuntimeDownloader::node_archive_platform(&linux), "linux");
        assert_eq!(RuntimeDownloader::node_index_platform(&linux), "linux");
        assert_eq!(RuntimeDownloader::node_archive_platform(&windows), "win");
        assert_eq!(RuntimeDownloader::node_index_platform(&windows), "win");
        assert_eq!(mac.arch.node_arch(), "arm64");
    }
}
