//! MCP Initialization Types
//!
//! This module defines types related to the initialization phase of the MCP protocol,
//! such as `InitializeRequestParams` and `InitializeResult`, ensuring proper setup and capability
//! negotiation between client and server.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for the initialize request
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct InitializeRequestParams {
    /// The version of the MCP protocol, e.g., "2024-11-05"
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Client capabilities for feature negotiation
    pub capabilities: ClientCapabilities,
    /// Information about the client implementation
    #[serde(rename = "clientInfo")]
    pub client_info: Option<Implementation>,
}

/// Result of the initialize request
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct InitializeResult {
    /// The negotiated protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Server capabilities for feature negotiation
    pub capabilities: ServerCapabilities,
    /// Information about the server implementation
    #[serde(rename = "serverInfo")]
    pub server_info: Option<Implementation>,
    /// Optional instructions for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Information about an implementation (client or server)
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Implementation {
    /// Name of the implementation
    pub name: String,
    /// Version of the implementation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Client capabilities
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ClientCapabilities {
    /// Root capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootCapabilities>,
    /// Sampling capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

/// Server capabilities
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, Default)]
pub struct ServerCapabilities {
    /// Resource capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    /// Tool capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    /// Prompt capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,
    /// Logging capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

/// Root capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RootCapabilities {
    /// Support for list changes
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Sampling capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SamplingCapabilities {}

/// Resource capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceCapabilities {
    /// Support for resource subscription
    pub subscribe: bool,
    /// Support for list changes
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Prompt capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptCapabilities {
    /// Support for list changes
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Tool capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolCapabilities {
    /// Support for list changes
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Logging capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoggingCapabilities {
    // Custom logging fields can be added here as needed
}
