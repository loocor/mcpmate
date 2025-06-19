//! Tests for runtime path structure

use anyhow::Result;
use mcpmate::runtime::{RuntimeManager, RuntimeType};

#[test]
fn test_runtime_path_structure() -> Result<()> {
    // Given: A runtime manager
    let manager = RuntimeManager::new();

    // When: Getting the runtimes directory
    let runtimes_dir = manager.runtimes_dir();

    // Then: It should point to ~/.mcpmate/runtimes/
    assert!(runtimes_dir.to_string_lossy().contains(".mcpmate"));
    assert!(runtimes_dir.to_string_lossy().ends_with("runtimes"));

    Ok(())
}

#[test]
fn test_runtime_subdirectory_paths() -> Result<()> {
    // Given: A runtime manager
    let manager = RuntimeManager::new();
    let runtimes_dir = manager.runtimes_dir();

    // When: Checking expected subdirectory structure
    let expected_paths = vec![("uv", vec!["uv", "uvx"]), ("bun", vec!["bun", "bunx"])];

    // Then: The expected structure should be correct
    for (runtime_name, executables) in expected_paths {
        let runtime_dir = runtimes_dir.join(runtime_name);

        for exe in executables {
            let exe_name = if cfg!(windows) {
                format!("{}.exe", exe)
            } else {
                exe.to_string()
            };
            let exe_path = runtime_dir.join(exe_name);

            // We don't check if files exist (they might not be installed)
            // but we verify the expected path structure
            assert!(
                exe_path
                    .to_string_lossy()
                    .contains(&format!("runtimes/{}", runtime_name))
            );
        }
    }

    Ok(())
}

#[test]
fn test_runtime_manager_path_detection() -> Result<()> {
    // Given: A runtime manager
    let manager = RuntimeManager::new();

    // When: Checking for runtime paths (they might not exist, that's OK)
    let uv_path = manager.get_executable_path(RuntimeType::Uv);
    let bun_path = manager.get_executable_path(RuntimeType::Bun);

    // Then: If paths exist, they should be in the correct subdirectories
    if let Some(path) = uv_path {
        assert!(path.to_string_lossy().contains("runtimes/uv"));
    }

    if let Some(path) = bun_path {
        assert!(path.to_string_lossy().contains("runtimes/bun"));
    }

    Ok(())
}
