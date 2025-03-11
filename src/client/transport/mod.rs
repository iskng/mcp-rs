//! Transport module for MCP communication
//!
//! This module defines the Transport trait and various implementations,
//! such as SSE and STDIO transports.
pub mod sse;
pub mod state;
pub mod websocket;
pub mod stdio;

use crate::protocol::JSONRPCMessage;
use crate::protocol::errors::Error;
use crate::client::transport::state::TransportState;
use async_trait::async_trait;
use tokio::sync::{ broadcast, watch };

/// Simple connection status for transports
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

/// Transport trait for different communication channels
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport
    async fn start(&self) -> Result<(), Error>;

    /// Close the transport
    async fn close(&self) -> Result<(), Error>;

    /// Get a receiver for status updates
    fn subscribe_status(&self) -> broadcast::Receiver<ConnectionStatus>;

    /// Get current connection status (non-blocking)
    fn is_connected(&self) -> bool;

    /// Get a receiver for transport state updates
    /// This allows components to observe transport state without locking
    fn subscribe_state(&self) -> watch::Receiver<TransportState>;

    /// Send a message to the server
    async fn send(&self, message: &JSONRPCMessage) -> Result<(), Error>;

    /// Receive a message from the server
    /// Returns a tuple of (session_id, message)
    async fn receive(&self) -> Result<(Option<String>, JSONRPCMessage), Error>;
}
