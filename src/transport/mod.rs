//! Transport module for MCP communication
//!
//! This module defines the Transport trait and various implementations,
//! such as SSE and STDIO transports.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::protocol::errors::Error;
use crate::protocol::JSONRPCMessage;
use crate::server::handlers::RouteHandler;
use crate::server::server::AppState;

/// Handler for messages from the transport
#[async_trait]
pub trait TransportMessageHandler: Send + Sync {
    /// Handle a message and return an optional response
    ///
    /// # Arguments
    /// * `client_id` - The ID of the client sending the message
    /// * `message` - The message to handle
    ///
    /// # Returns
    /// * `Ok(Some(response))` - A response message to send back to the client
    /// * `Ok(None)` - No response needed
    /// * `Err(error)` - An error occurred while processing the message
    async fn handle_message(
        &self,
        client_id: &str,
        message: &JSONRPCMessage
    ) -> Result<Option<JSONRPCMessage>, Error>;
}

pub mod middleware;
pub mod sse_server;
pub mod stdio;

/// Factory for creating transports
#[derive(Clone)]
pub enum TransportType {
    /// Server-Sent Events (SSE) transport
    Sse(sse_server::SseServerOptions),
    /// Standard input/output transport
    Stdio,
}

impl TransportType {
    /// Create an SSE transport with default options on the specified address
    pub fn sse(address: &str) -> Self {
        let options = sse_server::SseServerOptions {
            bind_address: address.to_string(),
            auth_token: None,
            connection_timeout: Duration::from_secs(120),
            keep_alive_interval: 30,
            allowed_origins: None,
            require_auth: false,
            message_tx: None,
        };

        TransportType::Sse(options)
    }
}

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
