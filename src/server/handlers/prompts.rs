use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use crate::protocol::{ Error, JSONRPCMessage, Message, RequestId };
use crate::protocol::{
    GetPromptParams,
    GetPromptResult,
    ListPromptsResult,
    Prompt as ProtocolPrompt,
    PromptArgument as ProtocolPromptArgument,
};
use crate::server::handlers::RouteHandler;
use crate::server::services::ServiceProvider;

/// Handler for prompt-related methods
pub struct PromptsHandler {
    service_provider: Arc<ServiceProvider>,
}

impl PromptsHandler {
    /// Create a new prompts handler
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        Self { service_provider }
    }
}

#[async_trait]
impl RouteHandler for PromptsHandler {
    async fn handle_typed_message(
        &self,
        request_id: RequestId,
        client_id: Option<&str>,
        message: &Message
    ) -> Result<Option<JSONRPCMessage>, Error> {
        // Extract method and params based on message type
        match message {
            Message::Client(client_message) =>
                match client_message {
                    crate::protocol::ClientMessage::Request(request) => {
                        match request {
                            crate::protocol::ClientRequest::ListPrompts(list_prompts_request) => {
                                debug!("Handling prompts/list request");
                                let params = list_prompts_request.params.as_ref();

                                // Process request
                                let result = self.handle_list_prompts(client_id, params).await?;

                                // Create response
                                Ok(Some(crate::protocol::response_from_typed(request_id, result)))
                            }
                            crate::protocol::ClientRequest::GetPrompt(get_prompt_request) => {
                                debug!("Handling prompts/get request");

                                // Process request
                                let result = self.handle_get_prompt(
                                    client_id,
                                    &get_prompt_request.params
                                ).await?;

                                // Create response
                                Ok(Some(crate::protocol::response_from_typed(request_id, result)))
                            }
                            _ => Ok(None),
                        }
                    }
                    _ => Ok(None),
                }
            _ => Ok(None),
        }
    }

    fn service_provider(&self) -> Arc<ServiceProvider> {
        self.service_provider.clone()
    }
}

impl PromptsHandler {
    /// Handle a prompts/list request
    async fn handle_list_prompts(
        &self,
        _client_id: Option<&str>,
        _params: Option<&crate::protocol::PaginatedRequestParams>
    ) -> Result<ListPromptsResult, Error> {
        let prompt_manager = self.service_provider.prompt_manager();
        let prompts = prompt_manager.list_prompts().await;

        // Convert internal Prompt objects to protocol Prompt objects
        let protocol_prompts = prompts
            .iter()
            .map(|p| ProtocolPrompt {
                name: p.get_name().to_string(),
                description: p.get_description().map(|s| s.to_string()),
                arguments: Some(
                    p
                        .get_arguments()
                        .iter()
                        .map(|arg| ProtocolPromptArgument {
                            name: arg.name.clone(),
                            description: arg.description.clone(),
                            required: Some(arg.required),
                        })
                        .collect()
                ),
            })
            .collect();

        // Create the result object
        let result = ListPromptsResult {
            prompts: protocol_prompts,
            next_cursor: None,
            _meta: None,
        };

        Ok(result)
    }

    /// Handle a prompts/get request
    async fn handle_get_prompt(
        &self,
        _client_id: Option<&str>,
        params: &GetPromptParams
    ) -> Result<GetPromptResult, Error> {
        // Convert arguments from map of strings to map of Values if present
        let arguments = params.arguments.as_ref().map(|args| {
            let mut value_map = HashMap::new();
            for (k, v) in args {
                value_map.insert(k.clone(), Value::String(v.clone()));
            }
            value_map
        });

        // Render the prompt
        let prompt_manager = self.service_provider.prompt_manager();
        let messages = prompt_manager.render_prompt(&params.name, arguments).await?;

        // Create response
        let result = GetPromptResult {
            messages,
            description: None,
            _meta: None,
        };

        Ok(result)
    }
}
