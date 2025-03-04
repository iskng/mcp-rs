//! MCP Lifecycle Management
//!
//! This module implements lifecycle management for the Model Context Protocol (MCP),
//! handling client connection and disconnection events with a simple state model.

use std::collections::HashMap;
use std::sync::{ Arc, RwLock };
use tracing::{ debug, info };

use crate::errors::Error;

/// Client lifecycle states - simplified to just the essential states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Client is disconnected
    Disconnected,

    /// Client is connected and running
    Connected,
}

impl std::fmt::Display for ClientState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientState::Disconnected => write!(f, "Disconnected"),
            ClientState::Connected => write!(f, "Connected"),
        }
    }
}

/// Lifecycle event types
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    /// Client connected
    ClientConnected(String),

    /// Client disconnected
    ClientDisconnected(String),
}

/// Type for lifecycle event handlers
pub type LifecycleEventHandler = Box<dyn Fn(LifecycleEvent) + Send + Sync>;

/// Simplified lifecycle manager for MCP protocol
pub struct LifecycleManager {
    /// Client-specific lifecycle states
    client_states: RwLock<HashMap<String, ClientState>>,

    /// Event handlers for lifecycle events
    event_handlers: RwLock<Vec<LifecycleEventHandler>>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            client_states: RwLock::new(HashMap::new()),
            event_handlers: RwLock::new(Vec::new()),
        }
    }

    /// Register a handler for lifecycle events
    pub fn register_event_handler<F>(&self, handler: F)
        where F: Fn(LifecycleEvent) + Send + Sync + 'static
    {
        let mut handlers = self.event_handlers.write().unwrap();
        handlers.push(Box::new(handler));
    }

    /// Register a client connection
    pub fn register_client(&self, client_id: &str) -> Result<(), Error> {
        let mut states = self.client_states.write().unwrap();
        let prev_state = states.insert(client_id.to_string(), ClientState::Connected);

        // Notify handlers only if this is a new connection or a reconnection
        if prev_state != Some(ClientState::Connected) {
            debug!("Client {} connected", client_id);
            self.notify_event(LifecycleEvent::ClientConnected(client_id.to_string()));
        }

        Ok(())
    }

    /// Unregister a client connection
    pub fn unregister_client(&self, client_id: &str) -> Result<(), Error> {
        let mut states = self.client_states.write().unwrap();
        let prev_state = states.insert(client_id.to_string(), ClientState::Disconnected);

        // Notify handlers only if this was an active client
        if prev_state == Some(ClientState::Connected) {
            debug!("Client {} disconnected", client_id);
            self.notify_event(LifecycleEvent::ClientDisconnected(client_id.to_string()));
        }

        Ok(())
    }

    /// Get a client's current state
    pub fn client_state(&self, client_id: &str) -> ClientState {
        let states = self.client_states.read().unwrap();
        states.get(client_id).copied().unwrap_or(ClientState::Disconnected)
    }

    /// Check if a client is connected
    pub fn is_client_connected(&self, client_id: &str) -> bool {
        self.client_state(client_id) == ClientState::Connected
    }

    /// Get all connected clients
    pub fn connected_clients(&self) -> Vec<String> {
        let states = self.client_states.read().unwrap();
        states
            .iter()
            .filter_map(|(id, state)| {
                if *state == ClientState::Connected { Some(id.clone()) } else { None }
            })
            .collect()
    }

    /// Start the lifecycle (minimal implementation)
    pub async fn start(&self) -> Result<(), Error> {
        info!("Lifecycle manager started");
        Ok(())
    }

    /// Shutdown the lifecycle manager (minimal implementation)
    pub async fn shutdown(&self) -> Result<(), Error> {
        info!("Lifecycle manager shutting down");

        // Disconnect all clients
        let client_ids = {
            let states = self.client_states.read().unwrap();
            states
                .iter()
                .filter_map(|(id, state)| {
                    if *state == ClientState::Connected { Some(id.clone()) } else { None }
                })
                .collect::<Vec<_>>()
        };

        // Notify disconnection for each client
        for client_id in client_ids {
            let _ = self.unregister_client(&client_id);
        }

        Ok(())
    }

    /// Notify all handlers of an event
    pub fn notify_event(&self, event: LifecycleEvent) {
        let handlers = self.event_handlers.read().unwrap();
        for handler in handlers.iter() {
            handler(event.clone());
        }
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{ AtomicUsize, Ordering };

    #[test]
    fn client_state_transitions() {
        let manager = LifecycleManager::new();

        // Test client state tracking
        assert_eq!(manager.client_state("test"), ClientState::Disconnected);

        manager.register_client("test").unwrap();
        assert_eq!(manager.client_state("test"), ClientState::Connected);

        manager.unregister_client("test").unwrap();
        assert_eq!(manager.client_state("test"), ClientState::Disconnected);
    }

    #[test]
    fn lifecycle_event_handlers() {
        let manager = LifecycleManager::new();
        let connect_count = Arc::new(AtomicUsize::new(0));
        let disconnect_count = Arc::new(AtomicUsize::new(0));

        // Clone Arc for use in handler
        let connect_counter = connect_count.clone();
        let disconnect_counter = disconnect_count.clone();

        // Register handler
        manager.register_event_handler(move |event| {
            match event {
                LifecycleEvent::ClientConnected(_) => {
                    connect_counter.fetch_add(1, Ordering::SeqCst);
                }
                LifecycleEvent::ClientDisconnected(_) => {
                    disconnect_counter.fetch_add(1, Ordering::SeqCst);
                }
            }
        });

        // Test events
        manager.register_client("test1").unwrap();
        manager.register_client("test2").unwrap();
        manager.unregister_client("test1").unwrap();

        // Verify counts
        assert_eq!(connect_count.load(Ordering::SeqCst), 2);
        assert_eq!(disconnect_count.load(Ordering::SeqCst), 1);
    }
}
