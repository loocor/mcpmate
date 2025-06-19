//! Tests for runtime API with enhanced source information

use mcpmate::runtime::{RuntimeManager, RuntimeType};

#[test]
fn test_runtime_status_messages() {
    let manager = RuntimeManager::new();

    // Test UV runtime status message
    if let Some(uv_info) = manager
        .list_installed()
        .into_iter()
        .find(|info| info.runtime_type == RuntimeType::Uv)
    {
        println!("UV Runtime: {}", uv_info.message);

        if uv_info.available {
            // Should contain source information
            assert!(
                uv_info.message.contains("(MCPMate managed)")
                    || uv_info.message.contains("(system fallback)"),
                "UV message should contain source information: {}",
                uv_info.message
            );
        }
    }

    // Test Bun runtime status message
    if let Some(bun_info) = manager
        .list_installed()
        .into_iter()
        .find(|info| info.runtime_type == RuntimeType::Bun)
    {
        println!("Bun Runtime: {}", bun_info.message);

        if bun_info.available {
            // Should contain source information
            assert!(
                bun_info.message.contains("(MCPMate managed)")
                    || bun_info.message.contains("(system fallback)"),
                "Bun message should contain source information: {}",
                bun_info.message
            );
        }
    }
}

#[test]
fn test_runtime_path_detection() {
    let manager = RuntimeManager::new();

    // Test UV runtime path preference
    if let Some(uv_path) = manager.get_executable_path(RuntimeType::Uv) {
        println!("UV path: {}", uv_path.display());

        // Should prefer MCPMate managed uv for version consistency
        if uv_path.to_string_lossy().contains(".mcpmate") {
            println!("✅ Using MCPMate managed uv (preferred)");
        } else {
            println!("⚠️  Using system uv (fallback)");
        }
    }

    // Test Bun runtime path
    if let Some(bun_path) = manager.get_executable_path(RuntimeType::Bun) {
        println!("Bun path: {}", bun_path.display());

        if bun_path.to_string_lossy().contains(".mcpmate") {
            println!("✅ Using MCPMate managed bunx");
        } else {
            println!("⚠️  Using system bun");
        }
    }
}

#[test]
fn test_runtime_consistency() {
    let manager = RuntimeManager::new();

    for runtime_type in [RuntimeType::Uv, RuntimeType::Bun] {
        let available = manager.is_installed(runtime_type);
        let path = manager.get_executable_path(runtime_type);

        // Consistency check: if available is true, path should be Some
        if available {
            assert!(
                path.is_some(),
                "Runtime {} is marked as available but has no path",
                runtime_type.as_str()
            );
        } else {
            assert!(
                path.is_none(),
                "Runtime {} is marked as unavailable but has a path",
                runtime_type.as_str()
            );
        }
    }
}
