//! MCP Client Core implementation
//!
//! This module implements the core MCP client, responsible for managing the transport,
//! request tracking, notification routing, and lifecycle management.

use crate::client::transport::{ Transport, ConnectionStatus };
use crate::protocol::{
    ClientMessage,
    Error,
    JSONRPCMessage,
    JSONRPCNotification,
    Message,
    RequestId,
};
use crate::{
    client::services::{
        lifecycle::{ LifecycleManager, LifecycleState },
        notification::NotificationRouter,
        service_provider::ServiceProvider,
    },
    protocol::Method,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::{ Ordering, AtomicBool };
use std::time::Duration;
use tokio::sync::{ Mutex, mpsc, broadcast };
use tokio::task::JoinHandle;
use tracing::{ debug, error, info, warn };
use std::pin::Pin;
use std::future::Future;
use crate::client::transport::state::TransportState;

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

/// Builder for creating MCP clients
pub struct ClientBuilder {
    transport: Box<dyn Transport + 'static>,
    config: ClientConfig,
}

impl ClientBuilder {
    /// Create a new client builder with the given transport
    pub fn new(transport: Box<dyn Transport + 'static>) -> Self {
        // Make sure the transport state is properly initialized
        let state = transport.subscribe_state();
        let initial_state = state.borrow().clone();
        debug!(
            "Creating ClientBuilder with transport. Initial state: has_endpoint={}, has_connected={}",
            initial_state.has_endpoint,
            initial_state.has_connected
        );

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

    /// Set the reconnection delay
    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.reconnect_delay = delay;
        self
    }

    /// Set whether to automatically reconnect on connection loss
    pub fn with_auto_reconnect(mut self, auto_reconnect: bool) -> Self {
        self.config.auto_reconnect = auto_reconnect;
        self
    }

    /// Build the client
    pub fn build(self) -> Client {
        Client::new(self.transport, self.config)
    }
}

/// Main client for MCP protocol communication
pub struct Client {
    /// The transport used for communication
    transport: Arc<Box<dyn Transport + 'static>>,
    /// Service provider for accessing shared services
    service_provider: Arc<ServiceProvider>,
    /// Handle to the message processing task
    message_task: Mutex<Option<JoinHandle<()>>>,
    /// Shutdown signal sender for the message task
    message_task_shutdown: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    /// Client configuration
    config: ClientConfig,
    /// Flag indicating if the client is shut down - using atomic for better performance
    shutdown: Arc<AtomicBool>,
    /// Connection status broadcaster
    status_tx: Arc<broadcast::Sender<ConnectionStatus>>,
    /// Transport state receiver for lock-free connection status
    transport_state: tokio::sync::watch::Receiver<TransportState>,
}

impl Client {
    /// Create a new client with the given transport and configuration
    pub fn new(transport: Box<dyn Transport + 'static>, config: ClientConfig) -> Self {
        // Create a broadcast channel for status updates
        let (status_tx, _) = broadcast::channel(16);

        // Get a receiver for the transport state
        let transport_state = transport.subscribe_state();

        let service_provider = Arc::new(ServiceProvider::new());

        Self {
            // Wrap the transport directly in an Arc without a Mutex
            transport: Arc::new(transport),
            service_provider,
            message_task: Mutex::new(None),
            message_task_shutdown: Arc::new(Mutex::new(None)),
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
            status_tx: Arc::new(status_tx),
            transport_state,
        }
    }

    /// Start the client, ensuring the transport is connected and ready
    pub async fn start(&self) -> Result<(), Error> {
        info!("Starting MCP client");

        // Ensure we're not already started
        if let Some(_) = *self.message_task.lock().await {
            return Err(Error::Protocol("Client is already started".to_string()));
        }

        debug!("Starting transport");
        // Start the transport - no longer need to lock it
        self.transport.start().await?;

        // Get a status subscriber from transport and forward to client subscribers
        let mut status_rx = self.transport.subscribe_status();

        // Spawn a task to forward status updates
        let status_tx = self.status_tx.clone();
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            while !shutdown.load(Ordering::SeqCst) {
                match status_rx.recv().await {
                    Ok(status) => {
                        debug!("Forwarding transport status: {:?}", status);
                        let _ = status_tx.send(status);
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        // Wait for connection to be established (max 2 seconds)
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(2);

        // Create a receiver from the transport state
        let mut transport_state_rx = self.transport_state.clone();

        while !transport_state_rx.borrow().is_connected() {
            if start_time.elapsed() > timeout {
                return Err(Error::Transport("Failed to connect within timeout".to_string()));
            }

            // Log the current state for debugging
            let state = transport_state_rx.borrow().clone();
            debug!(
                "Waiting for transport connection. Current state: has_endpoint={}, has_connected={}, endpoint_url={:?}",
                state.has_endpoint,
                state.has_connected,
                state.endpoint_url
            );

            // Wait for the state to change or a short timeout
            match
                tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    transport_state_rx.changed()
                ).await
            {
                Ok(Ok(_)) => {
                    debug!("Transport state changed, checking connection status");
                    // The state changed, check if we're connected on next loop iteration
                }
                Ok(Err(e)) => {
                    warn!("Error waiting for transport state change: {}", e);
                    // Continue in the loop to check again after a short delay
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Err(_) => {
                    // Timeout waiting for change, just continue the loop
                    debug!("No transport state change within timeout, continuing to poll");
                }
            }
        }

        // Log the connected state
        let state = transport_state_rx.borrow().clone();
        debug!(
            "Transport connected. State: has_endpoint={}, has_connected={}, endpoint_url={:?}, session_id={:?}",
            state.has_endpoint,
            state.has_connected,
            state.endpoint_url,
            state.session_id
        );

        // Spawn the message processing task
        self.spawn_message_task().await?;

        info!("MCP client started successfully");
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
        let service_provider = self.service_provider.clone();
        let shutdown = self.shutdown.clone();

        // Spawn the message processing task
        let handle = tokio::spawn(async move {
            // Loop until shutdown is signaled
            loop {
                // Check for shutdown before acquiring any locks
                if shutdown.load(Ordering::Relaxed) {
                    debug!("Client is shutting down, breaking message loop");
                    break;
                }

                tokio::select! {
                    Some(_) = shutdown_rx.recv() => {
                        debug!("Received shutdown signal, stopping message loop");
                        break;
                    }
                    result = async {
                        // Directly receive messages without locking the transport
                     
                        let recv_result = tokio::time::timeout(
                            tokio::time::Duration::from_millis(500),
                            transport.receive()
                        ).await;
                        
                        match recv_result {
                            Ok(result) => {
                                debug!("Message loop received message successfully");
                                result
                            },
                            Err(_) => {
                                Err(Error::Transport("Receive operation timed out".to_string()))
                            }
                        }
                    } => {
                        match result {
                            Ok((_, message)) => {
                                // Process the received message
                                let _ = Self::process_message(
                                    message,
                                    &service_provider
                                ).await.map_err(|e| {
                                    error!("Error processing message: {}", e);
                                });
                            }
                            Err(e) => {
                                // Check if we're shutting down
                                if shutdown.load(Ordering::Relaxed) {
                                    debug!("Client is shutting down, stopping message loop");
                                    break;
                                }
                                
                                // If it's a timeout error, just continue to the next iteration
                                if e.to_string().contains("timed out") || e.to_string().contains("Timeout") {
                                    // Just yield to other tasks and continue
                                    tokio::task::yield_now().await;
                                    continue;
                                }
                                
                                // For other errors, log them
                                warn!("Error receiving message: {}", e);
                                
                                // Short sleep to avoid busy loops on error conditions
                                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                            }
                        }
                    }
                }
            }
        });

        *task_guard = Some(handle);
        Ok(())
    }

    /// Process a received message
    async fn process_message(
        message: JSONRPCMessage,
        service_provider: &Arc<ServiceProvider>
    ) -> Result<(), Error> {
        // Validate the message
        let config = crate::protocol::validation::ValidationConfig::default();
        if let Err(e) = crate::protocol::validation::validate_message(&message, &config) {
            warn!("Message validation failed: {}", e);
            return Err(e);
        }

        match message {
            JSONRPCMessage::Response(response) => {
                debug!("Received response: {:?}", response.id);

                // If this is an initialize response, we need special handling
                let current_state = service_provider.lifecycle_manager().current_state().await;
                if current_state == LifecycleState::Initialization {
                    // Transition to Operation state when we receive the initialize response
                    debug!("Received initialize response, transitioning to Operation state");

                    if
                        let Err(e) = service_provider
                            .lifecycle_manager()
                            .transition_to(LifecycleState::Operation).await
                    {
                        warn!("Failed to transition to Operation: {}", e);
                    }
                }

                // Handle the response using the request manager
                if let Err(e) = service_provider.request_manager().handle_response(response).await {
                    warn!("Failed to handle response: {}", e);
                    return Err(e);
                }
            }
            JSONRPCMessage::Error(error) => {
                debug!("Received error: {:?}", error);

                // Update lifecycle state for serious errors
                if error.error.code < 0 {
                    let _ = service_provider
                        .lifecycle_manager()
                        .set_error_state(format!("Protocol error: {}", error.error.message)).await;
                }

                // Handle the error using the request manager
                if let Err(e) = service_provider.request_manager().handle_error(error).await {
                    warn!("Failed to handle error: {}", e);
                    return Err(e);
                }
            }
            JSONRPCMessage::Notification(notification) => {
                debug!("Received notification: {:?}", notification.method);

                // Enforce proper sequencing for notifications
                let current_state = service_provider.lifecycle_manager().current_state().await;

                // Check if this is an initialized notification
                if notification.method == Method::NotificationsInitialized {
                    debug!("Received initialized notification");

                    if current_state != LifecycleState::Initialization {
                        warn!("Received initialized notification but not in initialization state");
                    } else {
                        // Transition to Operation state
                        let _ = service_provider
                            .lifecycle_manager()
                            .transition_to(LifecycleState::Operation).await;
                    }
                } else {
                    // For other notifications, we should be in Operation state
                    if current_state != LifecycleState::Operation {
                        debug!(
                            "Received notification in non-operation state: {:?}",
                            notification.method
                        );
                    }
                }

                // Process the notification through the router
                if
                    let Err(e) = service_provider
                        .notification_router()
                        .handle_notification(notification).await
                {
                    warn!("Failed to handle notification: {}", e);
                }
            }
            JSONRPCMessage::Request(request) => {
                debug!("Received request: {:?}", request.method);

                // Check if we're in a state to handle this
                let current_state = service_provider.lifecycle_manager().current_state().await;

                // Only allow ping requests in any state
                if request.method != Method::Ping && current_state != LifecycleState::Operation {
                    warn!("Received request in non-operation state: {:?}", request.method);
                    // TODO: Send error response
                    return Ok(());
                }

                // TODO: Route request to appropriate handler
                warn!("Client-side request handling not implemented yet");
            }
        }

        Ok(())
    }

    /// Send a request and wait for a response
    pub async fn send_request<P, R>(&self, method: Method, params: P) -> Result<R, Error>
        where P: Serialize + Send + Sync, R: DeserializeOwned + Send + Sync
    {
        info!("Sending request: {}", method);

        // Quick check for shutdown state - avoid lock if possible
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(Error::Other("Client is shut down".to_string()));
        }

        // Check if the client is in an appropriate state for this request
        if let Err(e) = self.service_provider.lifecycle_manager().validate_method(&method).await {
            return Err(
                Error::Protocol(
                    format!("Cannot send request {} in current lifecycle state: {}", method, e)
                )
            );
        }
        debug!("Client is in a valid state to send request");

        // Check if the client is connected
        if !self.is_connected() {
            return Err(Error::Transport("Not connected to server".to_string()));
        }
        debug!("Client is connected to server");

        // Serialize the parameters
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Protocol(format!("Failed to serialize params: {}", e)));
            }
        };

        // Use the request manager to handle the request
        debug!("Using request manager to send request: {}", method);

        // Create the send function here to avoid capturing self
        let transport_clone = self.transport.clone();
        let send_fn = move |message: JSONRPCMessage| {
            let transport = transport_clone.clone();
            Box::pin(async move {
                debug!("Request manager sending message");
                // No need for lock acquisition, directly send the message
                transport.send(&message).await
            }) as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
        };

        self.service_provider
            .request_manager()
            .send_request(send_fn, method, params_value, Some(self.config.request_timeout)).await
    }

    /// Send a notification
    pub async fn send_notification<P>(&self, method: Method, params: P) -> Result<(), Error>
        where P: Serialize + Send + Sync
    {
        // Check if client is shut down
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(Error::Other("Client is shut down".to_string()));
        }

        // Check if the client is in an appropriate state for this notification
        if
            let Err(e) = self.service_provider
                .lifecycle_manager()
                .validate_notification(&method).await
        {
            return Err(
                Error::Protocol(
                    format!("Cannot send notification {} in current lifecycle state: {}", method, e)
                )
            );
        }

        // Check if the client is connected
        if !self.is_connected() {
            return Err(Error::Transport("Not connected to server".to_string()));
        }

        // Serialize the parameters
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Protocol(format!("Failed to serialize params: {}", e)));
            }
        };

        // Create the notification
        let notification = JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: method,
            params: params_value,
        };

        self.send_raw_message(JSONRPCMessage::Notification(notification)).await
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
        self.service_provider.notification_router().register_handler(
            method,
            Box::new(move |notification| {
                let fut = handler(notification);
                Box::pin(fut)
            })
        ).await
    }

    /// Get the lifecycle manager
    pub fn lifecycle(&self) -> Arc<LifecycleManager> {
        self.service_provider.lifecycle_manager()
    }

    /// Get the notification router
    pub fn notification_router(&self) -> Arc<NotificationRouter> {
        self.service_provider.notification_router()
    }

    /// Check if the client is connected to the server
    pub fn is_connected(&self) -> bool {
        // First ensure the client isn't shut down
        if self.shutdown.load(Ordering::SeqCst) {
            return false;
        }

        // Use the transport state directly without locking the transport
        debug!("Transport state: {:?}", self.transport_state.borrow());
        self.transport_state.borrow().is_connected()
    }

    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<(), Error> {
        // Mark client as shutting down
        self.shutdown.store(true, Ordering::Relaxed);

        // Cancel all pending requests
        self.service_provider.request_manager().cancel_all_requests("Client shutting down").await;

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

        // Close the transport - no need to lock anymore
        if let Err(e) = self.transport.close().await {
            warn!("Error closing transport during shutdown: {}", e);
        }

        // Update lifecycle state to shutdown
        if
            let Err(e) = self.service_provider
                .lifecycle_manager()
                .transition_to(LifecycleState::Shutdown).await
        {
            warn!("Failed to update lifecycle state to shutdown: {}", e);
        }

        debug!("MCP client shutdown complete");
        Ok(())
    }

    /// Send a raw JSONRPCMessage without expecting a specific response type
    pub async fn send_raw_message(&self, message: JSONRPCMessage) -> Result<(), Error> {
        // Quick shutdown check without locking
        if self.shutdown.load(Ordering::Relaxed) {
            error!("Cannot send message - client is shut down");
            return Err(Error::Other("Client is shut down".to_string()));
        }

        // Check if the client is connected without acquiring the transport lock
        if !self.is_connected() {
            error!("Cannot send message - client not connected to server");
            return Err(Error::Transport("Not connected to server".to_string()));
        }

        // Validate the message before sending
        let config = crate::protocol::validation::ValidationConfig::default();
        if let Err(e) = crate::protocol::validation::validate_message(&message, &config) {
            error!("Message validation failed: {}", e);
            return Err(e);
        }

        // Directly send the message without locking
        debug!("Sending message through transport");
        let result = self.transport.send(&message).await;

        match result {
            Ok(_) => {
                debug!("Message sent successfully through transport");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send message through transport: {}", e);
                Err(e)
            }
        }
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

        // Send the message directly
        self.transport.send(&json_rpc_message).await
    }

    /// Subscribe to connection status updates
    pub fn subscribe_status(&self) -> broadcast::Receiver<ConnectionStatus> {
        self.status_tx.subscribe()
    }
}

// impl Drop for Client {
//     fn drop(&mut self) {
//         // When client is dropped, spawn a blocking task to ensure shutdown
//         let client_clone = self.clone();
//         tokio::spawn(async move {
//             if let Err(e) = client_clone.shutdown().await {
//                 error!("Error during client shutdown: {}", e);
//             }
//         });
//     }
// }

// Allow cloning the client for use in async contexts
impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            transport: self.transport.clone(),
            service_provider: self.service_provider.clone(),
            message_task: Mutex::new(None),
            message_task_shutdown: self.message_task_shutdown.clone(),
            config: self.config.clone(),
            shutdown: self.shutdown.clone(),
            status_tx: self.status_tx.clone(),
            transport_state: self.transport_state.clone(),
        }
    }
}
