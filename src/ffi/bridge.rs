//! C-compatible FFI Bridge
//!
//! This module provides C-compatible functions for Swift integration

#[cfg(feature = "ffi")]
use super::engine::MCPMateEngine;
#[cfg(feature = "ffi")]
use super::types::PortConfig;
#[cfg(feature = "ffi")]
use std::ffi::CString;
#[cfg(feature = "ffi")]
use std::os::raw::c_char;
#[cfg(feature = "ffi")]
use std::ptr;

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_new() -> *mut MCPMateEngine {
    let engine = MCPMateEngine::new();
    Box::into_raw(Box::new(engine))
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_free(engine: *mut MCPMateEngine) {
    if !engine.is_null() {
        unsafe {
            let _ = Box::from_raw(engine);
        }
    }
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_start(
    engine: *mut MCPMateEngine,
    api_port: u16,
    mcp_port: u16,
) -> bool {
    if engine.is_null() {
        return false;
    }

    unsafe {
        let engine_ref = &mut *engine;
        engine_ref.start(api_port, mcp_port)
    }
}

/// Start MCPMate engine with port configuration structure
#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_start_with_config(
    engine: *mut MCPMateEngine,
    config_json: *const c_char,
) -> bool {
    if engine.is_null() || config_json.is_null() {
        return false;
    }

    unsafe {
        let engine_ref = &mut *engine;

        // Parse JSON configuration
        let config_str = match std::ffi::CStr::from_ptr(config_json).to_str() {
            Ok(s) => {
                tracing::info!("FFI received config JSON: {}", s);
                s
            }
            Err(e) => {
                tracing::error!("Failed to parse config JSON string: {}", e);
                return false;
            }
        };

        let port_config: PortConfig = match serde_json::from_str::<PortConfig>(config_str) {
            Ok(config) => {
                tracing::info!(
                    "FFI parsed port config: API={}, MCP={}",
                    config.api_port,
                    config.mcp_port
                );
                config
            }
            Err(e) => {
                tracing::error!("Failed to deserialize port config: {}", e);
                return false;
            }
        };

        // Validate configuration
        if let Err(e) = port_config.validate() {
            tracing::error!("Port config validation failed: {}", e);
            return false;
        }

        tracing::info!("Starting engine with validated port config");
        engine_ref.start_with_config(port_config)
    }
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_stop(engine: *mut MCPMateEngine) {
    if !engine.is_null() {
        unsafe {
            let engine_ref = &mut *engine;
            engine_ref.stop();
        }
    }
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_is_running(engine: *const MCPMateEngine) -> bool {
    if engine.is_null() {
        return false;
    }

    unsafe {
        let engine_ref = &*engine;
        engine_ref.is_running()
    }
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_get_startup_progress_json(
    engine: *const MCPMateEngine
) -> *const c_char {
    if engine.is_null() {
        return ptr::null();
    }

    unsafe {
        let engine_ref = &*engine;
        let json = engine_ref.get_startup_progress_json();
        match CString::new(json) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => ptr::null(),
        }
    }
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_engine_get_service_info_json(
    engine: *const MCPMateEngine
) -> *const c_char {
    if engine.is_null() {
        return ptr::null();
    }

    unsafe {
        let engine_ref = &*engine;
        let json = engine_ref.get_service_info_json();
        match CString::new(json) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => ptr::null(),
        }
    }
}

#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub extern "C" fn mcpmate_string_free(string: *mut c_char) {
    if !string.is_null() {
        unsafe {
            let _ = CString::from_raw(string);
        }
    }
}
