use crate::runtime::{RuntimeManager, RuntimeType};
use anyhow::Result;

/// List installed runtime environments for a specific type
pub fn list_runtime(
    runtime_manager: &RuntimeManager,
    runtime_type: RuntimeType,
) -> Result<()> {
    use std::fs;

    // Get the runtime directory
    let runtime_dir = runtime_manager.get_runtime_dir(runtime_type)?;

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
                if runtime_manager.is_runtime_available(runtime_type, Some(&version))? {
                    let exec_path =
                        runtime_manager.get_runtime_path(runtime_type, Some(&version))?;
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
