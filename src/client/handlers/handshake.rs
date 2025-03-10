//! Handshake Handler
//!
//! This module provides handlers for core protocol operations like
//! initialization, ping, and shutdown.

use async_trait::async_trait;
use std::sync::Arc;

use crate::client::client::Client;
use crate::client::services::ServiceProvider;
use crate::client::transport::DirectIOTransport;
use crate::protocol::{
    ClientCapabilities, Error, Implementation, InitializeParams, InitializeResult, JSONRPCMessage,
    JSONRPCNotification, Method, PROTOCOL_VERSION,
};

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
pub struct DefaultHandshakeHandler<T: DirectIOTransport + 'static> {
    /// The underlying client
    client: Arc<Client<T>>,

    /// Service provider for accessing services
    service_provider: Arc<ServiceProvider>,
}

impl<T: DirectIOTransport + 'static> DefaultHandshakeHandler<T> {
    /// Create a new handshake handler
    pub fn new(client: Arc<Client<T>>, service_provider: Arc<ServiceProvider>) -> Self {
        Self {
            client,
            service_provider,
        }
    }
}

#[async_trait]
impl<T: DirectIOTransport + 'static> HandshakeHandler for DefaultHandshakeHandler<T> {
    async fn initialize(&self) -> Result<InitializeResult, Error> {
        // Get the lifecycle manager
        let lifecycle = self.service_provider.lifecycle_manager();

        // Create the initialize params
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

        // Update the lifecycle manager with the server info
        lifecycle.set_server_info(result.clone()).await?;

        // Send the initialized notification
        self.send_initialized().await?;

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
        lifecycle
            .transition_to(crate::client::services::lifecycle::LifecycleState::Ready)
            .await?;

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
        lifecycle
            .transition_to(crate::client::services::lifecycle::LifecycleState::ShuttingDown)
            .await?;

        // Send the shutdown request
        // let _: serde_json::Value = self.client.send_request(Method::Close, ()).await?;

        // Update the lifecycle state
        lifecycle
            .transition_to(crate::client::services::lifecycle::LifecycleState::Closed)
            .await?;

        // Shutdown the client
        self.client.shutdown().await
    }
}
