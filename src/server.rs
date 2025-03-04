//! MCP Server
//!
//! This module implements the MCP server, managing state, handler registration, and
//! request processing. It provides a flexible and type-safe way to define and register
//! handlers for different MCP methods, ensuring that requests are processed correctly.

use async_trait::async_trait;
use futures::future::BoxFuture;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::errors::Error;
use crate::lifecycle::LifecycleManager;
use crate::messages::{ GenericRequest, Message, Notification, Response, ResponseOutcome };
use crate::tools::process_manager::ToolOutputType;
use crate::tools::progress::ToolProgressTracker;
use crate::tools::tool_registry::ToolRegistry;
use crate::transport::Transport;
use crate::types::initialize::{
    Implementation,
    InitializeRequestParams,
    InitializeResult,
    ServerCapabilities,
};
use crate::types::tools::CallToolResult;
use crate::types::CallToolParams;
use crate::utils::validation::{ ValidationConfig, validate_request };
use serde::{ Serialize, de::DeserializeOwned };
use serde_json::Value;

/// Handler trait for MCP method requests
#[async_trait]
pub trait MethodHandler<State>: Send + Sync {
    /// Handle a method request with the given state and parameters
    async fn handle(&self, state: &State, params: &Value) -> Result<Value, Error>;
}

/// Handler trait for MCP notifications
#[async_trait]
pub trait NotificationHandler<State>: Send + Sync {
    /// Handle a notification with the given state and parameters
    async fn handle(&self, state: &State, params: &Value) -> Result<(), Error>;
}

/// Type-safe wrapper for method handlers
struct TypedHandler<F, P, R> {
    handler: F,
    _marker: std::marker::PhantomData<(P, R)>,
}

impl<F, P, R> TypedHandler<F, P, R> {
    fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<State, F, P, R> MethodHandler<State>
    for TypedHandler<F, P, R>
    where
        State: Send + Sync,
        F: Fn(&State, P) -> BoxFuture<'static, Result<R, Error>> + Send + Sync,
        P: DeserializeOwned + Send + Sync + 'static,
        R: Serialize + Send + Sync + 'static
{
    async fn handle(&self, state: &State, params: &Value) -> Result<Value, Error> {
        let params: P = serde_json
            ::from_value(params.clone())
            .map_err(|e| Error::InvalidParams(e.to_string()))?;
        let result = (self.handler)(state, params).await?;
        serde_json::to_value(result).map_err(Error::Json)
    }
}

/// Server for the Model Context Protocol
pub struct Server<S> {
    /// Application state
    state: S,

    /// Method handlers
    handlers: HashMap<String, Box<dyn MethodHandler<S>>>,

    /// Server capabilities
    capabilities: ServerCapabilities,

    /// Server implementation info
    server_info: Implementation,

    /// Validation configuration
    validation_config: ValidationConfig,

    /// Tool registry for managing and executing tools
    pub tool_registry: Arc<ToolRegistry>,

    /// Progress tracker for tools
    progress_tracker: Arc<ToolProgressTracker>,

    /// Lifecycle manager for managing server state
    lifecycle_manager: LifecycleManager,
}

impl<S> Server<S> where S: Clone + Send + Sync + 'static {
    /// Create a new server
    pub fn new(state: S) -> Self {
        Self {
            state,
            handlers: HashMap::new(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "MCP-rs Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            },
            validation_config: ValidationConfig::default(),
            tool_registry: Arc::new(ToolRegistry::new()),
            progress_tracker: Arc::new(ToolProgressTracker::new()),
            lifecycle_manager: LifecycleManager::new(),
        }
    }

    /// Configure validation settings
    pub fn with_validation_config(mut self, config: ValidationConfig) -> Self {
        self.validation_config = config;
        self
    }

    /// Enable/disable request validation
    pub fn validate_requests(mut self, validate: bool) -> Self {
        self.validation_config.validate_requests = validate;
        self
    }

    /// Enable/disable response validation
    pub fn validate_responses(mut self, validate: bool) -> Self {
        self.validation_config.validate_responses = validate;
        self
    }

    /// Register a method handler
    pub fn register_handler<P, R, F>(&mut self, method: &str, handler: F) -> &mut Self
        where
            P: DeserializeOwned + Send + Sync + 'static,
            R: Serialize + Send + Sync + 'static,
            F: Fn(&S, P) -> BoxFuture<'static, Result<R, Error>> + Send + Sync + 'static
    {
        let handler = Box::new(TypedHandler::new(handler));
        self.handlers.insert(method.to_string(), handler);
        self
    }

    /// Register initialize handler
    pub fn register_initialize_handler<F>(&mut self, handler: F) -> &mut Self
        where
            F: Fn(
                &S,
                InitializeRequestParams
            ) -> BoxFuture<'static, Result<InitializeResult, Error>> +
                Send +
                Sync +
                'static
    {
        self.register_handler("initialize", handler)
    }

    /// Register a new client with the lifecycle manager
    pub fn register_client(&self, client_id: &str) -> Result<(), Error> {
        self.lifecycle_manager.register_client(client_id)
    }

    /// Run the server with a transport
    pub async fn run<T>(&self, mut transport: T) -> Result<(), Error>
        where T: Transport, S: Send + Sync + 'static
    {
        // Start the lifecycle manager
        self.lifecycle_manager.start().await?;
        tracing::info!("Server lifecycle started");

        // Start the transport before entering the main loop
        transport.start().await?;
        tracing::info!("Transport started");

        // Server main loop
        loop {
            match transport.receive().await {
                Ok((client_id_opt, message)) => {
                    match message {
                        Message::Request(request) => {
                            // Use the client ID from transport if available, otherwise use "default"
                            let client_id = client_id_opt.unwrap_or_else(|| "default".to_string());

                            // Handle the request with the client ID
                            let result = self.handle_request_with_client_id(
                                &mut transport,
                                request,
                                client_id
                            ).await;
                            if let Err(e) = result {
                                tracing::error!("Error handling request: {}", e);
                            }
                        }
                        Message::Notification(notification) => {
                            // Handle notification with client ID if available
                            if
                                let Err(e) = self.handle_notification(
                                    &mut transport,
                                    notification,
                                    client_id_opt
                                ).await
                            {
                                tracing::error!("Error handling notification: {}", e);
                            }
                        }
                        _ => {
                            tracing::warn!("Unexpected message type received");
                        }
                    }
                }
                Err(e) => {
                    // Check if this is a timeout error
                    match e {
                        Error::Timeout(msg) => {
                            // For timeout errors, just log at debug level and continue
                            tracing::debug!("Transport timeout: {}", msg);
                            // Short sleep to avoid tight loop
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                        // For any other errors, log and break the loop
                        _ => {
                            tracing::error!("Transport error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        // Shutdown the lifecycle manager
        self.lifecycle_manager.shutdown().await?;

        Ok(())
    }

    /// Handle a request with client ID
    async fn handle_request_with_client_id<T>(
        &self,
        transport: &mut T,
        request: GenericRequest,
        client_id: String
    )
        -> Result<(), Error>
        where T: Transport, S: Send + Sync + 'static
    {
        // Try to register the client - this is idempotent so safe to call multiple times
        // If it's already registered, this will be a no-op
        let _ = self.lifecycle_manager.register_client(&client_id);

        // For initialize requests, handle specially
        if request.method == "initialize" {
            // Parse the initialize parameters
            let params = match &request.params {
                Some(params) => params.clone(),
                None => {
                    let response = Error::InvalidParams(
                        "Missing parameters for initialize".to_string()
                    ).to_response_payload(request.id.unwrap_or(0));
                    return transport.send_to(&client_id, &Message::Response(response)).await;
                }
            };

            // Simple initialization - just register the client
            if let Some(handler) = self.handlers.get(&request.method) {
                match handler.handle(&self.state, &params).await {
                    Ok(result) => {
                        let response = Response {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.unwrap_or(0),
                            outcome: ResponseOutcome::Success { result },
                        };
                        return transport.send_to(&client_id, &Message::Response(response)).await;
                    }
                    Err(e) => {
                        let response = e.to_response_payload(request.id.unwrap_or(0));
                        return transport.send_to(&client_id, &Message::Response(response)).await;
                    }
                }
            }
        }

        // For regular requests
        if let Some(handler) = self.handlers.get(&request.method) {
            // Extract parameters and handle null properly
            let params = match &request.params {
                Some(p) => p.clone(),
                None => serde_json::json!({}), // Use empty object instead of null for methods like list_tools
            };

            // Validate request if enabled
            if self.validation_config.validate_requests {
                if let Err(e) = validate_request(&request, &self.validation_config) {
                    let response = e.to_response_payload(request.id.unwrap_or(0));
                    return transport.send_to(&client_id, &Message::Response(response)).await;
                }
            }

            match handler.handle(&self.state, &params).await {
                Ok(result) => {
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.unwrap_or(0),
                        outcome: ResponseOutcome::Success { result },
                    };
                    return transport.send_to(&client_id, &Message::Response(response)).await;
                }
                Err(e) => {
                    let response = e.to_response_payload(request.id.unwrap_or(0));
                    return transport.send_to(&client_id, &Message::Response(response)).await;
                }
            }
        } else {
            // Method not found
            let response = Error::MethodNotFound(
                format!("Method not found: {}", request.method)
            ).to_response_payload(request.id.unwrap_or(0));
            return transport.send_to(&client_id, &Message::Response(response)).await;
        }
    }

    /// Handle a notification
    async fn handle_notification<T>(
        &self,
        transport: &mut T,
        notification: Notification,
        client_id: Option<String>
    )
        -> Result<(), Error>
        where T: Transport, S: Send + Sync + 'static
    {
        // If we have a client ID, ensure it's registered before proceeding
        if let Some(cid) = &client_id {
            // Try to register the client - this is idempotent so safe to call multiple times
            // If it's already registered, this will be a no-op
            let _ = self.lifecycle_manager.register_client(cid);
        }

        // Handle notifications - simplified
        match notification.method.as_str() {
            "notifications/initialized" => {
                // For the initialized notification, just log it - client is already registered
                if let Some(cid) = client_id {
                    tracing::info!("Received initialized notification from client {}", cid);
                } else {
                    tracing::info!("Received initialized notification");
                }
                Ok(())
            }
            _ => {
                // Other notifications
                tracing::debug!("Received notification: {}", notification.method);
                Ok(())
            }
        }
    }

    /// Handle a request without client ID
    async fn handle_request<T>(
        &self,
        transport: &mut T,
        request: GenericRequest
    )
        -> Result<(), Error>
        where T: Transport, S: Send + Sync + 'static
    {
        // For non-client specific requests, use a default client ID
        self.handle_request_with_client_id(transport, request, "default".to_string()).await
    }

    /// Gracefully shutdown the server
    pub async fn shutdown(&self) -> Result<(), Error> {
        tracing::info!("Initiating server shutdown");
        self.lifecycle_manager.shutdown().await?;
        tracing::info!("Server shutdown complete");
        Ok(())
    }
}

// Helper function to parse progress updates from tool output
fn parse_progress_update(output: &str) -> Option<f64> {
    // Look for lines like "PROGRESS: 0.5" or "PROGRESS 50%"
    if let Some(progress_str) = output.strip_prefix("PROGRESS:") {
        let progress_str = progress_str.trim();
        if let Ok(progress) = progress_str.parse::<f64>() {
            return Some(progress.clamp(0.0, 1.0));
        }

        if let Some(percent_str) = progress_str.strip_suffix('%') {
            if let Ok(percent) = percent_str.trim().parse::<f64>() {
                return Some((percent / 100.0).clamp(0.0, 1.0));
            }
        }
    }
    None
}

/// Utility function to create a BoxFuture from a Result
pub fn box_future<T, E>(result: Result<T, E>) -> BoxFuture<'static, Result<T, E>>
    where T: Send + 'static, E: Send + 'static
{
    Box::pin(async move { result })
}
