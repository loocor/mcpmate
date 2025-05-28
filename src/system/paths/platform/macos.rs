// macOS-specific path resolution

use anyhow::Result;
use std::path::PathBuf;

/// Get standard macOS application directories
pub fn get_applications_directories() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ]
}

/// Get user-specific application directories
pub fn get_user_applications_directories() -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        dirs.push(home_dir.join("Applications"));
    }

    Ok(dirs)
}

/// Get standard configuration directories for macOS
pub fn get_config_directories() -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        dirs.push(home_dir.join("Library/Application Support"));
        dirs.push(home_dir.join("Library/Preferences"));
        dirs.push(home_dir.join(".config"));
    }

    Ok(dirs)
}

/// Resolve application bundle path to executable
pub fn resolve_bundle_executable(
    bundle_path: &PathBuf,
    bundle_id: &str,
) -> Result<PathBuf> {
    // For most macOS apps, the executable is at Contents/MacOS/{AppName}
    let contents_dir = bundle_path.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    // Try to find the executable
    if macos_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&macos_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Check if file is executable
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        use std::os::unix::fs::PermissionsExt;
                        if metadata.permissions().mode() & 0o111 != 0 {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    // Fallback: try to extract app name from bundle_id
    let app_name = bundle_id.split('.').last().unwrap_or("Unknown");
    let executable_path = macos_dir.join(app_name);

    if executable_path.exists() {
        Ok(executable_path)
    } else {
        Err(anyhow::anyhow!(
            "Could not find executable in bundle: {}",
            bundle_path.display()
        ))
    }
}

/// Get application version from bundle
pub fn get_bundle_version(bundle_path: &PathBuf) -> Result<String> {
    let info_plist_path = bundle_path.join("Contents/Info.plist");

    if !info_plist_path.exists() {
        return Err(anyhow::anyhow!("Info.plist not found in bundle"));
    }

    // For now, return a placeholder. In a full implementation, we would parse the plist
    // This would require adding a plist parsing dependency
    Ok("Unknown".to_string())
}
