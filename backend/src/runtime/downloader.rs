//! Simplified Runtime Downloader
//!
//! Provides basic download functionality for runtime binaries.
//! Replaces the complex download/ directory with a simple, focused implementation.

use anyhow::{Context, Result};
use percent_encoding::percent_decode_str;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::common::RuntimeType;
use crate::common::env::{Architecture, OperatingSystem, detect_environment};

const RUNTIME_DISTRIBUTION_ORIGIN: &str = "https://public.mcp.umate.ai";
const DOWNLOAD_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const RUNTIME_DISTRIBUTION_DECODE_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_CONTENT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
struct ResolvedRuntimeDownload {
    file_name: String,
    resolved_version: String,
    url: String,
    sha256: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeDistributionResolution {
    runtime: String,
    version: String,
    target: String,
    download_url: String,
    sha256: String,
    size: u64,
}

/// Simple runtime downloader
pub struct RuntimeDownloader {
    client: Client,
    distribution_origin: Url,
}

impl RuntimeDownloader {
    /// Create a new downloader
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            distribution_origin: Url::parse(RUNTIME_DISTRIBUTION_ORIGIN)
                .expect("the runtime distribution origin must be a valid URL"),
        }
    }

    #[cfg(test)]
    fn with_distribution_origin(
        client: Client,
        origin: &str,
    ) -> Result<Self> {
        let distribution_origin = Url::parse(origin).context("Invalid runtime distribution origin")?;
        Ok(Self {
            client,
            distribution_origin,
        })
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

        let response = self.client.get(&resolved.url).send();

        let response = tokio::time::timeout(DOWNLOAD_REQUEST_TIMEOUT, response)
            .await
            .context("Download request timed out")?
            .context("Failed to start download")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
        }

        let content = tokio::time::timeout(DOWNLOAD_CONTENT_TIMEOUT, response.bytes())
            .await
            .context("Download content timed out")?
            .context("Failed to read download content")?;

        Self::verify_download(&resolved, &content)?;

        let file_path = target_dir.join(&resolved.file_name);
        let mut file = File::create(&file_path)
            .await
            .context("Failed to create download file")?;

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
        self.resolve_download_for_environment(runtime_type, version, &env).await
    }

    async fn resolve_download_for_environment(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
        env: &crate::common::env::Environment,
    ) -> Result<ResolvedRuntimeDownload> {
        let requested_version = Self::distribution_version(runtime_type, version)?;
        let platform = Self::distribution_platform(env);
        let arch = Self::distribution_arch(env);
        let mut url = self
            .distribution_origin
            .join("runtimes/v1/resolve")
            .context("Failed to construct runtime distribution resolve URL")?;
        url.query_pairs_mut()
            .append_pair("runtime", runtime_type.as_str())
            .append_pair("version", &requested_version)
            .append_pair("os", platform)
            .append_pair("arch", arch);

        let response = self.client.get(url).send();
        let response = tokio::time::timeout(DOWNLOAD_REQUEST_TIMEOUT, response)
            .await
            .context("Runtime distribution resolve request timed out")?
            .context("Failed to resolve runtime distribution")?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Runtime distribution resolve request failed with status: {}",
                response.status()
            ));
        }
        let resolution = tokio::time::timeout(
            RUNTIME_DISTRIBUTION_DECODE_TIMEOUT,
            response.json::<RuntimeDistributionResolution>(),
        )
        .await
        .context("Runtime distribution response decode timed out")?
        .context("Failed to decode runtime distribution response")?;
        Self::validate_resolution(runtime_type, platform, arch, &requested_version, resolution)
    }

    fn distribution_platform(env: &crate::common::env::Environment) -> &'static str {
        match env.os {
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
        }
    }

    fn distribution_arch(env: &crate::common::env::Environment) -> &'static str {
        match env.arch {
            Architecture::X86_64 => "x64",
            Architecture::Aarch64 => "arm64",
        }
    }

    fn distribution_version(
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<String> {
        let requested = version.map(str::trim).filter(|value| !value.is_empty());
        match requested {
            None => Ok("default".to_string()),
            Some("default") | Some("latest") | Some("lts") => Ok("default".to_string()),
            Some(value) => {
                let normalized = value.trim_start_matches('v');
                if semver::Version::parse(normalized).is_err() {
                    return Err(anyhow::anyhow!(
                        "Runtime distribution requires an exact semver or default; unsupported {} version '{}'.",
                        runtime_type.as_str(),
                        value
                    ));
                }
                Ok(normalized.to_string())
            }
        }
    }

    fn validate_resolution(
        runtime_type: RuntimeType,
        platform: &str,
        arch: &str,
        requested_version: &str,
        resolution: RuntimeDistributionResolution,
    ) -> Result<ResolvedRuntimeDownload> {
        if resolution.runtime != runtime_type.as_str() {
            return Err(anyhow::anyhow!("Runtime distribution returned a mismatched runtime."));
        }
        if resolution.target != format!("{platform}-{arch}") {
            return Err(anyhow::anyhow!("Runtime distribution returned a mismatched target."));
        }
        let resolved_version = semver::Version::parse(&resolution.version)
            .map_err(|_| anyhow::anyhow!("Runtime distribution returned an invalid exact version."))?;
        if requested_version != "default"
            && resolved_version != semver::Version::parse(requested_version).expect("validated exact version")
        {
            return Err(anyhow::anyhow!(
                "Runtime distribution returned a mismatched exact version."
            ));
        }
        if resolution.sha256.len() != 64 || !resolution.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(anyhow::anyhow!(
                "Runtime distribution returned an invalid SHA-256 digest."
            ));
        }
        let download_url =
            Url::parse(&resolution.download_url).context("Runtime distribution returned an invalid download URL")?;
        if download_url.scheme() != "https" || !download_url.username().is_empty() || download_url.password().is_some()
        {
            return Err(anyhow::anyhow!("Runtime distribution returned an unsafe download URL."));
        }
        let encoded_file_name = download_url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|file_name| !file_name.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Runtime distribution download URL is missing a file name."))?;
        let file_name = percent_decode_str(encoded_file_name)
            .decode_utf8()
            .context("Runtime distribution download URL has an invalid file name encoding")?;
        if file_name == "."
            || file_name == ".."
            || file_name.chars().any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
        {
            return Err(anyhow::anyhow!(
                "Runtime distribution returned an unsafe download file name."
            ));
        }
        Ok(ResolvedRuntimeDownload {
            file_name: file_name.to_string(),
            resolved_version: resolution.version,
            url: download_url.into(),
            sha256: resolution.sha256.to_ascii_lowercase(),
            size: resolution.size,
        })
    }

    fn verify_download(
        resolved: &ResolvedRuntimeDownload,
        content: &[u8],
    ) -> Result<()> {
        if content.len() as u64 != resolved.size {
            return Err(anyhow::anyhow!(
                "Runtime download size mismatch: expected {}, received {}.",
                resolved.size,
                content.len()
            ));
        }
        let actual_sha256 = format!("{:x}", Sha256::digest(content));
        if actual_sha256 != resolved.sha256 {
            return Err(anyhow::anyhow!("Runtime download SHA-256 mismatch."));
        }
        Ok(())
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
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn maps_legacy_default_aliases_and_exact_versions_to_the_distribution_contract() {
        assert_eq!(
            RuntimeDownloader::distribution_version(RuntimeType::Bun, None).unwrap(),
            "default"
        );
        assert_eq!(
            RuntimeDownloader::distribution_version(RuntimeType::Uv, Some("latest")).unwrap(),
            "default"
        );
        assert_eq!(
            RuntimeDownloader::distribution_version(RuntimeType::Node, Some("lts")).unwrap(),
            "default"
        );
        assert_eq!(
            RuntimeDownloader::distribution_version(RuntimeType::Node, Some("v24.19.0")).unwrap(),
            "24.19.0"
        );
        assert!(RuntimeDownloader::distribution_version(RuntimeType::Node, Some("24")).is_err());
    }

    fn distribution_resolution(
        version: &str,
        download_url: &str,
    ) -> RuntimeDistributionResolution {
        RuntimeDistributionResolution {
            runtime: "bun".to_string(),
            version: version.to_string(),
            target: "darwin-arm64".to_string(),
            download_url: download_url.to_string(),
            sha256: "d8b96221828ad6f97ac7ac0ab7e95872341af763001e8803e8267652c2652620".to_string(),
            size: 23_586_433,
        }
    }

    #[test]
    fn rejects_mismatched_exact_versions_and_unsafe_download_file_names() {
        assert!(
            RuntimeDownloader::validate_resolution(
                RuntimeType::Bun,
                "darwin",
                "arm64",
                "1.3.14",
                distribution_resolution(
                    "1.3.15",
                    "https://downloads.example.test/runtimes/bun/1.3.15/darwin-arm64.zip",
                ),
            )
            .is_err()
        );
        assert!(
            RuntimeDownloader::validate_resolution(
                RuntimeType::Bun,
                "darwin",
                "arm64",
                "default",
                distribution_resolution(
                    "1.3.14",
                    "https://downloads.example.test/runtimes/bun/1.3.14/runtime%2Farchive.zip",
                ),
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_downloads_that_do_not_match_the_resolved_integrity_metadata() {
        let resolved = ResolvedRuntimeDownload {
            file_name: "darwin-arm64.zip".to_string(),
            resolved_version: "1.3.14".to_string(),
            url: "https://downloads.example.test/runtimes/bun/1.3.14/darwin-arm64.zip".to_string(),
            sha256: "d8b96221828ad6f97ac7ac0ab7e95872341af763001e8803e8267652c2652620".to_string(),
            size: 4,
        };

        assert!(RuntimeDownloader::verify_download(&resolved, b"abc").is_err());
        assert!(RuntimeDownloader::verify_download(&resolved, b"abcd").is_err());
    }

    #[tokio::test]
    async fn resolves_runtime_downloads_through_the_admin_distribution_contract() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/runtimes/v1/resolve"))
            .and(query_param("runtime", "bun"))
            .and(query_param("version", "default"))
            .and(query_param("os", "darwin"))
            .and(query_param("arch", "arm64"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "runtime": "bun",
                "version": "1.3.14",
                "target": "darwin-arm64",
                "artifactId": "asset_bun_1_3_14_darwin_arm64_zip",
                "downloadUrl": "https://downloads.example.test/runtimes/bun/1.3.14/darwin-arm64.zip",
                "sha256": "d8b96221828ad6f97ac7ac0ab7e95872341af763001e8803e8267652c2652620",
                "size": 23586433
            })))
            .mount(&server)
            .await;

        let downloader = RuntimeDownloader::with_distribution_origin(Client::new(), &server.uri()).unwrap();
        let resolved = downloader
            .resolve_download_for_environment(
                RuntimeType::Bun,
                None,
                &Environment {
                    os: OperatingSystem::MacOS,
                    arch: Architecture::Aarch64,
                },
            )
            .await
            .unwrap();

        assert_eq!(resolved.resolved_version, "1.3.14");
        assert_eq!(resolved.file_name, "darwin-arm64.zip");
        assert_eq!(
            resolved.url,
            "https://downloads.example.test/runtimes/bun/1.3.14/darwin-arm64.zip"
        );
        assert_eq!(
            resolved.sha256,
            "d8b96221828ad6f97ac7ac0ab7e95872341af763001e8803e8267652c2652620"
        );
        assert_eq!(resolved.size, 23586433);
    }
}
