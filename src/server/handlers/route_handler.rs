//! Server handler trait
//!
//! This module defines the main handler trait that processes client messages
//! and dispatches them to the appropriate domain handlers.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::protocol::Error;
use crate::protocol::{JSONRPCMessage, Message, RequestId};
use crate::server::services::ServiceProvider;
use crate::server::transport::middleware::ClientSession;

/// Main handler trait for processing client messages
#[async_trait]
pub trait RouteHandler: Send + Sync {
    /// Process a JSON-RPC message from a client
    ///
    /// This method takes a raw JSON-RPC message and a client session,
    /// and returns an optional response message or an error.
    async fn handle_message(
        &self,
        message: JSONRPCMessage,
        session: &ClientSession,
    ) -> Result<Option<JSONRPCMessage>, Error> {
        // Extract client ID from session
        let client_id = session.client_id.as_deref();

        // Log the incoming message
        if let Some(id) = client_id {
            tracing::debug!("Processing message from client {}: {:?}", id, &message);
        } else {
            tracing::debug!("Processing message from anonymous client: {:?}", &message);
        }

        // Extract the request ID before moving the message
        let request_id = message
            .id()
            .cloned()
            .unwrap_or_else(|| RequestId::Number(0));

        info!("Processing message with request ID: {:?}", request_id);
        // Convert to typed message for processing
        match message.into_message() {
            Ok(typed_message) => {
                tracing::debug!("Converted to typed message: {:?}", typed_message);
                // Process the typed message
                match self
                    .handle_typed_message(request_id, client_id, &typed_message)
                    .await
                {
                    Ok(response) => {
                        tracing::debug!("Got response from handler: {:?}", response);
                        Ok(response)
                    }
                    Err(e) => {
                        tracing::error!("Error in handle_typed_message: {}", e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                // Log the error
                tracing::error!("Error converting message to typed format: {}", e);

                // Create error response with the preserved ID
                let response = crate::protocol::to_error_message(request_id, &e.into());
                tracing::debug!("Created error response: {:?}", response);

                Ok(Some(response))
            }
        }
    }

    /// Process a typed protocol message
    ///
    /// This method takes a typed protocol message and optional client ID,
    /// and returns an optional response message or an error.
    ///
    /// Implementations should pattern match on the message type and delegate
    /// to domain-specific handlers.
    async fn handle_typed_message(
        &self,
        request_id: RequestId,
        client_id: Option<&str>,
        message: &Message,
    ) -> Result<Option<JSONRPCMessage>, Error>;

    /// Get a reference to the service provider, if supported by this handler
    /// Returns None if the handler doesn't use a service provider
    fn service_provider(&self) -> Arc<ServiceProvider>;
}
