//! MCP Server Session
//!
//! This module provides a high-level server session interface that automatically
//! handles protocol operations like initialization and shutdown.
//!
use futures::future::BoxFuture;
use std::collections::HashMap;
use tracing::{ debug, info };
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::errors::Error;
use crate::server::Server;
use crate::transport::Transport;
use crate::types::initialize::{
    Implementation,
    InitializeRequestParams,
    InitializeResult,
    ServerCapabilities,
};
use crate::types::tools::{ CallToolResult, Tool, CallToolParams, ToolBuilder, ToolParameterType };
use crate::utils::validation::ValidationConfig;

/// Configuration options for the server session
#[derive(Clone, Debug)]
pub struct ServerSessionOptions {
    /// Server name
    pub name: String,
    /// Server version
    pub version: Option<String>,
    /// Validation configuration
    pub validation: ValidationConfig,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Instructions to send to client during initialization
    pub instructions: Option<String>,
}

impl Default for ServerSessionOptions {
    fn default() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            validation: ValidationConfig::default(),
            capabilities: ServerCapabilities {
                resources: None,
                tools: None,
                prompts: None,
                logging: None,
                experimental: None,
            },
            instructions: None,
        }
    }
}

/// MCP server session that automatically handles protocol operations
pub struct ServerSession<T: Clone + Send + Sync + 'static> {
    /// Server instance
    server: Server<T>,
    /// Session options
    options: ServerSessionOptions,
    /// Whether the session has been started
    started: bool,
}

impl<T: Clone + Send + Sync + 'static> ServerSession<T> {
    /// Create a new server session with the given state and options
    pub fn new(state: T, options: ServerSessionOptions) -> Self {
        // Create server with the given validation config
        let server = Server::new(state)
            .validate_requests(options.validation.validate_requests)
            .validate_responses(options.validation.validate_responses);

        // Create the session
        let mut session = Self {
            server,
            options: options.clone(),
            started: false,
        };

        // Register core protocol handlers
        Self::register_core_handlers(&mut session.server, options);

        session
    }

    /// Create a new server session with default options
    pub fn with_default_options(state: T) -> Self {
        Self::new(state, ServerSessionOptions::default())
    }

    /// Register a client with the server
    pub fn register_client(&self, client_id: &str) -> Result<(), Error> {
        info!("Registering client: {}", client_id);
        self.server.register_client(client_id)
    }

    /// Register a method handler
    pub fn register_handler<P, R, F>(&mut self, method: &str, handler: F)
        where
            P: for<'de> serde::de::Deserialize<'de> + Send + Sync + 'static,
            R: serde::Serialize + Send + Sync + 'static,
            F: Fn(&T, P) -> BoxFuture<'static, Result<R, Error>> + Send + Sync + 'static
    {
        self.server.register_handler(method, handler);
    }

    /// Register an external tool with the server
    pub fn register_external_tool(
        &mut self,
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>
    ) {
        // Register with the server's tool registry asynchronously
        let tool_clone = tool.clone();
        let tool_registry = self.server.tool_registry.clone();

        tokio::spawn(async move {
            let _ = tool_registry.register_external_tool(tool_clone, command, args, env).await;
        });
    }

    /// Register an in-process tool with a handler function
    pub fn register_in_process_tool<F>(&mut self, tool: Tool, handler: F)
        where F: Fn(CallToolParams) -> Result<CallToolResult, Error> + Send + Sync + 'static
    {
        // Register with the server's tool registry asynchronously
        let tool_clone = tool.clone();
        let tool_registry = self.server.tool_registry.clone();

        // Create a handler that will be processed asynchronously
        let handler_arc = Arc::new(handler);
        let handler_clone = handler_arc.clone();

        tokio::spawn(async move {
            let _ = tool_registry.register_in_process_tool(tool_clone, move |params|
                handler_clone(params)
            ).await;
        });
    }

    /// Start building a tool with the given name and description.
    /// Returns a ToolBuilder that can be used to configure the tool.
    pub fn build_tool(
        &self,
        name: impl Into<String>,
        description: impl Into<String>
    ) -> crate::types::tools::ToolBuilder {
        crate::types::tools::ToolBuilder::new(name, description)
    }

    /// Run the server session with the given transport
    pub async fn run<TR: Transport>(&mut self, transport: TR) -> Result<(), Error> {
        if self.started {
            return Err(Error::Initialization("Server already started".to_string()));
        }

        self.started = true;
        info!("Starting MCP server session with {} transport", std::any::type_name::<TR>());

        // Run the server
        self.server.run(transport).await
    }

    /// Register core protocol handlers
    fn register_core_handlers(server: &mut Server<T>, options: ServerSessionOptions) {
        // Initialize handler
        server.register_initialize_handler(move |_state, params: InitializeRequestParams| {
            let options_clone = options.clone();

            Box::pin(async move {
                debug!("Initializing server");

                Ok(InitializeResult {
                    protocol_version: params.protocol_version,
                    server_info: Some(Implementation {
                        name: options_clone.name,
                        version: options_clone.version,
                    }),
                    capabilities: options_clone.capabilities,
                    instructions: options_clone.instructions,
                })
            })
        });

        // Ping handler
        server.register_handler("ping", |state, _: serde_json::Value| {
            let state_clone = state.clone();

            Box::pin(async move {
                debug!("Ping received");
                Ok(serde_json::json!({ "status": "ok" }))
            })
        });

        // Tools/list handler to return all registered tools
        let tool_registry = server.tool_registry.clone();
        server.register_handler(
            "tools/list",
            move |_state, _params: crate::types::tools::ListToolsParams| {
                let registry = tool_registry.clone();

                Box::pin(async move {
                    // Get the list of registered tools using the tool registry
                    let tools = registry.list_tools().await;

                    // Return the tools
                    Ok(crate::types::tools::ListToolsResult {
                        tools,
                        next_page_token: None,
                    })
                })
            }
        );

        // Tools/call handler - generic handler for all tools
        let tool_registry = server.tool_registry.clone();
        server.register_handler("tools/call", move |_state, params: CallToolParams| {
            let registry = tool_registry.clone();
            let params_clone = params.clone();

            Box::pin(async move {
                // Execute the tool using the enhanced registry
                match registry.execute_tool_with_params(params_clone).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        // Return a properly formatted error
                        Ok(CallToolResult {
                            content: vec![
                                serde_json::json!({
                                "error": e.to_string(),
                                "status": "error"
                            })
                            ],
                            is_error: true,
                        })
                    }
                }
            })
        });
    }
}
