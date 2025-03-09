//! MCP Client Core implementation
//!
//! This module implements the core MCP client, responsible for managing the transport,
//! request tracking, notification routing, and lifecycle management.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{ AtomicI64, Ordering };
use std::time::Duration;
use tokio::sync::{ mpsc, Mutex, oneshot, RwLock };
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{ debug, error, info, warn };
use crate::protocol::{
    JSONRPCError,
    JSONRPCMessage,
    JSONRPCNotification,
    JSONRPCRequest,
    JSONRPCResponse,
    RequestId,
    Error,
    Message,
    ClientMessage,
    errors::rpc_error_to_error,
};
use crate::transport::{ Transport, DirectIOTransport };
use crate::client::lifecycle::{ LifecycleManager, LifecycleState };
use crate::client::notification::NotificationRouter;
use crate::client::request::RequestManager;

/// Default request timeout in seconds
const DEFAULT_REQUEST_TIMEOUT: u64 = 30;

/// Configuration for the MCP client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Timeout for requests in seconds
    pub request_timeout: Duration,
    /// Maximum number of reconnection attempts
    pub max_reconnect_attempts: usize,
    /// Delay between reconnection attempts
    pub reconnect_delay: Duration,
    /// Whether to automatically reconnect on connection loss
    pub auto_reconnect: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT),
            max_reconnect_attempts: 3,
            reconnect_delay: Duration::from_secs(1),
            auto_reconnect: true,
        }
    }
}

/// Builder for creating Client instances with custom configuration
pub struct ClientBuilder<T: DirectIOTransport + 'static> {
    transport: T,
    config: ClientConfig,
}

impl<T: DirectIOTransport + 'static> ClientBuilder<T> {
    /// Create a new client builder with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            config: ClientConfig::default(),
        }
    }

    /// Set the request timeout
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.config.request_timeout = timeout;
        self
    }

    /// Set the maximum number of reconnection attempts
    pub fn with_max_reconnect_attempts(mut self, attempts: usize) -> Self {
        self.config.max_reconnect_attempts = attempts;
        self
    }

    /// Set the delay between reconnection attempts
    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.reconnect_delay = delay;
        self
    }

    /// Enable or disable automatic reconnection
    pub fn with_auto_reconnect(mut self, auto_reconnect: bool) -> Self {
        self.config.auto_reconnect = auto_reconnect;
        self
    }

    /// Build the client with the configured options
    pub fn build(self) -> Client<T> {
        Client::new(self.transport, self.config)
    }
}

/// Main client for MCP protocol communication
pub struct Client<T: DirectIOTransport + 'static> {
    /// The transport used for communication
    transport: Arc<Mutex<T>>,
    /// Counter for generating request IDs
    request_id_counter: AtomicI64,
    /// Map of pending requests by ID
    pending_requests: Arc<
        Mutex<HashMap<RequestId, oneshot::Sender<Result<JSONRPCResponse, Error>>>>
    >,
    /// Notification router for handling notifications
    notification_router: Arc<NotificationRouter>,
    /// Lifecycle manager for tracking connection state
    lifecycle: Arc<LifecycleManager>,
    /// Request manager for tracking requests
    request_manager: Arc<RequestManager>,
    /// Handle to the message processing task
    message_task: Mutex<Option<JoinHandle<()>>>,
    /// Client configuration
    config: ClientConfig,
    /// Flag indicating if the client is shut down
    shutdown: Arc<Mutex<bool>>,
}

impl<T: DirectIOTransport + 'static> Client<T> {
    /// Create a new client with the given transport
    pub fn new(transport: T, config: ClientConfig) -> Self {
        let lifecycle = Arc::new(LifecycleManager::new());
        let notification_router = Arc::new(NotificationRouter::new());
        let request_manager = Arc::new(RequestManager::new(config.request_timeout));

        Self {
            transport: Arc::new(Mutex::new(transport)),
            request_id_counter: AtomicI64::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            notification_router,
            lifecycle,
            request_manager,
            message_task: Mutex::new(None),
            config,
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the client message processing loop
    pub async fn start(&self) -> Result<(), Error> {
        debug!("Starting MCP client");

        // Check if we're already processing messages
        let mut task_guard = self.message_task.lock().await;
        if task_guard.is_some() {
            return Ok(());
        }

        // Create channels for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Clone the necessary Arc references
        let transport = self.transport.clone();
        let pending_requests = self.pending_requests.clone();
        let notification_router = self.notification_router.clone();
        let lifecycle = self.lifecycle.clone();
        let shutdown = self.shutdown.clone();

        // Spawn the message processing task
        let handle = tokio::spawn(async move {
            // Loop until shutdown is signaled
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        debug!("Received shutdown signal, stopping message loop");
                        break;
                    }
                    result = async {
                        let mut transport_guard = transport.lock().await;
                        transport_guard.receive().await
                    } => {
                        match result {
                            Ok((_, message)) => {
                                // Process the received message
                                let _ = Self::process_message(
                                    message,
                                    &pending_requests,
                                    &notification_router,
                                    &lifecycle
                                ).await;
                            }
                            Err(e) => {
                                error!("Error receiving message: {}", e);
                                
                                // Update lifecycle state
                                let _ = lifecycle.set_error_state(format!("Transport error: {}", e)).await;
                                
                                // Check if we're shutting down
                                let is_shutdown = *shutdown.lock().await;
                                if is_shutdown {
                                    debug!("Client is shutting down, stopping message loop");
                                    break;
                                }
                                
                                // Briefly pause before retrying
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
            }

            debug!("Message processing loop terminated");
        });

        // Store the task handle
        *task_guard = Some(handle);

        Ok(())
    }

    /// Process a received message
    async fn process_message(
        message: JSONRPCMessage,
        pending_requests: &Arc<
            Mutex<HashMap<RequestId, oneshot::Sender<Result<JSONRPCResponse, Error>>>>
        >,
        notification_router: &Arc<NotificationRouter>,
        lifecycle: &Arc<LifecycleManager>
    ) -> Result<(), Error> {
        match message {
            JSONRPCMessage::Response(response) => {
                debug!("Received response: {:?}", response.id);

                // Find the corresponding pending request
                let mut requests = pending_requests.lock().await;
                if let Some(sender) = requests.remove(&response.id) {
                    // Send the response to the waiting task
                    if sender.send(Ok(response.clone())).is_err() {
                        warn!("Failed to send response to requester (channel closed)");
                    }
                } else {
                    warn!("Received response for unknown request ID: {:?}", response.id);
                }

                Ok(())
            }
            JSONRPCMessage::Error(error) => {
                debug!("Received error response: {}", error.error.message);

                // Convert the JSONRPCError to an Error variant
                let error_variant = rpc_error_to_error(&error);

                // Find the corresponding pending request
                let mut requests = pending_requests.lock().await;
                if let Some(sender) = requests.remove(&error.id) {
                    // Send the error to the waiting task
                    if sender.send(Err(error_variant)).is_err() {
                        warn!("Failed to send error to requester (channel closed)");
                    }
                } else {
                    warn!("Received error for unknown request ID: {:?}", error.id);
                }

                // Update lifecycle state for serious errors
                if error.error.code < 0 {
                    let _ = lifecycle.set_error_state(
                        format!("Protocol error: {}", error.error.message)
                    ).await;
                }

                Ok(())
            }
            JSONRPCMessage::Notification(notification) => {
                debug!("Received notification: {}", notification.method);

                // Process the notification through the router
                if let Err(e) = notification_router.handle_notification(notification).await {
                    warn!("Failed to handle notification: {}", e);
                }

                Ok(())
            }
            JSONRPCMessage::Request(request) => {
                warn!("Received unexpected request: {}", request.method);
                // Client doesn't handle requests, only server does
                Ok(())
            }
        }
    }

    /// Send a request and wait for a response
    pub async fn send_request<P, R>(&self, method: &str, params: P) -> Result<R, Error>
        where P: Serialize + Send + Sync, R: DeserializeOwned + Send + Sync
    {
        // Validate current lifecycle state
        self.lifecycle.validate_request(method).await?;

        // Generate a new request ID
        let id = self.generate_id();

        // Serialize the parameters
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Protocol(format!("Invalid params: {}", e)));
            }
        };

        // Create the JSON-RPC request
        let request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.to_string(),
            params: params_value,
        };

        let message = JSONRPCMessage::Request(request);

        // Create a channel for receiving the response
        let (response_tx, response_rx) = oneshot::channel();

        // Register the pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id.clone(), response_tx);
        }

        // Send the request
        {
            let mut transport = self.transport.lock().await;
            transport.send(&message).await?;
        }

        // Wait for the response with a timeout
        let response = (match timeout(self.config.request_timeout, response_rx).await {
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
                // Timeout occurred, remove the pending request
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);

                return Err(
                    Error::Timeout(
                        format!("Request timed out after {:?}", self.config.request_timeout)
                    )
                );
            }
        })?;

        // Deserialize the response
        let result = serde_json
            ::from_value(serde_json::to_value(response.result.content)?)
            .map_err(|e| Error::Protocol(format!("Invalid response: {}", e)))?;

        Ok(result)
    }

    /// Send a notification
    pub async fn send_notification<P>(&self, method: &str, params: P) -> Result<(), Error>
        where P: Serialize + Send + Sync
    {
        // Validate current lifecycle state
        self.lifecycle.validate_notification(method).await?;

        // Serialize the parameters
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Other(format!("Failed to serialize params: {}", e)));
            }
        };

        // Create the JSON-RPC notification
        let notification = JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: params_value,
        };

        let message = JSONRPCMessage::Notification(notification);

        // Send the notification
        let mut transport = self.transport.lock().await;
        transport.send(&message).await?;

        Ok(())
    }

    /// Register a notification handler
    pub async fn register_notification_handler<F, Fut>(
        &self,
        method: &str,
        handler: F
    )
        -> Result<(), Error>
        where
            F: Fn(JSONRPCNotification) -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = Result<(), Error>> + Send + 'static
    {
        self.notification_router.register_handler(
            method.to_string(),
            Box::new(move |notification| {
                let fut = handler(notification);
                Box::pin(fut)
            })
        ).await
    }

    /// Get the lifecycle manager
    pub fn lifecycle(&self) -> Arc<LifecycleManager> {
        self.lifecycle.clone()
    }

    /// Get the notification router
    pub fn notification_router(&self) -> Arc<NotificationRouter> {
        self.notification_router.clone()
    }

    /// Check if the client is connected
    pub async fn is_connected(&self) -> bool {
        let transport = self.transport.lock().await;
        transport.is_connected().await
    }

    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<(), Error> {
        let mut shutdown = self.shutdown.lock().await;
        *shutdown = true;

        // Close the transport
        let mut transport = self.transport.lock().await;
        transport.close().await?;

        // Abort the message task if it exists
        let mut task = self.message_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }

        // Update lifecycle state
        self.lifecycle.transition_to(LifecycleState::Closed).await?;

        Ok(())
    }

    /// Generate a request ID
    fn generate_id(&self) -> RequestId {
        let id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        RequestId::Number(id)
    }

    /// Send a raw JSONRPCMessage without expecting a specific response type
    pub async fn send_raw_message(&self, message: JSONRPCMessage) -> Result<(), Error> {
        // Check if the client is connected
        if !self.is_connected().await {
            return Err(Error::Transport("Not connected to server".to_string()));
        }

        // Get a lock on the transport
        let mut transport = self.transport.lock().await;

        // Send the message
        transport.send(&message).await
    }

    /// Send a high-level Message directly
    pub async fn send_message(&self, message: Message, id: Option<RequestId>) -> Result<(), Error> {
        // Convert the message to a JSONRPCMessage
        let json_rpc_message = match &message {
            Message::Client(client_message) => {
                match client_message {
                    ClientMessage::Request(_) => {
                        // Need request ID for requests
                        let req_id = id.ok_or_else(||
                            Error::Other("RequestId is required for request messages".to_string())
                        )?;
                        message.from_message(req_id)?
                    }
                    ClientMessage::Notification(_) => {
                        // Notifications don't need ID, but from_message requires one
                        // We use a dummy ID and it will be ignored for notifications
                        message.from_message(RequestId::Number(0))?
                    }
                    ClientMessage::Result(_) => {
                        // Need request ID for results
                        let req_id = id.ok_or_else(||
                            Error::Other("RequestId is required for result messages".to_string())
                        )?;
                        message.from_message(req_id)?
                    }
                }
            }
            Message::Server(_) => {
                return Err(Error::Other("Cannot send server messages from client".to_string()));
            }
            Message::Error(_) => {
                // Need request ID for errors
                let req_id = id.ok_or_else(||
                    Error::Other("RequestId is required for error messages".to_string())
                )?;
                message.from_message(req_id)?
            }
        };

        // Get a lock on the transport
        let mut transport = self.transport.lock().await;

        // Send the message
        transport.send(&json_rpc_message).await
    }
}

impl<T: DirectIOTransport + 'static> Drop for Client<T> {
    fn drop(&mut self) {
        // When client is dropped, spawn a blocking task to ensure shutdown
        let client_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = client_clone.shutdown().await {
                error!("Error during client shutdown: {}", e);
            }
        });
    }
}

// Allow cloning the client for use in async contexts
impl<T: DirectIOTransport + 'static> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            request_id_counter: AtomicI64::new(self.request_id_counter.load(Ordering::SeqCst)),
            pending_requests: self.pending_requests.clone(),
            notification_router: self.notification_router.clone(),
            lifecycle: self.lifecycle.clone(),
            request_manager: self.request_manager.clone(),
            message_task: Mutex::new(None),
            config: self.config.clone(),
            shutdown: self.shutdown.clone(),
        }
    }
}
