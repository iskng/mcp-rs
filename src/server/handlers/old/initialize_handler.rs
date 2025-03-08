//! Handler for initialization messages

use async_trait::async_trait;
use tracing::{ debug, error, info, warn };
use std::collections::HashMap;

use crate::errors::Error;
// Import the re-exported types from the top-level protocol module
use crate::protocol::{
    Message,
    JSONRPCMessage,
    JSONRPCError,
    JSONRPCErrorDetails,
    ServerCapabilities,
    Implementation,
    ResourcesCapability,
    ToolsCapability,
    PromptsCapability,
    InitializeResult,
    ClientMessage,
    ClientRequest,
    ServerResult,
    response_from_typed,
};
use crate::server::MessageHandler;

/// Handler for Initialize messages
pub struct InitializeHandler {
    /// Server capabilities to report to the client
    server_capabilities: ServerCapabilities,
    /// Server information
    server_info: Implementation,
    /// Protocol version
    protocol_version: String,
}

impl InitializeHandler {
    /// Create a new instance of the initialization handler
    pub fn new() -> Self {
        Self {
            server_capabilities: ServerCapabilities {
                resources: None,
                prompts: None,
                tools: None,
                logging: None,
                experimental: None,
            },
            server_info: Implementation {
                name: "MCP-RS".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            protocol_version: "0.1.0".to_string(),
        }
    }

    /// Set the server information
    pub fn with_server_info(mut self, name: String, version: Option<String>) -> Self {
        self.server_info = Implementation {
            name,
            version: version.unwrap_or_else(|| "0.1.0".to_string()),
        };
        self
    }

    /// Set the protocol version
    pub fn with_protocol_version(mut self, version: String) -> Self {
        self.protocol_version = version;
        self
    }

    /// Set resource capabilities
    pub fn with_resource_capabilities(mut self, list_changed: bool, subscribe: bool) -> Self {
        self.server_capabilities.resources = Some(ResourcesCapability {
            list_changed: Some(list_changed),
            subscribe: Some(subscribe),
        });
        self
    }

    /// Set tool capabilities
    pub fn with_tool_capabilities(mut self, list_changed: bool) -> Self {
        self.server_capabilities.tools = Some(ToolsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Set prompt capabilities
    pub fn with_prompt_capabilities(mut self, list_changed: bool) -> Self {
        self.server_capabilities.prompts = Some(PromptsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Add logging capabilities
    pub fn with_logging_capabilities(mut self) -> Self {
        self.server_capabilities.logging = Some(HashMap::new());
        self
    }

    /// Add experimental capabilities
    pub fn with_experimental_capabilities(
        mut self,
        capabilities: HashMap<String, HashMap<String, serde_json::Value>>
    ) -> Self {
        self.server_capabilities.experimental = Some(capabilities);
        self
    }
}

#[async_trait]
impl MessageHandler for InitializeHandler {
    async fn handle(&self, _client_id: &str, message: &Message) -> Result<Option<Message>, Error> {
        match message {
            Message::Client(ClientMessage::Request(req)) => {
                // Check if it's an initialize request
                if let ClientRequest::Initialize(init_req) = req {
                    // Log details from params
                    info!(
                        "Initialize request received from client: {} v{}",
                        init_req.params.client_info.name,
                        init_req.params.client_info.version
                    );

                    if let Some(capabilities) = &init_req.params.capabilities.experimental {
                        debug!("Client experimental capabilities: {:?}", capabilities);
                    }

                    // Create the initialize result
                    let result = InitializeResult {
                        capabilities: self.server_capabilities.clone(),
                        protocol_version: self.protocol_version.clone(),
                        server_info: self.server_info.clone(),
                        instructions: None,
                        _meta: None,
                    };

                    // Create response message
                    let result_obj = ServerResult::Initialize(result);
                    let id = match message {
                        Message::Client(ClientMessage::Request(client_req)) => {
                            match client_req {
                                ClientRequest::Initialize(init) => {
                                    // Convert JSONRPCRequest back to RequestId
                                    // This is awkward but we need the ID
                                    let json_str = serde_json
                                        ::to_string(&init)
                                        .map_err(|e| Error::Json(e))?;
                                    let jsonrpc_msg: JSONRPCMessage = serde_json
                                        ::from_str(&json_str)
                                        .map_err(|e| Error::Json(e))?;

                                    match jsonrpc_msg {
                                        JSONRPCMessage::Request(req) => req.id,
                                        _ => {
                                            return Err(
                                                Error::Protocol("Expected request".to_string())
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    return Err(
                                        Error::Protocol("Expected initialize request".to_string())
                                    );
                                }
                            }
                        }
                        _ => {
                            return Err(Error::Protocol("Expected client request".to_string()));
                        }
                    };

                    // Convert to JSON-RPC Response
                    let response = response_from_typed(id, result);
                    match response.into_message() {
                        Ok(typed_message) => Ok(Some(typed_message)),
                        Err(e) => {
                            debug!("Error creating response: {}", e);
                            // Create error response
                            let err_msg = JSONRPCError {
                                jsonrpc: "2.0".to_string(),
                                id,
                                error: JSONRPCErrorDetails {
                                    code: -32700,
                                    message: format!("Error parsing response: {}", e),
                                    data: None,
                                },
                            };

                            let error_msg = JSONRPCMessage::Error(err_msg);
                            error_msg
                                .into_message()
                                .map(Some)
                                .map_err(|e|
                                    Error::Protocol(
                                        format!("Failed to create error message: {}", e)
                                    )
                                )
                        }
                    }
                } else {
                    // Not an initialize request
                    warn!("Received non-initialize request to initialize handler");
                    Ok(None)
                }
            }
            // Handle other cases
            _ => {
                warn!("Received non-client request to initialize handler");
                // Ignore other messages, this handler only handles initialize requests
                Ok(None)
            }
        }
    }
}
