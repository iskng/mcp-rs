//! MCP Error Types
//!
//! This module defines custom error types for the MCP library, providing detailed
//! and type-safe error handling for various failure scenarios, such as protocol
//! errors, transport issues, and message parsing failures.

use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };
use thiserror::Error;

/// Error data for JSON-RPC responses
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ErrorData {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// The main Error type for the MCP library
#[derive(Error, Debug)]
pub enum Error {
    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Transport-related errors
    #[error("Transport error: {0}")]
    Transport(String),

    /// Protocol errors (e.g., invalid message format)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Resource errors
    #[error("Resource error: {0}")]
    Resource(String),

    /// Tool errors
    #[error("Tool error: {0}")]
    Tool(String),

    /// Prompt errors
    #[error("Prompt error: {0}")]
    Prompt(String),

    /// Request validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Schema validation error
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    /// Version incompatibility error
    #[error("Version incompatibility: {0}")]
    VersionIncompatibility(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Request timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Initialization error
    #[error("Initialization error: {0}")]
    Initialization(String),

    /// Server busy or unavailable
    #[error("Server unavailable: {0}")]
    ServerUnavailable(String),

    /// Invalid state for requested operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

/// Standard JSON-RPC 2.0 error codes
pub mod error_codes {
    /// Parse error
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;
    /// Server error range start
    pub const SERVER_ERROR_START: i32 = -32099;
    /// Server error range end
    pub const SERVER_ERROR_END: i32 = -32000;
    /// MCP-specific error codes can be defined in the range below -32000
    pub const RESOURCE_NOT_FOUND: i32 = -33000;
    /// Tool not found
    pub const TOOL_NOT_FOUND: i32 = -33001;
    /// Tool execution error
    pub const TOOL_EXECUTION_ERROR: i32 = -33002;
    /// Prompt not found
    pub const PROMPT_NOT_FOUND: i32 = -33003;
    /// Prompt validation error
    pub const PROMPT_VALIDATION_ERROR: i32 = -33004;
    /// Server is not initialized
    pub const SERVER_NOT_INITIALIZED: i32 = -33005;
    /// Client is not initialized
    pub const CLIENT_NOT_INITIALIZED: i32 = -33006;
    /// Incompatible protocol version
    pub const INCOMPATIBLE_VERSION: i32 = -33007;
    /// Authentication error
    pub const AUTHENTICATION_ERROR: i32 = -33008;
    /// Authorization error
    pub const AUTHORIZATION_ERROR: i32 = -33009;
    /// Rate limit exceeded
    pub const RATE_LIMIT_EXCEEDED: i32 = -33010;
    /// Request timeout
    pub const REQUEST_TIMEOUT: i32 = -33011;
    /// Resource content error
    pub const RESOURCE_CONTENT_ERROR: i32 = -33012;
    /// Tool parameter error
    pub const TOOL_PARAMETER_ERROR: i32 = -33013;
    /// Server busy or unavailable
    pub const SERVER_UNAVAILABLE: i32 = -33014;
    /// Schema validation error
    pub const SCHEMA_VALIDATION_ERROR: i32 = -33015;
}

impl Error {
    /// Convert an error to a JSON-RPC error code
    pub fn to_code(&self) -> i32 {
        use error_codes::*;
        match self {
            Error::Json(_) => PARSE_ERROR,
            Error::Protocol(_) => INVALID_REQUEST,
            Error::MethodNotFound(_) => METHOD_NOT_FOUND,
            Error::InvalidParams(_) => INVALID_PARAMS,
            Error::Resource(_) => RESOURCE_NOT_FOUND,
            Error::Tool(_) => TOOL_NOT_FOUND,
            Error::Prompt(_) => PROMPT_NOT_FOUND,
            Error::Validation(_) => INVALID_PARAMS,
            Error::SchemaValidation(_) => SCHEMA_VALIDATION_ERROR,
            Error::VersionIncompatibility(_) => INCOMPATIBLE_VERSION,
            Error::Authentication(_) => AUTHENTICATION_ERROR,
            Error::Authorization(_) => AUTHORIZATION_ERROR,
            Error::RateLimit(_) => RATE_LIMIT_EXCEEDED,
            Error::Timeout(_) => REQUEST_TIMEOUT,
            Error::Initialization(_) => SERVER_NOT_INITIALIZED,
            Error::InvalidState(_) => SERVER_NOT_INITIALIZED,
            Error::ServerUnavailable(_) => SERVER_UNAVAILABLE,
            Error::Io(_) => INTERNAL_ERROR,
            Error::Transport(_) => INTERNAL_ERROR,
            Error::Other(_) => INTERNAL_ERROR,
        }
    }

    /// Create an error response payload from this error
    pub fn to_response_payload(&self, id: i32) -> crate::types::protocol::Response {
        let code = self.to_code();
        let message = self.to_string();

        crate::types::protocol::error_response(id, code, &message, None)
    }
}

// Manual implementation of Clone that handles non-cloneable types
impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Json(e) => Error::Transport(format!("JSON error: {}", e)),
            Error::Io(e) => Error::Transport(format!("I/O error: {}", e)),
            Error::Transport(s) => Error::Transport(s.clone()),
            Error::Protocol(s) => Error::Protocol(s.clone()),
            Error::MethodNotFound(s) => Error::MethodNotFound(s.clone()),
            Error::InvalidParams(s) => Error::InvalidParams(s.clone()),
            Error::Resource(s) => Error::Resource(s.clone()),
            Error::Tool(s) => Error::Tool(s.clone()),
            Error::Prompt(s) => Error::Prompt(s.clone()),
            Error::Validation(s) => Error::Validation(s.clone()),
            Error::SchemaValidation(s) => Error::SchemaValidation(s.clone()),
            Error::VersionIncompatibility(s) => Error::VersionIncompatibility(s.clone()),
            Error::Authentication(s) => Error::Authentication(s.clone()),
            Error::Authorization(s) => Error::Authorization(s.clone()),
            Error::RateLimit(s) => Error::RateLimit(s.clone()),
            Error::Timeout(s) => Error::Timeout(s.clone()),
            Error::Initialization(s) => Error::Initialization(s.clone()),
            Error::ServerUnavailable(s) => Error::ServerUnavailable(s.clone()),
            Error::InvalidState(s) => Error::InvalidState(s.clone()),
            Error::Other(s) => Error::Other(s.clone()),
        }
    }
}
