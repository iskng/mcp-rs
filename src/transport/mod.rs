//! MCP Transport Layer
//!
//! This module defines the transport abstraction for the MCP library, allowing
//! different communication channels (e.g., STDIO, SSE) to be used interchangeably.
//! It includes the `Transport` trait and utilities for managing message transmission.

use tokio::sync::{ mpsc, oneshot };
use async_trait::async_trait;

use crate::{ errors::Error, messages::Message };

pub mod sse;
pub mod sse_server;
pub mod stdio;
pub mod websocket;
pub mod websocket_server;

/// The Transport trait defines the interface for sending and receiving MCP messages
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport - this must be called before using the transport
    async fn start(&mut self) -> Result<(), Error>;

    /// Receive a message from the transport
    ///
    /// Returns a tuple containing:
    /// - Option<String>: The client ID (if applicable)
    /// - Message: The received message
    ///
    /// For client-side transports, the client ID will typically be None.
    /// For server-side transports, the client ID identifies which client sent the message.
    async fn receive(&mut self) -> Result<(Option<String>, Message), Error>;

    /// Send a message via the transport
    /// Note: For server implementations, this acts as a broadcast to all clients
    async fn send(&mut self, message: &Message) -> Result<(), Error>;

    /// Send a message to a specific client
    /// This is used by server implementations to respond to a specific client
    /// For client implementations, this is equivalent to `send`
    async fn send_to(&mut self, client_id: &str, message: &Message) -> Result<(), Error>;

    /// Check if the transport is connected
    async fn is_connected(&self) -> bool;

    /// Close the transport
    async fn close(&mut self) -> Result<(), Error>;
}
