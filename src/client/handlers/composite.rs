//! Composite Client Handler
//!
//! This module provides a composite handler that delegates to domain-specific handlers.

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::client::Client;
use crate::client::handlers::{
    completion::CompletionHandler,
    handshake::HandshakeHandler,
    prompts::PromptHandler,
    route_handler::RouteHandler,
    tools::ToolHandler,
};
use crate::client::services::ServiceProvider;
use crate::client::services::lifecycle::LifecycleState;
use crate::protocol::{
    CallToolParams,
    CallToolResult,
    CompleteParams,
    CompleteResult,
    Error,
    GetPromptResult,
    InitializeResult,
    JSONRPCMessage,
    ListPromptsResult,
    ListToolsResult,
    Method,
};

/// Client handler that combines multiple specialized handlers
pub struct CompositeClientHandler {
    /// The underlying client
    client: Arc<Client>,

    /// The service provider
    service_provider: Arc<ServiceProvider>,

    /// The handshake handler
    handshake_handler: Box<dyn HandshakeHandler>,

    /// The prompt handler
    prompt_handler: Box<dyn PromptHandler>,

    /// The tool handler
    tool_handler: Box<dyn ToolHandler>,

    /// The completion handler
    completion_handler: Box<dyn CompletionHandler>,
}

impl CompositeClientHandler {
    /// Create a new composite handler with default implementations
    pub fn new(client: Arc<Client>, service_provider: Arc<ServiceProvider>) -> Self {
        let handshake_handler = Box::new(
            crate::client::handlers::handshake::DefaultHandshakeHandler::new(
                client.clone(),
                service_provider.clone()
            )
        );

        let prompt_handler = Box::new(
            crate::client::handlers::prompts::DefaultPromptHandler::new(
                client.clone(),
                service_provider.clone()
            )
        );

        let tool_handler = Box::new(
            crate::client::handlers::tools::DefaultToolHandler::new(
                client.clone(),
                service_provider.clone()
            )
        );

        let completion_handler = Box::new(
            crate::client::handlers::completion::DefaultCompletionHandler::new(
                client.clone(),
                service_provider.clone()
            )
        );

        Self {
            client,
            service_provider,
            handshake_handler,
            prompt_handler,
            tool_handler,
            completion_handler,
        }
    }

    /// Create a new composite handler with custom implementations
    pub fn with_handlers(
        client: Arc<Client>,
        service_provider: Arc<ServiceProvider>,
        handshake_handler: Box<dyn HandshakeHandler>,
        prompt_handler: Box<dyn PromptHandler>,
        tool_handler: Box<dyn ToolHandler>,
        completion_handler: Box<dyn CompletionHandler>
    ) -> Self {
        Self {
            client,
            service_provider,
            handshake_handler,
            prompt_handler,
            tool_handler,
            completion_handler,
        }
    }

    // Delegate methods to the appropriate handlers

    /// Initialize the client
    pub async fn initialize(&self) -> Result<InitializeResult, Error> {
        // Delegate to the handshake handler
        let result = self.handshake_handler.initialize().await?;

        // Store server capabilities/info in the lifecycle manager
        self.service_provider.lifecycle_manager().set_server_info(result.clone()).await?;

        // Send initialized notification
        self.handshake_handler.send_initialized().await?;

        Ok(result)
    }

    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<(), Error> {
        // Notify the lifecycle manager
        self.service_provider.lifecycle_manager().transition_to(LifecycleState::Shutdown).await?;

        // Shut down the client
        let result = self.client.shutdown().await;

        result
    }
    /// List prompts
    pub async fn list_prompts(&self) -> Result<ListPromptsResult, Error> {
        self.prompt_handler.list_prompts().await
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, Value>>
    ) -> Result<GetPromptResult, Error> {
        self.prompt_handler.get_prompt(name, arguments).await
    }

    /// List tools
    pub async fn list_tools(&self) -> Result<ListToolsResult, Error> {
        self.tool_handler.list_tools().await
    }

    /// Call a tool
    pub async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error> {
        self.tool_handler.call_tool(params).await
    }

    /// Generate completions
    pub async fn complete(&self, params: CompleteParams) -> Result<CompleteResult, Error> {
        self.completion_handler.complete(params).await
    }

    /// Check if the server has a capability
    pub async fn has_capability(&self, capability: &str) -> bool {
        self.handshake_handler.has_capability(capability).await
    }
}

#[async_trait]
impl RouteHandler for CompositeClientHandler {
    async fn handle_message(&self, message: JSONRPCMessage) -> Result<(), Error> {
        // Process the message based on its type
        match &message {
            JSONRPCMessage::Response(response) => {
                // Let the client handle responses through its internal mechanism
                // We don't need to do anything here as the client will match responses to pending requests
                Ok(())
            }
            JSONRPCMessage::Notification(notification) => {
                // Get the notification router
                let notification_router = self.service_provider.notification_router();

                // Forward the notification to the router
                notification_router.handle_notification(notification.clone()).await
            }
            _ => {
                // We don't expect to receive requests or errors here
                tracing::warn!("Unexpected message type received in handler");
                Ok(())
            }
        }
    }

    async fn send_request<P, R>(&self, method: Method, params: P) -> Result<R, Error>
        where P: Serialize + Send + Sync + 'static, R: DeserializeOwned + Send + Sync + 'static
    {
        // Delegate to the client
        self.client.send_request(method, params).await
    }

    async fn send_notification<P>(&self, method: Method, params: P) -> Result<(), Error>
        where P: Serialize + Send + Sync
    {
        // Delegate to the client
        self.client.send_notification(method, params).await
    }
}
