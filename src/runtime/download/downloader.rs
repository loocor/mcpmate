//! Generic downloader for runtime files

use crate::runtime::{
    detection::Environment,
    types::{DownloadConfig, DownloadProgress, DownloadStage, RuntimeError, RuntimeType},
};
use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

/// Generic file downloader with progress tracking and timeout support
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

    /// Download file from URL with progress tracking and timeout
    pub async fn download_file(
        &self,
        url: &str,
        runtime_type: RuntimeType,
        version: &str,
        temp_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let start_time = Instant::now();

        // Report initialization stage
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

        // Pre-download network diagnostics
        if let Err(e) = self.run_network_diagnostics(url).await {
            let error_msg = format!("Network diagnostics failed: {}", e);
            self.report_progress(DownloadProgress {
                downloaded: 0,
                total: None,
                speed: None,
                stage: DownloadStage::Failed(error_msg.clone()),
                message: None,
            });
            return Err(RuntimeError::DownloadFailed(error_msg).into());
        }

        // Extract file extension from URL
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
                        // Wait before retry (exponential backoff)
                        let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // All attempts failed - provide diagnostic information
        let base_error = last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error"));
        let error_msg = format!(
            "Download failed after {} attempts: {}",
            self.config.max_retries, base_error
        );

        // Add diagnostic suggestions
        let suggestions = super::diagnostics::get_diagnostic_suggestions(&base_error.to_string());
        let detailed_error = if !suggestions.is_empty() {
            format!(
                "{}\n\nTroubleshooting suggestions:\n{}",
                error_msg,
                suggestions
                    .iter()
                    .map(|s| format!("- {}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            error_msg.clone()
        };

        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Failed(detailed_error.clone()),
            message: None,
        });

        Err(RuntimeError::DownloadFailed(detailed_error).into())
    }

    /// Run network diagnostics before download
    async fn run_network_diagnostics(
        &self,
        url: &str,
    ) -> Result<()> {
        use super::diagnostics::NetworkDiagnosticsRunner;

        let verbose = self.config.verbose;

        // For network diagnostics, we'll use a simpler approach without progress callback
        // to avoid lifetime issues. The diagnostics are fast anyway.
        let diagnostics_runner = NetworkDiagnosticsRunner::new(None, verbose);
        let diagnostics = diagnostics_runner.diagnose_url(url).await?;

        if let Some(ref error) = diagnostics.error {
            if verbose {
                let report = diagnostics_runner.generate_report(&diagnostics);
                tracing::error!("Network diagnostics report:\n{}", report);
            }
            return Err(anyhow::anyhow!(error.clone()));
        }

        if verbose {
            tracing::info!(
                "Network diagnostics passed - DNS: {:?}, Connection: {:?}",
                diagnostics.dns_resolution_time,
                diagnostics.connection_time
            );
        }

        Ok(())
    }

    /// Perform the actual download with progress tracking and intelligent timeout diagnostics
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

        // Report sending request stage
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::SendingRequest,
            message: Some("Sending HTTP request...".to_string()),
        });

        // Start download with intelligent timeout handling
        let request_start = Instant::now();

        // Use a shorter timeout for initial response (15 seconds)
        let initial_timeout = Duration::from_secs(15);
        let response_result = tokio::time::timeout(initial_timeout, client.get(url).send()).await;

        let response = match response_result {
            Ok(Ok(response)) => {
                // Report successful response
                self.report_progress(DownloadProgress {
                    downloaded: 0,
                    total: None,
                    speed: None,
                    stage: DownloadStage::WaitingResponse,
                    message: Some(format!(
                        "Received response in {:?}",
                        request_start.elapsed()
                    )),
                });
                response
            }
            Ok(Err(e)) => {
                // HTTP error
                if e.is_timeout() {
                    return Err(RuntimeError::DownloadTimeout {
                        seconds: self.config.timeout.unwrap_or(0),
                    }
                    .into());
                } else {
                    return Err(RuntimeError::DownloadFailed(e.to_string()).into());
                }
            }
            Err(_) => {
                // 15-second timeout reached - trigger intelligent diagnostics
                return self
                    .handle_timeout_with_diagnostics(url, initial_timeout.as_secs())
                    .await;
            }
        };

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

            // Check for timeout
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

    /// Handle timeout with intelligent network diagnostics
    async fn handle_timeout_with_diagnostics(
        &self,
        url: &str,
        timeout_secs: u64,
    ) -> Result<()> {
        // Report timeout and start diagnostics
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Failed(format!(
                "Request timeout after {}s - Running network diagnostics...",
                timeout_secs
            )),
            message: Some("Analyzing network connectivity...".to_string()),
        });

        if self.config.verbose {
            tracing::warn!(
                "Request timeout after {}s, running network diagnostics for {}",
                timeout_secs,
                url
            );
        }

        // Run comprehensive network diagnostics without progress callback to avoid lifetime issues
        use super::diagnostics::NetworkDiagnosticsRunner;

        let diagnostics_runner = NetworkDiagnosticsRunner::new(None, self.config.verbose);
        let diagnostics = diagnostics_runner.diagnose_url(url).await?;

        // Generate diagnostic report
        let report = diagnostics_runner.generate_report(&diagnostics);

        if self.config.verbose {
            tracing::error!("Network diagnostics completed:\n{}", report);
        }

        // Handle interactive mode if enabled
        if self.config.interactive {
            use super::interactive::InteractiveHandler;

            let interactive_handler = InteractiveHandler::new(true);
            let action = interactive_handler
                .handle_timeout(url, timeout_secs, &report)
                .await?;

            match action {
                super::interactive::TimeoutAction::Continue => {
                    // User chose to continue - this would require restructuring the download logic
                    // For now, we'll still return an error but with guidance
                    let error_message = format!(
                        "Download timeout after {}s. You chose to continue, but this requires restarting the download with extended timeout.\n\n{}\n\nPlease try again with a longer timeout using --timeout <seconds>",
                        timeout_secs, report
                    );
                    self.report_progress(DownloadProgress {
                        downloaded: 0,
                        total: None,
                        speed: None,
                        stage: DownloadStage::Failed(error_message.clone()),
                        message: Some(
                            "User chose to continue - please restart with longer timeout"
                                .to_string(),
                        ),
                    });
                    return Err(RuntimeError::DownloadTimeout {
                        seconds: timeout_secs,
                    }
                    .into());
                }
                super::interactive::TimeoutAction::Retry => {
                    // User chose to retry - this would require restructuring the download logic
                    // For now, we'll still return an error but with guidance
                    let error_message = format!(
                        "Download timeout after {}s. You chose to retry.\n\n{}\n\nPlease run the command again to retry the download",
                        timeout_secs, report
                    );
                    self.report_progress(DownloadProgress {
                        downloaded: 0,
                        total: None,
                        speed: None,
                        stage: DownloadStage::Failed(error_message.clone()),
                        message: Some(
                            "User chose to retry - please restart the command".to_string(),
                        ),
                    });
                    return Err(RuntimeError::DownloadCancelled.into());
                }
                super::interactive::TimeoutAction::Cancel => {
                    // User chose to cancel
                    let error_message =
                        format!("Download cancelled by user after timeout.\n\n{}", report);
                    self.report_progress(DownloadProgress {
                        downloaded: 0,
                        total: None,
                        speed: None,
                        stage: DownloadStage::Failed(error_message.clone()),
                        message: Some("Download cancelled by user".to_string()),
                    });
                    return Err(RuntimeError::DownloadCancelled.into());
                }
            }
        } else {
            // Non-interactive mode - show diagnostic message and exit
            use super::interactive::InteractiveHandler;

            let interactive_handler = InteractiveHandler::new(false);
            interactive_handler.show_timeout_message(url, timeout_secs, &report);
        }

        // Create detailed error message with diagnostics and user guidance
        let error_message = if let Some(ref error) = diagnostics.error {
            let suggestions = super::diagnostics::get_diagnostic_suggestions(error);
            format!(
                "Download timeout after {}s. Network diagnostics found issues:\n\n{}\n\nTroubleshooting suggestions:\n{}\n\nPlease check your network connection and try again.",
                timeout_secs,
                report,
                suggestions
                    .iter()
                    .map(|s| format!("- {}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            format!(
                "Download timeout after {}s. Network diagnostics passed - the server may be slow or overloaded.\n\n{}\n\nSuggestions:\n- Try again later when the server may be less busy\n- Increase timeout with --timeout option\n- Check if the server is experiencing high load",
                timeout_secs, report
            )
        };

        // Report final diagnostic result
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Failed(error_message.clone()),
            message: Some("Network diagnostics completed - user intervention required".to_string()),
        });

        Err(RuntimeError::DownloadTimeout {
            seconds: timeout_secs,
        }
        .into())
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
