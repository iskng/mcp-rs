//! MCP Prompt Types
//!
//! This module defines types related to prompts in the MCP protocol, including
//! `Prompt` and `PromptArgument`, which are used to manage and template interactive
//! user inputs or AI queries.

use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

use super::protocol::Role;

/// Text provided to or from an LLM.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct TextContent {
    /// The type of content
    #[serde(rename = "type")]
    pub type_: String,

    /// The text content of the message.
    pub text: String,

    /// Optional annotations for the content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
}

/// Content annotations to provide metadata about content items
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ContentAnnotations {
    /// Describes who the intended customer of this object or data is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,

    /// Describes how important this data is for operating the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
}

/// An image provided to or from an LLM.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ImageContent {
    /// The type of content
    #[serde(rename = "type")]
    pub type_: String,

    /// The base64-encoded image data.
    pub data: String,

    /// The MIME type of the image. Different providers may support different image types.
    pub mime_type: String,

    /// Optional annotations for the content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
}

/// The contents of a resource, embedded into a prompt.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct EmbeddedResource {
    /// The type of content
    #[serde(rename = "type")]
    pub type_: String,

    /// The resource content
    pub resource: ResourceContent,

    /// Optional annotations for the content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
}

/// The content of a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
pub enum ResourceContent {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

/// Text resource contents
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct TextResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// The text of the item.
    pub text: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Binary resource contents
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct BlobResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// A base64-encoded string representing the binary data of the item.
    pub blob: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Content that can be included in a prompt message
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
pub enum ContentPart {
    Text(TextContent),
    Image(ImageContent),
    Resource(EmbeddedResource),
}

/// Describes a message returned as part of a prompt.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PromptMessage {
    /// Role of the message sender
    pub role: Role,

    /// Content of the message
    pub content: ContentPart,
}

/// Describes an argument that a prompt can accept.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
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

/// A prompt or prompt template that the server offers.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Prompt {
    /// The name of the prompt or prompt template.
    pub name: String,

    /// An optional description of what this prompt provides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A list of arguments to use for templating the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Additional metadata attached to requests, responses, or notifications.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct MetaData {
    /// This field may contain any JSON-serializable value.
    #[serde(flatten)]
    pub additional_properties: HashMap<String, serde_json::Value>,
}

/// Sent from the client to request a list of prompts and prompt templates the server has.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListPromptsParams {
    /// An opaque token representing the current pagination position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Additional metadata for the request
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaData>,
}

/// The server's response to a prompts/list request from the client.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListPromptsResult {
    /// List of prompts
    pub prompts: Vec<Prompt>,

    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,

    /// Additional metadata for the response
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaData>,
}

/// Used by the client to get a prompt provided by the server.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GetPromptParams {
    /// The name of the prompt or prompt template.
    pub name: String,

    /// Arguments to use for templating the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,

    /// Additional metadata for the request
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaData>,
}

/// The server's response to a prompts/get request from the client.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GetPromptResult {
    /// Rendered messages
    pub messages: Vec<PromptMessage>,

    /// An optional description for the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Additional metadata for the response
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaData>,
}

/// Identifies a prompt.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PromptReference {
    /// Type of reference
    #[serde(rename = "type")]
    pub type_: String,

    /// The name of the prompt or prompt template
    pub name: String,
}

/// Request to complete an argument with suggestions
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CompleteRequest {
    /// The reference to the prompt or resource
    pub ref_: serde_json::Value, // Can be PromptReference or ResourceReference

    /// The argument's information
    pub argument: ArgumentInfo,

    /// Additional metadata for the request
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaData>,
}

/// Information about an argument for completion
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ArgumentInfo {
    /// The name of the argument
    pub name: String,

    /// The value of the argument to use for completion matching
    pub value: String,
}

/// Result of a completion request
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CompleteResult {
    /// Completion options
    pub completion: CompletionOptions,

    /// Additional metadata for the response
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaData>,
}

/// Options for completion
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CompletionOptions {
    /// An array of completion values
    pub values: Vec<String>,

    /// The total number of completion options available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i32>,

    /// Indicates whether there are additional completion options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}
