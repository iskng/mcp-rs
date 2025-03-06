//! Server implementation for managing message handling

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{ debug, error };

use crate::errors::Error;
use crate::types::protocol::Message;
use crate::server::handlers::{ NotificationHandler, RequestHandler };
use crate::server::{ MessageHandler, MessageType, RequestType, NotificationType, ResponseType };
use crate::transport::{ TransportType, ServerHandle };

use super::handlers::InitializeHandler;

/// Server configuration
pub struct ServerConfig {
    /// Name of the server
    pub name: String,
    /// Version of the server
    pub version: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Server state - this can be generic and depend on the application
#[derive(Clone)]
pub struct ServerState<S> {
    /// Application-specific state
    pub app_state: S,
}

impl<S> ServerState<S> {
    /// Create a new server state
    pub fn new(app_state: S) -> Self {
        Self { app_state }
    }
}

/// Server implementation
pub struct Server<S> {
    /// Server configuration
    config: ServerConfig,

    /// Server state
    state: Arc<RwLock<ServerState<S>>>,

    /// Message handlers, keyed by message type
    handlers: HashMap<MessageType, Arc<dyn MessageHandler + 'static>>,
}

impl<S: Clone + Send + Sync + 'static> Server<S> {
    /// Create a new server instance
    pub fn new(state: S) -> Self {
        let server = Self {
            config: ServerConfig::default(),
            state: Arc::new(RwLock::new(ServerState::new(state))),
            handlers: HashMap::new(),
        };
        server
    }
    /// Add default handlers to the server
    pub fn with_default_handlers(mut self) -> Self {
        self.register_handler(
            MessageType::Notification(NotificationType::Other("*".to_string())),
            NotificationHandler::new()
        );
        self.register_handler(
            MessageType::Request(RequestType::Other("*".to_string())),
            RequestHandler::new()
        );
        // Create and register handlers
        let initialize_handler = InitializeHandler::new()
            .with_server_info(self.config.name.clone(), Some(self.config.version.clone()))
            .with_protocol_version("2024-11-05".to_string())
            .with_resource_capabilities(true, true)
            .with_tool_capabilities(true) // Enable tool capabilities!
            .with_prompt_capabilities(true)
            .with_logging_capabilities();

        // Register initialize handler
        self.register_handler(MessageType::Request(RequestType::Initialize), initialize_handler);

        self
    }

    /// Create a new server with custom configuration
    pub fn with_config(state: S, config: ServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ServerState::new(state))),
            handlers: HashMap::new(),
        }
    }

    /// Register a message handler for a specific message type
    pub fn register_handler<H>(&mut self, message_type: MessageType, handler: H)
        where H: MessageHandler + 'static
    {
        debug!("Registering handler for message type: {:?}", message_type);
        self.handlers.insert(message_type, Arc::new(handler));
    }

    /// Register a handler for a specific request type
    pub fn register_request_handler<H>(&mut self, request_type: RequestType, handler: H)
        where H: MessageHandler + 'static
    {
        self.register_handler(MessageType::Request(request_type), handler);
    }

    /// Register a handler for a specific notification type
    pub fn register_notification_handler<H>(
        &mut self,
        notification_type: NotificationType,
        handler: H
    )
        where H: MessageHandler + 'static
    {
        self.register_handler(MessageType::Notification(notification_type), handler);
    }

    /// Register a handler for a specific response type
    pub fn register_response_handler<H>(&mut self, response_type: ResponseType, handler: H)
        where H: MessageHandler + 'static
    {
        self.register_handler(MessageType::Response(response_type), handler);
    }

    /// Register a domain handler for multiple request types
    pub fn register_domain_handler<H>(&mut self, request_types: Vec<RequestType>, handler: H)
        where H: MessageHandler + 'static + Clone
    {
        for request_type in request_types {
            self.register_request_handler(request_type, handler.clone());
        }
    }

    /// Process an incoming message by delegating to the appropriate handler
    pub async fn process_message(
        &self,
        client_id: &str,
        message: &Message
    ) -> Result<Option<Message>, Error> {
        let message_type = message.message_type();

        debug!("Processing message of type {:?} from client {}", message_type, client_id);

        if let MessageType::Response(_) = message_type {
            debug!("Skipping processing for Response message type");
            return Ok(None);
        }

        // First try to find a specific handler for this message type
        if let Some(handler) = self.handlers.get(&message_type) {
            return handler.handle(client_id, message).await;
        }

        // If no specific handler found, try to find a wildcard handler
        match &message_type {
            MessageType::Request(_) => {
                if
                    let Some(handler) = self.handlers.get(
                        &MessageType::Request(RequestType::Other("*".to_string()))
                    )
                {
                    return handler.handle(client_id, message).await;
                }
            }
            MessageType::Notification(_) => {
                if
                    let Some(handler) = self.handlers.get(
                        &MessageType::Notification(NotificationType::Other("*".to_string()))
                    )
                {
                    return handler.handle(client_id, message).await;
                }
            }
            MessageType::Response(_) => {
                if
                    let Some(handler) = self.handlers.get(
                        &MessageType::Response(ResponseType::Other("*".to_string()))
                    )
                {
                    return handler.handle(client_id, message).await;
                }
            }
        }

        // No handler found
        error!("No handler registered for message type: {:?}", message_type);
        Err(Error::MethodNotFound(format!("No handler for message type: {:?}", message_type)))
    }

    /// Get a reference to the server state
    pub async fn get_state(&self) -> ServerState<S> {
        self.state.read().await.clone()
    }

    /// Update the server state
    pub async fn update_state<F, E>(&self, update_fn: F) -> Result<(), E>
        where F: FnOnce(&mut ServerState<S>) -> Result<(), E>
    {
        let mut state = self.state.write().await;
        update_fn(&mut state)
    }

    /// Add a transport to the server
    pub fn with_transport(self, transport_type: TransportType) -> ServerWithTransport<S> {
        ServerWithTransport {
            server: self,
            transport_type,
        }
    }

    /// Add a tool registry to the server
    pub fn with_tool_registry(
        mut self,
        tool_registry: Arc<crate::server::tools::tool_registry::ToolRegistry>
    ) -> Self {
        // Create a tool handler for the registry
        let tool_handler = crate::server::handlers::ToolHandler::new(tool_registry);

        // Register handlers for tool-related requests
        self.register_handler(MessageType::Request(RequestType::ListTools), tool_handler.clone());
        self.register_handler(MessageType::Request(RequestType::CallTool), tool_handler);

        self
    }
}

/// Clone implementation for the server
impl<S: Clone + Send + Sync + 'static> Clone for Server<S> {
    fn clone(&self) -> Self {
        Self {
            config: ServerConfig {
                name: self.config.name.clone(),
                version: self.config.version.clone(),
            },
            state: self.state.clone(),
            handlers: self.handlers.clone(),
        }
    }
}

/// A server with an associated transport, ready to start
pub struct ServerWithTransport<S> {
    server: Server<S>,
    transport_type: TransportType,
}

impl<S: Clone + Send + Sync + 'static> ServerWithTransport<S> {
    /// Start the server with the configured transport
    pub async fn start(self) -> Result<ServerHandle, Error> {
        // Create Arc for server to share with the handler
        let server_arc = Arc::new(self.server);

        // Create the handler
        let handler = crate::transport::message_handler::ServerMessageHandler::new(server_arc);

        // Start the appropriate transport type and attach handler
        match self.transport_type {
            TransportType::Sse(options) => {
                // Create and start SSE transport
                let mut transport = crate::transport::sse_server::SseServerTransport::new(options);

                // First register the handler
                transport.register_message_handler(handler);

                // Then start the transport (which will use the handler in app state)
                transport.start().await?;

                // Return the server handle
                Ok(ServerHandle::new_sse(transport))
            }
            TransportType::Stdio => {
                // Currently not supported in this implementation
                let transport = crate::transport::stdio::StdioTransport::new();

                // Just return a handle without registering the handler for now
                Ok(ServerHandle::new_stdio(transport))
            }
        }
    }
}
