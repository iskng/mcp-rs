//! Route Handler trait
//!
//! This module defines the main handler trait that processes server messages
//! and routes them to the appropriate domain handlers.

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::protocol::{Error, JSONRPCMessage, Method};

/// Main handler trait for processing server messages
#[async_trait]
pub trait RouteHandler: Send + Sync {
    /// Process a JSON-RPC message from the server
    ///
    /// This method takes a raw JSON-RPC message and processes it,
    /// routing to the appropriate handler based on message type.
    async fn handle_message(&self, message: JSONRPCMessage) -> Result<(), Error>;

    /// Send a request to the server with typed parameters and response
    ///
    /// This method serializes the parameters, sends the request to the server,
    /// and deserializes the response into the requested type.
    async fn send_request<P, R>(&self, method: Method, params: P) -> Result<R, Error>
    where
        P: Serialize + Send + Sync + 'static,
        R: DeserializeOwned + Send + Sync + 'static;

    /// Send a notification to the server with typed parameters
    ///
    /// This method serializes the parameters and sends a notification
    /// to the server (a request without an ID that expects no response).
    async fn send_notification<P>(&self, method: Method, params: P) -> Result<(), Error>
    where
        P: Serialize + Send + Sync;
}
