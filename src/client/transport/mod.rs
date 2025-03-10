//! Transport module for MCP communication
//!
//! This module defines the Transport trait and various implementations,
//! such as SSE and STDIO transports.
pub mod sse;
pub mod websocket;

use crate::protocol::JSONRPCMessage;
use crate::protocol::errors::Error;
use crate::server::server::AppState;
use async_trait::async_trait;
use std::sync::Arc;

/// Transport trait for different communication channels
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport
    async fn start(&mut self) -> Result<(), Error>;

    /// Close the transport
    async fn close(&mut self) -> Result<(), Error>;

    /// Check if the transport is connected
    async fn is_connected(&self) -> bool;

    /// Send to specific client
    async fn send_to(&mut self, client_id: &str, message: &JSONRPCMessage) -> Result<(), Error>;

    /// Set the app state
    async fn set_app_state(&mut self, app_state: Arc<AppState>);
}

/// Trait for transports that support direct sending and receiving of messages
#[async_trait]
pub trait DirectIOTransport: Transport {
    /// Receive a message from the transport
    async fn receive(&mut self) -> Result<(Option<String>, JSONRPCMessage), Error>;

    /// Send a message to the transport
    async fn send(&mut self, message: &JSONRPCMessage) -> Result<(), Error>;
}

/// BoxedDirectIOTransport is a wrapper around Box<dyn DirectIOTransport>
/// that implements DirectIOTransport
pub struct BoxedDirectIOTransport(pub Box<dyn DirectIOTransport + 'static>);

#[async_trait]
impl Transport for BoxedDirectIOTransport {
    async fn start(&mut self) -> Result<(), Error> {
        self.0.start().await
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.0.close().await
    }

    async fn is_connected(&self) -> bool {
        self.0.is_connected().await
    }

    async fn send_to(&mut self, client_id: &str, message: &JSONRPCMessage) -> Result<(), Error> {
        self.0.send_to(client_id, message).await
    }

    async fn set_app_state(&mut self, app_state: Arc<AppState>) {
        self.0.set_app_state(app_state).await
    }
}

#[async_trait]
impl DirectIOTransport for BoxedDirectIOTransport {
    async fn receive(&mut self) -> Result<(Option<String>, JSONRPCMessage), Error> {
        self.0.receive().await
    }

    async fn send(&mut self, message: &JSONRPCMessage) -> Result<(), Error> {
        self.0.send(message).await
    }
}
