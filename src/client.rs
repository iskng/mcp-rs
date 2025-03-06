//! MCP Client
//!
//! This module implements the MCP client, responsible for connecting to MCP servers,
//! sending requests, and receiving responses. It abstracts the underlying transport
//! layer, allowing for easy integration with different communication channels.

use serde::{ Serialize, de::DeserializeOwned };
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{ AtomicI32, Ordering };
use tokio::sync::oneshot;
use tokio::sync::{ Mutex, mpsc };
use tokio::task::JoinHandle;

use crate::errors::Error;
use crate::types::protocol::{
    Message as McpMessage,
    Notification,
    Request,
    Response,
    ResponseOutcome,
    Role,
};
use crate::transport::Transport;
use crate::types::initialize::{ InitializeRequestParams, InitializeResult };
use crate::types::prompts::{ ContentPart, ListPromptsParams, ListPromptsResult, PromptMessage };
use crate::types::resources::{
    CreateResourceParams,
    CreateResourceResult,
    DeleteResourceParams,
    DeleteResourceResult,
    FileResource,
    ListResourcesParams,
    ListResourcesResult,
    ReadResourceParams,
    ReadResourceResult,
    UpdateResourceParams,
    UpdateResourceResult,
};
use crate::types::tools::{ CallToolParams, CallToolResult, ListToolsParams, ListToolsResult };
use crate::utils::validation::{ ValidationConfig, validate_request, validate_response };

/// A client for communicating with an MCP server
pub struct Client<T> {
    /// The transport used for communication
    transport: Arc<Mutex<T>>,
    /// Counter for generating request IDs
    request_id_counter: AtomicI32,
    /// Pending requests waiting for responses
    pending_requests: Arc<Mutex<HashMap<i32, oneshot::Sender<Result<Response, Error>>>>>,
    /// Notification listeners
    notification_listeners: Arc<Mutex<Vec<mpsc::Sender<Notification>>>>,
    /// Background task handle for the message loop
    message_loop_handle: Option<JoinHandle<Result<(), Error>>>,
    /// Validation configuration
    validation_config: ValidationConfig,
}

impl<T: Transport + 'static> Client<T> {
    /// Create a new client with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(Mutex::new(transport)),
            request_id_counter: AtomicI32::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            notification_listeners: Arc::new(Mutex::new(Vec::new())),
            message_loop_handle: None,
            validation_config: ValidationConfig::default(),
        }
    }

    /// Set validation configuration
    pub fn with_validation_config(mut self, config: ValidationConfig) -> Self {
        self.validation_config = config;
        self
    }

    /// Enable or disable request validation
    pub fn validate_requests(mut self, validate: bool) -> Self {
        self.validation_config.validate_requests = validate;
        self
    }

    /// Enable or disable response validation
    pub fn validate_responses(mut self, validate: bool) -> Self {
        self.validation_config.validate_responses = validate;
        self
    }

    /// Start the message handling loop in a background task
    pub fn start_message_loop(&mut self) -> Result<(), Error> {
        if self.message_loop_handle.is_some() {
            return Err(Error::Other("Message loop already started".to_string()));
        }

        let transport = self.transport.clone();
        let pending_requests = self.pending_requests.clone();
        let notification_listeners = self.notification_listeners.clone();

        let handle = tokio::spawn(async move {
            tracing::info!("Starting client message loop");
            loop {
                // Use a timeout to periodically check for messages and ensure we don't hold the lock indefinitely
                let mut transport_guard = transport.lock().await;

                let message_result = match
                    tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        transport_guard.receive()
                    ).await
                {
                    Ok(result) => {
                        // Received a message or error before timeout
                        drop(transport_guard);

                        result
                    }
                    Err(_) => {
                        // Timeout occurred, release lock and continue
                        drop(transport_guard);
                        tracing::debug!("Message loop released transport lock after timeout");
                        // Small sleep to avoid tight loop
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        continue;
                    }
                };

                // Process the message or error
                match message_result {
                    Ok((_, message)) =>
                        match message {
                            McpMessage::Response(response) => {
                                tracing::debug!("Received response with id={}", response.id);
                                let mut requests = pending_requests.lock().await;
                                let sender = requests.remove(&response.id);
                                tracing::debug!(
                                    "Sender found for id={}: {}",
                                    response.id,
                                    sender.is_some()
                                );
                                if let Some(sender) = sender {
                                    if sender.send(Ok(response.clone())).is_err() {
                                        tracing::warn!(
                                            "Failed to send response to oneshot channel for id={}",
                                            response.id
                                        );
                                    }
                                } else {
                                    tracing::warn!(
                                        "No sender found for response id={}",
                                        response.id
                                    );
                                }
                            }
                            McpMessage::Notification(notification) => {
                                tracing::debug!("Received notification: {}", notification.method);
                                let listeners = notification_listeners.lock().await.clone();
                                for listener in listeners {
                                    let _ = listener.try_send(notification.clone());
                                }
                            }
                            McpMessage::Request(_) => {
                                tracing::warn!("Client received unexpected request");
                            }
                        }
                    Err(e) => {
                        tracing::error!("Message loop error: {}", e);
                        return Err(e);
                    }
                }
            }
        });

        self.message_loop_handle = Some(handle);
        Ok(())
    }
    /// Stop the message handling loop
    pub async fn stop_message_loop(&mut self) -> Result<(), Error> {
        if let Some(handle) = self.message_loop_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        Ok(())
    }

    /// Send a request and wait for a response
    pub async fn send_request<P, R>(&self, method: &str, params: P) -> Result<R, Error>
        where P: Serialize + Send + Sync, R: DeserializeOwned + Send + Sync
    {
        let id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        tracing::debug!("Preparing request id={} for method={}", id, method);

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params: Some(serde_json::to_value(params).map_err(Error::Json)?),
        };

        if let Err(e) = validate_request(&request, &self.validation_config) {
            tracing::error!("Request validation failed: {}", e);
            return Err(e);
        }

        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            tracing::debug!("Registering request id={} in pending_requests", id);
            pending.insert(id, tx);
        }

        tracing::debug!("About to acquire transport lock for request id={}", id);
        let send_result = {
            let start = std::time::Instant::now();

            // Use a timeout to acquire the transport lock to avoid deadlocks
            let transport_lock_result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                self.transport.lock()
            ).await;

            match transport_lock_result {
                Ok(mut transport_guard) => {
                    let elapsed = start.elapsed();
                    tracing::debug!(
                        "Acquired transport lock for request id={} after {:?}",
                        id,
                        elapsed
                    );

                    tracing::debug!("Sending request id={} via transport", id);
                    let result = transport_guard.send(&McpMessage::Request(request)).await;
                    tracing::debug!(
                        "Transport.send completed for request id={}, result: {:?}",
                        id,
                        result.is_ok()
                    );
                    result
                }
                Err(_) => {
                    tracing::error!(
                        "Timeout waiting to acquire transport lock for request id={}",
                        id
                    );
                    Err(Error::Transport("Timeout waiting to acquire transport lock".to_string()))
                }
            }
        };
        tracing::debug!("Released transport lock for request id={}", id);

        if let Err(e) = send_result {
            tracing::error!("Failed to send request id={}: {}", id, e);
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&id);
            return Err(e);
        }

        tracing::debug!("Request id={} sent successfully, waiting for response", id);

        let response = tokio::time
            ::timeout(std::time::Duration::from_secs(30), rx).await
            .map_err(|_| {
                Error::Transport(format!("Timeout waiting for response to request id={}", id))
            })?
            .map_err(|_| Error::Transport("Response channel closed".to_string()))?;

        let response = response?;

        if self.validation_config.validate_responses {
            if let Err(e) = validate_response(&response, method, &self.validation_config) {
                tracing::error!("Response validation failed: {}", e);
                return Err(e);
            }
        }

        match response.outcome {
            ResponseOutcome::Success { result } => {
                tracing::debug!("Received successful response for id={}", id);
                Ok(serde_json::from_value(result).map_err(Error::Json)?)
            }
            ResponseOutcome::Error { error } => {
                tracing::error!(
                    "Received error response for id={}: {} (code: {})",
                    id,
                    error.message,
                    error.code
                );
                Err(
                    Error::Protocol(
                        format!("Server returned error: {} (code: {})", error.message, error.code)
                    )
                )
            }
        }
    }

    /// Register a listener for notifications
    pub async fn register_notification_listener(&self) -> mpsc::Receiver<Notification> {
        let (tx, rx) = mpsc::channel(100);

        let mut listeners = self.notification_listeners.lock().await;
        listeners.push(tx);

        rx
    }

    /// Initialize the connection with the server
    pub async fn initialize(
        &self,
        params: InitializeRequestParams
    ) -> Result<InitializeResult, Error> {
        self.send_request("initialize", params).await
    }

    /// List available resources
    pub async fn list_resources(
        &self,
        params: ListResourcesParams
    ) -> Result<ListResourcesResult, Error> {
        self.send_request("resources/list", params).await
    }

    /// Get a resource by URI
    pub async fn read_resource(
        &self,
        params: ReadResourceParams
    ) -> Result<ReadResourceResult, Error> {
        self.send_request("resources/get", params).await
    }

    /// Create a new resource
    pub async fn create_resource(
        &self,
        params: CreateResourceParams
    ) -> Result<CreateResourceResult, Error> {
        self.send_request("resources/create", params).await
    }

    /// Update an existing resource
    pub async fn update_resource(
        &self,
        params: UpdateResourceParams
    ) -> Result<UpdateResourceResult, Error> {
        self.send_request("resources/update", params).await
    }

    /// Delete a resource
    pub async fn delete_resource(
        &self,
        params: DeleteResourceParams
    ) -> Result<DeleteResourceResult, Error> {
        self.send_request("resources/delete", params).await
    }

    /// List available prompts
    pub async fn list_prompts(
        &self,
        params: ListPromptsParams
    ) -> Result<ListPromptsResult, Error> {
        self.send_request("prompts/list", params).await
    }

    /// List available tools
    pub async fn list_tools(&self, params: ListToolsParams) -> Result<ListToolsResult, Error> {
        self.send_request("tools/list", params).await
    }

    /// Call a tool
    pub async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error> {
        self.send_request("tools/call", params).await
    }

    /// Close the client and its transport
    pub async fn close(&self) -> Result<(), Error> {
        let mut transport = self.transport.lock().await;
        transport.close().await
    }

    /// Create a FileResource instance without uploading it
    pub fn file_resource(
        &self,
        name: &str,
        path: &std::path::Path,
        description: Option<String>,
        mime_type: Option<String>,
        is_binary: bool
    ) -> Result<FileResource, Error> {
        FileResource::new(None, name.to_string(), path, description, mime_type, is_binary).map_err(
            |e| Error::Resource(format!("Failed to create file resource: {}", e))
        )
    }
}

/// Builder for creating and configuring a client
pub struct ClientBuilder<T> {
    transport: T,
}

impl<T: Transport + 'static> ClientBuilder<T> {
    /// Create a new client builder with the given transport
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Build the client and start its message loop
    pub async fn build(self) -> Result<Client<T>, Error> {
        let mut client = Client::new(self.transport);
        client.start_message_loop()?;
        Ok(client)
    }
}

/// A session-oriented wrapper for the MCP client
///
/// This provides a higher-level session-based interface for working with MCP servers,
/// including support for file resources, progress notifications, and message creation.
pub struct ClientSession<T: Transport + 'static> {
    /// The underlying client
    client: Client<T>,
    /// Session state information
    session_info: Arc<Mutex<Option<InitializeResult>>>,
    /// Indicates whether the session is started
    is_started: bool,
    /// Server information
    server_info: Option<InitializeResult>,
}

impl<T: Transport + 'static> ClientSession<T> {
    /// Create a new client session with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            client: Client::new(transport),
            session_info: Arc::new(Mutex::new(None)),
            is_started: false,
            server_info: None,
        }
    }

    /// Configure validation for this session
    pub fn with_validation_config(self, config: ValidationConfig) -> Self {
        Self {
            client: self.client.with_validation_config(config),
            session_info: self.session_info,
            is_started: self.is_started,
            server_info: self.server_info,
        }
    }

    /// Start the session by initializing the message loop
    pub async fn start(&mut self) -> Result<(), Error> {
        if self.is_started {
            tracing::warn!("ClientSession.start() called but session is already started");
            return Ok(());
        }

        tracing::info!("Starting client session message loop");
        self.client.start_message_loop()?;
        self.is_started = true;
        Ok(())
    }

    /// Initialize the connection with the server and store session info
    pub async fn initialize(
        &mut self,
        params: InitializeRequestParams
    ) -> Result<InitializeResult, Error> {
        if !self.is_started {
            return Err(
                Error::Protocol(
                    "Session not started. Call start() before making requests.".to_string()
                )
            );
        }

        tracing::info!("Sending initialize request");
        let result = self.client.initialize(params).await?;
        self.server_info = Some(result.clone());
        tracing::info!("Initialize request completed successfully");
        Ok(result)
    }

    /// Get the current session information if initialized
    pub async fn session_info(&self) -> Option<InitializeResult> {
        let session_info = self.session_info.lock().await;
        session_info.clone()
    }

    /// Close the session, stopping the message loop and closing the transport
    pub async fn close(&mut self) -> Result<(), Error> {
        self.client.stop_message_loop().await?;
        self.client.close().await
    }

    /// Send a request to the server and await the response
    pub async fn send_request<P, R>(&self, method: &str, params: P) -> Result<R, Error>
        where P: Serialize + Send + Sync, R: DeserializeOwned + Send + Sync
    {
        self.client.send_request(method, params).await
    }

    /// Register a listener for notifications
    pub async fn register_notification_listener(&self) -> mpsc::Receiver<Notification> {
        self.client.register_notification_listener().await
    }

    //==== File Resource Methods ====

    /// Create a file resource from a local path
    pub async fn create_file_resource(
        &self,
        name: &str,
        path: &std::path::Path,
        description: Option<String>,
        mime_type: Option<String>
    ) -> Result<CreateResourceResult, Error> {
        // Create a FileResource object
        let file_resource = FileResource::new(
            None,
            name.to_string(),
            path,
            description,
            mime_type,
            false // Not binary
        ).map_err(|e| Error::Resource(format!("Failed to create file resource: {}", e)))?;

        // Convert to params and create on the server
        let params = file_resource
            .to_create_params()
            .map_err(|e| Error::Resource(format!("Failed to read file: {}", e)))?;

        self.client.create_resource(params).await
    }

    /// Create a binary file resource from a local path
    pub async fn create_binary_file_resource(
        &self,
        name: &str,
        path: &std::path::Path,
        description: Option<String>,
        mime_type: Option<String>
    ) -> Result<CreateResourceResult, Error> {
        // Create a FileResource object
        let file_resource = FileResource::new(
            None,
            name.to_string(),
            path,
            description,
            mime_type,
            true // Binary
        ).map_err(|e| Error::Resource(format!("Failed to create file resource: {}", e)))?;

        // Convert to params and create on the server
        let params = file_resource
            .to_create_params()
            .map_err(|e| Error::Resource(format!("Failed to read file: {}", e)))?;

        self.client.create_resource(params).await
    }

    //==== Tools and Notifications Methods ====

    /// Call a tool with the given parameters
    pub async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error> {
        self.client.call_tool(params).await
    }

    /// Send a progress notification to the server
    pub async fn send_progress_notification(
        &self,
        tool_name: &str,
        progress: f64,
        message: Option<String>
    ) -> Result<(), Error> {
        // Create the notification message
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "tools/progress".to_string(),
            params: Some(
                serde_json::json!({
                "tool_name": tool_name,
                "progress": progress,
                "message": message
            })
            ),
        };

        // Send the notification
        let mut transport = self.client.transport.lock().await;
        transport.send(&McpMessage::Notification(notification)).await
    }

    //==== Standard MCP Operations ====

    /// List available resources
    pub async fn list_resources(
        &self,
        params: ListResourcesParams
    ) -> Result<ListResourcesResult, Error> {
        self.client.list_resources(params).await
    }

    /// Get a specific resource
    pub async fn read_resource(
        &self,
        params: ReadResourceParams
    ) -> Result<ReadResourceResult, Error> {
        self.client.read_resource(params).await
    }

    /// Create a new resource
    pub async fn create_resource(
        &self,
        params: CreateResourceParams
    ) -> Result<CreateResourceResult, Error> {
        self.client.create_resource(params).await
    }

    /// Update an existing resource
    pub async fn update_resource(
        &self,
        params: UpdateResourceParams
    ) -> Result<UpdateResourceResult, Error> {
        self.client.update_resource(params).await
    }

    /// Delete a resource
    pub async fn delete_resource(
        &self,
        params: DeleteResourceParams
    ) -> Result<DeleteResourceResult, Error> {
        self.client.delete_resource(params).await
    }

    /// List available prompts
    pub async fn list_prompts(
        &self,
        params: ListPromptsParams
    ) -> Result<ListPromptsResult, Error> {
        self.client.list_prompts(params).await
    }

    /// List available tools
    pub async fn list_tools(&self, params: ListToolsParams) -> Result<ListToolsResult, Error> {
        self.client.list_tools(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::error_codes;
    use crate::types::protocol::error_response;
    use async_trait::async_trait;
    use std::sync::atomic::{ AtomicBool, Ordering };

    struct MockTransport {
        connected: Arc<AtomicBool>,
        response: Option<McpMessage>,
    }

    impl MockTransport {
        fn new(response: Option<McpMessage>) -> Self {
            Self {
                connected: Arc::new(AtomicBool::new(true)),
                response,
            }
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn receive(&mut self) -> Result<(Option<String>, McpMessage), Error> {
            if let Some(response) = self.response.take() {
                return Ok((None, response));
            }
            Err(Error::Transport("No more messages".to_string()))
        }

        async fn send(&mut self, _message: &McpMessage) -> Result<(), Error> {
            Ok(())
        }

        async fn send_to(&mut self, _client_id: &str, message: &McpMessage) -> Result<(), Error> {
            // For client transports, send_to is the same as send
            self.send(message).await
        }

        async fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        async fn close(&mut self) -> Result<(), Error> {
            self.connected.store(false, Ordering::SeqCst);
            Ok(())
        }

        async fn start(&mut self) -> Result<(), Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_send_request_success() {
        let response = McpMessage::Response(Response {
            jsonrpc: "2.0".to_string(),
            id: 1,
            outcome: ResponseOutcome::Success {
                result: serde_json::json!({"protocol_version": "1.0"}),
            },
        });

        let transport = MockTransport::new(Some(response));
        let client = Client::new(transport);

        let result: InitializeResult = client
            .send_request("initialize", InitializeRequestParams {
                protocol_version: "1.0".to_string(),
                capabilities: crate::types::initialize::ClientCapabilities {
                    roots: None,
                    experimental: None,
                    sampling: None,
                },
                client_info: None,
            }).await
            .unwrap();

        assert_eq!(result.protocol_version, "1.0");
    }

    #[tokio::test]
    async fn test_send_request_error() {
        let response = McpMessage::Response(
            error_response(1, error_codes::METHOD_NOT_FOUND, "Method not found", None)
        );

        let transport = MockTransport::new(Some(response));
        let client = Client::new(transport);

        let result = client.send_request::<(), ()>("unknown_method", ()).await;

        assert!(result.is_err());
    }
}
