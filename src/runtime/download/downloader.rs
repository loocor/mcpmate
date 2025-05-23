//! Generic downloader for runtime files

use crate::runtime::{
    detection::Environment,
    types::{RuntimeError, RuntimeType},
};
use anyhow::Result;
use std::path::PathBuf;

/// Generic file downloader
#[derive(Debug)]
pub struct FileDownloader {
    environment: Environment,
}

impl FileDownloader {
    /// Create a new file downloader
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }

    /// Download file from URL
    pub async fn download_file(
        &self,
        url: &str,
        runtime_type: RuntimeType,
        version: &str,
        temp_dir: &PathBuf,
    ) -> Result<PathBuf> {
        std::fs::create_dir_all(temp_dir)?;

        // Extract file extension from URL
        let url_path = url.split('/').next_back().unwrap_or("download");
        let extension = if url_path.contains('.') {
            url_path.split('.').skip(1).collect::<Vec<_>>().join(".")
        } else {
            self.environment.os.archive_extension().to_string()
        };

        let filename = format!("{}-{}.{}", runtime_type.as_str(), version, extension);
        let temp_file = temp_dir.join(filename);

        // Download file using reqwest
        let response = reqwest::get(url)
            .await
            .map_err(|e| RuntimeError::DownloadFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuntimeError::DownloadFailed(format!(
                "HTTP {}: {}",
                response.status(),
                url
            ))
            .into());
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| RuntimeError::DownloadFailed(e.to_string()))?;

        std::fs::write(&temp_file, bytes)?;

        Ok(temp_file)
    }
}
