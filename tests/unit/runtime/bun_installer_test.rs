//! Tests for Bun installer

use anyhow::Result;
use mcpmate::runtime::{Architecture, BunInstaller, Environment, OperatingSystem};

#[tokio::test]
async fn test_bun_installer_supports_windows_arm() -> Result<()> {
    // Given: A Windows ARM environment
    let environment = Environment {
        os: OperatingSystem::Windows,
        arch: Architecture::Aarch64,
    };

    // When: Creating a Bun installer
    let installer = BunInstaller::new(environment);

    // Then: It should generate URLs for x64 version (since Windows ARM uses x64 through emulation)
    let url = installer.get_download_url("latest")?;
    assert!(url.contains("windows"));
    assert!(url.contains("x64")); // Windows ARM uses x64 version, not aarch64
    assert!(url.contains("github.com/oven-sh/bun"));

    Ok(())
}

#[tokio::test]
async fn test_bun_installer_download_url_formats() -> Result<()> {
    // Test different platform combinations
    let test_cases = vec![
        (
            OperatingSystem::Windows,
            Architecture::X86_64,
            "windows",
            "x64",
        ),
        (
            OperatingSystem::Windows,
            Architecture::Aarch64,
            "windows",
            "x64", // Windows ARM uses x64 version through emulation
        ),
        (
            OperatingSystem::MacOS,
            Architecture::X86_64,
            "darwin",
            "x64",
        ),
        (
            OperatingSystem::MacOS,
            Architecture::Aarch64,
            "darwin",
            "aarch64",
        ),
        (OperatingSystem::Linux, Architecture::X86_64, "linux", "x64"),
        (
            OperatingSystem::Linux,
            Architecture::Aarch64,
            "linux",
            "aarch64",
        ),
    ];

    for (os, arch, expected_os, expected_arch) in test_cases {
        let environment = Environment { os, arch };
        let installer = BunInstaller::new(environment);

        // Test latest version URL
        let latest_url = installer.get_download_url("latest")?;
        assert!(latest_url.contains(expected_os));
        assert!(latest_url.contains(expected_arch));
        assert!(latest_url.contains("releases/latest/download"));

        // Test specific version URL
        let version_url = installer.get_download_url("1.0.0")?;
        assert!(version_url.contains(expected_os));
        assert!(version_url.contains(expected_arch));
        assert!(version_url.contains("releases/download/bun-v1.0.0"));
    }

    Ok(())
}
