//! MCP Client Core implementation
//!
//! This module implements the core MCP client, responsible for managing the transport,
//! request tracking, notification routing, and lifecycle management.

use crate::client::transport::DirectIOTransport;
use crate::protocol::{
    ClientMessage,
    Error,
    JSONRPCMessage,
    JSONRPCNotification,
    JSONRPCRequest,
    JSONRPCResponse,
    Message,
    RequestId,
    errors::rpc_error_to_error,
    JSONRPCError,
};
use crate::{
    client::services::{
        lifecycle::{ LifecycleManager, LifecycleState },
        notification::NotificationRouter,
        request::RequestManager,
    },
    protocol::Method,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{ AtomicI64, Ordering };
use std::time::Duration;
use tokio::sync::{ Mutex, mpsc, oneshot };
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{ debug, error, info, warn };

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
    /// Shutdown signal sender for the message task
    message_task_shutdown: Arc<Mutex<Option<mpsc::Sender<()>>>>,
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
            message_task_shutdown: Arc::new(Mutex::new(None)),
            config,
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the client, ensuring the transport is connected and ready
    pub async fn start(&self) -> Result<(), Error> {
        info!("Starting MCP client");

        // Check that we're in the initialization state
        let current_state = self.lifecycle.current_state().await;
        if current_state != LifecycleState::Initialization {
            return Err(
                Error::Protocol(
                    format!(
                        "Client must be in Initialization state to start (current state: {:?})",
                        current_state
                    )
                )
            );
        }

        // Start the transport and ensure it's connected
        {
            let mut transport = self.transport.lock().await;
            // Start the transport - this will wait for the connection to be established
            transport.start().await?;
        }

        // Once transport is connected, spawn message processing task
        self.spawn_message_task().await?;

        // We intentionally keep the lifecycle in Initialization state
        // This allows sending the Initialize request which requires Initialization state

        info!("MCP client started successfully - ready for initialization");
        Ok(())
    }

    /// Spawn the message processing task in the background
    async fn spawn_message_task(&self) -> Result<(), Error> {
        debug!("Starting MCP client message processing");

        // Check if we're already processing messages
        let mut task_guard = self.message_task.lock().await;
        let mut shutdown_guard = self.message_task_shutdown.lock().await;

        if task_guard.is_some() {
            return Ok(());
        }

        // Create channels for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Store the shutdown sender for later use
        *shutdown_guard = Some(shutdown_tx);

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
                    Some(_) = shutdown_rx.recv() => {
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

                // If this is an initialize response, we need special handling
                let current_state = lifecycle.current_state().await;
                if current_state == LifecycleState::Initialization {
                    // Transition to Operation state when we receive the initialize response
                    debug!(
                        "Received response during Initialization state, transitioning to Operation"
                    );

                    if let Err(e) = lifecycle.transition_to(LifecycleState::Operation).await {
                        warn!("Failed to transition to Operation: {}", e);
                    }
                }

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
                    if sender.send(Err(error_variant.clone())).is_err() {
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

                // Enforce proper sequencing for notifications
                let current_state = lifecycle.current_state().await;

                // Check if this is an initialized notification
                if notification.method == Method::NotificationsInitialized {
                    // The only place we expect this is in Initialization state
                    if current_state != LifecycleState::Initialization {
                        warn!(
                            "Received initialized notification in unexpected state: {:?}",
                            current_state
                        );
                    } else {
                        // Transition to Operation state
                        let _ = lifecycle.transition_to(LifecycleState::Operation).await;
                    }
                } else {
                    // For other notifications, generally they should come when we're in Operation state

                    // Logging is an exception which can come early
                    if
                        current_state != LifecycleState::Operation &&
                        !notification.method.is_logging_method() &&
                        notification.method != Method::NotificationsProgress &&
                        notification.method != Method::NotificationsCancelled
                    {
                        debug!(
                            "Received notification {} in non-Operation state: {:?}",
                            notification.method,
                            current_state
                        );
                    }
                }

                // Process the notification through the router
                if let Err(e) = notification_router.handle_notification(notification).await {
                    warn!("Failed to handle notification: {}", e);
                }

                Ok(())
            }
            JSONRPCMessage::Request(request) => {
                warn!("Received unexpected request: {}", request.method);

                // Check if we're in a state to handle this
                let current_state = lifecycle.current_state().await;

                // Only allow ping requests in any state
                if request.method != Method::Ping && current_state != LifecycleState::Operation {
                    return Err(
                        Error::Protocol(
                            format!(
                                "Client must be in Operation state to send requests (current: {:?})",
                                current_state
                            )
                        )
                    );
                }

                // Client doesn't typically handle requests, only server does
                Ok(())
            }
        }
    }

    /// Send a request and wait for a response
    pub async fn send_request<P, R>(&self, method: Method, params: P) -> Result<R, Error>
        where P: Serialize + Send + Sync, R: DeserializeOwned + Send + Sync
    {
        info!("Sending request: {}", method);

        // Check if client is shut down
        {
            let shutdown = self.shutdown.lock().await;
            if *shutdown {
                return Err(Error::Other("Client is shut down".to_string()));
            }
        }

        // Check if the client is in an appropriate state for this request
        if let Err(e) = self.lifecycle.validate_method(&method).await {
            return Err(
                Error::Protocol(
                    format!("Cannot send request {} in current lifecycle state: {}", method, e)
                )
            );
        }

        // Generate request ID
        let id = self.generate_id();

        // Create a one-shot channel to receive the response
        let (tx, rx) = oneshot::channel();

        // Store the request in the pending requests map
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id.clone(), tx);
        }

        // Create the JSONRPC request with parameters
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Protocol(format!("Failed to serialize params: {}", e)));
            }
        };

        let request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.clone(),
            params: params_value,
        };

        let message = JSONRPCMessage::Request(request);

        // Send the request through the transport
        match self.send_raw_message(message).await {
            Ok(_) => {
                debug!("Waiting for response to request {} with id {:?}", method, id);
            }
            Err(e) => {
                // Failed to send the request, clean up the pending request
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);

                return Err(e);
            }
        }

        // Wait for the response with timeout handling
        let timeout = self.config.request_timeout;
        match tokio::time::timeout(timeout, rx).await {
            Ok(result) => {
                match result {
                    Ok(response) => {
                        match response {
                            Ok(response) => {
                                // Try to deserialize the result content
                                debug!("Received response for request {}: {:?}", method, response);

                                let result_value = match
                                    serde_json::to_value(&response.result.content)
                                {
                                    Ok(value) => value,
                                    Err(e) => {
                                        return Err(
                                            Error::Protocol(
                                                format!("Failed to convert response content to value: {}", e)
                                            )
                                        );
                                    }
                                };

                                match serde_json::from_value::<R>(result_value) {
                                    Ok(typed_result) => Ok(typed_result),
                                    Err(e) => {
                                        Err(
                                            Error::Protocol(
                                                format!("Failed to deserialize response: {}", e)
                                            )
                                        )
                                    }
                                }
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(_) => {
                        Err(Error::Protocol("Response channel closed unexpectedly".to_string()))
                    }
                }
            }
            Err(_) => {
                // Timeout occurred, remove the pending request
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);

                Err(Error::Timeout(format!("Request {} timed out after {:?}", method, timeout)))
            }
        }
    }

    /// Send a notification
    pub async fn send_notification<P>(&self, method: Method, params: P) -> Result<(), Error>
        where P: Serialize + Send + Sync
    {
        // Check if client is shut down
        {
            let shutdown = self.shutdown.lock().await;
            if *shutdown {
                return Err(Error::Other("Client is shut down".to_string()));
            }
        }

        // Check if client is connected
        if !self.is_connected().await {
            return Err(Error::Other("Client is not connected".to_string()));
        }

        // Validate operation based on lifecycle state and capabilities
        self.lifecycle.validate_method(&method).await?;

        // Serialize parameters
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Other(format!("Failed to serialize params: {}", e)));
            }
        };

        // Create notification message
        let notification = JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: params_value,
        };

        // Convert to JSONRPCMessage
        let message = JSONRPCMessage::Notification(notification);

        // Send the notification
        self.send_raw_message(message).await
    }

    /// Register a notification handler
    pub async fn register_notification_handler<F, Fut>(
        &self,
        method: Method,
        handler: F
    )
        -> Result<(), Error>
        where
            F: Fn(JSONRPCNotification) -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = Result<(), Error>> + Send + 'static
    {
        self.notification_router.register_handler(
            method,
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
        debug!("Shutting down MCP client");

        // Guard against multiple shutdown calls
        {
            let mut shutdown = self.shutdown.lock().await;
            if *shutdown {
                debug!("Client already shut down");
                return Ok(());
            }
            *shutdown = true;
        }

        // Notify the lifecycle manager that we're shutting down
        if let Err(e) = self.lifecycle.transition_to(LifecycleState::Shutdown).await {
            warn!("Failed to update lifecycle state during shutdown: {}", e);
        }

        // Cancel pending requests with a cancellation error
        {
            let mut pending = self.pending_requests.lock().await;
            for (id, sender) in pending.drain() {
                let _ = sender.send(
                    Err(Error::Other(format!("Request {:?} cancelled due to client shutdown", id)))
                );
            }
        }

        // Signal the message processing task to stop
        {
            let mut shutdown_sender = self.message_task_shutdown.lock().await;
            if let Some(sender) = shutdown_sender.take() {
                // Send a shutdown signal to the message processing task
                let _ = sender.send(()).await;
                debug!("Sent shutdown signal to message processing task");
            }

            // Wait a moment for the task to process the shutdown signal
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Now take the task handle if it's still there
            if let Some(handle) = self.message_task.lock().await.take() {
                // Abort the task if it's still running
                handle.abort();
                debug!("Aborted message processing task");
            }
        }

        // Close the transport
        {
            let mut transport = self.transport.lock().await;
            // Transport implementations handle their own cleanup
            if let Err(e) = transport.close().await {
                warn!("Error closing transport during shutdown: {}", e);
            }
        }

        // Update lifecycle state to shutdown
        if let Err(e) = self.lifecycle.transition_to(LifecycleState::Shutdown).await {
            warn!("Failed to update lifecycle state to shutdown: {}", e);
        }

        debug!("MCP client shutdown complete");
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
                        let req_id = id.ok_or_else(|| {
                            Error::Other("RequestId is required for request messages".to_string())
                        })?;
                        message.from_message(req_id)?
                    }
                    ClientMessage::Notification(_) => {
                        // Notifications don't need ID, but from_message requires one
                        // We use a dummy ID and it will be ignored for notifications
                        message.from_message(RequestId::Number(0))?
                    }
                    ClientMessage::Result(_) => {
                        // Need request ID for results
                        let req_id = id.ok_or_else(|| {
                            Error::Other("RequestId is required for result messages".to_string())
                        })?;
                        message.from_message(req_id)?
                    }
                }
            }
            Message::Server(_) => {
                return Err(Error::Other("Cannot send server messages from client".to_string()));
            }
            Message::Error(_) => {
                // Need request ID for errors
                let req_id = id.ok_or_else(|| {
                    Error::Other("RequestId is required for error messages".to_string())
                })?;
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
            message_task_shutdown: Arc::new(Mutex::new(None)),
            config: self.config.clone(),
            shutdown: self.shutdown.clone(),
        }
    }
}
