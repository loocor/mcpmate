use crate::common::paths::global_paths;
use crate::runtime::RuntimeType;
use anyhow::Result;

/// List installed runtime environments for a specific type
pub fn list_runtime(runtime_type: RuntimeType) -> Result<()> {
    use std::fs;

    let paths = global_paths();

    // Get the runtime directory
    let runtime_dir = paths.runtime_type_dir(&runtime_type.to_string());

    // Check if directory exists
    if !runtime_dir.exists() {
        println!("  No installations found");
        return Ok(());
    }

    // List subdirectories (versions)
    let mut found = false;
    if let Ok(entries) = fs::read_dir(runtime_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let version = path.file_name().unwrap().to_string_lossy();

                // Check if this version is actually installed
                if is_runtime_installed(runtime_type, Some(&version))? {
                    let exec_path = crate::runtime::executable::get_runtime_executable_path(
                        runtime_type,
                        Some(&version),
                    )?;
                    println!("  {} ({})", version, exec_path.display());

                    found = true;
                }
            }
        }
    }

    if !found {
        println!("  No installations found");
    }

    Ok(())
}

/// Check if a runtime is installed and available
fn is_runtime_installed(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<bool> {
    // Get the executable path
    let executable_path =
        crate::runtime::executable::get_runtime_executable_path(runtime_type, version)?;

    // Check if the executable file exists
    Ok(executable_path.exists())
}
