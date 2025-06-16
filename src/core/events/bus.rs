//! Event bus for the core event system

use std::sync::{Arc, RwLock};

use tokio::sync::broadcast;
use tracing::{debug, error, info};

use super::types::Event;

/// Maximum number of events that can be queued
const MAX_EVENTS: usize = 100;

/// Type alias for event handler function
type EventHandler = Box<dyn Fn(Event) + Send + Sync + 'static>;

/// Global event bus singleton
static EVENT_BUS: once_cell::sync::Lazy<EventBus> = once_cell::sync::Lazy::new(|| {
    info!("Initializing core event bus");
    EventBus::new()
});

/// Event bus for publishing and subscribing to events
#[derive(Clone)]
pub struct EventBus {
    /// Channel for broadcasting events
    sender: broadcast::Sender<Event>,
    /// Event handlers
    handlers: Arc<RwLock<Vec<EventHandler>>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(MAX_EVENTS);
        Self {
            sender,
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the global event bus instance
    pub fn global() -> &'static Self {
        &EVENT_BUS
    }

    /// Publish an event to all subscribers
    pub fn publish(
        &self,
        event: Event,
    ) {
        debug!("Publishing core event: {:?}", event);

        // Send to broadcast channel for async subscribers
        let _ = self.sender.send(event.clone());

        // Call all registered handlers
        if let Ok(handlers) = self.handlers.read() {
            for handler in handlers.iter() {
                handler(event.clone());
            }
        } else {
            error!("Failed to acquire read lock on event handlers");
        }
    }

    /// Subscribe to events with a handler function
    pub fn subscribe<F>(
        &self,
        handler: F,
    ) where
        F: Fn(Event) + Send + Sync + 'static,
    {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.push(Box::new(handler));
            debug!(
                "Added core event handler, total handlers: {}",
                handlers.len()
            );
        } else {
            error!("Failed to acquire write lock on event handlers");
        }
    }

    /// Subscribe to events with an async receiver
    pub fn subscribe_async(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
