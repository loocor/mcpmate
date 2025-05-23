//! Network diagnostics for download operations

use anyhow::Result;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use url::Url;

use crate::runtime::types::{DownloadProgress, DownloadStage};

/// Network diagnostic information
#[derive(Debug, Clone)]
pub struct NetworkDiagnostics {
    pub url: String,
    pub host: String,
    pub port: u16,
    pub resolved_ips: Vec<IpAddr>,
    pub dns_resolution_time: Option<Duration>,
    pub connection_time: Option<Duration>,
    pub total_time: Option<Duration>,
    pub error: Option<String>,
}

/// Network diagnostics runner
pub struct NetworkDiagnosticsRunner {
    progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    verbose: bool,
}

impl NetworkDiagnosticsRunner {
    pub fn new(
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
        verbose: bool,
    ) -> Self {
        Self {
            progress_callback,
            verbose,
        }
    }

    /// Run comprehensive network diagnostics for a URL
    pub async fn diagnose_url(
        &self,
        url: &str,
    ) -> Result<NetworkDiagnostics> {
        let start_time = Instant::now();
        let parsed_url = Url::parse(url)?;

        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host found"))?;

        let port = parsed_url
            .port_or_known_default()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no port found"))?;

        if self.verbose {
            tracing::info!("Starting network diagnostics for {}:{}", host, port);
        }

        let mut diagnostics = NetworkDiagnostics {
            url: url.to_string(),
            host: host.to_string(),
            port,
            resolved_ips: Vec::new(),
            dns_resolution_time: None,
            connection_time: None,
            total_time: None,
            error: None,
        };

        // Step 1: DNS Resolution
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::ResolvingDns,
            message: Some(format!("Resolving DNS for {}", host)),
        });

        let dns_start = Instant::now();
        match self.resolve_dns(host, port).await {
            Ok(ips) => {
                diagnostics.dns_resolution_time = Some(dns_start.elapsed());
                diagnostics.resolved_ips = ips;

                if self.verbose {
                    tracing::info!(
                        "DNS resolved in {:?}: {:?}",
                        diagnostics.dns_resolution_time.unwrap(),
                        diagnostics.resolved_ips
                    );
                }
            }
            Err(e) => {
                diagnostics.error = Some(format!("DNS resolution failed: {}", e));
                diagnostics.total_time = Some(start_time.elapsed());
                return Ok(diagnostics);
            }
        }

        // Step 2: Connection Test
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Connecting,
            message: Some(format!("Testing connection to {}:{}", host, port)),
        });

        let connection_start = Instant::now();
        match self.test_connection(&diagnostics.resolved_ips, port).await {
            Ok(_) => {
                diagnostics.connection_time = Some(connection_start.elapsed());

                if self.verbose {
                    tracing::info!(
                        "Connection established in {:?}",
                        diagnostics.connection_time.unwrap()
                    );
                }
            }
            Err(e) => {
                diagnostics.error = Some(format!("Connection failed: {}", e));
                diagnostics.total_time = Some(start_time.elapsed());
                return Ok(diagnostics);
            }
        }

        diagnostics.total_time = Some(start_time.elapsed());
        Ok(diagnostics)
    }

    /// Resolve DNS for a hostname
    async fn resolve_dns(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Vec<IpAddr>> {
        let socket_addrs: Vec<SocketAddr> = tokio::task::spawn_blocking({
            let host = host.to_string();
            move || {
                format!("{}:{}", host, port)
                    .to_socket_addrs()
                    .map(|iter| iter.collect())
                    .map_err(|e| anyhow::anyhow!("DNS resolution failed: {}", e))
            }
        })
        .await??;

        let ips: Vec<IpAddr> = socket_addrs.into_iter().map(|addr| addr.ip()).collect();

        if ips.is_empty() {
            return Err(anyhow::anyhow!("No IP addresses resolved for {}", host));
        }

        Ok(ips)
    }

    /// Test connection to resolved IPs
    async fn test_connection(
        &self,
        ips: &[IpAddr],
        port: u16,
    ) -> Result<()> {
        let mut last_error = None;

        for ip in ips {
            let addr = SocketAddr::new(*ip, port);

            if self.verbose {
                tracing::debug!("Testing connection to {}", addr);
            }

            match tokio::time::timeout(
                Duration::from_secs(10), // 10 second timeout for connection test
                TcpStream::connect(addr),
            )
            .await
            {
                Ok(Ok(_stream)) => {
                    if self.verbose {
                        tracing::info!("Successfully connected to {}", addr);
                    }
                    return Ok(());
                }
                Ok(Err(e)) => {
                    last_error = Some(anyhow::anyhow!("Connection to {} failed: {}", addr, e));
                    if self.verbose {
                        tracing::warn!("Connection to {} failed: {}", addr, e);
                    }
                }
                Err(_) => {
                    last_error = Some(anyhow::anyhow!("Connection to {} timed out", addr));
                    if self.verbose {
                        tracing::warn!("Connection to {} timed out", addr);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All connection attempts failed")))
    }

    /// Generate diagnostic report
    pub fn generate_report(
        &self,
        diagnostics: &NetworkDiagnostics,
    ) -> String {
        let mut report = Vec::new();

        report.push(format!(
            "Network Diagnostics Report for {}",
            diagnostics.url
        ));
        report.push("=".repeat(50));

        report.push(format!("Host: {}", diagnostics.host));
        report.push(format!("Port: {}", diagnostics.port));

        if let Some(dns_time) = diagnostics.dns_resolution_time {
            report.push(format!("DNS Resolution: {:?}", dns_time));
            report.push(format!("Resolved IPs: {:?}", diagnostics.resolved_ips));
        }

        if let Some(conn_time) = diagnostics.connection_time {
            report.push(format!("Connection Time: {:?}", conn_time));
        }

        if let Some(total_time) = diagnostics.total_time {
            report.push(format!("Total Time: {:?}", total_time));
        }

        if let Some(ref error) = diagnostics.error {
            report.push(format!("Error: {}", error));
            report.push(String::new());
            report.push("Troubleshooting suggestions:".to_string());

            if error.contains("DNS resolution failed") {
                report.push("- Check your internet connection".to_string());
                report.push("- Verify DNS settings (try 8.8.8.8 or 1.1.1.1)".to_string());
                report.push("- Check if the hostname is correct".to_string());
            } else if error.contains("Connection failed") || error.contains("timed out") {
                report.push("- Check firewall settings".to_string());
                report.push("- Verify the service is running on the target host".to_string());
                report.push("- Check if you're behind a proxy".to_string());
                report.push("- Try using a VPN if the service is geo-restricted".to_string());
            }
        } else {
            report.push("Status: All network checks passed".to_string());
        }

        report.join("\n")
    }

    /// Report progress if callback is configured
    fn report_progress(
        &self,
        progress: DownloadProgress,
    ) {
        if let Some(ref callback) = self.progress_callback {
            callback(progress);
        }
    }
}

/// Quick network connectivity test
pub async fn quick_connectivity_test(url: &str) -> Result<bool> {
    let diagnostics_runner = NetworkDiagnosticsRunner::new(None, false);
    let diagnostics = diagnostics_runner.diagnose_url(url).await?;
    Ok(diagnostics.error.is_none())
}

/// Get network diagnostic suggestions based on error
pub fn get_diagnostic_suggestions(error: &str) -> Vec<String> {
    let mut suggestions = Vec::new();

    let error_lower = error.to_lowercase();

    if error_lower.contains("dns") || error_lower.contains("resolve") {
        suggestions.extend_from_slice(&[
            "Check your internet connection".to_string(),
            "Try using different DNS servers (8.8.8.8, 1.1.1.1)".to_string(),
            "Verify the hostname is correct".to_string(),
            "Check if you're behind a corporate firewall".to_string(),
        ]);
    }

    if error_lower.contains("timeout") || error_lower.contains("connection") {
        suggestions.extend_from_slice(&[
            "Check your firewall settings".to_string(),
            "Verify you have internet access".to_string(),
            "Try using a VPN if the service is geo-restricted".to_string(),
            "Check if you're behind a proxy".to_string(),
        ]);
    }

    if error_lower.contains("ssl") || error_lower.contains("tls") {
        suggestions.extend_from_slice(&[
            "Check system date and time".to_string(),
            "Update your system certificates".to_string(),
            "Try disabling SSL verification temporarily".to_string(),
        ]);
    }

    if suggestions.is_empty() {
        suggestions.push("Check your internet connection and try again".to_string());
    }

    suggestions
}
