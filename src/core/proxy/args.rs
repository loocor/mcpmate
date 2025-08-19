use clap::Parser;

/// Command line arguments for the MCP proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on for MCP server
    #[arg(short, long, default_value = "8000")]
    pub mcp_port: u16,

    /// Port to listen on for API server
    #[arg(long, default_value = "8080")]
    pub api_port: u16,

    /// Log level (when RUST_LOG is not set)
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// Transport type (sse, str, or uni)
    #[arg(long, alias = "trans", default_value = "uni")]
    pub transport: String,

    /// Configuration suites to load (comma-separated list of IDs)
    /// Use empty string or no value to load no suites
    /// If not specified, loads the active default configuration suite
    #[arg(long, value_delimiter = ',')]
    pub config_suites: Option<Vec<String>>,

    /// Start in minimal mode (API only, no config suites loaded)
    /// This flag has highest priority and overrides config-suites
    #[arg(long)]
    pub minimal: bool,
}

impl Args {
    /// Validate the command line arguments
    pub fn validate(&self) -> Result<(), String> {
        // Validate port ranges
        if self.mcp_port == 0 {
            return Err("MCP port cannot be 0".to_string());
        }
        if self.api_port == 0 {
            return Err("API port cannot be 0".to_string());
        }
        if self.mcp_port == self.api_port {
            return Err("MCP port and API port cannot be the same".to_string());
        }

        // Validate config suites if provided
        if let Some(ref suites) = self.config_suites {
            for suite in suites {
                if !suite.trim().is_empty() {
                    // Only validate non-empty suite IDs
                    // Empty strings are allowed to represent "no suites"
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Get the effective startup mode based on arguments
    pub fn get_startup_mode(&self) -> StartupMode {
        if self.minimal {
            StartupMode::Minimal
        } else if let Some(ref suites) = self.config_suites {
            // Filter out empty strings and check if we have any valid suite IDs
            let valid_suites: Vec<String> = suites
                .iter()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .collect();

            if valid_suites.is_empty() {
                StartupMode::NoSuites
            } else {
                StartupMode::SpecificSuites(valid_suites)
            }
        } else {
            StartupMode::Default
        }
    }

    /// Check if any configuration suites should be loaded
    pub fn should_load_suites(&self) -> bool {
        !self.minimal && !matches!(self.config_suites, Some(ref suites) if suites.is_empty())
    }
}

/// Startup mode enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum StartupMode {
    /// Load active default configuration suite
    Default,
    /// Load specific configuration suites by ID
    SpecificSuites(Vec<String>),
    /// Don't load any configuration suites (empty list provided)
    NoSuites,
    /// Minimal mode - API only, no suites (--minimal flag)
    Minimal,
}
