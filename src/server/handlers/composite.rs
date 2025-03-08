//! Composite server handler
//!
//! This module provides a concrete implementation of the ServerHandler trait
//! that pattern matches on message types and dispatches to domain-specific handlers.

use async_trait::async_trait;
use tracing::info;
use std::sync::Arc;

use super::handshake::PingResult;
use crate::protocol::Error;
use crate::protocol::{
    CallToolRequest,
    CallToolResult,
    CancelledNotification,
    ClientMessage,
    ClientNotification,
    ClientRequest,
    InitializeRequest,
    InitializeResult,
    InitializedNotification,
    JSONRPCError,
    JSONRPCMessage,
    JSONRPCResponse,
    ListResourceTemplatesRequest,
    ListResourceTemplatesResult,
    ListResourcesRequest,
    ListResourcesResult,
    ListToolsRequest,
    ListToolsResult,
    Message,
    PingRequest,
    ProgressNotification,
    ReadResourceRequest,
    ReadResourceResult,
    RequestId,
    Result as EmptyResult,
    RootsListChangedNotification,
    ServerMessage,
    SubscribeRequest,
    UnsubscribeRequest,
    response_from_typed,
};
use crate::server::services::ServiceProvider;
use crate::transport::middleware::ClientSession;

use super::handshake::DefaultHandshakeHandler;
use super::handshake::HandshakeHandler;
use super::initialize::{ DefaultInitializeHandler, InitializeHandler };
use super::resources::{ DefaultResourceHandler, ResourceHandler };
use super::route_handler::RouteHandler;
use super::tools::{ DefaultToolHandler, ToolHandler };

/// Composite server handler that implements all domain-specific functionality
pub struct CompositeServerHandler {
    /// Service provider for accessing services
    service_provider: Arc<ServiceProvider>,

    /// Initialize handler for server initialization
    initialize_handler: Box<dyn InitializeHandler>,

    /// Handshake handler for core protocol operations
    handshake_handler: Box<dyn HandshakeHandler>,

    /// Tool handler for tool operations
    tool_handler: Box<dyn ToolHandler>,

    /// Resource handler for resource operations
    resource_handler: Box<dyn ResourceHandler>,
}

impl CompositeServerHandler {
    /// Create a new composite server handler with default handlers
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        let initialize_handler = Box::new(DefaultInitializeHandler::new(service_provider.clone()));
        let handshake_handler = Box::new(DefaultHandshakeHandler::new(service_provider.clone()));
        let tool_handler = Box::new(DefaultToolHandler::new(service_provider.clone()));
        let resource_handler = Box::new(DefaultResourceHandler::new(service_provider.clone()));

        Self {
            service_provider,
            initialize_handler,
            handshake_handler,
            tool_handler,
            resource_handler,
        }
    }

    /// Create a new composite server handler with a custom initialize handler
    pub fn with_initialize_handler(
        service_provider: Arc<ServiceProvider>,
        initialize_handler: Box<dyn InitializeHandler>
    ) -> Self {
        let handshake_handler = Box::new(DefaultHandshakeHandler::new(service_provider.clone()));
        let tool_handler = Box::new(DefaultToolHandler::new(service_provider.clone()));
        let resource_handler = Box::new(DefaultResourceHandler::new(service_provider.clone()));

        Self {
            service_provider,
            initialize_handler,
            handshake_handler,
            tool_handler,
            resource_handler,
        }
    }

    /// Get a reference to the service provider
    pub fn service_provider(&self) -> &Arc<ServiceProvider> {
        &self.service_provider
    }

    //
    // Core protocol handlers
    //

    /// Handle initialize request
    async fn handle_initialize(
        &self,
        client_id: Option<&str>,
        request: &InitializeRequest
    ) -> Result<InitializeResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the initialize handler
        self.initialize_handler.handle_initialize(request, &session).await
    }

    /// Handle ping request
    async fn handle_ping(
        &self,
        client_id: Option<&str>,
        request: &PingRequest
    ) -> Result<PingResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the handshake handler
        self.handshake_handler.handle_ping(request, &session).await
    }

    /// Handle initialized notification
    async fn handle_initialized(
        &self,
        client_id: Option<&str>,
        notification: &InitializedNotification
    ) -> Result<(), Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the handshake handler
        self.handshake_handler.handle_initialized(notification, &session).await
    }

    /// Handle cancelled notification
    async fn handle_cancelled(
        &self,
        client_id: Option<&str>,
        notification: &CancelledNotification
    ) -> Result<(), Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the handshake handler
        self.handshake_handler.handle_cancelled(notification, &session).await
    }

    /// Handle progress notification
    async fn handle_progress(
        &self,
        client_id: Option<&str>,
        notification: &ProgressNotification
    ) -> Result<(), Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the handshake handler
        self.handshake_handler.handle_progress(notification, &session).await
    }

    //
    // Resource handlers
    //

    /// Handle list resources request
    async fn handle_list_resources(
        &self,
        client_id: Option<&str>,
        request: &ListResourcesRequest
    ) -> Result<ListResourcesResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the resource handler
        self.resource_handler.handle_list_resources(request, &session).await
    }

    /// Handle read resource request
    async fn handle_read_resource(
        &self,
        client_id: Option<&str>,
        request: &ReadResourceRequest
    ) -> Result<ReadResourceResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the resource handler
        self.resource_handler.handle_read_resource(request, &session).await
    }

    /// Handle list resource templates request
    async fn handle_list_templates(
        &self,
        client_id: Option<&str>,
        request: &ListResourceTemplatesRequest
    ) -> Result<ListResourceTemplatesResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the resource handler
        self.resource_handler.handle_list_templates(request, &session).await
    }

    /// Handle subscribe request
    async fn handle_subscribe(
        &self,
        client_id: Option<&str>,
        request: &SubscribeRequest
    ) -> Result<(), Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the resource handler
        self.resource_handler.handle_subscribe(request, &session).await
    }

    /// Handle unsubscribe request
    async fn handle_unsubscribe(
        &self,
        client_id: Option<&str>,
        request: &UnsubscribeRequest
    ) -> Result<(), Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the resource handler
        self.resource_handler.handle_unsubscribe(request, &session).await
    }

    /// Handle roots list changed notification
    async fn handle_roots_list_changed(
        &self,
        client_id: Option<&str>,
        notification: &RootsListChangedNotification
    ) -> Result<(), Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the resource handler
        self.resource_handler.handle_roots_list_changed(notification, &session).await
    }

    //
    // Tool handlers
    //

    /// Handle list tools request
    async fn handle_list_tools(
        &self,
        client_id: Option<&str>,
        request: &ListToolsRequest
    ) -> Result<ListToolsResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the tool handler
        self.tool_handler.handle_list_tools(request, &session).await
    }

    /// Handle call tool request
    async fn handle_call_tool(
        &self,
        client_id: Option<&str>,
        request: &CallToolRequest
    ) -> Result<CallToolResult, Error> {
        // Create a dummy session for now
        let mut session = ClientSession::new();
        if let Some(id) = client_id {
            session.set_client_id(id.to_string());
        }

        // Call the tool handler
        self.tool_handler.handle_call_tool(request, &session).await
    }

    /// Create a response from a request and result
    fn create_response<T: serde::Serialize>(&self, id: RequestId, result: T) -> JSONRPCMessage {
        info!("Create response:");
        response_from_typed(id, result)
    }

    /// Create an empty success response
    fn create_empty_response(&self, id: RequestId) -> JSONRPCMessage {
        // Create an empty result with required fields
        let empty_result =
            serde_json::json!({
            "_meta": null,
            "content": {}
        });

        response_from_typed(id, empty_result)
    }
}

// /// BaseHandler implementation for CompositeServerHandler
// impl BaseHandler for CompositeServerHandler {
//     fn service_provider(&self) -> &Arc<ServiceProvider> {
//         &self.service_provider
//     }
// }

/// ServerHandler implementation for CompositeServerHandler
#[async_trait]
impl RouteHandler for CompositeServerHandler {
    async fn handle_typed_message(
        &self,
        id: RequestId,
        client_id: Option<&str>,
        message: &Message
    ) -> Result<Option<JSONRPCMessage>, Error> {
        match message {
            Message::Client(client_message) => {
                match client_message {
                    ClientMessage::Request(req) => {
                        match req {
                            ClientRequest::Initialize(req) => {
                                let result = self.handle_initialize(client_id, req).await?;
                                tracing::info!("Initialize result: {:?}", result);
                                Ok(Some(self.create_response(id, result)))
                            }
                            ClientRequest::Ping(req) => {
                                let result = self.handle_ping(client_id, req).await?;
                                Ok(Some(self.create_response(id, result)))
                            }

                            // Resource domain
                            ClientRequest::ListResources(req) => {
                                let result = self.handle_list_resources(client_id, req).await?;

                                Ok(Some(self.create_response(id, result)))
                            }
                            ClientRequest::ReadResource(req) => {
                                let result = self.handle_read_resource(client_id, req).await?;

                                Ok(Some(self.create_response(id, result)))
                            }
                            ClientRequest::ListResourceTemplates(req) => {
                                let result = self.handle_list_templates(client_id, req).await?;

                                Ok(Some(self.create_response(id, result)))
                            }
                            ClientRequest::Subscribe(req) => {
                                self.handle_subscribe(client_id, req).await?;

                                Ok(Some(self.create_empty_response(id)))
                            }
                            ClientRequest::Unsubscribe(req) => {
                                self.handle_unsubscribe(client_id, req).await?;

                                Ok(Some(self.create_empty_response(id)))
                            }

                            // Tool domain
                            ClientRequest::ListTools(req) => {
                                let result = self.handle_list_tools(client_id, req).await?;

                                Ok(Some(self.create_response(id, result)))
                            }
                            ClientRequest::CallTool(req) => {
                                let result = self.handle_call_tool(client_id, req).await?;

                                Ok(Some(self.create_response(id, result)))
                            }

                            // Other domains - not implemented
                            _ =>
                                Err(
                                    Error::MethodNotFound(
                                        format!("Method not implemented: {:?}", req)
                                    )
                                ),
                        }
                    }
                    ClientMessage::Notification(notification) => {
                        match notification {
                            ClientNotification::Initialized(n) => {
                                self.handle_initialized(client_id, n).await?;
                                Ok(None) // No response for notifications
                            }
                            ClientNotification::Cancelled(n) => {
                                self.handle_cancelled(client_id, n).await?;
                                Ok(None)
                            }
                            ClientNotification::Progress(n) => {
                                self.handle_progress(client_id, n).await?;
                                Ok(None)
                            }
                            ClientNotification::RootsListChanged(n) => {
                                self.handle_roots_list_changed(client_id, n).await?;
                                Ok(None)
                            }
                        }
                    }
                    // Handle results from client
                    ClientMessage::Result(_) => {
                        // We don't expect results from clients, but we handle them gracefully
                        tracing::warn!("Received unexpected result from client");
                        Ok(None)
                    }
                }
            }
            Message::Server(_) => {
                // We don't process server messages here
                tracing::warn!(
                    "Received server message that should not be processed by server handler"
                );
                Ok(None)
            }
            Message::Error(err) => {
                // Just log and pass through errors
                tracing::error!("Received error message: {:?}", err);
                Ok(None)
            }
        }
    }

    fn service_provider(&self) -> Arc<ServiceProvider> {
        self.service_provider.clone()
    }
}
