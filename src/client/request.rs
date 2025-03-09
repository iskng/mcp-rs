//! MCP Client Request Management
//!
//! This module implements the request tracking and correlation system for the MCP client.
//! It generates request IDs, tracks pending requests, and matches responses to their
//! corresponding requests.

use std::collections::HashMap;
use std::sync::{ Arc, atomic::{ AtomicI32, Ordering } };
use std::time::Duration;
use tokio::sync::{ Mutex, oneshot };
use tokio::time::timeout;
use tracing::{ debug, error, warn };

use crate::protocol::{
    Error,
    JSONRPCError,
    JSONRPCMessage,
    JSONRPCRequest,
    JSONRPCResponse,
    RequestId,
};

/// Manager for handling requests and correlating them with responses
pub struct RequestManager {
    /// Counter for generating unique request IDs
    request_id_counter: AtomicI32,

    /// Map of pending requests by their ID
    pending_requests: Mutex<HashMap<RequestId, oneshot::Sender<Result<JSONRPCResponse, Error>>>>,

    /// Default timeout for requests
    default_timeout: Duration,
}

impl RequestManager {
    /// Create a new request manager
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            request_id_counter: AtomicI32::new(1),
            pending_requests: Mutex::new(HashMap::new()),
            default_timeout,
        }
    }

    /// Generate a new unique request ID
    pub fn generate_id(&self) -> RequestId {
        let id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        RequestId::Number(id as i64)
    }

    /// Create a request and register it
    pub async fn create_request<P>(
        &self,
        method: String,
        params: Option<P>
    ) -> Result<
            (JSONRPCRequest, RequestId, oneshot::Receiver<Result<JSONRPCResponse, Error>>),
            Error
        >
        where P: serde::Serialize
    {
        // Generate a new request ID
        let request_id = self.generate_id();

        // Serialize parameters if provided
        let params_value = match params {
            Some(p) => {
                Some(
                    serde_json
                        ::to_value(p)
                        .map_err(|e| {
                            Error::Other(format!("Failed to serialize params: {}", e))
                        })?
                )
            }
            None => None,
        };

        // Create the JSON-RPC request
        let request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id.clone(),
            method,
            params: params_value,
        };

        // Register the request
        let rx = self.register_request(request_id.clone()).await;

        Ok((request, request_id, rx))
    }

    /// Register a pending request and get a future for its response
    pub async fn register_request(
        &self,
        id: RequestId
    ) -> oneshot::Receiver<Result<JSONRPCResponse, Error>> {
        let (tx, rx) = oneshot::channel();

        // Store the sender in the pending requests map
        let mut pending = self.pending_requests.lock().await;
        pending.insert(id, tx);

        rx
    }

    /// Complete a pending request with a response
    pub async fn complete_request(
        &self,
        id: &RequestId,
        response: Result<JSONRPCResponse, Error>
    ) -> bool {
        let mut pending = self.pending_requests.lock().await;

        if let Some(sender) = pending.remove(id) {
            // Send the response through the channel
            if sender.send(response).is_err() {
                warn!("Failed to send response for request {:?} - receiver dropped", id);
                return false;
            }
            true
        } else {
            warn!("No pending request found for id {:?}", id);
            false
        }
    }

    /// Handle a response message by matching it to a pending request
    pub async fn handle_response(&self, response: JSONRPCResponse) -> Result<(), Error> {
        let id = &response.id;

        // Check if we have this ID in our pending requests
        if self.complete_request(id, Ok(response.clone())).await {
            debug!("Completed request {:?}", id);
            Ok(())
        } else {
            // If we didn't have this ID, it might be a response to a request that timed out
            warn!("Received response for unknown request ID: {:?}", id);
            Ok(())
        }
    }

    /// Handle an error message by matching it to a pending request
    pub async fn handle_error(&self, error: JSONRPCError) -> Result<(), Error> {
        // Find the request by ID and complete it with an error
        let err = Error::Protocol(format!("JSON-RPC Error: {}", error.error.message));

        // Complete the request with an error
        if self.complete_request(&error.id, Err(err.clone())).await {
            debug!("Completed request {:?} with error", error.id);
            Ok(())
        } else {
            warn!("Received error for unknown request ID: {:?}", error.id);
            Ok(())
        }
    }

    /// Send a request and wait for its response
    pub async fn send_request<F, R>(
        &self,
        send_fn: F,
        method: String,
        params: Option<serde_json::Value>,
        timeout_duration: Option<Duration>
    )
        -> Result<R, Error>
        where
            F: FnOnce(JSONRPCMessage) -> futures::future::BoxFuture<'static, Result<(), Error>>,
            R: serde::de::DeserializeOwned
    {
        // 1. Create the request
        let (request, request_id) = {
            // Create synchronously - no await involved
            let id = self.generate_id();

            // Create the JSON-RPC request
            let request = JSONRPCRequest {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                method: method.clone(),
                params,
            };

            (request, id)
        };

        // 2. Register for the response
        let response_rx = self.register_request(request_id.clone()).await;

        // 3. Send the request
        let json_rpc_request = JSONRPCMessage::Request(request);
        send_fn(json_rpc_request).await?;

        // 4. Wait for response with timeout
        let timeout_duration = timeout_duration.unwrap_or(self.default_timeout);

        let response = (match timeout(timeout_duration, response_rx).await {
            Ok(result) => {
                match result {
                    Ok(response) => response,
                    Err(_) => {
                        // Channel closed without a response
                        return Err(
                            Error::Transport("Response channel closed unexpectedly".to_string())
                        );
                    }
                }
            }
            Err(_) => {
                // Timeout occurred, remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);

                return Err(
                    Error::Timeout(format!("Request timed out after {:?}", timeout_duration))
                );
            }
        })?;

        // Convert the response result to the expected type
        match response.result.content.get("result") {
            Some(value) => {
                match serde_json::from_value(value.clone()) {
                    Ok(typed_result) => Ok(typed_result),
                    Err(e) => Err(Error::Other(format!("Failed to deserialize response: {}", e))),
                }
            }
            None => {
                // Try deserializing the whole result
                match serde_json::from_value(serde_json::to_value(response.result.content)?) {
                    Ok(typed_result) => Ok(typed_result),
                    Err(e) => Err(Error::Other(format!("Failed to deserialize response: {}", e))),
                }
            }
        }
    }

    /// Cancel all pending requests with an error
    pub async fn cancel_all_requests(&self, error_message: &str) {
        // Get all pending requests
        let mut pending = self.pending_requests.lock().await;

        // Create an error for all pending requests
        let err = Error::Other(error_message.to_string());

        // Complete all pending requests with the error
        for (id, sender) in pending.drain() {
            if sender.send(Err(err.clone())).is_err() {
                warn!("Failed to send cancellation for request {:?}", id);
            }
        }
    }
}

impl Default for RequestManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}
