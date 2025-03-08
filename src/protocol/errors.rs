//! Error handling for the MCP protocol
//!
//! This module defines custom error types, error codes, and helper functions for
//! creating standardized JSON-RPC error responses.

use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use thiserror::Error;

use crate::protocol::{ JSONRPCError, JSONRPCErrorDetails, JSONRPCMessage, RequestId };

/// Standard JSON-RPC 2.0 error codes and MCP-specific error codes
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

/// Error data for JSON-RPC responses
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ErrorData {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
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
    pub fn to_response_payload(&self, id: RequestId) -> JSONRPCError {
        let code = self.to_code();
        let message = self.to_string();

        create_error_response(id, code, &message, None)
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

/// Create an error response with the given code, message, and optional data
pub fn create_error_response(
    id: RequestId,
    code: i32,
    message: &str,
    data: Option<Value>
) -> JSONRPCError {
    JSONRPCError {
        jsonrpc: "2.0".to_string(),
        id,
        error: JSONRPCErrorDetails {
            code,
            message: message.to_string(),
            data,
        },
    }
}

/// Create a standard error response with a standard error code
pub fn standard_error_response(id: RequestId, error_code: i32, message: &str) -> JSONRPCError {
    create_error_response(id, error_code, message, None)
}

/// Create an invalid request error response
pub fn invalid_request(id: RequestId, message: &str) -> JSONRPCError {
    standard_error_response(id, error_codes::INVALID_REQUEST, message)
}

/// Create an invalid params error response
pub fn invalid_params(id: RequestId, message: &str) -> JSONRPCError {
    standard_error_response(id, error_codes::INVALID_PARAMS, message)
}

/// Create a method not found error response
pub fn method_not_found(id: RequestId, message: &str) -> JSONRPCError {
    standard_error_response(id, error_codes::METHOD_NOT_FOUND, message)
}

/// Create an internal error response
pub fn internal_error(id: RequestId, message: &str) -> JSONRPCError {
    standard_error_response(id, error_codes::INTERNAL_ERROR, message)
}

/// Create a parse error response
pub fn parse_error(id: RequestId, message: &str) -> JSONRPCError {
    standard_error_response(id, error_codes::PARSE_ERROR, message)
}

/// Create a validation error response
pub fn validation_error(id: RequestId, message: &str) -> JSONRPCError {
    standard_error_response(id, error_codes::INVALID_REQUEST, message)
}

/// Create a tool execution error response
pub fn tool_execution_error(id: RequestId, tool_name: &str, message: &str) -> JSONRPCError {
    standard_error_response(
        id,
        error_codes::TOOL_EXECUTION_ERROR,
        &format!("Error executing tool '{}': {}", tool_name, message)
    )
}

/// Create a resource not found error response
pub fn resource_not_found_error(id: RequestId, uri: &str) -> JSONRPCError {
    standard_error_response(
        id,
        error_codes::RESOURCE_NOT_FOUND,
        &format!("Resource not found: {}", uri)
    )
}

/// Create an error message from an Error enum
pub fn error_to_rpc_error(id: RequestId, error: &Error) -> JSONRPCError {
    match error {
        Error::MethodNotFound(msg) => method_not_found(id.clone(), msg),
        Error::InvalidParams(msg) => invalid_params(id.clone(), msg),
        Error::Json(err) => parse_error(id.clone(), &format!("JSON Error: {}", err)),
        Error::Protocol(msg) => internal_error(id.clone(), msg),
        Error::Transport(msg) => internal_error(id.clone(), msg),
        Error::Resource(msg) => resource_not_found_error(id.clone(), msg),
        Error::Validation(msg) => validation_error(id.clone(), msg),
        Error::Io(msg) => internal_error(id.clone(), &format!("IO Error: {}", msg)),
        Error::Tool(msg) => tool_execution_error(id.clone(), "unknown", msg),
        Error::Prompt(msg) => internal_error(id.clone(), &format!("Prompt Error: {}", msg)),
        Error::SchemaValidation(msg) => validation_error(id.clone(), msg),
        Error::VersionIncompatibility(msg) => internal_error(id.clone(), msg),
        Error::Authentication(msg) => internal_error(id.clone(), msg),
        Error::Authorization(msg) => internal_error(id.clone(), msg),
        Error::RateLimit(msg) => internal_error(id.clone(), msg),
        Error::Timeout(msg) => internal_error(id.clone(), msg),
        Error::Initialization(msg) => internal_error(id.clone(), msg),
        Error::ServerUnavailable(msg) => internal_error(id.clone(), msg),
        Error::InvalidState(msg) => internal_error(id.clone(), msg),
        Error::Other(msg) => internal_error(id.clone(), &format!("Other error: {}", msg)),
    }
}

/// Helper to convert to an error message
pub fn to_error_message(id: RequestId, error: &Error) -> JSONRPCMessage {
    JSONRPCMessage::Error(error_to_rpc_error(id, error))
}
