//! Handler for initialization messages

use async_trait::async_trait;
use tracing::{ debug, error, info, warn };

use crate::errors::Error;
use crate::types::protocol::{ Message, RequestMessage, ResponseMessage, error_response };
use crate::server::MessageHandler;
use crate::types::initialize::{
    ServerCapabilities,
    Implementation,
    ResourceCapabilities,
    ToolCapabilities,
    PromptCapabilities,
    LoggingCapabilities,
    InitializeResult,
};

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
            server_capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "MCP-RS".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            },
            protocol_version: "0.1.0".to_string(),
        }
    }

    /// Set server information
    pub fn with_server_info(mut self, name: String, version: Option<String>) -> Self {
        self.server_info = Implementation {
            name,
            version,
        };
        self
    }

    /// Set protocol version
    pub fn with_protocol_version(mut self, version: String) -> Self {
        self.protocol_version = version;
        self
    }

    /// Set resource capabilities
    pub fn with_resource_capabilities(mut self, list_changed: bool, subscribe: bool) -> Self {
        self.server_capabilities.resources = Some(ResourceCapabilities {
            list_changed,
            subscribe,
        });
        self
    }

    /// Set tool capabilities
    pub fn with_tool_capabilities(mut self, list_changed: bool) -> Self {
        self.server_capabilities.tools = Some(ToolCapabilities {
            list_changed,
        });
        self
    }

    /// Set prompt capabilities
    pub fn with_prompt_capabilities(mut self, list_changed: bool) -> Self {
        self.server_capabilities.prompts = Some(PromptCapabilities {
            list_changed,
        });
        self
    }

    /// Add logging capabilities
    pub fn with_logging_capabilities(mut self) -> Self {
        self.server_capabilities.logging = Some(LoggingCapabilities {});
        self
    }

    /// Add experimental capabilities
    pub fn with_experimental_capabilities(mut self, capabilities: serde_json::Value) -> Self {
        self.server_capabilities.experimental = Some(capabilities);
        self
    }
}

#[async_trait]
impl MessageHandler for InitializeHandler {
    async fn handle(&self, _client_id: &str, message: &Message) -> Result<Option<Message>, Error> {
        match message {
            Message::Request(req) if req.method == "initialize" => {
                // Parse into typed request
                match Message::parse_request(req) {
                    Ok(RequestMessage::Initialize { params }) => {
                        // Log details from params
                        info!(
                            "Initialize request with protocol version: {}",
                            params.protocol_version
                        );
                        if let Some(client_info) = &params.client_info {
                            info!(
                                "Client info: {} {}",
                                client_info.name,
                                client_info.version.as_deref().unwrap_or("unknown")
                            );
                        }

                        // Create the initialize result
                        let result = InitializeResult {
                            capabilities: self.server_capabilities.clone(),
                            server_info: Some(self.server_info.clone()),
                            protocol_version: self.protocol_version.clone(),
                            instructions: None,
                        };

                        // Create the response message
                        let response_message = ResponseMessage::Initialize {
                            result,
                            meta: None,
                        };

                        // Convert to JSON-RPC Response
                        match Message::response_from_typed(req.id, response_message) {
                            Ok(response) => Ok(Some(Message::Response(response))),
                            Err(e) => {
                                debug!("Error creating response: {}", e);
                                Ok(
                                    Some(
                                        Message::Response(
                                            error_response(
                                                req.id,
                                                -32700,
                                                &format!("Error creating response: {}", e),
                                                None
                                            )
                                        )
                                    )
                                )
                            }
                        }
                    }
                    Ok(_) => {
                        // This shouldn't happen, but handle it just in case
                        warn!(
                            "Request didn't parse as Initialize despite method being 'initialize'"
                        );
                        Ok(
                            Some(
                                Message::Response(
                                    error_response(
                                        req.id,
                                        -32700,
                                        "Parse error: Request method is 'initialize' but didn't parse as Initialize",
                                        None
                                    )
                                )
                            )
                        )
                    }
                    Err(e) => {
                        error!("Error parsing initialize request: {}", e);
                        Ok(
                            Some(
                                Message::Response(
                                    error_response(
                                        req.id,
                                        -32700,
                                        &format!("Parse error: {}", e),
                                        None
                                    )
                                )
                            )
                        )
                    }
                }
            }
            _ => {
                // This handler only handles initialize requests
                Err(Error::Transport("Non-initialize message received".into()))
            }
        }
    }
}
