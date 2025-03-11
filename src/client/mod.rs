//! MCP Client Implementation
//!
//! This module provides a Rust client for the Model Control Protocol (MCP).
//! It implements the protocol specification and provides a high-level API for
//! interacting with MCP servers.

// Export submodules
pub mod client;
pub mod clientsession;
pub mod handlers;
pub mod model;
pub mod services;
pub mod transport;
pub mod utils;

// Re-export key types for easier access
pub use client::{ Client, ClientBuilder, ClientConfig };
pub use clientsession::{
    ClientSession,
    ClientSessionBuilder,
    ClientSessionConfig,
    ClientSessionGuard,
};
pub use model::ClientInfo;
pub use services::{
    lifecycle::{ LifecycleManager, LifecycleState },
    notification::NotificationRouter,
    progress::{ ProgressInfo, ProgressStatus, ProgressTracker },
    request::RequestManager,
    subscription::{ Subscription, SubscriptionManager },
};

use crate::client::transport::{ Transport, sse::SseTransport };
use crate::protocol::Error;

/// Connect to an MCP server using the provided transport
pub async fn connect(
    transport: Box<dyn Transport + 'static>,
    client_name: Option<String>,
    client_version: Option<String>
) -> Result<ClientSession, Error> {
    // Create a builder with the transport
    let mut builder = ClientSessionBuilder::new(transport);

    // Set client info if provided
    if let Some(name) = client_name {
        builder = builder.name(name);
    }

    if let Some(version) = client_version {
        builder = builder.version(version);
    }

    // Build the session
    let session = builder.build();

    // Initialize the session
    session.initialize().await?;

    Ok(session)
}

/// Connect to an MCP server using stdio transport
// pub async fn connect_stdio(
//     client_name: Option<String>,
//     client_version: Option<String>
// ) -> Result<ClientSession, Error> {
//     use crate::client::transport::stdio::StdioTransport;

//     // Create stdio transport
//     let transport = StdioTransport::new();

//     // Connect using the transport
//     connect(transport, client_name, client_version).await
// }

/// Connect to an MCP server using SSE transport
pub async fn connect_sse(
    url: &str,
    headers: Option<std::collections::HashMap<String, String>>,
    client_name: Option<String>,
    client_version: Option<String>
) -> Result<ClientSession, Error> {
    // Create SSE transport
    let transport = SseTransport::new(url).await?;

    // Connect using the transport
    connect(Box::new(transport), client_name, client_version).await
}

/// Connect to an MCP server using WebSocket transport
// pub async fn connect_websocket(
//     url: &str,
//     headers: Option<std::collections::HashMap<String, String>>,
//     client_name: Option<String>,
//     client_version: Option<String>
// ) -> Result<ClientSession, Error> {
//     use crate::client::transport::websocket::WebSocketTransport;

//     // Create WebSocket transport
//     let transport = WebSocketTransport::new(url, headers).await?;

//     // Connect using the transport
//     connect(transport, client_name, client_version).await
// }

#[cfg(test)]
pub mod tests;
