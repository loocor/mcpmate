use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

/// System environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub os: OperatingSystem,
    pub arch: Architecture,
}

/// Operating system type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
}

/// System architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    Aarch64,
}

impl OperatingSystem {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "macos",
            OperatingSystem::Linux => "linux",
        }
    }

    /// Get file extension
    pub fn archive_extension(&self) -> &'static str {
        match self {
            OperatingSystem::Windows => "zip",
            OperatingSystem::MacOS | OperatingSystem::Linux => "tar.gz",
        }
    }
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        }
    }

    /// Get Node.js architecture name
    pub fn node_arch(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x64",
            Architecture::Aarch64 => "arm64",
        }
    }
}

/// Detect current system environment
pub fn detect_environment() -> Result<Environment> {
    let os = detect_os()?;
    let arch = detect_arch()?;

    Ok(Environment { os, arch })
}

/// Detect operating system
fn detect_os() -> Result<OperatingSystem> {
    match env::consts::OS {
        "windows" => Ok(OperatingSystem::Windows),
        "macos" => Ok(OperatingSystem::MacOS),
        "linux" => Ok(OperatingSystem::Linux),
        other => Err(anyhow::anyhow!("Unsupported operating system: {}", other)),
    }
}

/// Detect system architecture
fn detect_arch() -> Result<Architecture> {
    match env::consts::ARCH {
        "x86_64" => Ok(Architecture::X86_64),
        "aarch64" => Ok(Architecture::Aarch64),
        other => Err(anyhow::anyhow!(
            "Unsupported system architecture: {}",
            other
        )),
    }
}
