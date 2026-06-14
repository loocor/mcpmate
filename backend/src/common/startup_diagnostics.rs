//! Structured startup diagnostic events for degrade/fatal paths.
//!
//! Fields are written to `mcpmate.log` and included in Desktop diagnostics export.

pub mod component {
    pub const MAIN: &str = "startup_main";
    pub const INIT: &str = "startup_init";
    pub const LOADER: &str = "startup_loader";
    pub const PROXY: &str = "startup_proxy";
    pub const BACKGROUND: &str = "startup_background";
    pub const API: &str = "startup_api";
}

#[derive(Clone, Copy, Debug)]
pub struct StartupDegradedEvent {
    pub component: &'static str,
    pub phase: &'static str,
    pub reason_code: &'static str,
    pub action_taken: &'static str,
    pub subsystem: &'static str,
}

pub fn warn_degraded(
    event: StartupDegradedEvent,
    error: &dyn std::fmt::Display,
    message: &'static str,
) {
    tracing::warn!(
        component = event.component,
        phase = event.phase,
        subsystem = event.subsystem,
        degraded = true,
        startup_continues = true,
        action_taken = event.action_taken,
        reason_code = event.reason_code,
        error = %error,
        "{message}"
    );
}

pub fn warn_degraded_reason(
    component: &'static str,
    phase: &'static str,
    reason_code: &str,
    action_taken: &'static str,
    subsystem: &'static str,
    detail: &str,
    message: &'static str,
) {
    tracing::warn!(
        component,
        phase,
        subsystem,
        degraded = true,
        startup_continues = true,
        action_taken,
        reason_code = %reason_code,
        detail = %detail,
        "{message}"
    );
}

pub fn error_fatal(
    phase: &'static str,
    reason_code: &'static str,
    error: &dyn std::fmt::Display,
) {
    tracing::error!(
        component = component::MAIN,
        phase,
        degraded = false,
        startup_continues = false,
        action_taken = "abort_startup",
        reason_code,
        error = %error,
        "MCPMate startup failed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for CaptureWriter {
        fn write(
            &mut self,
            buf: &[u8],
        ) -> std::io::Result<usize> {
            self.0.lock().expect("capture lock").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn error_fatal_emits_structured_startup_fields() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer({
                let buffer = Arc::clone(&buffer);
                move || CaptureWriter(Arc::clone(&buffer))
            })
            .with_ansi(false)
            .without_time()
            .with_level(false)
            .with_target(false)
            .finish();

        let _guard = tracing::subscriber::set_default(subscriber);
        error_fatal("proxy_setup", "proxy_setup_failed", &"database unavailable");

        let output = String::from_utf8(buffer.lock().expect("capture lock").clone()).expect("utf8 logs");
        assert!(output.contains("startup_main"));
        assert!(output.contains("proxy_setup_failed"));
        assert!(output.contains("abort_startup"));
    }
}
