use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::protocol::{Error, JSONRPCMessage};

use super::server::AppState;

pub mod middleware;
pub mod sse;
pub mod stdio;
pub mod websocket;

/// Factory for creating transports
#[derive(Clone)]
pub enum TransportType {
    /// Server-Sent Events (SSE) transport
    Sse(sse::SseServerOptions),
    /// Standard input/output transport
    Stdio,
}

impl TransportType {
    /// Create an SSE transport with default options on the specified address
    pub fn sse(address: &str) -> Self {
        let options = sse::SseServerOptions {
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
