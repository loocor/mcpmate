//! Simplified file downloader for runtime installations

use crate::common::env::Environment;
use crate::runtime::types::{
    DownloadConfig, DownloadProgress, DownloadStage, RuntimeError, RuntimeType,
};
use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

/// Simplified file downloader with progress tracking
#[derive(Debug)]
pub struct FileDownloader {
    environment: Environment,
    config: DownloadConfig,
}

impl FileDownloader {
    /// Create a new file downloader with default configuration
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            config: DownloadConfig::default(),
        }
    }

    /// Create a new file downloader with custom configuration
    pub fn with_config(
        environment: Environment,
        config: DownloadConfig,
    ) -> Self {
        Self {
            environment,
            config,
        }
    }

    /// Get environment for other components
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Download file from URL with progress tracking
    pub async fn download_file(
        &self,
        url: &str,
        runtime_type: RuntimeType,
        version: &str,
        temp_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let start_time = Instant::now();

        // Report initialization
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Initializing,
            message: Some(format!(
                "Preparing to download {} v{}",
                runtime_type, version
            )),
        });

        if self.config.verbose {
            tracing::info!("Starting download: {} -> {}", url, temp_dir.display());
        }

        std::fs::create_dir_all(temp_dir)?;

        // Generate filename
        let url_path = url.split('/').next_back().unwrap_or("download");
        let extension = if url_path.contains('.') {
            url_path.split('.').skip(1).collect::<Vec<_>>().join(".")
        } else {
            self.environment.os.archive_extension().to_string()
        };

        let filename = format!("{}-{}.{}", runtime_type.as_str(), version, extension);
        let temp_file = temp_dir.join(filename);

        // Attempt download with retries
        let mut last_error = None;

        for attempt in 1..=self.config.max_retries {
            if self.config.verbose && attempt > 1 {
                tracing::info!(
                    "Download attempt {} of {}",
                    attempt,
                    self.config.max_retries
                );
            }

            match self
                .download_with_progress(url, &temp_file, start_time)
                .await
            {
                Ok(_) => {
                    self.report_progress(DownloadProgress {
                        downloaded: 0,
                        total: None,
                        speed: None,
                        stage: DownloadStage::Complete,
                        message: Some("Download completed successfully".to_string()),
                    });

                    if self.config.verbose {
                        tracing::info!("Download completed: {}", temp_file.display());
                    }

                    return Ok(temp_file);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        if self.config.verbose {
                            tracing::warn!("Download attempt {} failed, retrying...", attempt);
                        }
                        // Simple exponential backoff
                        let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // All attempts failed
        let base_error = last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error"));
        let error_msg = format!(
            "Download failed after {} attempts: {}",
            self.config.max_retries, base_error
        );

        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Failed(error_msg.clone()),
            message: None,
        });

        Err(RuntimeError::DownloadFailed(error_msg).into())
    }

    /// Perform the actual download with progress tracking
    async fn download_with_progress(
        &self,
        url: &str,
        temp_file: &PathBuf,
        start_time: Instant,
    ) -> Result<()> {
        // Create HTTP client with timeout
        let client = if let Some(timeout_secs) = self.config.timeout {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .map_err(|e| RuntimeError::DownloadFailed(e.to_string()))?
        } else {
            reqwest::Client::new()
        };

        // Report sending request
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::SendingRequest,
            message: Some("Sending HTTP request...".to_string()),
        });

        // Start download
        let response = client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                RuntimeError::DownloadTimeout {
                    seconds: self.config.timeout.unwrap_or(0),
                }
            } else {
                RuntimeError::DownloadFailed(e.to_string())
            }
        })?;

        if !response.status().is_success() {
            return Err(RuntimeError::DownloadFailed(format!(
                "HTTP {}: {}",
                response.status(),
                url
            ))
            .into());
        }

        // Get content length for progress tracking
        let total_size = response.content_length();

        if self.config.verbose {
            if let Some(size) = total_size {
                tracing::info!("Download size: {} bytes", size);
            } else {
                tracing::info!("Download size: unknown");
            }
        }

        // Report download start
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: total_size,
            speed: None,
            stage: DownloadStage::Downloading,
            message: Some("Starting download...".to_string()),
        });

        // Create file and download with progress tracking
        let mut file = tokio::fs::File::create(temp_file).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        let mut last_progress_time = Instant::now();
        let mut last_downloaded = 0u64;

        use tokio_stream::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| RuntimeError::DownloadFailed(e.to_string()))?;

            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // Report progress every 100KB or 1 second
            let now = Instant::now();
            if downloaded - last_downloaded >= 100_000
                || now.duration_since(last_progress_time) >= Duration::from_secs(1)
            {
                let elapsed = now.duration_since(start_time);
                let speed = if elapsed.as_secs() > 0 {
                    Some(downloaded / elapsed.as_secs())
                } else {
                    None
                };

                self.report_progress(DownloadProgress {
                    downloaded,
                    total: total_size,
                    speed,
                    stage: DownloadStage::Downloading,
                    message: self.format_progress_message(downloaded, total_size, speed),
                });

                last_progress_time = now;
                last_downloaded = downloaded;
            }

            // Simple timeout check
            if let Some(timeout_secs) = self.config.timeout {
                if start_time.elapsed() > Duration::from_secs(timeout_secs) {
                    return Err(RuntimeError::DownloadTimeout {
                        seconds: timeout_secs,
                    }
                    .into());
                }
            }
        }

        file.flush().await?;
        Ok(())
    }

    /// Report progress to callback if configured
    fn report_progress(
        &self,
        progress: DownloadProgress,
    ) {
        if let Some(ref callback) = self.config.progress_callback {
            callback(progress);
        }
    }

    /// Public method to report progress (for use by RuntimeDownloader)
    pub fn report_progress_external(
        &self,
        progress: DownloadProgress,
    ) {
        self.report_progress(progress);
    }

    /// Format progress message
    fn format_progress_message(
        &self,
        downloaded: u64,
        total: Option<u64>,
        speed: Option<u64>,
    ) -> Option<String> {
        let downloaded_mb = downloaded as f64 / 1_048_576.0;

        let mut parts = vec![format!("{:.1} MB downloaded", downloaded_mb)];

        if let Some(total) = total {
            let total_mb = total as f64 / 1_048_576.0;
            let percentage = (downloaded as f64 / total as f64) * 100.0;
            parts.push(format!("of {:.1} MB ({:.1}%)", total_mb, percentage));
        }

        if let Some(speed) = speed {
            let speed_mb = speed as f64 / 1_048_576.0;
            parts.push(format!("at {:.1} MB/s", speed_mb));
        }

        Some(parts.join(" "))
    }
}
