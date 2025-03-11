// Transport state module
//
// This module provides a shared state mechanism for transport implementations
// to track their connection status without lock contention.

use tokio::sync::watch;
use tracing::debug;

/// Represents the current state of a transport
#[derive(Clone, Debug, PartialEq)]
pub struct TransportState {
    /// Whether the transport has received the endpoint event
    pub has_endpoint: bool,
    /// Whether the transport has received the connected event
    pub has_connected: bool,
    /// Session ID assigned by the server
    pub session_id: Option<String>,
    /// Endpoint URL for sending messages
    pub endpoint_url: Option<String>,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            has_endpoint: false,
            has_connected: false,
            session_id: None,
            endpoint_url: None,
        }
    }
}

impl TransportState {
    /// Create a new disconnected state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the transport is fully connected
    pub fn is_connected(&self) -> bool {
        self.has_endpoint && self.has_connected
    }

    /// Reset the state to disconnected
    pub fn reset(&mut self) {
        self.has_endpoint = false;
        self.has_connected = false;
        // We intentionally don't clear session_id and endpoint_url
        // as they might be needed for reconnection attempts
    }

    /// Clear all state including session and endpoint
    pub fn clear(&mut self) {
        self.has_endpoint = false;
        self.has_connected = false;
        self.session_id = None;
        self.endpoint_url = None;
    }
}

/// Channel for watching and updating transport state
pub struct TransportStateChannel {
    /// Sender for updating the state
    tx: watch::Sender<TransportState>,
    /// Receiver for watching the state
    rx: watch::Receiver<TransportState>,
}

impl TransportStateChannel {
    /// Create a new transport state channel
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(TransportState::default());
        Self { tx, rx }
    }

    /// Get a receiver that can be cloned and shared
    pub fn receiver(&self) -> watch::Receiver<TransportState> {
        self.rx.clone()
    }

    /// Update the state with a modifier function
    pub fn update<F>(&self, f: F) where F: FnOnce(&mut TransportState) {
        let mut state = self.tx.borrow().clone();
        f(&mut state);
        if let Err(e) = self.tx.send(state.clone()) {
            debug!("Failed to update transport state: {}", e);
        }
    }

    /// Get the current state
    pub fn current(&self) -> TransportState {
        self.tx.borrow().clone()
    }

    /// Check if the transport is connected
    pub fn is_connected(&self) -> bool {
        self.tx.borrow().is_connected()
    }

    /// Get a watcher that can be used to detect state changes
    pub async fn changed(&mut self) -> Result<(), tokio::sync::watch::error::RecvError> {
        self.rx.changed().await
    }

    /// Reset the state to disconnected
    pub fn reset(&self) {
        self.update(|state| state.reset());
    }

    /// Clear all state
    pub fn clear(&self) {
        self.update(|state| state.clear());
    }
}

impl Default for TransportStateChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TransportStateChannel {
    fn clone(&self) -> Self {
        // Create a new receiver from the same sender
        // This ensures all receivers get updates from the same source
        Self {
            tx: self.tx.clone(), // Clone the sender - Arc under the hood
            rx: self.tx.subscribe(), // Create a new receiver from the same sender
        }
    }
}
