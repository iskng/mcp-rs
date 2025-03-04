//! MCP Prompt Types
//!
//! This module defines types related to prompts in the MCP protocol, including
//! `Prompt` and `PromptArgument`, which are used to manage and template interactive
//! user inputs or AI queries.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A type of content that can be sent in a message
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(tag = "type", content = "content")]
pub enum ContentPart {
    /// Text content
    #[serde(rename = "text")]
    Text(String),
    /// Image content (referenced by resource URI)
    #[serde(rename = "image")]
    Image { uri: String },
    /// Tool call content
    #[serde(rename = "tool_call")]
    ToolCall {
        /// Name of the tool to call
        tool_name: String,
        /// Parameters for the tool call
        parameters: HashMap<String, serde_json::Value>,
    },
    /// Tool result content
    #[serde(rename = "tool_result")]
    ToolResult {
        /// Name of the tool that was called
        tool_name: String,
        /// Result of the tool call
        result: serde_json::Value,
    },
}

/// Role of a message sender
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message
    System,
    /// Human user message
    User,
    /// AI assistant message
    Assistant,
    /// Tool message (for tool calls or results)
    Tool,
}

/// A message in a prompt template
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Content of the message
    pub content: Vec<ContentPart>,
}

/// Represents a template argument
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PromptArgument {
    /// Name of the argument
    pub name: String,
    /// Type of the argument (e.g., "string", "number")
    #[serde(rename = "type")]
    pub type_name: String,
    /// Description of the argument
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Default value for the argument
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Schema for the argument, if complex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Represents a prompt template
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Prompt {
    /// Unique name for the prompt
    pub name: String,
    /// Description of the prompt
    pub description: String,
    /// Template messages
    pub messages: Vec<Message>,
    /// Arguments for templating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Parameters for listing prompts
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListPromptsParams {
    /// Optional pagination token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    /// Optional limit on the number of prompts to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

/// Result of listing prompts
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListPromptsResult {
    /// List of prompts
    pub prompts: Vec<Prompt>,
    /// Token for the next page (if there are more prompts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Parameters for creating a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CreatePromptParams {
    /// Unique name for the prompt
    pub name: String,
    /// Description of the prompt
    pub description: String,
    /// Template messages
    pub messages: Vec<Message>,
    /// Arguments for templating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Result of creating a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CreatePromptResult {
    /// The created prompt
    pub prompt: Prompt,
}

/// Parameters for updating a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct UpdatePromptParams {
    /// Name of the prompt to update
    pub name: String,
    /// New description for the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// New template messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,
    /// New arguments for templating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Result of updating a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct UpdatePromptResult {
    /// The updated prompt
    pub prompt: Prompt,
}

/// Parameters for deleting a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DeletePromptParams {
    /// Name of the prompt to delete
    pub name: String,
}

/// Result of deleting a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DeletePromptResult {
    /// Indicates whether the prompt was successfully deleted
    pub success: bool,
}

/// Parameters for rendering a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct RenderPromptParams {
    /// Name of the prompt to render
    pub name: String,
    /// Arguments for template substitution
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Result of rendering a prompt
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct RenderPromptResult {
    /// Rendered messages
    pub messages: Vec<Message>,
}
