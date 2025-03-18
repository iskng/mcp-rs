use crate::protocol::Error;
use crate::protocol::{ JSONRPCRequest, JSONRPCResponse, JSONRPCMessage, JSONRPCNotification };
use jsonschema::JSONSchema;
use serde_json::Value;
use std::sync::OnceLock;

use super::Method;

/// Configuration for validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Whether to validate requests
    pub validate_requests: bool,
    /// Whether to validate responses
    pub validate_responses: bool,
    /// Whether to use schema validation
    pub use_schema_validation: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validate_requests: true,
            validate_responses: true,
            use_schema_validation: true,
        }
    }
}

// Static reference to compiled full schema
static FULL_SCHEMA: OnceLock<JSONSchema> = OnceLock::new();

/// Initialize the full MCP schema from file
pub fn init_full_schema() -> Result<(), Error> {
    if FULL_SCHEMA.get().is_some() {
        return Ok(());
    }

    // Try to use the embedded schema if available
    let schema_content = include_str!("protocol.json");
    let schema_value: Value = serde_json
        ::from_str(schema_content)
        .map_err(|e| Error::SchemaValidation(format!("Failed to parse schema.json: {}", e)))?;

    // Compile the schema
    let compiled = JSONSchema::compile(&schema_value).map_err(|e|
        Error::SchemaValidation(format!("Failed to compile schema: {}", e))
    )?;

    // Store the compiled schema
    FULL_SCHEMA.set(compiled).map_err(|_|
        Error::SchemaValidation("Failed to set compiled schema".to_string())
    )?;

    Ok(())
}

/// Validate any JSONRPC message
pub fn validate_message(message: &JSONRPCMessage, config: &ValidationConfig) -> Result<(), Error> {
    // Basic validations common to all message types
    match message {
        JSONRPCMessage::Request(req) => validate_request(req, config),
        JSONRPCMessage::Response(resp) => {
            // For responses, we need the method for full validation, but we can still do basic validation
            validate_response(resp, &Method::Initialize, config)
        }
        JSONRPCMessage::Notification(notif) => validate_notification(notif, config),
        JSONRPCMessage::Error(_) => Ok(()), // Error messages don't need validation
    }
}

/// Validate a request message
pub fn validate_request(request: &JSONRPCRequest, config: &ValidationConfig) -> Result<(), Error> {
    // Basic validations
    if request.jsonrpc != "2.0" {
        return Err(Error::Validation("JSON-RPC version must be 2.0".to_string()));
    }

    // Schema validation if enabled
    if config.use_schema_validation && config.validate_requests {
        let json_value = serde_json
            ::to_value(request)
            .map_err(|e| Error::SchemaValidation(format!("Failed to serialize request: {}", e)))?;

        validate_against_full_schema(&json_value)?;
    }

    Ok(())
}

/// Validate a response message
pub fn validate_response(
    response: &JSONRPCResponse,
    _method: &Method,
    config: &ValidationConfig
) -> Result<(), Error> {
    // Basic validations
    if response.jsonrpc != "2.0" {
        return Err(Error::Validation("JSON-RPC version must be 2.0".to_string()));
    }

    // Schema validation if enabled
    if config.use_schema_validation && config.validate_responses {
        let json_value = serde_json
            ::to_value(response)
            .map_err(|e| Error::SchemaValidation(format!("Failed to serialize response: {}", e)))?;

        validate_against_full_schema(&json_value)?;
    }

    Ok(())
}

/// Validate a notification message
pub fn validate_notification(
    notification: &JSONRPCNotification,
    config: &ValidationConfig
) -> Result<(), Error> {
    // Basic validations
    if notification.jsonrpc != "2.0" {
        return Err(Error::Validation("JSON-RPC version must be 2.0".to_string()));
    }

    // Schema validation if enabled
    if config.use_schema_validation && config.validate_requests {
        let json_value = serde_json
            ::to_value(notification)
            .map_err(|e|
                Error::SchemaValidation(format!("Failed to serialize notification: {}", e))
            )?;

        validate_against_full_schema(&json_value)?;
    }

    Ok(())
}

/// Validate a value against the full MCP JSON schema
pub fn validate_against_full_schema(value: &Value) -> Result<(), Error> {
    if let Some(schema) = FULL_SCHEMA.get() {
        let validation = schema.validate(value);
        if let Err(errors) = validation {
            let error_msgs: Vec<String> = errors.map(|e| format!("{}", e)).collect();
            return Err(
                Error::SchemaValidation(
                    format!("Schema validation failed: {}", error_msgs.join(", "))
                )
            );
        }
        Ok(())
    } else {
        // Schema not loaded, initialize it first
        init_full_schema()?;
        validate_against_full_schema(value)
    }
}
