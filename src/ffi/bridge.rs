//! C-compatible FFI Bridge
//!
//! This module provides C-compatible functions for Swift integration

#[cfg(feature = "ffi")]
use super::engine::MCPMateEngine;
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
