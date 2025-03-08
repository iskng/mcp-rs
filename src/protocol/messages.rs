//! Helper functions for working with JSON-RPC messages

use serde::de::Error as DeError;
use serde::{ Deserialize, Serialize };
use serde_json::{ Map, Value };
use std::collections::HashMap;
use std::fmt;

use crate::protocol::errors::Error;
use crate::protocol::{
    JSONRPCError,
    JSONRPCMessage,
    JSONRPCNotification,
    JSONRPCRequest,
    JSONRPCResponse,
    RequestId,
    Result as ProtocolResult,
};

use super::{
    ClientNotification,
    ClientRequest,
    ClientResult,
    ServerNotification,
    ServerRequest,
    ServerResult,
};

impl JSONRPCMessage {
    /// Parse a request into its typed variant
    pub fn parse_request<T>(&self) -> Result<T, serde_json::Error>
        where T: for<'de> Deserialize<'de>
    {
        match self {
            JSONRPCMessage::Request(request) => {
                // Create a temporary object with method and params for deserialization
                let mut value = Map::new();
                value.insert("method".to_string(), Value::String(request.method.clone()));

                if let Some(params) = &request.params {
                    value.insert("params".to_string(), params.clone());
                } else {
                    value.insert("params".to_string(), Value::Object(Map::new()));
                }

                serde_json::from_value(Value::Object(value))
            }
            _ => Err(serde_json::Error::custom("Not a request message")),
        }
    }

    /// Parse a notification into its typed variant
    pub fn parse_notification<T>(&self) -> Result<T, serde_json::Error>
        where T: for<'de> Deserialize<'de>
    {
        match self {
            JSONRPCMessage::Notification(notification) => {
                // Create a temporary object with method and params for deserialization
                let mut value = Map::new();
                value.insert("method".to_string(), Value::String(notification.method.clone()));

                if let Some(params) = &notification.params {
                    value.insert("params".to_string(), params.clone());
                } else {
                    value.insert("params".to_string(), Value::Object(Map::new()));
                }

                serde_json::from_value(Value::Object(value))
            }
            _ => Err(serde_json::Error::custom("Not a notification message")),
        }
    }

    /// Parse a response into its typed variant
    pub fn parse_response<T>(&self) -> Result<T, serde_json::Error>
        where T: for<'de> Deserialize<'de>
    {
        match self {
            JSONRPCMessage::Response(response) => {
                // Method needs to be added for proper deserialization
                let method = infer_method_from_result(&response.result.content)?;

                let mut value = Map::new();
                value.insert("method".to_string(), Value::String(method));
                value.insert("result".to_string(), serde_json::to_value(&response.result)?);

                serde_json::from_value(Value::Object(value))
            }
            JSONRPCMessage::Error(error) =>
                Err(
                    serde_json::Error::custom(
                        format!(
                            "Cannot parse error response to typed variant: {}",
                            error.error.message
                        )
                    )
                ),
            _ => Err(serde_json::Error::custom("Not a response message")),
        }
    }

    /// Get the request ID if this is a request or response
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            JSONRPCMessage::Request(req) => Some(&req.id),
            JSONRPCMessage::Response(resp) => Some(&resp.id),
            JSONRPCMessage::Error(err) => Some(&err.id),
            JSONRPCMessage::Notification(_) => None,
        }
    }

    /// Get the method name if this is a request or notification
    pub fn method(&self) -> Option<&str> {
        match self {
            JSONRPCMessage::Request(req) => Some(&req.method),
            JSONRPCMessage::Notification(notification) => Some(&notification.method),
            JSONRPCMessage::Response(_) | JSONRPCMessage::Error(_) => None,
        }
    }

    /// Get the protocol message type without consuming the message
    pub fn get_type(&self) -> McpMessageType {
        match self {
            JSONRPCMessage::Request(req) => {
                match req.method.as_str() {
                    // Client requests
                    "initialize" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::Initialize)
                        ),
                    "ping" => {
                        McpMessageType::Client(ClientMessageType::Request(ClientRequestType::Ping))
                    }
                    "resources/list" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::ListResources)
                        ),
                    "resources/templates/list" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::ListResourceTemplates)
                        ),
                    "resources/read" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::ReadResource)
                        ),
                    "resources/subscribe" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::Subscribe)
                        ),
                    "resources/unsubscribe" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::Unsubscribe)
                        ),
                    "prompts/list" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::ListPrompts)
                        ),
                    "prompts/get" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::GetPrompt)
                        ),
                    "tools/list" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::ListTools)
                        ),
                    "tools/call" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::CallTool)
                        ),
                    "logging/setLevel" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::SetLevel)
                        ),
                    "completion/complete" =>
                        McpMessageType::Client(
                            ClientMessageType::Request(ClientRequestType::Complete)
                        ),

                    // Server requests
                    "sampling/createMessage" =>
                        McpMessageType::Server(
                            ServerMessageType::Request(ServerRequestType::CreateMessage)
                        ),
                    "roots/list" =>
                        McpMessageType::Server(
                            ServerMessageType::Request(ServerRequestType::ListRoots)
                        ),

                    // Unknown method
                    _ => McpMessageType::Unknown(req.method.clone()),
                }
            }
            JSONRPCMessage::Notification(notif) => {
                match notif.method.as_str() {
                    // Client notifications
                    "notifications/cancelled" =>
                        McpMessageType::Client(
                            ClientMessageType::Notification(ClientNotificationType::Cancelled)
                        ),
                    "notifications/initialized" =>
                        McpMessageType::Client(
                            ClientMessageType::Notification(ClientNotificationType::Initialized)
                        ),
                    "notifications/progress" =>
                        McpMessageType::Client(
                            ClientMessageType::Notification(ClientNotificationType::Progress)
                        ),
                    "notifications/roots/list_changed" =>
                        McpMessageType::Client(
                            ClientMessageType::Notification(
                                ClientNotificationType::RootsListChanged
                            )
                        ),

                    // Server notifications
                    "notifications/resources/list_changed" => {
                        McpMessageType::Server(
                            ServerMessageType::Notification(
                                ServerNotificationType::ResourceListChanged
                            )
                        )
                    }
                    "notifications/resources/updated" =>
                        McpMessageType::Server(
                            ServerMessageType::Notification(ServerNotificationType::ResourceUpdated)
                        ),
                    "notifications/prompts/list_changed" =>
                        McpMessageType::Server(
                            ServerMessageType::Notification(
                                ServerNotificationType::PromptListChanged
                            )
                        ),
                    "notifications/tools/list_changed" =>
                        McpMessageType::Server(
                            ServerMessageType::Notification(ServerNotificationType::ToolListChanged)
                        ),
                    "notifications/logging/message" =>
                        McpMessageType::Server(
                            ServerMessageType::Notification(ServerNotificationType::LoggingMessage)
                        ),

                    // Unknown method
                    _ => McpMessageType::Unknown(notif.method.clone()),
                }
            }
            JSONRPCMessage::Response(resp) => {
                // Try to infer the method from the result structure
                if let Ok(method) = infer_method_from_result(&resp.result.content) {
                    match method.as_str() {
                        // Client results
                        "sampling/createMessage" =>
                            McpMessageType::Client(
                                ClientMessageType::Result(ClientResultType::CreateMessage)
                            ),
                        "roots/list" =>
                            McpMessageType::Client(
                                ClientMessageType::Result(ClientResultType::ListRoots)
                            ),

                        // Server results
                        "initialize" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::Initialize)
                            ),
                        "resources/list" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::ListResources)
                            ),
                        "resources/templates/list" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::ListResourceTemplates)
                            ),
                        "resources/read" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::ReadResource)
                            ),
                        "prompts/list" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::ListPrompts)
                            ),
                        "prompts/get" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::GetPrompt)
                            ),
                        "tools/list" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::ListTools)
                            ),
                        "tools/call" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::CallTool)
                            ),
                        "completion/complete" =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::Complete)
                            ),

                        // Empty result
                        _ =>
                            McpMessageType::Server(
                                ServerMessageType::Result(ServerResultType::Empty)
                            ),
                    }
                } else {
                    // If we can't infer the method, default to empty server result
                    McpMessageType::Server(ServerMessageType::Result(ServerResultType::Empty))
                }
            }
            JSONRPCMessage::Error(_) => McpMessageType::Error,
        }
    }

    /// Convert this JSON-RPC message into a strongly-typed protocol Message
    pub fn into_message(self) -> Result<Message, serde_json::Error> {
        match self {
            JSONRPCMessage::Request(req) => {
                // Create a JSON object with method and params for deserialization
                let json =
                    serde_json::json!({
                    "method": req.method,
                    "params": req.params.unwrap_or(serde_json::Value::Null)
                });

                match req.method.as_str() {
                    // Client requests
                    "initialize" => {
                        let typed: crate::protocol::InitializeRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Request(ClientRequest::Initialize(typed))
                            )
                        )
                    }
                    "ping" => {
                        let typed: crate::protocol::PingRequest = serde_json::from_value(json)?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::Ping(typed))))
                    }
                    "resources/list" => {
                        let typed: crate::protocol::ListResourcesRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Request(ClientRequest::ListResources(typed))
                            )
                        )
                    }
                    "resources/templates/list" => {
                        let typed: crate::protocol::ListResourceTemplatesRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Request(ClientRequest::ListResourceTemplates(typed))
                            )
                        )
                    }
                    "resources/read" => {
                        let typed: crate::protocol::ReadResourceRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Request(ClientRequest::ReadResource(typed))
                            )
                        )
                    }
                    "resources/subscribe" => {
                        let typed: crate::protocol::SubscribeRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::Subscribe(typed))))
                    }
                    "resources/unsubscribe" => {
                        let typed: crate::protocol::UnsubscribeRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Request(ClientRequest::Unsubscribe(typed))
                            )
                        )
                    }
                    "prompts/list" => {
                        let typed: crate::protocol::ListPromptsRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Request(ClientRequest::ListPrompts(typed))
                            )
                        )
                    }
                    "prompts/get" => {
                        let typed: crate::protocol::GetPromptRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::GetPrompt(typed))))
                    }
                    "tools/list" => {
                        let typed: crate::protocol::ListToolsRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::ListTools(typed))))
                    }
                    "tools/call" => {
                        let typed: crate::protocol::CallToolRequest = serde_json::from_value(json)?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::CallTool(typed))))
                    }
                    "logging/setLevel" => {
                        let typed: crate::protocol::SetLevelRequest = serde_json::from_value(json)?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::SetLevel(typed))))
                    }
                    "completion/complete" => {
                        let typed: crate::protocol::CompleteRequest = serde_json::from_value(json)?;
                        Ok(Message::Client(ClientMessage::Request(ClientRequest::Complete(typed))))
                    }

                    // Server requests
                    "roots/list" => {
                        let typed: crate::protocol::ListRootsRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(Message::Server(ServerMessage::Request(ServerRequest::ListRoots(typed))))
                    }
                    "sampling/createMessage" => {
                        let typed: crate::protocol::CreateMessageRequest = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Server(
                                ServerMessage::Request(ServerRequest::CreateMessage(typed))
                            )
                        )
                    }

                    // Unknown method
                    _ =>
                        Err(
                            serde_json::Error::custom(
                                format!("Unknown request method: {}", req.method)
                            )
                        ),
                }
            }
            JSONRPCMessage::Notification(notif) => {
                // Create a JSON object with method and params for deserialization
                let json =
                    serde_json::json!({
                    "method": notif.method,
                    "params": notif.params.unwrap_or(serde_json::Value::Null)
                });

                match notif.method.as_str() {
                    // Client notifications
                    "notifications/cancelled" => {
                        let typed: crate::protocol::CancelledNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Notification(ClientNotification::Cancelled(typed))
                            )
                        )
                    }
                    "notifications/initialized" => {
                        let typed: crate::protocol::InitializedNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Notification(ClientNotification::Initialized(typed))
                            )
                        )
                    }
                    "notifications/progress" => {
                        let typed: crate::protocol::ProgressNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Notification(ClientNotification::Progress(typed))
                            )
                        )
                    }
                    "notifications/roots/list_changed" => {
                        let typed: crate::protocol::RootsListChangedNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Client(
                                ClientMessage::Notification(
                                    ClientNotification::RootsListChanged(typed)
                                )
                            )
                        )
                    }

                    // Server notifications
                    "notifications/resources/list_changed" => {
                        let typed: crate::protocol::ResourceListChangedNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Server(
                                ServerMessage::Notification(
                                    ServerNotification::ResourceListChanged(typed)
                                )
                            )
                        )
                    }
                    "notifications/resources/updated" => {
                        let typed: crate::protocol::ResourceUpdatedNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Server(
                                ServerMessage::Notification(
                                    ServerNotification::ResourceUpdated(typed)
                                )
                            )
                        )
                    }
                    "notifications/prompts/list_changed" => {
                        let typed: crate::protocol::PromptListChangedNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Server(
                                ServerMessage::Notification(
                                    ServerNotification::PromptListChanged(typed)
                                )
                            )
                        )
                    }
                    "notifications/tools/list_changed" => {
                        let typed: crate::protocol::ToolListChangedNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Server(
                                ServerMessage::Notification(
                                    ServerNotification::ToolListChanged(typed)
                                )
                            )
                        )
                    }
                    "notifications/logging/message" => {
                        let typed: crate::protocol::LoggingMessageNotification = serde_json::from_value(
                            json
                        )?;
                        Ok(
                            Message::Server(
                                ServerMessage::Notification(
                                    ServerNotification::LoggingMessage(typed)
                                )
                            )
                        )
                    }

                    // Unknown method
                    _ =>
                        Err(
                            serde_json::Error::custom(
                                format!("Unknown notification method: {}", notif.method)
                            )
                        ),
                }
            }
            JSONRPCMessage::Response(resp) => {
                // Try to infer the method from the result structure
                if let Ok(method) = infer_method_from_result(&resp.result.content) {
                    match method.as_str() {
                        // Client results
                        "sampling/createMessage" => {
                            let typed: crate::protocol::CreateMessageResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Client(
                                    ClientMessage::Result(ClientResult::CreateMessage(typed))
                                )
                            )
                        }
                        "roots/list" => {
                            let typed: crate::protocol::ListRootsResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Client(
                                    ClientMessage::Result(ClientResult::ListRoots(typed))
                                )
                            )
                        }

                        // Server results
                        "initialize" => {
                            let typed: crate::protocol::InitializeResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::Initialize(typed))
                                )
                            )
                        }
                        "resources/list" => {
                            let typed: crate::protocol::ListResourcesResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::ListResources(typed))
                                )
                            )
                        }
                        "resources/templates/list" => {
                            let typed: crate::protocol::ListResourceTemplatesResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(
                                        ServerResult::ListResourceTemplates(typed)
                                    )
                                )
                            )
                        }
                        "resources/read" => {
                            let typed: crate::protocol::ReadResourceResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::ReadResource(typed))
                                )
                            )
                        }
                        "prompts/list" => {
                            let typed: crate::protocol::ListPromptsResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::ListPrompts(typed))
                                )
                            )
                        }
                        "prompts/get" => {
                            let typed: crate::protocol::GetPromptResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::GetPrompt(typed))
                                )
                            )
                        }
                        "tools/list" => {
                            let typed: crate::protocol::ListToolsResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::ListTools(typed))
                                )
                            )
                        }
                        "tools/call" => {
                            let typed: crate::protocol::CallToolResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::CallTool(typed))
                                )
                            )
                        }
                        "completion/complete" => {
                            let typed: crate::protocol::CompleteResult = serde_json::from_value(
                                serde_json::to_value(&resp.result)?
                            )?;
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::Complete(typed))
                                )
                            )
                        }

                        // Empty result
                        _ =>
                            Ok(
                                Message::Server(
                                    ServerMessage::Result(ServerResult::Empty(resp.result))
                                )
                            ),
                    }
                } else {
                    // If we can't infer the method, create an empty result
                    Ok(Message::Server(ServerMessage::Result(ServerResult::Empty(resp.result))))
                }
            }
            JSONRPCMessage::Error(err) => {
                // Error messages
                Ok(Message::Error(err))
            }
        }
    }
}

/// Type of protocol message - detailed hierarchical type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpMessageType {
    Client(ClientMessageType),
    Server(ServerMessageType),
    Error,
    Unknown(String),
}

/// Type of client message
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMessageType {
    Request(ClientRequestType),
    Notification(ClientNotificationType),
    Result(ClientResultType),
}

/// Type of server message
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessageType {
    Request(ServerRequestType),
    Notification(ServerNotificationType),
    Result(ServerResultType),
}

/// Type of client request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientRequestType {
    Initialize,
    Ping,
    ListResources,
    ListResourceTemplates,
    ReadResource,
    Subscribe,
    Unsubscribe,
    ListPrompts,
    GetPrompt,
    ListTools,
    CallTool,
    SetLevel,
    Complete,
}

/// Type of client notification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientNotificationType {
    Cancelled,
    Initialized,
    Progress,
    RootsListChanged,
}

/// Type of client result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientResultType {
    Empty,
    CreateMessage,
    ListRoots,
}

/// Type of server request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerRequestType {
    Ping,
    CreateMessage,
    ListRoots,
}

/// Type of server notification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerNotificationType {
    Cancelled,
    Progress,
    ResourceListChanged,
    ResourceUpdated,
    PromptListChanged,
    ToolListChanged,
    LoggingMessage,
}

/// Type of server result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerResultType {
    Empty,
    Initialize,
    ListResources,
    ListResourceTemplates,
    ReadResource,
    ListPrompts,
    GetPrompt,
    ListTools,
    CallTool,
    Complete,
}

/// Hierarchical representation of all protocol message types
#[derive(Debug)]
pub enum Message {
    Client(ClientMessage),
    Server(ServerMessage),
    Error(JSONRPCError),
}

/// All client messages
#[derive(Debug)]
pub enum ClientMessage {
    Request(ClientRequest),
    Notification(ClientNotification),
    Result(ClientResult),
}

/// All server messages
#[derive(Debug)]
pub enum ServerMessage {
    Request(ServerRequest),
    Notification(ServerNotification),
    Result(ServerResult),
}

/// Convert a typed request to a JSON-RPC request
pub fn request_from_typed<T>(id: RequestId, request: T) -> Result<JSONRPCRequest, serde_json::Error>
    where T: Serialize
{
    // Serialize then extract method and params
    let serialized = serde_json::to_value(request)?;
    let (method, params) = match serialized {
        Value::Object(mut map) => {
            let method = map
                .remove("method")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            let params = map.remove("params");

            (method, params)
        }
        _ => (String::new(), None),
    };

    Ok(JSONRPCRequest {
        jsonrpc: "2.0".to_string(),
        id,
        method,
        params,
    })
}

/// Convert a typed response to a JSON-RPC response
pub fn response_from_typed<T>(id: RequestId, response: T) -> JSONRPCMessage where T: Serialize {
    // Create a new ProtocolResult with the response content
    let result = match serde_json::to_value(&response) {
        Ok(Value::Object(map)) =>
            ProtocolResult {
                _meta: None,
                content: map.into_iter().collect(),
            },
        Ok(value) => {
            // If it's not an object, we'll put it under a "result" key
            let mut map = Map::new();
            map.insert("result".to_string(), value);
            ProtocolResult {
                _meta: None,
                content: map.into_iter().collect(),
            }
        }
        Err(_) => {
            // Create an error response for serialization failures
            return crate::protocol::errors::to_error_message(
                id,
                &Error::Json(serde_json::Error::custom("Failed to serialize response"))
            );
        }
    };
    tracing::info!("Response from typed: {:?}", result);
    JSONRPCMessage::Response(JSONRPCResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
    })
}

/// Convert a typed notification to a JSON-RPC notification
pub fn notification_from_typed<T>(notification: T) -> Result<JSONRPCNotification, serde_json::Error>
    where T: Serialize
{
    // Serialize then extract method and params
    let serialized = serde_json::to_value(notification)?;
    let (method, params) = match serialized {
        Value::Object(mut map) => {
            let method = map
                .remove("method")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            let params = map.remove("params");

            (method, params)
        }
        _ => (String::new(), None),
    };

    Ok(JSONRPCNotification {
        jsonrpc: "2.0".to_string(),
        method,
        params,
    })
}

/// Create a success response with the given result
pub fn success_response(id: RequestId, content: Value) -> JSONRPCResponse {
    let mut map = Map::new();
    map.insert("result".to_string(), content);

    JSONRPCResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: ProtocolResult {
            _meta: None,
            content: map.into_iter().collect(),
        },
    }
}

/// Infer the method name from a result value
/// This is a heuristic - in real implementations, you should track request methods
fn infer_method_from_result(result: &HashMap<String, Value>) -> Result<String, serde_json::Error> {
    // Use heuristics to determine the method based on fields in the result
    if result.get("capabilities").is_some() && result.get("serverInfo").is_some() {
        return Ok("initialize".to_string());
    }

    if result.get("resources").is_some() {
        return Ok("resources/list".to_string());
    }

    if result.get("resource").is_some() || result.get("contents").is_some() {
        return Ok("resources/read".to_string());
    }

    if result.get("tools").is_some() {
        return Ok("tools/list".to_string());
    }

    if result.get("prompts").is_some() {
        return Ok("prompts/list".to_string());
    }

    // Fallback
    Err(serde_json::Error::custom("Could not infer method from result"))
}

impl fmt::Display for JSONRPCMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JSONRPCMessage::Request(req) => {
                write!(f, "Request {{ id: {:?}, method: {} }}", req.id, req.method)
            }
            JSONRPCMessage::Response(resp) => {
                write!(f, "Response {{ id: {:?}, success: true }}", resp.id)
            }
            JSONRPCMessage::Error(err) => {
                write!(
                    f,
                    "Error {{ id: {:?}, code: {}, message: {} }}",
                    err.id,
                    err.error.code,
                    err.error.message
                )
            }
            JSONRPCMessage::Notification(notif) => {
                write!(f, "Notification {{ method: {} }}", notif.method)
            }
        }
    }
}
