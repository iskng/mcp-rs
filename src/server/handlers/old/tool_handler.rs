//! Handler for tool-related messages

use async_trait::async_trait;
use tracing::{ info, error };
use std::sync::Arc;

use crate::errors::Error;
use crate::protocol::{
    JSONRPCMessage as Message,
    JSONRPCRequest as RequestMessage,
    JSONRPCResponse,
    CallToolParams,
    ListToolsRequest as ListToolsParams,
    ListToolsResult,
};
use crate::protocol::utils::{ create_success_response, create_error_response, request_id_to_i32 };
use crate::server::MessageHandler;
use crate::server::tools::tool_registry::ToolRegistry;

/// Handler for tool-related requests
#[derive(Clone)]
pub struct ToolHandler {
    /// Tool registry to manage available tools
    tool_registry: Arc<ToolRegistry>,
}

impl ToolHandler {
    /// Create a new instance of the tool handler
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            tool_registry,
        }
    }

    /// Handle tools/list request
    async fn handle_list_tools(
        &self,
        params: ListToolsParams,
        client_id: &str,
        req: &RequestMessage
    ) -> Result<Option<Message>, Error> {
        info!("TOOL LIST request from client {}", client_id);

        // Get all tools from registry
        let all_tools = self.tool_registry.list_tools().await;

        // Apply pagination
        let page_size = match params.page_size {
            Some(size) => size.min(100) as usize, // Max 100 items per page
            None => 20, // Default to 20 items per page
        };

        let start_index = match params.page_token {
            Some(ref token) => token.parse::<usize>().unwrap_or(0),
            None => 0,
        };

        // Slice the tools based on pagination
        let end_index = (start_index + page_size).min(all_tools.len());
        let tools = all_tools[start_index..end_index].to_vec();

        // Generate next page token if there are more tools
        let next_page_token = if end_index < all_tools.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        // Create the result
        let result = ListToolsResult {
            tools,
            next_page_token,
        };

        // Create response message
        let response_message = ResponseMessage::ListTools {
            result,
            meta: None,
        };

        // Convert to JSON-RPC Response
        match Message::response_from_typed(req.id, response_message) {
            Ok(response) => Ok(Some(Message::Response(response))),
            Err(e) => {
                error!("Error creating response: {}", e);
                let error_resp = crate::utils::errors::tool_execution_error(
                    req.id,
                    "list_tools",
                    &format!("Failed to serialize response: {}", e)
                );
                Ok(Some(Message::Response(error_resp)))
            }
        }
    }

    /// Handle tools/call request
    async fn handle_call_tool(
        &self,
        client_id: &str,
        req: &crate::types::protocol::Request,
        params: CallToolParams
    ) -> Result<Option<Message>, Error> {
        info!("TOOL CALL {} from client {}", params.name, client_id);
        // Check if tool exists
        if !self.tool_registry.has_tool(&params.name.clone()).await {
            let error_resp = crate::utils::errors::tool_not_found(req.id, &params.name);
            return Ok(Some(Message::Response(error_resp)));
        }

        // Execute the tool directly using the registry
        match self.tool_registry.execute_tool_with_params(params.clone()).await {
            Ok(result) => {
                // Create response message
                let response_message = ResponseMessage::CallTool {
                    result,
                    meta: None,
                };

                // Convert to JSON-RPC Response
                match Message::response_from_typed(req.id, response_message) {
                    Ok(response) => Ok(Some(Message::Response(response))),
                    Err(e) => {
                        error!("Error creating response: {}", e);
                        let error_resp = crate::utils::errors::tool_execution_error(
                            req.id,
                            &params.name,
                            &format!("Failed to serialize response: {}", e)
                        );
                        Ok(Some(Message::Response(error_resp)))
                    }
                }
            }
            Err(e) => {
                error!("Error executing tool {}: {}", params.name, e);
                let error_resp = crate::utils::errors::tool_execution_error(
                    req.id,
                    &params.name,
                    &format!("Error executing tool: {}", e)
                );
                Ok(Some(Message::Response(error_resp)))
            }
        }
    }
}

#[async_trait]
impl MessageHandler for ToolHandler {
    async fn handle(&self, client_id: &str, message: &Message) -> Result<Option<Message>, Error> {
        match message {
            Message::Request(req) => {
                match Message::parse_request(req) {
                    Ok(RequestMessage::ListTools { params }) => {
                        self.handle_list_tools(params, client_id, req).await
                    }

                    Ok(RequestMessage::CallTool { params }) => {
                        self.handle_call_tool(client_id, req, params).await
                    }

                    _ => {
                        // Not a tool-related request
                        Err(Error::MethodNotFound("Not a tool-related request".into()))
                    }
                }
            }
            _ => {
                // Not a request message
                Err(Error::MethodNotFound("Not a request message".into()))
            }
        }
    }
}
