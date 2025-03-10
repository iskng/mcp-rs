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
    completion::CompletionHandler, handshake::HandshakeHandler, prompts::PromptHandler,
    resources::ResourceHandler, route_handler::RouteHandler, tools::ToolHandler,
};
use crate::client::services::ServiceProvider;
use crate::client::services::subscription::Subscription;
use crate::client::transport::DirectIOTransport;
use crate::protocol::{
    CallToolParams, CallToolResult, CompleteParams, CompleteResult, Error, GetPromptResult,
    InitializeResult, JSONRPCMessage, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, Method, ReadResourceParams, ReadResourceResult,
    ResourceListChangedNotification, ResourceUpdatedNotification,
};

/// Composite client handler that delegates to domain-specific handlers
pub struct CompositeClientHandler<T: DirectIOTransport + 'static> {
    /// The underlying client
    client: Arc<Client<T>>,

    /// The service provider
    service_provider: Arc<ServiceProvider>,

    /// The resource handler
    resource_handler: Box<dyn ResourceHandler>,

    /// The handshake handler
    handshake_handler: Box<dyn HandshakeHandler>,

    /// The prompt handler
    prompt_handler: Box<dyn PromptHandler>,

    /// The tool handler
    tool_handler: Box<dyn ToolHandler>,

    /// The completion handler
    completion_handler: Box<dyn CompletionHandler>,
}

impl<T: DirectIOTransport + 'static> CompositeClientHandler<T> {
    /// Create a new composite handler with default implementations
    pub fn new(client: Arc<Client<T>>, service_provider: Arc<ServiceProvider>) -> Self {
        // Create the handlers
        let resource_handler = Box::new(
            crate::client::handlers::resources::DefaultResourceHandler::new(
                client.clone(),
                service_provider.clone(),
            ),
        );

        let handshake_handler = Box::new(
            crate::client::handlers::handshake::DefaultHandshakeHandler::new(
                client.clone(),
                service_provider.clone(),
            ),
        );

        let prompt_handler = Box::new(crate::client::handlers::prompts::DefaultPromptHandler::new(
            client.clone(),
            service_provider.clone(),
        ));

        let tool_handler = Box::new(crate::client::handlers::tools::DefaultToolHandler::new(
            client.clone(),
            service_provider.clone(),
        ));

        let completion_handler = Box::new(
            crate::client::handlers::completion::DefaultCompletionHandler::new(
                client.clone(),
                service_provider.clone(),
            ),
        );

        Self {
            client,
            service_provider,
            resource_handler,
            handshake_handler,
            prompt_handler,
            tool_handler,
            completion_handler,
        }
    }

    /// Create a new composite handler with custom implementations
    pub fn with_handlers(
        client: Arc<Client<T>>,
        service_provider: Arc<ServiceProvider>,
        resource_handler: Box<dyn ResourceHandler>,
        handshake_handler: Box<dyn HandshakeHandler>,
        prompt_handler: Box<dyn PromptHandler>,
        tool_handler: Box<dyn ToolHandler>,
        completion_handler: Box<dyn CompletionHandler>,
    ) -> Self {
        Self {
            client,
            service_provider,
            resource_handler,
            handshake_handler,
            prompt_handler,
            tool_handler,
            completion_handler,
        }
    }

    // Delegate methods to the appropriate handlers

    /// Initialize the client
    pub async fn initialize(&self) -> Result<InitializeResult, Error> {
        self.handshake_handler.initialize().await
    }

    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.handshake_handler.shutdown().await
    }

    /// List resources
    pub async fn list_resources(
        &self,
        params: Option<Value>,
    ) -> Result<ListResourcesResult, Error> {
        self.resource_handler.list_resources(params).await
    }

    /// Read a resource
    pub async fn read_resource(
        &self,
        params: ReadResourceParams,
    ) -> Result<ReadResourceResult, Error> {
        self.resource_handler.read_resource(params).await
    }

    /// List resource templates
    pub async fn list_resource_templates(&self) -> Result<ListResourceTemplatesResult, Error> {
        self.resource_handler.list_resource_templates().await
    }

    /// Create a resource
    pub async fn create_resource(&self, params: Value) -> Result<ReadResourceResult, Error> {
        self.resource_handler.create_resource(params).await
    }

    /// Update a resource
    pub async fn update_resource(&self, params: Value) -> Result<ReadResourceResult, Error> {
        self.resource_handler.update_resource(params).await
    }

    /// Delete a resource
    pub async fn delete_resource(&self, uri: String) -> Result<(), Error> {
        self.resource_handler.delete_resource(uri).await
    }

    /// List prompts
    pub async fn list_prompts(&self) -> Result<ListPromptsResult, Error> {
        self.prompt_handler.list_prompts().await
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, Value>>,
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

    /// Subscribe to resource updates
    pub async fn subscribe_resource_updates(&self) -> Subscription<ResourceUpdatedNotification> {
        self.resource_handler.subscribe_resource_updates().await
    }

    /// Subscribe to resource list changes
    pub async fn subscribe_resource_list_changes(
        &self,
    ) -> Subscription<ResourceListChangedNotification> {
        self.resource_handler
            .subscribe_resource_list_changes()
            .await
    }

    /// Create a resource from a template
    pub async fn create_resource_from_template(
        &self,
        template_id: &str,
        name: &str,
        properties: Option<HashMap<String, Value>>,
    ) -> Result<ReadResourceResult, Error> {
        self.resource_handler
            .create_resource_from_template(template_id, name, properties)
            .await
    }

    /// Update a resource's properties
    pub async fn update_resource_properties(
        &self,
        uri: &str,
        properties: HashMap<String, Value>,
    ) -> Result<ReadResourceResult, Error> {
        self.resource_handler
            .update_resource_properties(uri, properties)
            .await
    }

    /// Wait for a resource to be updated
    pub async fn wait_for_resource_update(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        self.resource_handler.wait_for_resource_update(uri).await
    }

    /// Check if the server has a capability
    pub async fn has_capability(&self, capability: &str) -> bool {
        self.handshake_handler.has_capability(capability).await
    }
}

#[async_trait]
impl<T: DirectIOTransport + 'static> RouteHandler for CompositeClientHandler<T> {
    async fn handle_message(&self, message: JSONRPCMessage) -> Result<(), Error> {
        // Process the message based on its type
        match message {
            JSONRPCMessage::Response(response) => {
                // Get the request manager
                let request_manager = self.service_provider.request_manager();

                // Handle the response
                request_manager.handle_response(response).await
            }
            JSONRPCMessage::Notification(notification) => {
                // Get the notification router
                let notification_router = self.service_provider.notification_router();

                // Handle the notification
                notification_router.handle_notification(notification).await
            }
            JSONRPCMessage::Error(error) => {
                // Get the request manager
                let request_manager = self.service_provider.request_manager();

                // Handle the error
                request_manager.handle_error(error).await
            }
            _ => {
                // We don't expect to receive requests from the server
                Err(Error::Other("Unexpected message type".to_string()))
            }
        }
    }

    async fn send_request<P, R>(&self, method: Method, params: P) -> Result<R, Error>
    where
        P: Serialize + Send + Sync + 'static,
        R: DeserializeOwned + Send + Sync + 'static,
    {
        self.client.send_request(method, params).await
    }

    async fn send_notification<P>(&self, method: Method, params: P) -> Result<(), Error>
    where
        P: Serialize + Send + Sync,
    {
        self.client.send_notification(method, params).await
    }
}
