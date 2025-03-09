//! MCP Client Session
//!
//! This module provides the high-level API for interacting with an MCP server.
//! It wraps the lower-level Client and provides convenient methods for common operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{ mpsc, oneshot, RwLock };
use serde_json;
use tracing::{ debug, warn };
use std::time::SystemTime;

use crate::protocol::{
    CallToolParams,
    CallToolResult,
    ClientRequest,
    ClientMessage,
    ClientNotification,
    CompleteParams,
    CompleteResult,
    Error,
    GetPromptParams,
    GetPromptResult,
    Implementation,
    InitializeParams,
    InitializeRequest,
    InitializeResult,
    InitializedNotification,
    JSONRPCMessage,
    JSONRPCNotification,
    JSONRPCRequest,
    ListPromptsRequest,
    ListPromptsResult,
    ListResourceTemplatesRequest,
    ListResourceTemplatesResult,
    ListResourcesRequest,
    ListResourcesResult,
    ListToolsRequest,
    ListToolsResult,
    Message,
    ProgressNotification,
    ReadResourceParams,
    ReadResourceRequest,
    ReadResourceResult,
    RequestId,
    ResourceListChangedNotification,
    ResourceUpdatedNotification,
    ClientCapabilities,
    PROTOCOL_VERSION,
};
use crate::transport::{ DirectIOTransport, BoxedDirectIOTransport };
use crate::client::client::{ Client, ClientConfig };
use crate::client::progress::{ ProgressInfo, ProgressTracker };
use crate::client::subscription::{ Subscription, SubscriptionManager };
use crate::client::model::ServerInfo;
use crate::client::notification::NotificationRouter;

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
    client: Arc<Client<BoxedDirectIOTransport>>,
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
    pub fn new(transport: Box<dyn DirectIOTransport + 'static>) -> Self {
        debug!("Creating ClientSession with BoxedDirectIOTransport");

        // Wrap the boxed transport in our wrapper type
        let boxed_transport = BoxedDirectIOTransport(transport);

        // Create the client with the wrapped transport
        let client = Arc::new(Client::new(boxed_transport, ClientConfig::default()));

        // Create the notification router from the client
        let notification_router = client.notification_router().clone();

        Self {
            client,
            subscription_manager: Arc::new(SubscriptionManager::new(notification_router)),
            progress_tracker: Arc::new(ProgressTracker::new()),
            config: ClientSessionConfig::default(),
            server_info: RwLock::new(None),
        }
    }

    /// Create a new client session builder
    pub fn builder(transport: Box<dyn DirectIOTransport + 'static>) -> ClientSessionBuilder {
        // Wrap the transport in our wrapper type
        let boxed_transport = BoxedDirectIOTransport(transport);

        // Create a builder with the wrapped transport
        ClientSessionBuilder::new(boxed_transport)
    }

    /// Generate a unique request ID based on current timestamp
    fn generate_id(&self) -> RequestId {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        RequestId::Number((now.as_secs() as i64) * 1000 + (now.subsec_millis() as i64))
    }

    /// Initialize the client session
    pub async fn initialize(&self) -> Result<InitializeResult, Error> {
        // Create initialize params
        let params = InitializeParams {
            client_info: self.config.client_info.clone(),
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities {
                sampling: None,
                roots: None,
                experimental: None,
            },
        };

        // For now, return a dummy initialize result since we can't access client.send_request
        debug!("initialize called - this is a placeholder implementation");
        Err(Error::Other("InitializeResult not implemented yet".to_string()))
    }

    /// Shutdown the client session
    pub async fn shutdown(&self) -> Result<(), Error> {
        debug!("shutdown called - not implemented");
        Ok(())
    }

    /// Get the server info
    pub async fn server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().await.clone()
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

    /// Send a request with generic params and response types
    pub async fn send_request<P, R>(&self, method: &str, params: P) -> Result<R, Error>
        where P: serde::Serialize + Send + Sync, R: serde::de::DeserializeOwned + Send + Sync
    {
        // Create a request ID
        let request_id = self.generate_id();

        // Convert params to Value
        let params_value = match serde_json::to_value(params) {
            Ok(value) => Some(value),
            Err(e) => {
                return Err(Error::Other(format!("Failed to serialize params: {}", e)));
            }
        };

        // Create a JSONRPCRequest
        let request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id.clone(),
            method: method.to_string(),
            params: params_value,
        };

        // For now, return an error since we can't access client methods directly
        debug!("send_request called for method {} - this is a placeholder implementation", method);
        Err(Error::Other(format!("Method {} not implemented yet", method)))
    }

    /// List resources
    pub async fn list_resources(
        &self,
        params: Option<serde_json::Value>
    ) -> Result<ListResourcesResult, Error> {
        // Create the query parameters
        let list_params = params.unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        // For now, return a placeholder error since we can't access the client methods
        debug!("list_resources called - this is a placeholder implementation");
        Err(Error::Other("list_resources not implemented yet".to_string()))
    }

    /// List resource templates
    pub async fn list_resource_templates(&self) -> Result<ListResourceTemplatesResult, Error> {
        debug!("list_resource_templates called - this is a placeholder implementation");
        Err(Error::Other("list_resource_templates not implemented yet".to_string()))
    }

    /// Read a resource
    pub async fn read_resource(
        &self,
        params: ReadResourceParams
    ) -> Result<ReadResourceResult, Error> {
        debug!("read_resource called - this is a placeholder implementation");
        Err(Error::Other("read_resource not implemented yet".to_string()))
    }

    /// Create a resource
    pub async fn create_resource(
        &self,
        params: serde_json::Value
    ) -> Result<ReadResourceResult, Error> {
        debug!("create_resource called - this is a placeholder implementation");
        Err(Error::Other("create_resource not implemented yet".to_string()))
    }

    /// Update a resource
    pub async fn update_resource(
        &self,
        params: serde_json::Value
    ) -> Result<ReadResourceResult, Error> {
        debug!("update_resource called - this is a placeholder implementation");
        Err(Error::Other("update_resource not implemented yet".to_string()))
    }

    /// Delete a resource
    pub async fn delete_resource(&self, uri: String) -> Result<(), Error> {
        debug!("delete_resource called - this is a placeholder implementation");
        Err(Error::Other("delete_resource not implemented yet".to_string()))
    }

    /// List prompts
    pub async fn list_prompts(&self) -> Result<ListPromptsResult, Error> {
        debug!("list_prompts called - this is a placeholder implementation");
        Err(Error::Other("list_prompts not implemented yet".to_string()))
    }

    /// Get a prompt by ID with optional arguments
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>
    ) -> Result<GetPromptResult, Error> {
        debug!("get_prompt called - this is a placeholder implementation");
        Err(Error::Other("get_prompt not implemented yet".to_string()))
    }

    /// List tools
    pub async fn list_tools(&self) -> Result<ListToolsResult, Error> {
        debug!("list_tools called - this is a placeholder implementation");
        Err(Error::Other("list_tools not implemented yet".to_string()))
    }

    /// Call a tool
    pub async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error> {
        debug!("call_tool called - this is a placeholder implementation");
        Err(Error::Other("call_tool not implemented yet".to_string()))
    }

    /// Generate completions
    pub async fn complete(&self, params: CompleteParams) -> Result<CompleteResult, Error> {
        debug!("complete called - this is a placeholder implementation");
        Err(Error::Other("complete not implemented yet".to_string()))
    }

    /// Subscribe to all notifications
    pub async fn subscribe_all(&self) -> Subscription<JSONRPCNotification> {
        self.subscription_manager.subscribe_all().await
    }

    /// Subscribe to progress notifications
    pub async fn subscribe_progress(&self) -> Subscription<ProgressInfo> {
        // For now, just use a simple implementation that forwards everything
        let progress_sub = self.progress_tracker.subscribe();

        // Convert to subscription type
        self.subscription_manager.create_subscription(progress_sub, "progress".to_string())
    }

    /// Subscribe to resource list changes
    pub async fn subscribe_resource_list_changes(
        &self
    ) -> Subscription<ResourceListChangedNotification> {
        self.subscription_manager.subscribe_resource_list_changes().await
    }

    /// Subscribe to resource updates
    pub async fn subscribe_resource_updates(&self) -> Subscription<ResourceUpdatedNotification> {
        self.subscription_manager.subscribe_resource_updates().await
    }

    /// Wait for a progress operation to complete
    pub async fn wait_for_progress(&self, token: &str) -> Result<ProgressInfo, Error> {
        self.progress_tracker.wait_for_completion(token).await
    }
}

/// Builder for creating ClientSession instances with BoxedDirectIOTransport
pub struct ClientSessionBuilder {
    /// Transport to use for communication
    transport: BoxedDirectIOTransport,

    /// Session configuration
    config: ClientSessionConfig,
}

impl ClientSessionBuilder {
    /// Create a new client session builder
    pub fn new(transport: BoxedDirectIOTransport) -> Self {
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

    /// Build the client session with the current configuration
    pub fn build(self) -> ClientSession {
        debug!("Building ClientSession from builder");

        // Create the client with the wrapped transport
        let client = Arc::new(Client::new(self.transport, ClientConfig::default()));

        // Create the notification router from the client
        let notification_router = client.notification_router().clone();

        ClientSession {
            client,
            subscription_manager: Arc::new(SubscriptionManager::new(notification_router)),
            progress_tracker: Arc::new(ProgressTracker::new()),
            config: self.config,
            server_info: RwLock::new(None),
        }
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
