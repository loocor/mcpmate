use mcpmate::common::get_bridge_path;
use std::env;

#[test]
fn test_get_bridge_path_basic() {
    let result = get_bridge_path();

    // In test environment, bridge may not exist, so we test both scenarios
    match result {
        Ok(bridge_path) => {
            // Should not be empty
            assert!(!bridge_path.is_empty(), "Bridge path should not be empty");

            // Should end with bridge executable
            assert!(
                bridge_path.ends_with("bridge") || bridge_path.ends_with("bridge.exe"),
                "Bridge path should end with bridge executable: {}",
                bridge_path
            );

            println!("✅ Dynamic bridge path resolved to: {}", bridge_path);
        }
        Err(e) => {
            // Should provide helpful but focused error message
            let error_msg = e.to_string();
            assert!(error_msg.contains("Bridge executable"));
            assert!(error_msg.contains("not found"));
            assert!(error_msg.contains("same directory"));

            println!("✅ Bridge not found in test environment, clear error provided");
        }
    }
}

#[test]
fn test_get_bridge_path_with_env_override() {
    // Test environment variable override
    let custom_path = "/custom/test/bridge";
    unsafe {
        env::set_var("MCPMATE_BRIDGE_PATH", custom_path);
    }

    let result = get_bridge_path();
    assert!(result.is_ok());

    let bridge_path = result.unwrap();
    assert_eq!(bridge_path, custom_path);

    // Cleanup
    unsafe {
        env::remove_var("MCPMATE_BRIDGE_PATH");
    }

    println!("✅ Environment variable override works: {}", bridge_path);
}

#[test]
fn test_get_bridge_path_empty_env_var() {
    // Test empty environment variable should be ignored
    unsafe {
        env::set_var("MCPMATE_BRIDGE_PATH", "");
    }

    let result = get_bridge_path();
    // Should either succeed (found bridge in exe dir) or fail with meaningful error
    match result {
        Ok(path) => {
            assert!(!path.is_empty());
            println!("✅ Empty env var ignored, found bridge at: {}", path);
        }
        Err(e) => {
            // Should have helpful error message
            let error_msg = e.to_string();
            assert!(error_msg.contains("Bridge executable"));
            assert!(error_msg.contains("not found"));
            assert!(error_msg.contains("same directory"));
            println!("✅ Helpful error message provided: {}", error_msg);
        }
    }

    // Cleanup
    unsafe {
        env::remove_var("MCPMATE_BRIDGE_PATH");
    }
}

#[test]
fn test_get_bridge_path_without_env() {
    // Ensure no environment variable is set
    unsafe {
        env::remove_var("MCPMATE_BRIDGE_PATH");
    }

    let result = get_bridge_path();

    match result {
        Ok(bridge_path) => {
            // Should not be empty
            assert!(!bridge_path.is_empty(), "Bridge path should not be empty");

            // Should end with bridge executable
            assert!(
                bridge_path.ends_with("bridge") || bridge_path.ends_with("bridge.exe"),
                "Bridge path should end with bridge executable: {}",
                bridge_path
            );

            println!("✅ Bridge path resolved to: {}", bridge_path);
        }
        Err(e) => {
            // If bridge is not found, error should be helpful and focused
            let error_msg = e.to_string();
            assert!(error_msg.contains("Bridge executable"));
            assert!(error_msg.contains("not found"));
            assert!(error_msg.contains("same directory"));
            assert!(error_msg.contains("MCPMATE_BRIDGE_PATH"));

            println!("✅ Clear error message provided when bridge not found");
            println!("Error: {}", error_msg);
        }
    }
}

#[test]
fn test_get_bridge_path_platform_specific() {
    // Remove env var to test platform-specific logic
    unsafe {
        env::remove_var("MCPMATE_BRIDGE_PATH");
    }

    let result = get_bridge_path();

    match result {
        Ok(bridge_path) => {
            // Verify platform-specific executable name
            if cfg!(windows) {
                assert!(
                    bridge_path.ends_with("bridge.exe"),
                    "Windows bridge should end with .exe: {}",
                    bridge_path
                );
            } else {
                assert!(
                    bridge_path.ends_with("bridge") && !bridge_path.ends_with(".exe"),
                    "Unix bridge should not have .exe extension: {}",
                    bridge_path
                );
            }

            println!("✅ Platform-specific bridge path: {}", bridge_path);
        }
        Err(e) => {
            // Error message should still be platform-appropriate
            let error_msg = e.to_string();
            if cfg!(windows) {
                assert!(error_msg.contains("bridge.exe"));
            } else {
                assert!(error_msg.contains("bridge"));
                assert!(!error_msg.contains("bridge.exe"));
            }

            println!("✅ Platform-specific error message provided");
        }
    }
}

#[test]
fn test_get_bridge_path_error_message_quality() {
    // Remove env var to test error message in realistic scenario
    unsafe {
        env::remove_var("MCPMATE_BRIDGE_PATH");
    }

    // In test environment, bridge likely won't exist
    let result = get_bridge_path();

    if let Err(e) = result {
        let error_msg = e.to_string();

        // Check that error message is concise and actionable
        assert!(error_msg.contains("Bridge executable"));
        assert!(error_msg.contains("not found"));
        assert!(error_msg.contains("same directory"));
        assert!(error_msg.contains("MCPMATE_BRIDGE_PATH"));
        assert!(error_msg.contains("properly installed"));

        // Should NOT contain excessive search paths anymore
        assert!(!error_msg.contains("following locations:"));
        assert!(!error_msg.contains("/usr/local/bin"));
        assert!(!error_msg.contains("homebrew"));

        println!("✅ Error message is concise and actionable");
        println!("Error message: {}", error_msg);
    } else {
        println!("✅ Bridge found in test environment, error message test skipped");
    }
}
