//! MCP Message Types
//!
//! This module defines the core message structures for the MCP protocol, including
//! requests, responses, and notifications. It leverages Serde for JSON serialization
//! and Schemars for JSON schema generation to ensure compliance with the MCP specification.

use crate::errors::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Import the types we need from our types module

/// The top-level Message enum for handling JSON-RPC 2.0 messages
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Message {
    Request(GenericRequest),
    Response(Response),
    Notification(Notification),
}

/// Generic request type for initial deserialization of requests
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GenericRequest {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request ID, optional for notifications
    pub id: Option<i32>,
    /// Method name (e.g., "initialize")
    pub method: String,
    /// Generic parameters, to be parsed into specific types based on method
    pub params: Option<serde_json::Value>,
}

/// Typed request with specific parameter type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request<T> {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request ID, optional for notifications
    pub id: Option<i32>,
    /// Method name (e.g., "initialize")
    pub method: String,
    /// Typed parameters for the method
    pub params: T,
}

/// Response for a JSON-RPC request
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Response {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// ID from the request
    pub id: i32,
    /// Either a result or an error
    #[serde(flatten)]
    pub outcome: ResponseOutcome,
}

/// Represents either a successful result or an error
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
pub enum ResponseOutcome {
    /// Success case with a result
    Success { result: serde_json::Value },
    /// Error case with error details
    Error { error: ErrorData },
}

/// Error information for failed requests
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ErrorData {
    /// Error code (e.g., -32600 for Invalid Request)
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Optional additional data
    pub data: Option<serde_json::Value>,
}

/// Notification is a request without an expected response
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Notification {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Method name (e.g., "textDocument/didOpen")
    pub method: String,
    /// Optional parameters for the notification
    pub params: Option<serde_json::Value>,
}

/// Enum for type-safe handling of client requests
#[derive(Debug, Clone)]
pub enum ClientRequest {
    /// Initialize request
    Initialize(Request<crate::types::initialize::InitializeRequestParams>),
    /// List resources request
    ListResources(Request<crate::types::resources::ListResourcesParams>),
    // Additional methods will be added as needed
}

impl ClientRequest {
    /// Convert a GenericRequest to a typed ClientRequest based on the method
    pub fn from_generic(req: GenericRequest) -> Result<Self, Box<dyn std::error::Error>> {
        match req.method.as_str() {
            "initialize" => {
                let params = serde_json::from_value(req.params.unwrap_or_default())?;
                Ok(ClientRequest::Initialize(Request {
                    jsonrpc: req.jsonrpc,
                    id: req.id,
                    method: req.method,
                    params,
                }))
            }
            "resources/list" => {
                let params = serde_json::from_value(req.params.unwrap_or_default())?;
                Ok(ClientRequest::ListResources(Request {
                    jsonrpc: req.jsonrpc,
                    id: req.id,
                    method: req.method,
                    params,
                }))
            }
            _ => Err(format!("Unknown method: {}", req.method).into()),
        }
    }
}

/// Enum for type-safe handling of server responses
#[derive(Debug, Clone)]
pub enum ClientResponse {
    /// Initialize response
    Initialize(crate::types::initialize::InitializeResult),
    /// List resources response
    ListResources(crate::types::resources::ListResourcesResult),
    // Additional responses will be added as needed
}

impl ClientResponse {
    /// Convert a ClientResponse to a JSON-RPC Response
    pub fn to_response(&self, id: i32) -> Response {
        let result = match self {
            ClientResponse::Initialize(init_result) => serde_json::to_value(init_result).unwrap(),
            ClientResponse::ListResources(list_result) => {
                serde_json::to_value(list_result).unwrap()
            }
        };

        Response {
            jsonrpc: "2.0".to_string(),
            id,
            outcome: ResponseOutcome::Success { result },
        }
    }
}

/// Create an error response with the given code, message, and optional data
pub fn error_response(
    id: i32,
    code: i32,
    message: &str,
    data: Option<serde_json::Value>,
) -> Response {
    Response {
        jsonrpc: "2.0".to_string(),
        id,
        outcome: ResponseOutcome::Error {
            error: ErrorData {
                code,
                message: message.to_string(),
                data,
            },
        },
    }
}

impl Message {
    /// Try to split a message that might contain client ID information
    ///
    /// This is intended for use with server-side transports (WebSocketServerTransport, SseServerTransport)
    /// where the transport might include the client ID with the message.
    ///
    /// Returns None if the message doesn't have client ID information
    pub fn try_split_with_client_id(&self) -> Option<(String, Result<Message, Error>)> {
        // This is a placeholder for transport-specific implementations
        // In a real implementation, transports would wrap their messages
        // with client ID information
        None
    }
}

// Add custom deserialization to correctly distinguish between requests and notifications
impl<'de> serde::de::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error;

        // First deserialize to a generic Value
        let value = serde_json::Value::deserialize(deserializer)?;

        // Check for jsonrpc field - all messages must have this
        if !value.get("jsonrpc").is_some() {
            return Err(D::Error::custom("Missing jsonrpc field"));
        }

        // Check message type based on fields
        if value.get("result").is_some() || value.get("error").is_some() {
            // Has result or error - it's a response
            let response: Response = serde_json::from_value(value.clone())
                .map_err(|e| D::Error::custom(e.to_string()))?;
            return Ok(Message::Response(response));
        } else if value.get("id").is_some() {
            // Has ID - it's a request
            let request: GenericRequest = serde_json::from_value(value.clone())
                .map_err(|e| D::Error::custom(e.to_string()))?;
            return Ok(Message::Request(request));
        } else if value.get("method").is_some() {
            // Has method but no ID - it's a notification
            let notification: Notification = serde_json::from_value(value.clone())
                .map_err(|e| D::Error::custom(e.to_string()))?;
            return Ok(Message::Notification(notification));
        }

        Err(D::Error::custom("Invalid JSON-RPC message format"))
    }
}
