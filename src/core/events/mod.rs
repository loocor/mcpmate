//! Event system for the core architecture
//!
//! This module provides a decoupled event-driven communication system that supports
//! both synchronous and asynchronous event handling. It's designed to replace the
//! core/events module with improved performance and better separation of concerns.
//!
//! # Features
//!
//! - Global event bus with broadcast capabilities
//! - Support for both sync and async event subscribers
//! - Transport layer readiness waiting mechanism
//! - Dependency injection for event handlers
//! - Event classification for optimized processing
//!
//! # Usage
//!
//! ## Basic Event Publishing
//!
//! ```rust
//! use crate::core::events::{EventBus, Event};
//!
//! // Publish an event
//! EventBus::global().publish(Event::ServerGlobalStatusChanged {
//!     server_id: "test".to_string(),
//!     server_name: "test_server".to_string(),
//!     enabled: true,
//! });
//! ```
//!
//! ## Initialize with Custom Handlers
//!
//! ```rust
//! use crate::core::events::{EventHandlers, init_with_handlers};
//!
//! let mut handlers = EventHandlers::new();
//! handlers.set_suit_service(suit_service);
//!     Box::pin(async {
//!         // Your server sync logic here
//!         Ok(())
//!     })
//! });
//!
//! init_with_handlers(handlers)?;
//! ```

pub mod bus;
pub mod capability;
pub mod handlers;
pub mod types;
pub mod wait;

// Re-export public API
pub use bus::EventBus;
pub use capability::EventDrivenCapabilityManager;
pub use handlers::{EventHandlers, init, init_with_handlers};
pub use types::Event;
pub use wait::{needs_transport_ready_wait, wait_for_transport_ready};

// Re-export for compatibility
pub use tokio::sync::broadcast::Receiver as EventReceiver;
