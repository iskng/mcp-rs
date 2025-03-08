//! Type definitions for the Model Context Protocol (MCP).
//! These types are generated to match the schema defined in schema.json.

use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// The complete schema definitions matching schema.json
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Definitions {
    /// Base for objects that include optional annotations for the client
    pub annotated: Annotated,
    /// Binary resource contents
    pub blob_resource_contents: BlobResourceContents,
    /// Tool call request
    pub call_tool_request: CallToolRequest,
    /// Tool call result
    pub call_tool_result: CallToolResult,
    /// Cancellation notification
    pub cancelled_notification: CancelledNotification,
    /// Client capabilities
    pub client_capabilities: ClientCapabilities,
    /// Collection of client notification types
    pub client_notification: ClientNotification,
    /// Collection of client request types
    pub client_request: ClientRequest,
    /// Collection of client result types
    pub client_result: ClientResult,
    /// Completion request
    pub complete_request: CompleteRequest,
    /// Completion result
    pub complete_result: CompleteResult,
    /// Content item
    pub content: Content,
    /// Create message request
    pub create_message_request: CreateMessageRequest,
    /// Create message result
    pub create_message_result: CreateMessageResult,
    /// An opaque token used to represent a cursor for pagination
    pub cursor: Cursor,
    /// Empty result with no additional fields
    pub empty_result: EmptyResult,
    /// Embedded resource
    pub embedded_resource: EmbeddedResource,
    /// Get prompt request
    pub get_prompt_request: GetPromptRequest,
    /// Get prompt result
    pub get_prompt_result: GetPromptResult,
    /// Image content
    pub image_content: ImageContent,
    /// Implementation info
    pub implementation: Implementation,
    /// Initialize request
    pub initialize_request: InitializeRequest,
    /// Initialize result
    pub initialize_result: InitializeResult,
    /// Initialized notification
    pub initialized_notification: InitializedNotification,
    /// JSON-RPC error response
    pub jsonrpc_error: JSONRPCError,
    /// Any JSON-RPC message type
    pub jsonrpc_message: JSONRPCMessage,
    /// JSON-RPC notification without ID
    pub jsonrpc_notification: JSONRPCNotification,
    /// JSON-RPC request with ID
    pub jsonrpc_request: JSONRPCRequest,
    /// JSON-RPC response with result
    pub jsonrpc_response: JSONRPCResponse,
    /// List prompts request
    pub list_prompts_request: ListPromptsRequest,
    /// List prompts result
    pub list_prompts_result: ListPromptsResult,
    /// List resource templates request
    pub list_resource_templates_request: ListResourceTemplatesRequest,
    /// List resource templates result
    pub list_resource_templates_result: ListResourceTemplatesResult,
    /// List resources request
    pub list_resources_request: ListResourcesRequest,
    /// List resources result
    pub list_resources_result: ListResourcesResult,
    /// List roots request
    pub list_roots_request: ListRootsRequest,
    /// List roots result
    pub list_roots_result: ListRootsResult,
    /// List tools request
    pub list_tools_request: ListToolsRequest,
    /// List tools result
    pub list_tools_result: ListToolsResult,
    /// Logging level
    pub logging_level: LoggingLevel,
    /// Logging message notification
    pub logging_message_notification: LoggingMessageNotification,
    /// Model hint
    pub model_hint: ModelHint,
    /// Model preferences
    pub model_preferences: ModelPreferences,
    /// Base notification structure
    pub notification: Notification,
    /// Base for pagination requests
    pub paginated_request: PaginatedRequest,
    /// Base for pagination results
    pub paginated_result: PaginatedResult,
    /// Ping request
    pub ping_request: PingRequest,
    /// Progress notification
    pub progress_notification: ProgressNotification,
    /// Prompt
    pub prompt: Prompt,
    /// Prompt argument
    pub prompt_argument: PromptArgument,
    /// Prompt list changed notification
    pub prompt_list_changed_notification: PromptListChangedNotification,
    /// Prompt message
    pub prompt_message: PromptMessage,
    /// Prompt reference
    pub prompt_reference: PromptReference,
    /// Read resource request
    pub read_resource_request: ReadResourceRequest,
    /// Read resource result
    pub read_resource_result: ReadResourceResult,
    /// Reference
    pub reference: Reference,
    /// Base request structure
    pub request: Request,
    /// Base result structure with optional metadata
    pub result: Result,
    /// Resource
    pub resource: Resource,
    /// Resource contents base
    pub resource_contents: ResourceContents,
    /// Resource list changed notification
    pub resource_list_changed_notification: ResourceListChangedNotification,
    /// Resource reference
    pub resource_reference: ResourceReference,
    /// Resource template
    pub resource_template: ResourceTemplate,
    /// Resource updated notification
    pub resource_updated_notification: ResourceUpdatedNotification,
    /// Role
    pub role: Role,
    /// Root
    pub root: Root,
    /// Roots list changed notification
    pub roots_list_changed_notification: RootsListChangedNotification,
    /// Sampling message
    pub sampling_message: SamplingMessage,
    /// Collection of server notification types
    pub server_notification: ServerNotification,
    /// Collection of server request types
    pub server_request: ServerRequest,
    /// Collection of server result types
    pub server_result: ServerResult,
    /// Set level request
    pub set_level_request: SetLevelRequest,
    /// Subscribe request
    pub subscribe_request: SubscribeRequest,
    /// Text content
    pub text_content: TextContent,
    /// Text resource contents
    pub text_resource_contents: TextResourceContents,
    /// Tool
    pub tool: Tool,
    /// Tool list changed notification
    pub tool_list_changed_notification: ToolListChangedNotification,
    /// Unsubscribe request
    pub unsubscribe_request: UnsubscribeRequest,
}

/// An opaque token used to represent a cursor for pagination
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Cursor(pub String);

/// Empty result with no additional fields
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmptyResult {
    pub result: Result,
}

/// A uniquely identifying ID for a request in JSON-RPC.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

/// The sender or recipient of messages and data in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Assistant,
    User,
}

/// The severity of a log message, based on RFC-5424 syslog message severities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

/// A progress token, used to associate progress notifications with the original request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProgressToken {
    String(String),
    Number(i64),
}

/// Base for objects that include optional annotations for the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Annotations {
    /// Describes who the intended customer of this object or data is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,

    /// Describes how important this data is for operating the server.
    /// A value of 1 means "most important" and effectively required,
    /// while 0 means "least important" and entirely optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

/// Base for objects that include optional annotations for the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Annotated {
    /// Optional annotations for the client to inform how objects are used or displayed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Text provided to or from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TextContent {
    /// The type of content (always "text").
    #[serde(rename = "type")]
    pub type_field: String,

    /// The text content of the message.
    pub text: String,

    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// An image provided to or from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImageContent {
    /// The type of content (always "image").
    #[serde(rename = "type")]
    pub type_field: String,

    /// The base64-encoded image data.
    pub data: String,

    /// The MIME type of the image.
    #[serde(rename = "mimeType")]
    pub mime_type: String,

    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// The contents of a resource that is text-based.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TextResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// The text of the item.
    pub text: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// The contents of a resource that is binary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlobResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// A base64-encoded string representing the binary data of the item.
    pub blob: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// The contents of a resource, embedded into a prompt or tool call result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddedResource {
    /// The type of content (always "resource").
    #[serde(rename = "type")]
    pub type_field: String,

    /// The resource content.
    pub resource: ResourceContentType,

    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Base structure for resource contents with common fields.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// The content of a resource, which can be either text or binary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ResourceContentType {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

/// Content that can be included in messages.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
    Resource(EmbeddedResource),
}

/// Describes the name and version of an MCP implementation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Implementation {
    /// The name of the implementation.
    pub name: String,

    /// The version of the implementation.
    pub version: String,
}

/// Base JSON-RPC message structure.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONRPCBase {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,
}

/// A JSON-RPC request that expects a response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONRPCRequest {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// The request ID.
    pub id: RequestId,

    /// The method name.
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A JSON-RPC notification which does not expect a response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONRPCNotification {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// The method name.
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A successful JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONRPCResponse {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// The request ID this is responding to.
    pub id: RequestId,

    /// The response result.
    pub result: Result,
}

/// A JSON-RPC error response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONRPCError {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// The request ID this error is for.
    pub id: RequestId,

    /// The error details.
    pub error: JSONRPCErrorDetails,
}

/// Details of a JSON-RPC error.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONRPCErrorDetails {
    /// The error code.
    pub code: i32,

    /// A short description of the error.
    pub message: String,

    /// Additional information about the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Any JSON-RPC message type.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request(JSONRPCRequest),
    Notification(JSONRPCNotification),
    Response(JSONRPCResponse),
    Error(JSONRPCError),
}

/// Base result structure with optional metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Result {
    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,

    /// Additional properties not defined by the protocol.
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub content: HashMap<String, serde_json::Value>,
}

/// Base request structure with support for progress notifications.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Request {
    /// The method name.
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

/// Request parameters with metadata support.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RequestParams {
    /// Optional metadata for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<RequestMeta>,
}

/// Request metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RequestMeta {
    /// If specified, the caller is requesting out-of-band progress notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "progressToken")]
    pub progress_token: Option<ProgressToken>,
}

/// Base notification structure.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Notification {
    /// The method name.
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

/// Notification parameters with metadata support.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotificationParams {
    /// Optional metadata for the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// Capabilities a client may support.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClientCapabilities {
    /// Present if the client supports sampling from an LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<HashMap<String, serde_json::Value>>,

    /// Present if the client supports listing roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Experimental, non-standard capabilities that the client supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
}

/// Roots capability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RootsCapability {
    /// Whether the client supports notifications for changes to the roots list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Capabilities that a server may support.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerCapabilities {
    /// Present if the server offers any resources to read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Present if the server offers any prompt templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Present if the server offers any tools to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Present if the server supports sending log messages to the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<HashMap<String, serde_json::Value>>,

    /// Experimental, non-standard capabilities that the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
}

/// Resources capability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourcesCapability {
    /// Whether this server supports subscribing to resource updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether this server supports notifications for changes to the resource list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Prompts capability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptsCapability {
    /// Whether this server supports notifications for changes to the prompt list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Tools capability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolsCapability {
    /// Whether this server supports notifications for changes to the tool list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Represents a root directory or file that the server can operate on.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Root {
    /// The URI identifying the root. This must start with file:// for now.
    pub uri: String,

    /// An optional name for the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A known resource that the server is capable of reading.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Resource {
    /// The URI of this resource.
    pub uri: String,

    /// A human-readable name for this resource.
    pub name: String,

    /// A description of what this resource represents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,

    /// The size of the raw resource content, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,

    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// A template description for resources available on the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceTemplate {
    /// A URI template that can be used to construct resource URIs.
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,

    /// A human-readable name for the type of resource this template refers to.
    pub name: String,

    /// A description of what this template is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The MIME type for all resources that match this template.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,

    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// A reference to a resource or resource template definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceReference {
    /// The type of reference (always "ref/resource").
    #[serde(rename = "type")]
    pub type_field: String,

    /// The URI or URI template of the resource.
    pub uri: String,
}

/// A prompt or prompt template that the server offers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Prompt {
    /// The name of the prompt or prompt template.
    pub name: String,

    /// An optional description of what this prompt provides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A list of arguments to use for templating the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Describes an argument that a prompt can accept.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptArgument {
    /// The name of the argument.
    pub name: String,

    /// A human-readable description of the argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this argument must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Identifies a prompt.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptReference {
    /// The type of reference (always "ref/prompt").
    #[serde(rename = "type")]
    pub type_field: String,

    /// The name of the prompt or prompt template.
    pub name: String,
}

/// Describes a message returned as part of a prompt.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptMessage {
    /// The role of the message sender.
    pub role: Role,

    /// The content of the message.
    pub content: Content,
}

/// Describes a message issued to or received from an LLM API.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SamplingMessage {
    /// The role of the message sender.
    pub role: Role,

    /// The content of the message.
    pub content: SamplingContent,
}

/// Content types for sampling messages.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SamplingContent {
    Text(TextContent),
    Image(ImageContent),
}

/// Hints to use for model selection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelHint {
    /// A hint for a model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The server's preferences for model selection, requested of the client during sampling.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelPreferences {
    /// How much to prioritize sampling speed when selecting a model.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "speedPriority")]
    pub speed_priority: Option<f64>,

    /// How much to prioritize intelligence and capabilities when selecting a model.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "intelligencePriority")]
    pub intelligence_priority: Option<f64>,

    /// How much to prioritize cost when selecting a model.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "costPriority")]
    pub cost_priority: Option<f64>,

    /// Optional hints to use for model selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
}

/// Definition for a tool the client can call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    /// The name of the tool.
    pub name: String,

    /// A JSON Schema object defining the expected parameters for the tool.
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,

    /// A human-readable description of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A JSON Schema object defining the expected parameters for a tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolInputSchema {
    /// The type of the input (always "object").
    #[serde(rename = "type")]
    pub type_field: String,

    /// The properties of the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, HashMap<String, serde_json::Value>>>,

    /// The required properties of the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Base for pagination requests.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaginatedRequest {
    /// The method name.
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedRequestParams>,
}

/// Parameters for paginated requests.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaginatedRequestParams {
    /// An opaque token representing the current pagination position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// Base for pagination results.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaginatedResult {
    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<Cursor>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// This request is sent from the client to the server when it first connects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitializeRequest {
    /// The method name (always "initialize").
    pub method: String,

    /// The initialize parameters.
    pub params: InitializeParams,
}

/// Initialize request parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitializeParams {
    /// The client's capabilities.
    pub capabilities: ClientCapabilities,

    /// Information about the client.
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,

    /// The latest version of the Model Context Protocol that the client supports.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
}

/// After receiving an initialize request from the client, the server sends this response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitializeResult {
    /// The server's capabilities.
    pub capabilities: ServerCapabilities,

    /// The version of the Model Context Protocol that the server wants to use.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Information about the server.
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,

    /// Instructions describing how to use the server and its features.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// This notification is sent from the client to the server after initialization has finished.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitializedNotification {
    /// The method name (always "notifications/initialized").
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// A ping request to check if the other party is still alive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PingRequest {
    /// The method name (always "ping").
    pub method: String,

    /// The ping parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// Sent from the server to request a list of root URIs from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListRootsRequest {
    /// The method name (always "roots/list").
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// The client's response to a roots/list request from the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListRootsResult {
    /// The list of roots.
    pub roots: Vec<Root>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// A notification from the client to the server, informing it that the list of roots has changed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RootsListChangedNotification {
    /// The method name (always "notifications/roots/list_changed").
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// Sent from the client to request a list of resources the server has.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListResourcesRequest {
    /// The method name (always "resources/list").
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedRequestParams>,
}

/// The server's response to a resources/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListResourcesResult {
    /// The list of resources.
    pub resources: Vec<Resource>,

    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<Cursor>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// Sent from the client to request a list of resource templates the server has.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListResourceTemplatesRequest {
    /// The method name (always "resources/templates/list").
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedRequestParams>,
}

/// The server's response to a resources/templates/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListResourceTemplatesResult {
    /// The list of resource templates.
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: Vec<ResourceTemplate>,

    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<Cursor>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// Sent from the client to the server, to read a specific resource URI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResourceRequest {
    /// The method name (always "resources/read").
    pub method: String,

    /// The request parameters.
    pub params: ReadResourceParams,
}

/// Parameters for a read resource request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResourceParams {
    /// The URI of the resource to read.
    pub uri: String,
}

/// The server's response to a resources/read request from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResourceResult {
    /// The contents of the resource.
    pub contents: Vec<ResourceContentType>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// An optional notification from the server to the client, informing it that the list of resources has changed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceListChangedNotification {
    /// The method name (always "notifications/resources/list_changed").
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// Sent from the client to request resources/updated notifications from the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubscribeRequest {
    /// The method name (always "resources/subscribe").
    pub method: String,

    /// The request parameters.
    pub params: SubscribeParams,
}

/// Parameters for a subscribe request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubscribeParams {
    /// The URI of the resource to subscribe to.
    pub uri: String,
}

/// Sent from the client to request cancellation of resources/updated notifications.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnsubscribeRequest {
    /// The method name (always "resources/unsubscribe").
    pub method: String,

    /// The request parameters.
    pub params: UnsubscribeParams,
}

/// Parameters for an unsubscribe request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnsubscribeParams {
    /// The URI of the resource to unsubscribe from.
    pub uri: String,
}

/// A notification from the server to the client, informing it that a resource has changed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceUpdatedNotification {
    /// The method name (always "notifications/resources/updated").
    pub method: String,

    /// The notification parameters.
    pub params: ResourceUpdatedParams,
}

/// Parameters for a resource updated notification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceUpdatedParams {
    /// The URI of the resource that has been updated.
    pub uri: String,
}

/// Sent from the client to request a list of prompts and prompt templates the server has.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListPromptsRequest {
    /// The method name (always "prompts/list").
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedRequestParams>,
}

/// The server's response to a prompts/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListPromptsResult {
    /// The list of prompts.
    pub prompts: Vec<Prompt>,

    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<Cursor>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// Used by the client to get a prompt provided by the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPromptRequest {
    /// The method name (always "prompts/get").
    pub method: String,

    /// The request parameters.
    pub params: GetPromptParams,
}

/// Parameters for a get prompt request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPromptParams {
    /// The name of the prompt or prompt template.
    pub name: String,

    /// Arguments to use for templating the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

/// The server's response to a prompts/get request from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPromptResult {
    /// The messages in the prompt.
    pub messages: Vec<PromptMessage>,

    /// An optional description for the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// An optional notification from the server to the client, informing it that the list of prompts has changed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptListChangedNotification {
    /// The method name (always "notifications/prompts/list_changed").
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// Used by the client to get a list of tools provided by the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListToolsRequest {
    /// The method name (always "tools/list").
    pub method: String,

    /// The request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedRequestParams>,
}

/// The server's response to a tools/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListToolsResult {
    /// The list of tools.
    pub tools: Vec<Tool>,

    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<Cursor>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// Used by the client to invoke a tool provided by the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallToolRequest {
    /// The method name (always "tools/call").
    pub method: String,

    /// The request parameters.
    pub params: CallToolParams,
}

/// Parameters for a call tool request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallToolParams {
    /// The name of the tool to call.
    pub name: String,

    /// The arguments to pass to the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// The server's response to a tool call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallToolResult {
    /// The content returned by the tool.
    pub content: Vec<Content>,

    /// Whether the tool call ended in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// An optional notification from the server to the client, informing it that the list of tools has changed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolListChangedNotification {
    /// The method name (always "notifications/tools/list_changed").
    pub method: String,

    /// The notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// A request from the server to sample an LLM via the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateMessageRequest {
    /// The method name (always "sampling/createMessage").
    pub method: String,

    /// The request parameters.
    pub params: CreateMessageParams,
}

/// Parameters for a create message request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateMessageParams {
    /// The messages to use for sampling.
    pub messages: Vec<SamplingMessage>,

    /// The maximum number of tokens to sample.
    #[serde(rename = "maxTokens")]
    pub max_tokens: i64,

    /// A request to include context from one or more MCP servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "includeContext")]
    pub include_context: Option<String>,

    /// The server's preferences for which model to select.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "modelPreferences")]
    pub model_preferences: Option<ModelPreferences>,

    /// An optional system prompt the server wants to use for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,

    /// The temperature to use for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Optional stop sequences to use for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stopSequences")]
    pub stop_sequences: Option<Vec<String>>,

    /// Optional metadata to pass through to the LLM provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// The client's response to a sampling/create_message request from the server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateMessageResult {
    /// The role of the message.
    pub role: Role,

    /// The content of the message.
    pub content: SamplingContent,

    /// The name of the model that generated the message.
    pub model: String,

    /// The reason why sampling stopped, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stopReason")]
    pub stop_reason: Option<String>,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// A request from the client to the server, to ask for completion options.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteRequest {
    /// The method name (always "completion/complete").
    pub method: String,

    /// The request parameters.
    pub params: CompleteParams,
}

/// Parameters for a complete request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteParams {
    /// The argument's information.
    pub argument: CompleteArgument,

    /// The reference to complete against.
    #[serde(rename = "ref")]
    pub ref_: Reference,
}

/// The argument information for completion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteArgument {
    /// The name of the argument.
    pub name: String,

    /// The value of the argument to use for completion matching.
    pub value: String,
}

/// A reference for completion, either a prompt or resource.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Reference {
    Prompt(PromptReference),
    Resource(ResourceReference),
}

/// The server's response to a completion/complete request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteResult {
    /// The completion information.
    pub completion: CompletionInfo,

    /// Optional metadata for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, serde_json::Value>>,
}

/// Completion information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompletionInfo {
    /// An array of completion values.
    pub values: Vec<String>,

    /// The total number of completion options available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,

    /// Indicates whether there are additional completion options.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hasMore")]
    pub has_more: Option<bool>,
}

/// A request from the client to the server, to enable or adjust logging.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetLevelRequest {
    /// The method name (always "logging/setLevel").
    pub method: String,

    /// The request parameters.
    pub params: SetLevelParams,
}

/// Parameters for a set level request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetLevelParams {
    /// The level of logging that the client wants to receive from the server.
    pub level: LoggingLevel,
}

/// A logging message notification from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoggingMessageNotification {
    /// The method name (always "notifications/logging/message").
    pub method: String,

    /// The notification parameters.
    pub params: LoggingMessageParams,
}

/// Parameters for a logging message notification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoggingMessageParams {
    /// The severity of this log message.
    pub level: LoggingLevel,

    /// The data to be logged.
    pub data: serde_json::Value,

    /// An optional name of the logger issuing this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

/// An out-of-band notification used to inform the receiver of a progress update.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProgressNotification {
    /// The method name (always "notifications/progress").
    pub method: String,

    /// The notification parameters.
    pub params: ProgressParams,
}

/// Parameters for a progress notification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProgressParams {
    /// The progress token from the initial request.
    #[serde(rename = "progressToken")]
    pub progress_token: ProgressToken,

    /// The progress thus far.
    pub progress: f64,

    /// Total number of items to process, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// This notification can be sent by either side to indicate cancellation of a request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CancelledNotification {
    /// The method name (always "notifications/cancelled").
    pub method: String,

    /// The notification parameters.
    pub params: CancelledParams,
}

/// Parameters for a cancelled notification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CancelledParams {
    /// The ID of the request to cancel.
    #[serde(rename = "requestId")]
    pub request_id: RequestId,

    /// An optional string describing the reason for the cancellation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Collection of client request types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ClientRequest {
    Initialize(InitializeRequest),
    Ping(PingRequest),
    ListResources(ListResourcesRequest),
    ListResourceTemplates(ListResourceTemplatesRequest),
    ReadResource(ReadResourceRequest),
    Subscribe(SubscribeRequest),
    Unsubscribe(UnsubscribeRequest),
    ListPrompts(ListPromptsRequest),
    GetPrompt(GetPromptRequest),
    ListTools(ListToolsRequest),
    CallTool(CallToolRequest),
    SetLevel(SetLevelRequest),
    Complete(CompleteRequest),
}

/// Collection of server request types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServerRequest {
    Ping(PingRequest),
    CreateMessage(CreateMessageRequest),
    ListRoots(ListRootsRequest),
}

/// Collection of client notification types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ClientNotification {
    Cancelled(CancelledNotification),
    Initialized(InitializedNotification),
    Progress(ProgressNotification),
    RootsListChanged(RootsListChangedNotification),
}

/// Collection of server notification types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServerNotification {
    Cancelled(CancelledNotification),
    Progress(ProgressNotification),
    ResourceListChanged(ResourceListChangedNotification),
    ResourceUpdated(ResourceUpdatedNotification),
    PromptListChanged(PromptListChangedNotification),
    ToolListChanged(ToolListChangedNotification),
    LoggingMessage(LoggingMessageNotification),
}

/// Collection of client result types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ClientResult {
    Empty(Result),
    CreateMessage(CreateMessageResult),
    ListRoots(ListRootsResult),
}

/// Collection of server result types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServerResult {
    Empty(Result),
    Initialize(InitializeResult),
    ListResources(ListResourcesResult),
    ListResourceTemplates(ListResourceTemplatesResult),
    ReadResource(ReadResourceResult),
    ListPrompts(ListPromptsResult),
    GetPrompt(GetPromptResult),
    ListTools(ListToolsResult),
    CallTool(CallToolResult),
    Complete(CompleteResult),
}
