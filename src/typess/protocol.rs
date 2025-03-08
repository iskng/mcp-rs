//! Message types definitions for the Model Context Protocol (MCP)
//!
//! This module defines the core JSON-RPC 2.0 message structures and type-safe
//! representations of MCP protocol messages, providing serialization and
//! deserialization between JSON and Rust types.

use serde::{ Serialize, Deserialize };
use schemars::JsonSchema;

use crate::types::initialize::{ InitializeRequestParams, InitializeResult };
use crate::types::prompts::{ ListPromptsParams, ListPromptsResult };
use crate::types::resources::{
    CreateResourceParams,
    CreateResourceResult,
    DeleteResourceParams,
    DeleteResourceResult,
    ReadResourceParams,
    ReadResourceResult,
    ListResourcesParams,
    ListResourcesResult,
    UpdateResourceParams,
    UpdateResourceResult,
};
use crate::types::tools::{ CallToolParams, CallToolResult, ListToolsParams, ListToolsResult };

/// Role in the conversation
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User role
    User,
    /// Assistant role
    Assistant,
}

/// The top-level Message enum for handling JSON-RPC 2.0 messages
#[derive(Debug, Clone)]
pub enum Message {
    /// A request message requiring a response
    Request(Request),
    /// A response to a request
    Response(Response),
    /// A notification message (no response expected)
    Notification(Notification),
}

/// A JSON-RPC 2.0 request message
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Request {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request ID for matching responses
    pub id: i32,
    /// Method name (e.g., "initialize")
    pub method: String,
    /// Method-specific parameters
    pub params: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 response message
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Response {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request ID this response is for
    pub id: i32,
    /// Either a result or an error
    #[serde(flatten)]
    pub outcome: ResponseOutcome,
}

/// Either a successful result or an error
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ResponseOutcome {
    /// Success case with a result
    Success {
        result: serde_json::Value,
    },
    /// Error case with error details
    Error {
        error: ErrorData,
    },
}

/// Error information for failed requests
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorData {
    /// Error code (e.g., -32600 for Invalid Request)
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Optional additional data
    pub data: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 notification message
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Notification {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Method name (e.g., "notifications/initialized")
    pub method: String,
    /// Method-specific parameters
    pub params: Option<serde_json::Value>,
}

/// Notification messages that do not expect responses
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "method", content = "params")]
pub enum NotificationMessage {
    #[serde(rename = "notifications/initialized")] Initialized {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "notifications/progress")] Progress {
        progress: f64,
        #[serde(rename = "progressToken")]
        progress_token: serde_json::Value,
        total: Option<f64>,
    },
    #[serde(rename = "notifications/resources/list_changed")] ResourcesListChanged {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "notifications/resources/updated")] ResourcesUpdated {
        uri: String,
    },
    #[serde(rename = "notifications/prompts/list_changed")] PromptsListChanged {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "notifications/tools/list_changed")] ToolsListChanged {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "$/cancelRequest")] CancelRequest {
        #[serde(rename = "requestId")]
        request_id: serde_json::Value,
        reason: Option<String>,
    },
}

/// Request messages that expect responses
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "method", content = "params")]
pub enum RequestMessage {
    #[serde(rename = "initialize")] Initialize {
        #[serde(flatten)]
        params: InitializeRequestParams,
    },
    #[serde(rename = "ping")] Ping {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/list")] ListResources {
        #[serde(flatten)]
        params: ListResourcesParams,
    },
    #[serde(rename = "resources/read")] ReadResource {
        #[serde(flatten)]
        params: ReadResourceParams,
    },
    #[serde(rename = "resources/create")] CreateResource {
        #[serde(flatten)]
        params: CreateResourceParams,
    },
    #[serde(rename = "resources/update")] UpdateResource {
        #[serde(flatten)]
        params: UpdateResourceParams,
    },
    #[serde(rename = "resources/delete")] DeleteResource {
        #[serde(flatten)]
        params: DeleteResourceParams,
    },
    #[serde(rename = "resources/subscribe")] SubscribeResource {
        uri: String,
    },
    #[serde(rename = "resources/unsubscribe")] UnsubscribeResource {
        uri: String,
    },
    #[serde(rename = "tools/list")] ListTools {
        #[serde(flatten)]
        params: ListToolsParams,
    },
    #[serde(rename = "tools/call")] CallTool {
        #[serde(flatten)]
        params: CallToolParams,
    },
    #[serde(rename = "prompts/list")] ListPrompts {
        #[serde(flatten)]
        params: ListPromptsParams,
    },
    #[serde(rename = "prompts/get")] GetPrompt {
        name: String,
    },
}

/// Response message types for each request type
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "method", content = "result")]
pub enum ResponseMessage {
    #[serde(rename = "initialize")] Initialize {
        #[serde(flatten)]
        result: InitializeResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "ping")] Ping {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/list")] ListResources {
        #[serde(flatten)]
        result: ListResourcesResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/read")] ReadResource {
        #[serde(flatten)]
        result: ReadResourceResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/create")] CreateResource {
        #[serde(flatten)]
        result: CreateResourceResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/update")] UpdateResource {
        #[serde(flatten)]
        result: UpdateResourceResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/delete")] DeleteResource {
        #[serde(flatten)]
        result: DeleteResourceResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/subscribe")] SubscribeResource {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "resources/unsubscribe")] UnsubscribeResource {
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "tools/list")] ListTools {
        #[serde(flatten)]
        result: ListToolsResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "tools/call")] CallTool {
        #[serde(flatten)]
        result: CallToolResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
    #[serde(rename = "prompts/list")] ListPrompts {
        #[serde(flatten)]
        result: ListPromptsResult,
        #[serde(flatten)]
        meta: Option<serde_json::Value>,
    },
  
}

/// Enum representing the types of requests in the MCP protocol
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestType {
    Initialize,
    Ping,
    ListResources,
    ReadResource,
    CreateResource,
    UpdateResource,
    DeleteResource,
    SubscribeResource,
    UnsubscribeResource,
    ListTools,
    CallTool,
    ListPrompts,
    CreatePrompt,
    UpdatePrompt,
    DeletePrompt,
    RenderPrompt,
    GetPrompt,
    Other(String),
}

/// Enum representing the types of notifications in the MCP protocol
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotificationType {
    Initialized,
    Progress,
    ResourcesListChanged,
    ResourcesUpdated,
    PromptsListChanged,
    ToolsListChanged,
    CancelRequest,
    Other(String),
}

/// Enum representing the types of responses in the MCP protocol
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResponseType {
    Initialize,
    Ping,
    ListResources,
    ReadResource,
    CreateResource,
    UpdateResource,
    DeleteResource,
    SubscribeResource,
    UnsubscribeResource,
    ListTools,
    CallTool,
    ListPrompts,
    CreatePrompt,
    UpdatePrompt,
    DeletePrompt,
    RenderPrompt,
    GetPrompt,
    Error,
    Other(String),
}

/// High-level message type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageType {
    Request(RequestType),
    Notification(NotificationType),
    Response(ResponseType),
}

// Implementation for Message to convert between JSON-RPC and typed formats
impl Message {
    /// Parse a Notification into its typed variant
    pub fn parse_notification(
        notification: &Notification
    ) -> Result<NotificationMessage, serde_json::Error> {
        // Create a temporary object with method and params for deserialization
        let mut value = serde_json::Map::new();
        value.insert("method".to_string(), serde_json::Value::String(notification.method.clone()));

        if let Some(params) = &notification.params {
            value.insert("params".to_string(), params.clone());
        } else {
            value.insert("params".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        }

        serde_json::from_value(serde_json::Value::Object(value))
    }

    /// Parse a Request into its typed variant
    pub fn parse_request(request: &Request) -> Result<RequestMessage, serde_json::Error> {
        // Create a temporary object with method and params for deserialization
        let mut value = serde_json::Map::new();
        value.insert("method".to_string(), serde_json::Value::String(request.method.clone()));

        if let Some(params) = &request.params {
            value.insert("params".to_string(), params.clone());
        } else {
            value.insert("params".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        }

        serde_json::from_value(serde_json::Value::Object(value))
    }

    /// Parse a Response into its typed variant if it's a success
    pub fn parse_response(
        response: &Response
    ) -> Result<Option<ResponseMessage>, serde_json::Error> {
        match &response.outcome {
            ResponseOutcome::Success { result } => {
                // To deserialize properly, we need to include the method
                // The method should be extracted from a previous request, but we'll have to infer it here
                // In a real implementation, you'd store request method when sending
                let method = infer_method_from_result(result)?;

                let mut value = serde_json::Map::new();
                value.insert("method".to_string(), serde_json::Value::String(method));
                value.insert("result".to_string(), result.clone());

                let typed_response: ResponseMessage = serde_json::from_value(
                    serde_json::Value::Object(value)
                )?;
                Ok(Some(typed_response))
            }
            ResponseOutcome::Error { .. } => Ok(None), // No typed variant for errors
        }
    }

    /// Create a Response from a typed ResponseMessage
    pub fn response_from_typed(
        id: i32,
        response: ResponseMessage
    ) -> Result<Response, serde_json::Error> {
        // Serialize then extract just the result part
        let serialized = serde_json::to_value(response)?;
        let result = match serialized {
            serde_json::Value::Object(mut map) => {
                if let Some(result) = map.remove("result") {
                    result
                } else {
                    serde_json::Value::Object(map)
                }
            }
            value => value,
        };

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id,
            outcome: ResponseOutcome::Success { result },
        })
    }

    /// Create a Request from a typed RequestMessage
    pub fn request_from_typed(
        id: i32,
        request: RequestMessage
    ) -> Result<Request, serde_json::Error> {
        // Serialize then extract method and params
        let serialized = serde_json::to_value(request)?;
        let (method, params) = match serialized {
            serde_json::Value::Object(mut map) => {
                let method = map
                    .remove("method")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();

                let params = map.remove("params");

                (method, params)
            }
            _ => (String::new(), None),
        };

        Ok(Request {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        })
    }

    /// Create a Notification from a typed NotificationMessage
    pub fn notification_from_typed(
        notification: NotificationMessage
    ) -> Result<Notification, serde_json::Error> {
        // Serialize then extract method and params
        let serialized = serde_json::to_value(notification)?;
        let (method, params) = match serialized {
            serde_json::Value::Object(mut map) => {
                let method = map
                    .remove("method")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();

                let params = map.remove("params");

                (method, params)
            }
            _ => (String::new(), None),
        };

        Ok(Notification {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        })
    }

    /// Get the type of this message
    pub fn message_type(&self) -> MessageType {
        match self {
            Message::Request(req) => {
                let request_type = match req.method.as_str() {
                    "initialize" => RequestType::Initialize,
                    "ping" => RequestType::Ping,
                    "resources/list" => RequestType::ListResources,
                    "resources/read" => RequestType::ReadResource,
                    "resources/create" => RequestType::CreateResource,
                    "resources/update" => RequestType::UpdateResource,
                    "resources/delete" => RequestType::DeleteResource,
                    "resources/subscribe" => RequestType::SubscribeResource,
                    "resources/unsubscribe" => RequestType::UnsubscribeResource,
                    "tools/list" => RequestType::ListTools,
                    "tools/call" => RequestType::CallTool,
                    "prompts/list" => RequestType::ListPrompts,
                    "prompts/create" => RequestType::CreatePrompt,
                    "prompts/update" => RequestType::UpdatePrompt,
                    "prompts/delete" => RequestType::DeletePrompt,
                    "prompts/render" => RequestType::RenderPrompt,
                    "prompts/get" => RequestType::GetPrompt,
                    other => RequestType::Other(other.to_string()),
                };
                MessageType::Request(request_type)
            }
            Message::Notification(notif) => {
                let notification_type = match notif.method.as_str() {
                    "notifications/initialized" => NotificationType::Initialized,
                    "notifications/progress" => NotificationType::Progress,
                    "notifications/resources/list_changed" =>
                        NotificationType::ResourcesListChanged,
                    "notifications/resources/updated" => NotificationType::ResourcesUpdated,
                    "notifications/prompts/list_changed" => NotificationType::PromptsListChanged,
                    "notifications/tools/list_changed" => NotificationType::ToolsListChanged,
                    "$/cancelRequest" => NotificationType::CancelRequest,
                    other => NotificationType::Other(other.to_string()),
                };
                MessageType::Notification(notification_type)
            }
            Message::Response(resp) => {
                // For responses, we infer the type from the result if success
                match &resp.outcome {
                    ResponseOutcome::Success { result } => {
                        // Try to infer from result structure
                        if let Ok(method) = infer_method_from_result(result) {
                            let response_type = match method.as_str() {
                                "initialize" => ResponseType::Initialize,
                                "ping" => ResponseType::Ping,
                                "resources/list" => ResponseType::ListResources,
                                "resources/read" => ResponseType::ReadResource,
                                "resources/create" => ResponseType::CreateResource,
                                "resources/update" => ResponseType::UpdateResource,
                                "resources/delete" => ResponseType::DeleteResource,
                                "resources/subscribe" => ResponseType::SubscribeResource,
                                "resources/unsubscribe" => ResponseType::UnsubscribeResource,
                                "tools/list" => ResponseType::ListTools,
                                "tools/call" => ResponseType::CallTool,
                                "prompts/list" => ResponseType::ListPrompts,
                                "prompts/create" => ResponseType::CreatePrompt,
                                "prompts/update" => ResponseType::UpdatePrompt,
                                "prompts/delete" => ResponseType::DeletePrompt,
                                "prompts/render" => ResponseType::RenderPrompt,
                                "prompts/get" => ResponseType::GetPrompt,
                                other => ResponseType::Other(other.to_string()),
                            };
                            MessageType::Response(response_type)
                        } else {
                            MessageType::Response(ResponseType::Other("unknown".to_string()))
                        }
                    }
                    ResponseOutcome::Error { .. } => { MessageType::Response(ResponseType::Error) }
                }
            }
        }
    }

    /// Convenience method to check if this is a specific request type
    pub fn is_request_type(&self, request_type: &RequestType) -> bool {
        match self.message_type() {
            MessageType::Request(rt) => &rt == request_type,
            _ => false,
        }
    }

    /// Convenience method to check if this is a specific notification type
    pub fn is_notification_type(&self, notification_type: &NotificationType) -> bool {
        match self.message_type() {
            MessageType::Notification(nt) => &nt == notification_type,
            _ => false,
        }
    }

    /// Convenience method to check if this is a specific response type
    pub fn is_response_type(&self, response_type: &ResponseType) -> bool {
        match self.message_type() {
            MessageType::Response(rt) => &rt == response_type,
            _ => false,
        }
    }
}

/// Infer the method name from a result value
/// This is a placeholder - in a real implementation you would track request methods
fn infer_method_from_result(result: &serde_json::Value) -> Result<String, serde_json::Error> {
    // Use heuristics to determine the method based on fields in the result
    if result.get("capabilities").is_some() && result.get("serverInfo").is_some() {
        return Ok("initialize".to_string());
    }

    if result.get("resources").is_some() {
        return Ok("resources/list".to_string());
    }

    if result.get("contents").is_some() {
        return Ok("resources/read".to_string());
    }

    if result.get("tools").is_some() {
        return Ok("tools/list".to_string());
    }

    if result.get("content").is_some() {
        return Ok("tools/call".to_string());
    }

    if result.get("prompts").is_some() {
        return Ok("prompts/list".to_string());
    }

    if result.get("messages").is_some() {
        return Ok("prompts/get".to_string());
    }

    // Default to ping for empty results
    Ok("ping".to_string())
}

// Add custom deserialization to correctly distinguish between requests and notifications
impl<'de> serde::de::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::de::Deserializer<'de>
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
            let response: Response = serde_json
                ::from_value(value.clone())
                .map_err(|e| D::Error::custom(e.to_string()))?;
            return Ok(Message::Response(response));
        } else if value.get("id").is_some() {
            // Has ID - it's a request
            let request: Request = serde_json
                ::from_value(value.clone())
                .map_err(|e| D::Error::custom(e.to_string()))?;
            return Ok(Message::Request(request));
        } else if value.get("method").is_some() {
            // Has method but no ID - it's a notification
            let notification: Notification = serde_json
                ::from_value(value.clone())
                .map_err(|e| D::Error::custom(e.to_string()))?;
            return Ok(Message::Notification(notification));
        }

        Err(D::Error::custom("Invalid JSON-RPC message format"))
    }
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        match self {
            Message::Request(req) => req.serialize(serializer),
            Message::Response(resp) => resp.serialize(serializer),
            Message::Notification(notif) => notif.serialize(serializer),
        }
    }
}

// Helper functions
/// Create a success response with the given result
pub fn success_response(id: i32, result: serde_json::Value) -> Response {
    Response {
        jsonrpc: "2.0".to_string(),
        id,
        outcome: ResponseOutcome::Success { result },
    }
}

/// Create an error response with the given code, message, and optional data
pub fn error_response(
    id: i32,
    code: i32,
    message: &str,
    data: Option<serde_json::Value>
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

/// Resource contents
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
pub enum ResourceContents {
    Text {
        uri: String,
        text: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
    Blob {
        uri: String,
        blob: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
}

/// Content types for tool results and other messages
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")] Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
    },
    #[serde(rename = "image")] Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
    },
    #[serde(rename = "resource")] ResourceContent {
        resource: ResourceContents,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
    },
}

/// Annotations for content
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Annotations {
    pub priority: Option<f64>,
    pub audience: Option<Vec<Role>>,
}
