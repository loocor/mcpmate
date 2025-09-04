use crate::common::constants::ports;
use clap::Parser;

/// Command line arguments for the MCP proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on for MCP server
    #[arg(short, long, default_value_t = ports::MCP_PORT)]
    pub mcp_port: u16,

    /// Port to listen on for API server
    #[arg(long, default_value_t = ports::API_PORT)]
    pub api_port: u16,

    /// Log level (when RUST_LOG is not set)
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// Transport type (sse, str, or uni)
    #[arg(long, alias = "trans", default_value = "uni")]
    pub transport: String,

    /// Profile to load (comma-separated list of IDs)
    /// Use empty string or no value to load no profile
    /// If not specified, loads the active default profile
    #[arg(long, value_delimiter = ',')]
    pub profile: Option<Vec<String>>,

    /// Start in minimal mode (API only, no profile loaded)
    /// This flag has highest priority and overrides profile
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

        // Validate profile if provided
        if let Some(ref profile) = self.profile {
            for profile in profile {
                if !profile.trim().is_empty() {
                    // Only validate non-empty profile IDs
                    // Empty strings are allowed to represent "no profile"
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
        } else if let Some(ref profile) = self.profile {
            // Filter out empty strings and check if we have any valid profile IDs
            let valid_profile: Vec<String> = profile.iter().filter(|s| !s.trim().is_empty()).cloned().collect();

            if valid_profile.is_empty() {
                StartupMode::NoProfile
            } else {
                StartupMode::SpecificProfile(valid_profile)
            }
        } else {
            StartupMode::Default
        }
    }

    /// Check if any profile should be loaded
    pub fn should_load_profile(&self) -> bool {
        !self.minimal && !matches!(self.profile, Some(ref profile) if profile.is_empty())
    }
}

/// Startup mode enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum StartupMode {
    /// Load active default profile
    Default,
    /// Load specific profile by ID
    SpecificProfile(Vec<String>),
    /// Don't load any profile (empty list provided)
    NoProfile,
    /// Minimal mode - API only, no profile (--minimal flag)
    Minimal,
}
