/// Event system for MCPMate
///
/// This module provides a simple event system for MCPMate, allowing components
/// to communicate with each other without direct dependencies.
///
/// The event system is designed to be lightweight and focused on core scenarios,
/// particularly server status changes that need to trigger connection operations.
mod bus;
mod handlers;
mod types;
mod wait;

pub use bus::EventBus;
pub use tokio::sync::broadcast::Receiver as EventReceiver;
pub use types::Event;
pub use wait::{needs_transport_ready_wait, wait_for_transport_ready};

/// Initialize the event system
pub fn init() {
    // Register event handlers
    handlers::register_handlers();
}
