//! Transport module for MCP communication
//!
//! This module defines the Transport trait and various implementations,
//! such as SSE and STDIO transports.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;

use crate::errors::Error;
use crate::types::protocol::Message;

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
        message: &Message
    ) -> Result<Option<Message>, Error>;
}

pub mod sse_server;
pub mod stdio;
pub mod message_handler;
pub mod connection_manager;

pub use message_handler::ServerMessageHandler;

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

/// Handle for a running server, used for shutdown
pub struct ServerHandle {
    sse_transport: Option<sse_server::SseServerTransport>,
    stdio_transport: Option<stdio::StdioTransport>,
    sse_transport_arc: Option<Arc<Mutex<sse_server::SseServerTransport>>>,
    stdio_transport_arc: Option<Arc<Mutex<stdio::StdioTransport>>>,
    join_handle: Option<JoinHandle<Result<(), Error>>>,
}

impl ServerHandle {
    /// Create a new server handle for SSE transport
    pub fn new_sse(transport: sse_server::SseServerTransport) -> Self {
        Self {
            sse_transport: Some(transport),
            stdio_transport: None,
            sse_transport_arc: None,
            stdio_transport_arc: None,
            join_handle: None,
        }
    }

    /// Create a new server handle for stdio transport
    pub fn new_stdio(transport: stdio::StdioTransport) -> Self {
        Self {
            sse_transport: None,
            stdio_transport: Some(transport),
            sse_transport_arc: None,
            stdio_transport_arc: None,
            join_handle: None,
        }
    }

    /// Create a new server handle for Arc-wrapped SSE transport
    pub fn new_sse_arc(transport: Arc<Mutex<sse_server::SseServerTransport>>) -> Self {
        Self {
            sse_transport: None,
            stdio_transport: None,
            sse_transport_arc: Some(transport),
            stdio_transport_arc: None,
            join_handle: None,
        }
    }

    /// Create a new server handle for Arc-wrapped stdio transport
    pub fn new_stdio_arc(transport: Arc<Mutex<stdio::StdioTransport>>) -> Self {
        Self {
            sse_transport: None,
            stdio_transport: None,
            sse_transport_arc: None,
            stdio_transport_arc: Some(transport),
            join_handle: None,
        }
    }

    /// Create a new server handle for SSE transport with a join handle
    pub fn new_sse_with_handle(
        transport: sse_server::SseServerTransport,
        handle: JoinHandle<Result<(), Error>>
    ) -> Self {
        Self {
            sse_transport: Some(transport),
            stdio_transport: None,
            sse_transport_arc: None,
            stdio_transport_arc: None,
            join_handle: Some(handle),
        }
    }

    /// Shutdown the server
    pub async fn shutdown(self) -> Result<(), Error> {
        // First, close any direct transport instances
        if let Some(mut transport) = self.sse_transport {
            let _ = transport.close().await;
        }

        if let Some(mut transport) = self.stdio_transport {
            let _ = transport.close().await;
        }

        // Next, close any Arc-wrapped transports
        if let Some(transport_arc) = self.sse_transport_arc {
            let mut transport = transport_arc.lock().await;
            let _ = transport.close().await;
        }

        if let Some(transport_arc) = self.stdio_transport_arc {
            let mut transport = transport_arc.lock().await;
            let _ = transport.close().await;
        }

        // Finally, handle the join handle if present
        if let Some(handle) = self.join_handle {
            // Try graceful shutdown first
            if let Err(e) = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await {
                tracing::warn!("Server task did not complete within timeout: {}", e);
                // Handle was moved into the timeout, we don't need to abort
            }
        }

        Ok(())
    }
}

/// Transport trait for different communication channels
#[async_trait]
pub trait Transport: Send + Sync {
    /// Receive a message from the transport
    async fn receive(&mut self) -> Result<(Option<String>, Message), Error>;

    /// Send a message to the transport
    async fn send(&mut self, message: &Message) -> Result<(), Error>;

    /// Send a message to a specific client
    async fn send_to(&mut self, client_id: &str, message: &Message) -> Result<(), Error>;

    /// Check if the transport is connected
    async fn is_connected(&self) -> bool;

    /// Close the transport
    async fn close(&mut self) -> Result<(), Error>;

    /// Start the transport
    async fn start(&mut self) -> Result<(), Error>;
}
