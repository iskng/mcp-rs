use crate::errors::Error;
use crate::server::resources::{ ResourceRegistry, ResourceProvider, TemplateResourceProvider };
use crate::server::MessageHandler;
use crate::types::protocol::{
    Message,
    Notification,
    Request,
    RequestType,
    Response,
    ResponseOutcome,
    ErrorData,
};
use crate::types::resources::{
    Resource,
    ResourceTemplate,
    ResourceContent,
    Parameter,
    ListResourcesParams,
    ListResourcesResult,
    ReadResourceParams,
    ReadResourceResult,
    SubscribeResourceParams,
    SubscribeResourceResult,
    UnsubscribeResourceParams,
    UnsubscribeResourceResult,
    ListResourceTemplatesParams,
    ListResourceTemplatesResult,
    UriTemplate,
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use tokio::sync::{ mpsc, RwLock };
use tracing::{ debug, error, info, warn };

/// Handler for resource-related messages
#[derive(Clone)]
pub struct ResourceHandler {
    registry: Arc<ResourceRegistry>,
}

impl ResourceHandler {
    /// Create a new resource handler
    pub fn new(registry: Arc<ResourceRegistry>) -> Self {
        Self { registry }
    }

    /// Handle a list resources request
    async fn handle_list_resources(
        &self,
        client_id: &str,
        request: &Request
    ) -> Result<Response, Error> {
        let params = match &request.params {
            Some(p) =>
                serde_json
                    ::from_value::<ListResourcesParams>(p.clone())
                    .map_err(|e|
                        Error::InvalidParams(format!("Invalid list resources params: {}", e))
                    )?,
            None =>
                ListResourcesParams {
                    mime_type: None,
                    page_token: None,
                    page_size: None,
                },
        };

        let result = self.registry.list_resources(&params).await?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            outcome: ResponseOutcome::Success {
                result: json!(result),
            },
        })
    }

    /// Handle a list resource templates request
    async fn handle_list_templates(
        &self,
        client_id: &str,
        request: &Request
    ) -> Result<Response, Error> {
        let params = match &request.params {
            Some(p) =>
                serde_json
                    ::from_value::<ListResourceTemplatesParams>(p.clone())
                    .map_err(|e|
                        Error::InvalidParams(format!("Invalid list templates params: {}", e))
                    )?,
            None =>
                ListResourceTemplatesParams {
                    mime_type: None,
                },
        };

        let result = self.registry.list_templates(&params).await?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            outcome: ResponseOutcome::Success {
                result: json!(result),
            },
        })
    }

    /// Handle a read resource request
    async fn handle_read_resource(
        &self,
        client_id: &str,
        request: &Request
    ) -> Result<Response, Error> {
        let params = match &request.params {
            Some(p) =>
                serde_json
                    ::from_value::<ReadResourceParams>(p.clone())
                    .map_err(|e|
                        Error::InvalidParams(format!("Invalid read resource params: {}", e))
                    )?,
            None => {
                return Err(Error::InvalidParams("Missing params for read resource".to_string()));
            }
        };

        let content = self.registry.read_resource(&params.uri).await?;
        let result = ReadResourceResult { content };

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            outcome: ResponseOutcome::Success {
                result: json!(result),
            },
        })
    }

    /// Handle a subscribe resource request
    async fn handle_subscribe(
        &self,
        client_id: &str,
        request: &Request
    ) -> Result<Response, Error> {
        let params = match
            serde_json::from_value::<SubscribeResourceParams>(
                request.params.clone().unwrap_or(serde_json::json!({}))
            )
        {
            Ok(params) => params,
            Err(e) => {
                error!("Failed to parse subscribe parameters: {}", e);
                return Err(Error::Protocol(format!("Invalid subscribe parameters: {}", e)));
            }
        };

        // Let the registry handle the subscription and validation
        match self.registry.subscribe(client_id, &params.uri).await {
            Ok(_) => {
                let result = SubscribeResourceResult {
                    success: true,
                };

                Ok(Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    outcome: ResponseOutcome::Success {
                        result: json!(result),
                    },
                })
            }
            Err(e) => {
                // The registry will return appropriate errors if subscription isn't supported
                error!("Failed to subscribe: {}", e);
                Ok(Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    outcome: ResponseOutcome::Error {
                        error: ErrorData {
                            code: -32000,
                            message: format!("Failed to subscribe: {}", e),
                            data: None,
                        },
                    },
                })
            }
        }
    }

    /// Handle an unsubscribe resource request
    async fn handle_unsubscribe(
        &self,
        client_id: &str,
        request: &Request
    ) -> Result<Response, Error> {
        let params = match
            serde_json::from_value::<UnsubscribeResourceParams>(
                request.params.clone().unwrap_or(serde_json::json!({}))
            )
        {
            Ok(params) => params,
            Err(e) => {
                error!("Failed to parse unsubscribe parameters: {}", e);
                return Err(Error::Protocol(format!("Invalid unsubscribe parameters: {}", e)));
            }
        };

        // Let the registry handle the unsubscription and validation
        match self.registry.unsubscribe(client_id, &params.uri).await {
            Ok(_) => {
                let result = UnsubscribeResourceResult {
                    success: true,
                };

                Ok(Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    outcome: ResponseOutcome::Success {
                        result: json!(result),
                    },
                })
            }
            Err(e) => {
                // The registry will return appropriate errors if subscription isn't supported
                error!("Failed to unsubscribe: {}", e);
                Ok(Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    outcome: ResponseOutcome::Error {
                        error: ErrorData {
                            code: -32000,
                            message: format!("Failed to unsubscribe: {}", e),
                            data: None,
                        },
                    },
                })
            }
        }
    }
}

#[async_trait]
impl MessageHandler for ResourceHandler {
    async fn handle(&self, client_id: &str, message: &Message) -> Result<Option<Message>, Error> {
        match message {
            Message::Request(request) => {
                let response = match request.method.as_str() {
                    "resources/list" => self.handle_list_resources(client_id, request).await?,
                    "resources/templates/list" =>
                        self.handle_list_templates(client_id, request).await?,
                    "resources/read" => self.handle_read_resource(client_id, request).await?,
                    "resources/subscribe" => self.handle_subscribe(client_id, request).await?,
                    "resources/unsubscribe" => self.handle_unsubscribe(client_id, request).await?,
                    _ => {
                        return Err(
                            Error::MethodNotFound(format!("Method not found: {}", request.method))
                        );
                    }
                };

                Ok(Some(Message::Response(response)))
            }
            _ => Err(Error::Protocol("Invalid request format".to_string())),
        }
    }
}
