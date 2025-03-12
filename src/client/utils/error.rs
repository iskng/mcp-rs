//! MCP Client Error Utilities
//!
//! This module provides utility functions for working with MCP protocol errors.

use crate::protocol::errors::{ Error, error_codes };

/// Check if an error is a capability error
pub fn is_capability_error(error: &Error) -> bool {
    match error {
        Error::MethodNotFound(_) => true,
        _ => false,
    }
}

/// Check if an error is a resource not found error
pub fn is_resource_not_found_error(error: &Error) -> bool {
    match error {
        Error::Resource(msg) => msg.contains("not found") || msg.contains("Not Found"),
        _ => false,
    }
}

/// Convert an error to a user-friendly message
pub fn error_to_user_message(error: &Error) -> String {
    match error {
        Error::Json(err) => format!("JSON error: {}", err),
        Error::Io(err) => format!("I/O error: {}", err),
        Error::Transport(msg) => format!("Transport error: {}", msg),
        Error::Protocol(msg) => format!("Protocol error: {}", msg),
        Error::MethodNotFound(msg) => format!("Method not found: {}", msg),
        Error::InvalidParams(msg) => format!("Invalid parameters: {}", msg),
        Error::Lifecycle(msg) => format!("Lifecycle error: {}", msg),
        Error::Resource(msg) => format!("Resource error: {}", msg),
        Error::Tool(msg) => format!("Tool error: {}", msg),
        Error::Prompt(msg) => format!("Prompt error: {}", msg),
        Error::Validation(msg) => format!("Validation error: {}", msg),
        Error::SchemaValidation(msg) => format!("Schema validation error: {}", msg),
        Error::VersionIncompatibility(msg) => format!("Version incompatibility: {}", msg),
        Error::Authentication(msg) => format!("Authentication error: {}", msg),
        Error::Authorization(msg) => format!("Authorization error: {}", msg),
        Error::RateLimit(msg) => format!("Rate limit exceeded: {}", msg),
        Error::Timeout(msg) => format!("Timeout: {}", msg),
        Error::Initialization(msg) => format!("Initialization error: {}", msg),
        Error::ServerUnavailable(msg) => format!("Server unavailable: {}", msg),
        Error::InvalidState(msg) => format!("Invalid state: {}", msg),
        Error::Other(msg) => format!("Error: {}", msg),
        Error::PromptNotFound(msg) => format!("Prompt not found: {}", msg),
        Error::MissingArgument(msg) => format!("Missing argument: {}", msg),
        Error::MissingArguments => format!("Missing arguments"),
    }
}

/// Extract an error code from an Error
pub fn extract_error_code(error: &Error) -> Option<i32> {
    match error {
        Error::MethodNotFound(_) => Some(error_codes::METHOD_NOT_FOUND),
        Error::InvalidParams(_) => Some(error_codes::INVALID_PARAMS),
        Error::Resource(_) => Some(error_codes::RESOURCE_NOT_FOUND),
        Error::Tool(_) => Some(error_codes::TOOL_NOT_FOUND),
        Error::Prompt(_) => Some(error_codes::PROMPT_NOT_FOUND),
        Error::Authentication(_) => Some(error_codes::AUTHENTICATION_ERROR),
        Error::Authorization(_) => Some(error_codes::AUTHORIZATION_ERROR),
        Error::RateLimit(_) => Some(error_codes::RATE_LIMIT_EXCEEDED),
        Error::Timeout(_) => Some(error_codes::REQUEST_TIMEOUT),
        _ => None,
    }
}
