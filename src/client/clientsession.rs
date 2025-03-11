//! MCP Client Session
//!
//! This module provides the high-level API for interacting with an MCP server.
//! It wraps the lower-level Client and provides convenient methods for common operations.

use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{ debug, warn, error, info };

use crate::client::client::{ Client, ClientConfig };
use crate::client::model::ServerInfo;
use crate::client::services::{
    notification::NotificationRouter,
    progress::{ ProgressInfo, ProgressTracker },
    subscription::{ Subscription, SubscriptionManager },
    lifecycle::{ LifecycleState, LifecycleManager },
};
use crate::protocol::{
    CallToolParams,
    CallToolResult,
    ClientCapabilities,
    CompleteParams,
    CompleteResult,
    Error,
    GetPromptResult,
    Implementation,
    InitializeParams,
    InitializeResult,
    JSONRPCNotification,
    ListPromptsResult,
    ListResourceTemplatesResult,
    ListResourcesResult,
    ListToolsResult,
    Message,
    Method,
    PROTOCOL_VERSION,
    ReadResourceParams,
    ReadResourceResult,
    RequestId,
    ResourceListChangedNotification,
    ResourceUpdatedNotification,
};

use super::transport::Transport;
use crate::client::transport::ConnectionStatus;

/// Configuration for client session
#[derive(Debug, Clone)]
pub struct ClientSessionConfig {
    /// Client information
    pub client_info: Implementation,
}

impl Default for ClientSessionConfig {
    fn default() -> Self {
        Self {
            client_info: Implementation {
                name: "mcp-rs".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}

/// Main client session for handling MCP protocol communication
pub struct ClientSession {
    /// The client instance
    client: Arc<Client>,
    /// Subscription manager
    subscription_manager: Arc<SubscriptionManager>,
    /// Progress tracker
    progress_tracker: Arc<ProgressTracker>,
    /// Session configuration
    config: ClientSessionConfig,
    /// Server information
    server_info: RwLock<Option<ServerInfo>>,
}

impl ClientSession {
    /// Create a new client session with the given transport
    pub fn new(transport: Box<dyn Transport + 'static>) -> Self {
        debug!("Creating ClientSession with Transport");

        // Create the client with the transport
        let client = Arc::new(Client::new(transport, ClientConfig::default()));

        // Create separate notification router instead of using client's to avoid circular references
        let notification_router = Arc::new(NotificationRouter::new());

        Self {
            client,
            subscription_manager: Arc::new(SubscriptionManager::new(notification_router)),
            progress_tracker: Arc::new(ProgressTracker::new()),
            config: ClientSessionConfig::default(),
            server_info: RwLock::new(None),
        }
    }

    /// Create a new client session builder
    pub fn builder(transport: Box<dyn Transport + 'static>) -> ClientSessionBuilder {
        // Create a builder with the transport
        ClientSessionBuilder::new(transport)
    }

    /// Initialize the client session
    pub async fn initialize(&self) -> Result<InitializeResult, Error> {
        debug!("Initializing MCP client session");

        // First, ensure the client and transport are started and connected
        // This will wait for the transport to be connected and ready
        // The client should still be in Created state
        info!("Starting client transport");
        if let Err(e) = self.client.start().await {
            error!("Failed to start client: {}", e);
            return Err(e);
        }

        // The transport should be ready at this point because the client.start() method
        // already waits for the transport to be fully connected and ready
        info!("Transport is ready, proceeding with initialization");

        // Get the lifecycle manager to track state
        let lifecycle = self.client.lifecycle();
        let current_state = lifecycle.current_state().await;

        // Check current state - must be in Initialization state to initialize
        if current_state != LifecycleState::Initialization {
            return Err(
                Error::Protocol(format!("Cannot initialize client in state {:?}", current_state))
            );
        }

        // According to the MCP spec, we should be in Initialization state to send Initialize request,
        // but our validate_method will only allow Initialize in Initialization state.
        // We're already in the correct state, so no need to transition.

        // Create initialization parameters according to the spec
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            client_info: self.config.client_info.clone(),
            capabilities: ClientCapabilities {
                sampling: None,
                roots: None,
                experimental: None,
            },
        };

        // Send initialization request
        // The client will validate and enforce state transitions
        info!("Sending Initialize request");
        let result = match
            self.send_request::<InitializeParams, InitializeResult>(
                Method::Initialize,
                params
            ).await
        {
            Ok(result) => {
                info!("Received Initialize response - server is initialized");

                // Store the server info for later use - convert Implementation to ServerInfo
                {
                    let server_info = ServerInfo {
                        name: result.server_info.name.clone(),
                        version: result.server_info.version.clone(),
                    };

                    let mut server_info_guard = self.server_info.write().await;
                    *server_info_guard = Some(server_info);
                }

                // After receiving the initialize response, the server is in ServerInitialized state
                // but the client is not yet fully initialized - we need to send the initialized notification
                info!("Sending initialized notification");
                let notification = LifecycleManager::create_initialized_notification();
                self.client.send_raw_message(notification).await?;

                info!("Client initialization complete");
                Ok(result)
            }
            Err(e) => {
                // On error, try to stay in Initialization state
                error!("Initialize request failed: {}", e);
                if
                    let Err(transition_err) = lifecycle.transition_to(
                        LifecycleState::Initialization
                    ).await
                {
                    warn!("Failed to transition to Initialization state: {}", transition_err);
                }
                Err(e)
            }
        };

        // Return the initialize result
        result
    }

    /// Shutdown the client session
    pub async fn shutdown(&self) -> Result<(), Error> {
        // Shutdown the client
        self.client.shutdown().await
    }

    /// Get the server info
    pub async fn server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().await.clone()
    }

    /// Check if the client session is connected
    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Subscribe to connection status updates
    pub fn subscribe_status(&self) -> tokio::sync::broadcast::Receiver<ConnectionStatus> {
        self.client.subscribe_status()
    }

    /// Check if the server has a specific capability
    pub async fn has_capability(&self, capability: &str) -> bool {
        debug!("has_capability called - not implemented");
        false
    }

    /// Send a high-level Message through the client
    pub async fn send_message(&self, message: Message, id: Option<RequestId>) -> Result<(), Error> {
        self.client.send_message(message, id).await
    }

    /// Send a request to the server and wait for a response
    pub async fn send_request<P, R>(&self, method: Method, params: P) -> Result<R, Error>
        where P: serde::Serialize + Send + Sync, R: serde::de::DeserializeOwned + Send + Sync
    {
        // Double-check connection status directly
        debug!("Client session checking connection status before sending request");
        let is_connected = self.is_connected();
        if !is_connected {
            return Err(
                Error::Transport("Transport not connected - cannot send request".to_string())
            );
        }

        // Delegate to the client - handle the error but don't shutdown
        let result = self.client.send_request(method.clone(), params).await;

        if let Err(ref e) = result {
            // Log the error but don't shutdown the client
            error!("Error sending request {}: {}", method, e);
        }

        result
    }

    /// List resources
    pub async fn list_resources(
        &self,
        params: Option<serde_json::Value>
    ) -> Result<ListResourcesResult, Error> {
        // Create the query parameters
        let list_params = params.unwrap_or_else(||
            serde_json::Value::Object(serde_json::Map::new())
        );

        // Use the implemented send_request method to pass to the client
        self.send_request(Method::ResourcesList, list_params).await
    }

    /// List resource templates
    pub async fn list_resource_templates(&self) -> Result<ListResourceTemplatesResult, Error> {
        self.send_request(Method::ResourcesTemplatesList, ()).await
    }

    /// Read a resource
    pub async fn read_resource(
        &self,
        params: ReadResourceParams
    ) -> Result<ReadResourceResult, Error> {
        self.send_request(Method::ResourcesRead, params).await
    }

    /// Create a resource
    pub async fn create_resource(
        &self,
        params: serde_json::Value
    ) -> Result<ReadResourceResult, Error> {
        // Use the send_request method to call the resource creation endpoint
        self.send_request(Method::ResourcesCreate, params).await
    }

    /// Update a resource
    pub async fn update_resource(
        &self,
        params: serde_json::Value
    ) -> Result<ReadResourceResult, Error> {
        // Use the send_request method to call the resource update endpoint
        self.send_request(Method::ResourcesUpdate, params).await
    }

    /// Delete a resource
    pub async fn delete_resource(&self, uri: String) -> Result<(), Error> {
        // Create params object with the URI
        let params = serde_json::json!({ "uri": uri });

        // Use the send_request method to call the resource deletion endpoint
        self.send_request(Method::ResourcesDelete, params).await
    }

    /// List prompts
    pub async fn list_prompts(&self) -> Result<ListPromptsResult, Error> {
        // Use the send_request method to call the prompts listing endpoint
        self.send_request(Method::PromptsList, ()).await
    }

    /// Get a prompt by ID with optional arguments
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>
    ) -> Result<GetPromptResult, Error> {
        // Create the parameters
        let params =
            serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        // Send the request
        self.send_request(Method::PromptsGet, params).await
    }

    /// List tools
    pub async fn list_tools(&self) -> Result<ListToolsResult, Error> {
        debug!("list_tools called - this is a placeholder implementation");
        Err(Error::Other("list_tools not implemented yet".to_string()))
    }

    /// Call a tool
    pub async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error> {
        self.send_request(Method::ToolsCall, params).await
    }

    /// Generate completions
    pub async fn complete(&self, params: CompleteParams) -> Result<CompleteResult, Error> {
        // Send the completion request
        self.send_request(Method::CompletionComplete, params).await
    }

    /// Subscribe to all notifications
    pub async fn subscribe_all(&self) -> Subscription<JSONRPCNotification> {
        // Delegate to the subscription manager
        self.subscription_manager.subscribe_all().await
    }

    /// Subscribe to progress notifications
    pub async fn subscribe_progress(&self) -> Subscription<ProgressInfo> {
        // Get a progress tracker subscription
        let progress_tracker = self.progress_tracker.subscribe();

        // Create a subscription adapter that converts from broadcast to mpsc
        self.subscription_manager.create_subscription(progress_tracker, "progress".to_string())
    }

    /// Subscribe to resource list changes
    pub async fn subscribe_resource_list_changes(
        &self
    ) -> Subscription<ResourceListChangedNotification> {
        self.subscription_manager.subscribe_resource_list_changes().await
    }

    /// Subscribe to resource updates
    pub async fn subscribe_resource_updates(&self) -> Subscription<ResourceUpdatedNotification> {
        // Delegate to the subscription manager
        self.subscription_manager.subscribe_resource_updates().await
    }

    /// Wait for a progress operation to complete
    pub async fn wait_for_progress(&self, token: &str) -> Result<ProgressInfo, Error> {
        // Use the correct method name for the progress tracker
        self.progress_tracker.wait_for_completion(token).await
    }
}

/// Builder for creating ClientSession instances
pub struct ClientSessionBuilder {
    /// Transport to use for communication
    transport: Box<dyn Transport + 'static>,

    /// Session configuration
    config: ClientSessionConfig,
}

impl ClientSessionBuilder {
    /// Create a new client session builder
    pub fn new(transport: Box<dyn Transport + 'static>) -> Self {
        Self {
            transport,
            config: ClientSessionConfig::default(),
        }
    }

    /// Set the client name
    pub fn name(mut self, name: String) -> Self {
        self.config.client_info.name = name;
        self
    }

    /// Set the client version
    pub fn version(mut self, version: String) -> Self {
        self.config.client_info.version = version;
        self
    }

    /// Build the client session with the configured options
    pub fn build(self) -> ClientSession {
        let client = Arc::new(Client::new(self.transport, ClientConfig::default()));

        // Create the session with the client
        let session = ClientSession {
            client: client.clone(),
            subscription_manager: Arc::new(SubscriptionManager::new(client.notification_router())),
            progress_tracker: Arc::new(ProgressTracker::new()),
            config: self.config,
            server_info: RwLock::new(None),
        };

        // Note: The session must be initialized with session.initialize() before use
        tracing::debug!("ClientSession built - call initialize() before using");

        session
    }
}

/// Guard for a client session that ensures proper shutdown
pub struct ClientSessionGuard {
    /// The client session
    session: ClientSession,
}

impl ClientSessionGuard {
    /// Create a new client session guard
    pub fn new(session: ClientSession) -> Self {
        Self { session }
    }

    /// Get the client session
    pub fn session(&self) -> &ClientSession {
        &self.session
    }
}

impl Drop for ClientSessionGuard {
    fn drop(&mut self) {
        // When guard is dropped, spawn a blocking task to ensure shutdown
        let session = self.session.clone();
        tokio::spawn(async move {
            if let Err(e) = session.shutdown().await {
                warn!("Error during client session shutdown: {}", e);
            }
        });
    }
}

/// Allow cloning the client session
impl Clone for ClientSession {
    fn clone(&self) -> Self {
        // Note: This clone implementation is limited and will only clone
        // the surface-level references - not the actual underlying client
        debug!("Cloning ClientSession - note this is limited functionality");
        Self {
            client: self.client.clone(),
            subscription_manager: self.subscription_manager.clone(),
            progress_tracker: self.progress_tracker.clone(),
            config: self.config.clone(),
            server_info: RwLock::new(
                tokio::runtime::Handle
                    ::current()
                    .block_on(async { self.server_info.read().await.clone() })
            ),
        }
    }
}
