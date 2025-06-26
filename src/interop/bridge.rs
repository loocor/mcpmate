//! C-compatible Interop Bridge
//!
//! This module provides C-compatible functions for cross-language integration

#[cfg(feature = "interop")]
use super::engine::MCPMateEngine;
#[cfg(feature = "interop")]
use super::types::PortConfig;
#[cfg(feature = "interop")]
use std::ffi::{CStr, CString};
#[cfg(feature = "interop")]
use std::os::raw::c_char;
#[cfg(feature = "interop")]
use std::ptr;

/// Create a new MCPMate engine instance
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_new() -> *mut MCPMateEngine {
    let engine = MCPMateEngine::new();
    Box::into_raw(Box::new(engine))
}

/// Free MCPMate engine instance
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - Takes ownership of the memory pointed to by `engine`
/// - The caller must ensure `engine` was allocated by `mcpmate_engine_new`
/// - The caller must ensure `engine` is not used after this call
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_free(engine: *mut MCPMateEngine) {
    if !engine.is_null() {
        unsafe {
            let _ = Box::from_raw(engine);
        }
    }
}

/// Start MCPMate service
///
/// Note: This function internally converts to StartupConfig for unified processing.
/// For more advanced configuration, use mcpmate_engine_start_with_startup_config().
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_start(
    engine: *mut MCPMateEngine,
    api_port: u16,
    mcp_port: u16,
) -> bool {
    if engine.is_null() {
        tracing::error!("MCPMate engine pointer is null");
        return false;
    }

    let engine = unsafe { &mut *engine };
    let result = engine.start(api_port, mcp_port);

    tracing::info!("MCPMate engine start result: {}", result);
    result
}

/// Start MCPMate service with configuration
///
/// Note: This function internally converts PortConfig to StartupConfig for unified processing.
/// For more advanced configuration, use mcpmate_engine_start_with_startup_config().
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences raw pointers that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `config_json` points to a valid null-terminated C string
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_start_with_config(
    engine: *mut MCPMateEngine,
    config_json: *const c_char,
) -> bool {
    // Validate engine pointer
    if engine.is_null() {
        tracing::error!("MCPMate engine pointer is null");
        return false;
    }

    // Validate config JSON pointer
    if config_json.is_null() {
        tracing::error!("Config JSON pointer is null");
        return false;
    }

    // Convert C string to Rust string
    let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to convert config JSON to UTF-8 string: {}", e);
            return false;
        }
    };

    // Parse JSON to PortConfig
    let config: PortConfig = match serde_json::from_str(config_str) {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Failed to parse config JSON: {}", e);
            return false;
        }
    };

    // Get mutable reference to engine and start with config
    let engine = unsafe { &mut *engine };
    let result = engine.start_with_config(config);

    tracing::info!("MCPMate engine start_with_config result: {}", result);
    result
}

/// Stop MCPMate service
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_stop(engine: *mut MCPMateEngine) {
    if !engine.is_null() {
        let engine = unsafe { &mut *engine };
        engine.stop();
    }
}

/// Check if service is running
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_is_running(engine: *mut MCPMateEngine) -> bool {
    if engine.is_null() {
        return false;
    }

    let engine = unsafe { &*engine };
    engine.is_running()
}

/// Get startup progress as JSON
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must free the returned string using `mcpmate_string_free`
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_get_startup_progress_json(
    engine: *mut MCPMateEngine
) -> *const c_char {
    if engine.is_null() {
        return ptr::null();
    }

    let engine = unsafe { &*engine };
    let progress = engine.get_startup_progress();

    match serde_json::to_string(&progress) {
        Ok(json) => match CString::new(json) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null(),
        },
        Err(_) => ptr::null(),
    }
}

/// Get service info as JSON
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must free the returned string using `mcpmate_string_free`
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_get_service_info_json(
    engine: *mut MCPMateEngine
) -> *const c_char {
    if engine.is_null() {
        return ptr::null();
    }

    let engine = unsafe { &*engine };
    let info = engine.get_service_info();

    match serde_json::to_string(&info) {
        Ok(json) => match CString::new(json) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null(),
        },
        Err(_) => ptr::null(),
    }
}

/// Start MCPMate service with default configuration
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_start_default(
    engine: *mut MCPMateEngine,
    api_port: u16,
    mcp_port: u16,
) -> bool {
    if engine.is_null() {
        tracing::error!("MCPMate engine pointer is null");
        return false;
    }

    let engine = unsafe { &mut *engine };
    let result = engine.start_default(api_port, mcp_port);

    tracing::info!("MCPMate engine start_default result: {}", result);
    result
}

/// Start MCPMate service in minimal mode
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_start_minimal(
    engine: *mut MCPMateEngine,
    api_port: u16,
) -> bool {
    if engine.is_null() {
        tracing::error!("MCPMate engine pointer is null");
        return false;
    }

    let engine = unsafe { &mut *engine };
    let result = engine.start_minimal(api_port);

    tracing::info!("MCPMate engine start_minimal result: {}", result);
    result
}

/// Start MCPMate service with startup configuration
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences raw pointers that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `config_json` points to a valid null-terminated C string
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_start_with_startup_config(
    engine: *mut MCPMateEngine,
    config_json: *const c_char,
) -> bool {
    // Validate engine pointer
    if engine.is_null() {
        tracing::error!("MCPMate engine pointer is null");
        return false;
    }

    // Validate config JSON pointer
    if config_json.is_null() {
        tracing::error!("Startup config JSON pointer is null");
        return false;
    }

    // Convert C string to Rust string
    let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                "Failed to convert startup config JSON to UTF-8 string: {}",
                e
            );
            return false;
        }
    };

    // Parse JSON to StartupConfig
    let config: super::types::StartupConfig = match serde_json::from_str(config_str) {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Failed to parse startup config JSON: {}", e);
            return false;
        }
    };

    // Get mutable reference to engine and start with startup config
    let engine = unsafe { &mut *engine };
    let result = engine.start_with_startup_config(config);

    tracing::info!(
        "MCPMate engine start_with_startup_config result: {}",
        result
    );
    result
}

/// Start MCPMate service with specific config suites
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences raw pointers that must be valid and properly aligned
/// - The caller must ensure `engine` points to a valid MCPMateEngine instance
/// - The caller must ensure `suites_json` points to a valid null-terminated C string
/// - The caller must ensure `engine` is not accessed concurrently from other threads
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_engine_start_with_suites(
    engine: *mut MCPMateEngine,
    api_port: u16,
    mcp_port: u16,
    suites_json: *const c_char,
) -> bool {
    // Validate engine pointer
    if engine.is_null() {
        tracing::error!("MCPMate engine pointer is null");
        return false;
    }

    // Validate suites JSON pointer
    if suites_json.is_null() {
        tracing::error!("Suites JSON pointer is null");
        return false;
    }

    // Convert C string to Rust string
    let suites_str = match unsafe { CStr::from_ptr(suites_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to convert suites JSON to UTF-8 string: {}", e);
            return false;
        }
    };

    // Parse JSON to Vec<String>
    let suites: Vec<String> = match serde_json::from_str(suites_str) {
        Ok(suites) => suites,
        Err(e) => {
            tracing::error!("Failed to parse suites JSON: {}", e);
            return false;
        }
    };

    // Get mutable reference to engine and start with suites
    let engine = unsafe { &mut *engine };
    let result = engine.start_with_suites(api_port, mcp_port, suites);

    tracing::info!("MCPMate engine start_with_suites result: {}", result);
    result
}

/// Free string allocated by Rust
///
/// # Safety
///
/// This function is unsafe because it:
/// - Takes ownership of memory pointed to by `string`
/// - The caller must ensure `string` was allocated by a Rust CString::into_raw call
/// - The caller must ensure `string` is not used after this call
/// - The caller must ensure `string` is not freed multiple times
#[cfg(feature = "interop")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mcpmate_string_free(string: *const c_char) {
    if !string.is_null() {
        unsafe {
            let _ = CString::from_raw(string as *mut c_char);
        }
    }
}
