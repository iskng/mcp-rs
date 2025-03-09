//! Server implementation
//!
//! The Server manages the lifecycle of an MCP server, including initialization,
//! configuration, transport management, and service providers.

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{ info, warn };

use crate::protocol::errors::Error;
use crate::protocol::{
    CallToolParams,
    CallToolResult,
    Tool,
    ResourcesCapability,
    ToolsCapability,
    Annotations,
    PROTOCOL_VERSION,
};
use crate::server::handlers::CompositeServerHandler;
use crate::server::handlers::RouteHandler;
use crate::server::services::{
    resources::resource_registry::ResourceRegistry,
    ServiceProvider,
    tools::tool_registry::{ ExternalToolConfig, ToolRegistry },
};
// Not needed with the unified builder approach
// use crate::transport::ServerHandle;
use crate::transport::{ Transport, middleware::ClientSessionStore };

use super::handlers::InitializeHandlerBuilder;

/// Application state shared between the server and transport
pub struct AppState {
    /// Authentication token (if required)
    pub auth_token: Option<String>,

    /// Whether authentication is required
    pub require_auth: bool,

    /// Allowed origins for CORS
    pub allowed_origins: Option<Vec<String>>,

    /// Route handler for processing messages
    pub route_handler: Arc<dyn RouteHandler + Send + Sync>,

    /// Client session store
    pub session_store: Arc<ClientSessionStore>,
}

impl AppState {
    /// Create a new application state
    pub fn new(
        auth_token: Option<String>,
        require_auth: bool,
        allowed_origins: Option<Vec<String>>,
        route_handler: Arc<dyn RouteHandler + Send + Sync>
    ) -> Self {
        Self {
            auth_token,
            require_auth,
            allowed_origins,
            route_handler,
            session_store: Arc::new(ClientSessionStore::new()),
        }
    }
}

/// Server for Machine Control Protocol (MCP)
///
/// Responsible for managing resources, tools, and communication with clients.
pub struct Server {
    /// Application state shared with the transport layer
    app_state: Arc<AppState>,

    /// The transport, if any
    transport: Option<Box<dyn Transport + Send + Sync>>,
}

impl Server {
    /// Create a new server builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Start the server with the configured transport
    pub async fn start(&mut self) -> Result<(), Error> {
        // Check that we have a transport
        if self.transport.is_none() {
            return Err(Error::Protocol("No transport configured for server".to_string()));
        }

        // Get the transport
        let transport = self.transport.as_mut().unwrap();

        // For now, just start the transport
        // In the next steps, we'll implement transport_type in the Transport trait
        // and update the transport to work with AppState
        tracing::info!("Starting transport");

        // Start the transport
        transport.start().await
    }

    /// Shut down the server
    pub async fn shutdown(&mut self) {
        // If we have a transport, shut it down
        if let Some(transport) = &mut self.transport {
            if let Err(e) = transport.close().await {
                warn!("Error shutting down transport: {}", e);
            }
        }
    }

    /// Get the route handler
    pub fn route_handler(&self) -> Arc<dyn RouteHandler> {
        self.app_state.route_handler.clone()
    }

    /// Get the service provider from the route handler (if it has one)
    pub fn service_provider(&self) -> Arc<ServiceProvider> {
        self.route_handler().service_provider()
    }

    /// Register an external tool
    pub async fn register_external_tool(
        &self,
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        annotations: Option<Annotations>
    ) -> Result<(), Error> {
        let service_provider = self.service_provider();
        let tool_registry = service_provider.tool_registry();

        tool_registry.register_external_tool(tool, command, args, env, annotations).await
    }

    /// Register an in-process tool
    pub async fn register_in_process_tool<F>(&self, tool: Tool, handler: F) -> Result<(), Error>
        where F: Fn(CallToolParams) -> Result<CallToolResult, Error> + Send + Sync + 'static
    {
        let service_provider = self.service_provider();
        let tool_registry = service_provider.tool_registry();

        tool_registry.register_in_process_tool(tool, handler).await
    }

    /// Get the transport, if any
    pub fn transport(&mut self) -> Option<&mut Box<dyn Transport + Send + Sync>> {
        self.transport.as_mut()
    }
}

/// Builder for configuring and creating a Server
pub struct ServerBuilder {
    /// Server name
    server_name: Option<String>,

    /// Server version
    server_version: Option<String>,

    /// Protocol version
    protocol_version: Option<String>,

    /// Instructions
    instructions: Option<String>,

    /// Resource capabilities
    resource_capabilities: Option<ResourcesCapability>,

    /// Tool capabilities
    tool_capabilities: Option<ToolsCapability>,

    /// Resource registry
    resource_registry: Option<Arc<ResourceRegistry>>,

    /// Tool registry
    tool_registry: Option<Arc<ToolRegistry>>,

    /// Server handler
    server_handler: Option<Arc<dyn RouteHandler>>,

    /// External tools to register during build
    external_tools: Vec<(Tool, ExternalToolConfig)>,

    /// In-process tools to register during build
    in_process_tools: Vec<
        (Tool, Arc<dyn (Fn(CallToolParams) -> Result<CallToolResult, Error>) + Send + Sync>)
    >,

    /// Transport
    transport: Option<Box<dyn Transport + Send + Sync>>,

    /// Authentication token for the server
    auth_token: Option<String>,

    /// Whether authentication is required
    require_auth: bool,

    /// CORS allowed origins
    allowed_origins: Option<Vec<String>>,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            server_name: None,
            server_version: None,
            protocol_version: Some(PROTOCOL_VERSION.to_string()),
            resource_capabilities: None,
            tool_capabilities: None,
            resource_registry: None,
            tool_registry: None,
            server_handler: None,
            instructions: None,
            external_tools: Vec::new(),
            in_process_tools: Vec::new(),
            transport: None,
            auth_token: None,
            require_auth: false,
            allowed_origins: None,
        }
    }

    /// Set the server name
    pub fn with_server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = Some(name.into());
        self
    }

    /// Set the server version
    pub fn with_server_version(mut self, version: impl Into<String>) -> Self {
        self.server_version = Some(version.into());
        self
    }

    /// Set the instructions
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Set the protocol version
    pub fn with_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.protocol_version = Some(version.into());
        self
    }

    /// Set the resource capabilities
    pub fn with_resource_capabilities(mut self, capabilities: ResourcesCapability) -> Self {
        self.resource_capabilities = Some(capabilities);
        self
    }

    /// Set the tool capabilities
    pub fn with_tool_capabilities(mut self, capabilities: ToolsCapability) -> Self {
        self.tool_capabilities = Some(capabilities);
        self
    }

    /// Set the resource registry
    pub fn with_resource_registry(mut self, registry: Arc<ResourceRegistry>) -> Self {
        self.resource_registry = Some(registry);
        self
    }

    /// Set the tool registry
    pub fn with_tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Set the server handler
    pub fn with_server_handler(mut self, handler: Arc<dyn RouteHandler>) -> Self {
        self.server_handler = Some(handler);
        self
    }

    /// Register an external tool to be added during build
    pub fn register_external_tool(mut self, tool: Tool, config: ExternalToolConfig) -> Self {
        self.external_tools.push((tool, config));
        self
    }

    /// Register an in-process tool to be added during build
    pub fn register_in_process_tool<F>(mut self, tool: Tool, handler: F) -> Self
        where F: Fn(CallToolParams) -> Result<CallToolResult, Error> + Send + Sync + 'static
    {
        self.in_process_tools.push((tool, Arc::new(handler)));
        self
    }

    /// Add a transport to the builder
    pub fn with_transport<T>(mut self, transport: T) -> Self
        where T: Transport + Send + Sync + 'static
    {
        self.transport = Some(Box::new(transport));
        self
    }

    /// Build the server with the configured options
    pub async fn build(mut self) -> Result<Server, Error> {
        // Create resource registry if not provided
        let resource_registry = match self.resource_registry {
            Some(registry) => registry,
            None => {
                let capabilities = match self.resource_capabilities.clone() {
                    Some(caps) => caps,
                    None =>
                        ResourcesCapability {
                            subscribe: Some(true),
                            list_changed: Some(true),
                        },
                };
                info!("Creating new resource registry in build: {:?}", capabilities);
                Arc::new(
                    ResourceRegistry::new(
                        capabilities.subscribe.unwrap_or(true),
                        capabilities.list_changed.unwrap_or(true)
                    )
                )
            }
        };

        // Create tool registry if not provided
        let tool_registry = match self.tool_registry {
            Some(registry) => registry,
            None => {
                let capabilities = match self.tool_capabilities.clone() {
                    Some(caps) => caps,
                    None =>
                        ToolsCapability {
                            list_changed: Some(true),
                        },
                };
                info!("Creating new tool registry in build: {:?}", capabilities);
                Arc::new(ToolRegistry::new(capabilities))
            }
        };

        // Create a service provider
        let service_provider = Arc::new(ServiceProvider::new(resource_registry, tool_registry));

        //  Create or use the provided server handler
        let route_handler = match self.server_handler {
            Some(handler) => handler,
            None => {
                // Build an initialize handler with the builder's settings
                let init_handler = InitializeHandlerBuilder::new(service_provider.clone())
                    // Pass all relevant fields from the ServerBuilder
                    .with_server_name(self.server_name.unwrap_or_else(|| "Rust Server".to_string()))
                    .with_server_version(
                        self.server_version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
                    )
                    .with_protocol_version(
                        self.protocol_version.unwrap_or_else(|| PROTOCOL_VERSION.to_string())
                    );

                // Apply resource capabilities if set
                let init_handler = if let Some(caps) = self.resource_capabilities.clone() {
                    init_handler.with_resource_capabilities(
                        caps.list_changed.unwrap_or(true),
                        caps.subscribe.unwrap_or(true)
                    )
                } else {
                    init_handler
                };

                // Apply tool capabilities if set
                let init_handler = if let Some(caps) = self.tool_capabilities.clone() {
                    init_handler.with_tool_capabilities(caps.list_changed.unwrap_or(true))
                } else {
                    init_handler
                };

                // Apply instructions if set
                let init_handler = if let Some(instructions) = &self.instructions {
                    init_handler.with_instructions(instructions)
                } else {
                    init_handler
                };

                // Build the handler and create the composite handler
                Arc::new(
                    CompositeServerHandler::with_initialize_handler(
                        service_provider.clone(),
                        init_handler.build()
                    )
                )
            }
        };

        // Create app state
        let app_state = Arc::new(
            AppState::new(
                self.auth_token,
                self.require_auth,
                self.allowed_origins,
                route_handler.clone()
            )
        );
        if let Some(transport) = self.transport.as_mut() {
            transport.set_app_state(app_state.clone()).await;
        }
        // Create the server
        let server = Server {
            app_state,
            transport: self.transport,
        };

        // Register any tools provided during build
        for (tool, config) in self.external_tools {
            // Register with the service provider's tool registry
            service_provider
                .tool_registry()
                .register_external_tool(
                    tool,
                    config.command,
                    config.args,
                    config.env,
                    config.annotations
                ).await?;
        }

        for (tool, handler) in self.in_process_tools {
            tracing::info!("Registering in-process tool: {}", tool.name);
            // Register with the service provider's tool registry
            let handler_fn = move |params: CallToolParams| -> Result<CallToolResult, Error> {
                handler(params)
            };
            service_provider.tool_registry().register_in_process_tool(tool, handler_fn).await?;
            tracing::info!("Tool registration complete");
        }

        Ok(server)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
