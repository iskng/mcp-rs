//! Handshake Handler
//!
//! This module provides handlers for core protocol operations like
//! initialization, ping, and shutdown.

use async_trait::async_trait;
use std::sync::Arc;
use log::error;

use crate::client::client::Client;
use crate::client::services::ServiceProvider;
use crate::protocol::{
    ClientCapabilities,
    Error,
    Implementation,
    InitializeParams,
    InitializeResult,
    JSONRPCMessage,
    JSONRPCNotification,
    Method,
    PROTOCOL_VERSION,
    InitializedNotification,
    ServerCapabilities,
};
use crate::client::services::lifecycle::LifecycleState;

/// Handler trait for core protocol operations
#[async_trait]
pub trait HandshakeHandler: Send + Sync {
    /// Initialize the client
    async fn initialize(&self) -> Result<InitializeResult, Error>;

    /// Send an initialized notification
    async fn send_initialized(&self) -> Result<(), Error>;

    /// Check if the server has a capability
    async fn has_capability(&self, capability: &str) -> bool;

    /// Shutdown the client
    async fn shutdown(&self) -> Result<(), Error>;
}

/// Default implementation of the handshake handler
pub struct DefaultHandshakeHandler {
    /// The underlying client
    client: Arc<Client>,

    /// Service provider for accessing services
    service_provider: Arc<ServiceProvider>,
}

impl DefaultHandshakeHandler {
    /// Create a new handshake handler
    pub fn new(client: Arc<Client>, service_provider: Arc<ServiceProvider>) -> Self {
        Self {
            client,
            service_provider,
        }
    }
}

// Function to compare protocol versions
fn is_compatible_version(client_version: &str, server_version: &str) -> bool {
    // For simplicity, we consider versions compatible if they are exactly the same
    // In a more sophisticated implementation, we might handle major.minor.patch versioning
    // or have a list of compatible versions
    client_version == server_version
}

#[async_trait]
impl HandshakeHandler for DefaultHandshakeHandler {
    async fn initialize(&self) -> Result<InitializeResult, Error> {
        let lifecycle = self.service_provider.lifecycle_manager();

        // Can only initialize once
        if lifecycle.current_state().await != LifecycleState::Initialization {
            return Err(Error::Lifecycle("Client is already initialized".to_string()));
        }

        // Create initialize parameters
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            client_info: Implementation {
                name: "mcp-rs".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: ClientCapabilities {
                sampling: None,
                roots: None,
                experimental: None,
            },
        };

        // Send the initialize request
        let result: InitializeResult = self.client.send_request(Method::Initialize, params).await?;

        // Verify protocol version compatibility
        if !is_compatible_version(&PROTOCOL_VERSION, &result.protocol_version) {
            // Log the incompatibility
            error!(
                "Protocol version mismatch: client={}, server={}",
                PROTOCOL_VERSION,
                result.protocol_version
            );

            // Return a protocol error
            return Err(
                Error::Protocol(
                    format!(
                        "Incompatible protocol version: client supports {}, server requires {}",
                        PROTOCOL_VERSION,
                        result.protocol_version
                    )
                )
            );
        }

        // Store the result for future reference
        lifecycle.set_server_info(result.clone()).await?;

        Ok(result)
    }

    async fn send_initialized(&self) -> Result<(), Error> {
        // Get the lifecycle manager
        let lifecycle = self.service_provider.lifecycle_manager();

        // Create the initialized notification
        let notification = JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: Method::NotificationsInitialized,
            params: None,
        };
        let message = JSONRPCMessage::Notification(notification);

        // Send the notification
        self.client.send_raw_message(message).await?;

        // Update the lifecycle state
        lifecycle.transition_to(
            crate::client::services::lifecycle::LifecycleState::Operation
        ).await?;

        Ok(())
    }

    async fn has_capability(&self, capability: &str) -> bool {
        // Get the lifecycle manager
        let lifecycle = self.service_provider.lifecycle_manager();

        // Check the capability
        lifecycle.has_capability(capability).await
    }

    async fn shutdown(&self) -> Result<(), Error> {
        // Get the lifecycle manager
        let lifecycle = self.service_provider.lifecycle_manager();

        // Update the lifecycle state
        lifecycle.transition_to(
            crate::client::services::lifecycle::LifecycleState::Shutdown
        ).await?;

        // Send the shutdown request
        // let _: serde_json::Value = self.client.send_request(Method::Close, ()).await?;

        // Shutdown the client
        self.client.shutdown().await
    }
}
