//! MCP Client Implementation
//!
//! This module provides a Rust client for the Model Control Protocol (MCP).
//! It implements the protocol specification and provides a high-level API for
//! interacting with MCP servers.

// Export submodules
pub mod client;
pub mod clientsession;
pub mod lifecycle;
pub mod notification;
pub mod request;
pub mod subscription;
pub mod progress;
pub mod domain;
pub mod utils;
pub mod model;

// Re-export key types for easier access
pub use client::{ Client, ClientBuilder, ClientConfig };
pub use clientsession::{
    ClientSession,
    ClientSessionBuilder,
    ClientSessionConfig,
    ClientSessionGuard,
};
pub use lifecycle::{ LifecycleManager, LifecycleState };
pub use notification::NotificationRouter;
pub use request::RequestManager;
pub use subscription::{ Subscription, SubscriptionManager };
pub use progress::{ ProgressInfo, ProgressStatus, ProgressTracker };
pub use model::ClientInfo;

// Re-export domain traits for easier usage
pub use domain::resources::ResourceOperations;
pub use domain::tools::ToolOperations;

use crate::transport::{ DirectIOTransport, BoxedDirectIOTransport };
use crate::protocol::Error;

/// Connect to an MCP server using the provided transport
pub async fn connect<T: DirectIOTransport + 'static>(
    transport: T,
    client_name: Option<String>,
    client_version: Option<String>
) -> Result<ClientSession, Error> {
    // Box the transport
    let boxed_transport = Box::new(transport);

    // Create a builder with the transport
    let boxed_transport = BoxedDirectIOTransport(boxed_transport);
    let mut builder = ClientSessionBuilder::new(boxed_transport);

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
#[cfg(feature = "stdio")]
pub async fn connect_stdio(
    command: &str,
    args: Vec<&str>,
    client_name: Option<String>,
    client_version: Option<String>
) -> Result<ClientSession, Error> {
    use crate::transport::stdio::StdioTransport;

    // Create stdio transport
    let transport = StdioTransport::new(command, args).await?;

    // Connect using the transport
    connect(transport, client_name, client_version).await
}

/// Connect to an MCP server using SSE transport
#[cfg(feature = "sse")]
pub async fn connect_sse(
    url: &str,
    headers: Option<std::collections::HashMap<String, String>>,
    client_name: Option<String>,
    client_version: Option<String>
) -> Result<ClientSession, Error> {
    use crate::transport::sse::SSETransport;

    // Create SSE transport
    let transport = SSETransport::new(url, headers).await?;

    // Connect using the transport
    connect(transport, client_name, client_version).await
}

/// Connect to an MCP server using WebSocket transport
#[cfg(feature = "websocket")]
pub async fn connect_websocket(
    url: &str,
    headers: Option<std::collections::HashMap<String, String>>,
    client_name: Option<String>,
    client_version: Option<String>
) -> Result<ClientSession, Error> {
    use crate::transport::websocket::WebSocketTransport;

    // Create WebSocket transport
    let transport = WebSocketTransport::new(url, headers).await?;

    // Connect using the transport
    connect(transport, client_name, client_version).await
}

#[cfg(test)]
pub mod tests;
