//! Handshake handler for the server
//!
//! This module contains the handshake handler trait and implementation for handling
//! core protocol operations like ping and notifications.

use async_trait::async_trait;
use std::sync::Arc;

use crate::protocol::{
    CancelledNotification,
    InitializedNotification,
    PingRequest,
    ProgressNotification,
    errors::Error,
};
use crate::server::services::ServiceProvider;
use crate::transport::middleware::ClientSession;

/// Result of ping operation - response to ping request
#[derive(serde::Serialize)]
pub struct PingResult {
    /// Content of the ping response
    pub content: String,
    /// Optional metadata
    pub _meta: Option<serde_json::Value>,
}

/// Handshake handler trait for core protocol operations
#[async_trait]
pub trait HandshakeHandler: Send + Sync {
    /// Handle ping request
    async fn handle_ping(
        &self,
        request: &PingRequest,
        session: &ClientSession
    ) -> Result<PingResult, Error>;

    /// Handle initialized notification
    async fn handle_initialized(
        &self,
        notification: &InitializedNotification,
        session: &ClientSession
    ) -> Result<(), Error>;

    /// Handle cancelled notification
    async fn handle_cancelled(
        &self,
        notification: &CancelledNotification,
        session: &ClientSession
    ) -> Result<(), Error>;

    /// Handle progress notification
    async fn handle_progress(
        &self,
        notification: &ProgressNotification,
        session: &ClientSession
    ) -> Result<(), Error>;
}

/// Default implementation of the handshake handler
pub struct DefaultHandshakeHandler {
    /// Service provider
    service_provider: Arc<ServiceProvider>,
}

impl DefaultHandshakeHandler {
    /// Create a new handshake handler
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        Self { service_provider }
    }
}

#[async_trait]
impl HandshakeHandler for DefaultHandshakeHandler {
    async fn handle_ping(
        &self,
        request: &PingRequest,
        session: &ClientSession
    ) -> Result<PingResult, Error> {
        // Log the ping
        if let Some(id) = &session.client_id {
            tracing::debug!("Ping from client {}", id);
        } else {
            tracing::debug!("Ping from unknown client");
        }

        // Prepare content based on request params
        let content = if
            let Some(content) = request.params
                .as_ref()
                .and_then(|p| p.get("content"))
                .and_then(|v| v.as_str())
        {
            content.to_string()
        } else {
            "pong".to_string()
        };

        // Return ping response
        Ok(PingResult {
            content,
            _meta: None,
        })
    }

    async fn handle_initialized(
        &self,
        notification: &InitializedNotification,
        session: &ClientSession
    ) -> Result<(), Error> {
        // Log the initialization
        if let Some(id) = &session.client_id {
            tracing::info!("Client {} initialized", id);
        } else {
            tracing::info!("Unknown client initialized");
        }

        Ok(())
    }

    async fn handle_cancelled(
        &self,
        notification: &CancelledNotification,
        session: &ClientSession
    ) -> Result<(), Error> {
        // Get the ID from params
        let id = &notification.params.request_id;

        // Log the cancellation
        if let Some(client_id) = &session.client_id {
            tracing::info!("Client {} cancelled request {:?}", client_id, id);
        } else {
            tracing::info!("Unknown client cancelled request {:?}", id);
        }

        // Currently we don't do anything with cancellations
        // In the future, we could implement a request registry to track and cancel ongoing operations

        Ok(())
    }

    async fn handle_progress(
        &self,
        notification: &ProgressNotification,
        session: &ClientSession
    ) -> Result<(), Error> {
        // Get the progress info from params
        let params = &notification.params;

        // Log the progress
        if let Some(client_id) = &session.client_id {
            tracing::debug!(
                "Progress from client {}: token={:?} progress={}%",
                client_id,
                params.progress_token,
                params.progress * 100.0
            );
        } else {
            tracing::debug!(
                "Progress from unknown client: token={:?} progress={}%",
                params.progress_token,
                params.progress * 100.0
            );
        }

        Ok(())
    }
}
