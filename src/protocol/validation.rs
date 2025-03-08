use crate::protocol::Error;
use crate::protocol::{JSONRPCRequest, JSONRPCResponse};
use jsonschema::JSONSchema;
use schemars::JsonSchema;
use schemars::schema_for;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::{JSONRPCMessage, JSONRPCNotification};

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

/// Validate a request message
///
/// # Arguments
/// * `request` - The request to validate
/// * `method` - The method name being called (for logging purposes)
/// * `config` - The validation configuration
pub fn validate_request(
    request: &JSONRPCRequest,
    method: &str,
    config: &ValidationConfig,
) -> Result<(), Error> {
    // Basic validations
    if request.jsonrpc != "2.0" {
        return Err(Error::Validation(
            "JSON-RPC version must be 2.0".to_string(),
        ));
    }

    // Method cannot be empty
    if request.method.is_empty() {
        return Err(Error::Validation("Method cannot be empty".to_string()));
    }

    // Schema validation if enabled
    if config.use_schema_validation {
        if let Some(params) = &request.params {
            validate_method_params(&request.method, params)?;
        }

        if config.validate_requests {
            validate_request_against_full_schema(request)?;
        }
    }

    // Additional method-specific validations can be added here

    Ok(())
}

/// Validate a response message
///
/// # Arguments
/// * `response` - The response to validate
/// * `method` - The method name that was called (for logging purposes)
/// * `config` - The validation configuration
pub fn validate_response(
    response: &JSONRPCResponse,
    method: &str,
    config: &ValidationConfig,
) -> Result<(), Error> {
    // Basic validations
    if response.jsonrpc != "2.0" {
        return Err(Error::Validation(
            "JSON-RPC version must be 2.0".to_string(),
        ));
    }

    // Schema validation if enabled
    if config.use_schema_validation && config.validate_responses {
        validate_response_against_full_schema(response)?;

        // Convert response.result to Value for schema validation
        let result_value = serde_json::to_value(&response.result)?;
        validate_method_result(method, &result_value)?;
    }

    // Additional method-specific validations can be added here

    Ok(())
}

/// Validate a notification message
///
/// # Arguments
/// * `notification` - The notification to validate
/// * `config` - The validation configuration
pub fn validate_notification(
    notification: &JSONRPCNotification,
    config: &ValidationConfig,
) -> Result<(), Error> {
    // Basic validations
    if notification.jsonrpc != "2.0" {
        return Err(Error::Validation(
            "JSON-RPC version must be 2.0".to_string(),
        ));
    }

    // Method cannot be empty
    if notification.method.is_empty() {
        return Err(Error::Validation("Method cannot be empty".to_string()));
    }

    // Schema validation if enabled
    if config.use_schema_validation && config.validate_requests {
        // Convert notification to Value for schema validation
        let value = serde_json::to_value(notification)?;
        validate_against_full_schema(&value)?;

        if let Some(params) = &notification.params {
            validate_method_params(&notification.method, params)?;
        }
    }

    Ok(())
}

//=============================================================================
// Schema Validation Utilities
//=============================================================================

// Static reference to compiled full schema
static FULL_SCHEMA: OnceLock<JSONSchema> = OnceLock::new();

/// Initialize the full MCP schema from file
pub fn init_full_schema() -> Result<(), Error> {
    if FULL_SCHEMA.get().is_some() {
        return Ok(());
    }

    // Try to use the embedded schema if available
    let schema_content = include_str!("protocol.json");
    let schema_value: Value = serde_json::from_str(schema_content)
        .map_err(|e| Error::SchemaValidation(format!("Failed to parse schema.json: {}", e)))?;

    // Compile the schema
    let compiled = JSONSchema::compile(&schema_value)
        .map_err(|e| Error::SchemaValidation(format!("Failed to compile schema: {}", e)))?;

    // Store the compiled schema
    FULL_SCHEMA
        .set(compiled)
        .map_err(|_| Error::SchemaValidation("Failed to set compiled schema".to_string()))?;

    Ok(())
}

/// Validate a value against the full MCP JSON schema
pub fn validate_against_full_schema(value: &Value) -> Result<(), Error> {
    if let Some(schema) = FULL_SCHEMA.get() {
        let validation = schema.validate(value);
        if let Err(errors) = validation {
            let error_msgs: Vec<String> = errors.map(|e| format!("{}", e)).collect();
            return Err(Error::SchemaValidation(format!(
                "Schema validation failed: {}",
                error_msgs.join(", ")
            )));
        }
        Ok(())
    } else {
        // Schema not loaded, can't validate
        Ok(())
    }
}

/// Validate a request message against the full MCP schema
pub fn validate_request_against_full_schema(request: &JSONRPCRequest) -> Result<(), Error> {
    // Schema is expected to be initialized by the caller
    if FULL_SCHEMA.get().is_none() {
        // Schema not available, just succeed
        return Ok(());
    }

    let json_value = serde_json::to_value(request)
        .map_err(|e| Error::SchemaValidation(format!("Failed to serialize request: {}", e)))?;

    validate_against_full_schema(&json_value)
}

/// Validate a response message against the full MCP schema
pub fn validate_response_against_full_schema(response: &JSONRPCResponse) -> Result<(), Error> {
    // Schema is expected to be initialized by the caller
    if FULL_SCHEMA.get().is_none() {
        // Schema not available, just succeed
        return Ok(());
    }

    let json_value = serde_json::to_value(response)
        .map_err(|e| Error::SchemaValidation(format!("Failed to serialize response: {}", e)))?;

    validate_against_full_schema(&json_value)
}

/// Validate a generic message against the full MCP schema
pub fn validate_message_against_full_schema(message: &JSONRPCMessage) -> Result<(), Error> {
    // Schema is expected to be initialized by the caller
    if FULL_SCHEMA.get().is_none() {
        // Schema not available, just succeed
        return Ok(());
    }

    let json_value = serde_json::to_value(message)
        .map_err(|e| Error::SchemaValidation(format!("Failed to serialize message: {}", e)))?;

    validate_against_full_schema(&json_value)
}

/// Get or create the method parameter schema registry
fn param_schema_registry() -> &'static HashMap<String, Value> {
    static REGISTRY: OnceLock<HashMap<String, Value>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = HashMap::new();

        // Add schema entries for request parameters
        // These would be generated from your protocol types
        // Example:
        // registry.insert("resources/read".to_string(), generate_schema::<ReadResourceParams>());
        // registry.insert("tools/call".to_string(), generate_schema::<CallToolParams>());

        registry
    })
}

/// Get or create the method result schema registry
fn result_schema_registry() -> &'static HashMap<String, Value> {
    static REGISTRY: OnceLock<HashMap<String, Value>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = HashMap::new();

        // Add schema entries for response results
        // Example:
        // registry.insert("initialize".to_string(), generate_schema::<InitializeResult>());
        // registry.insert("resources/list".to_string(), generate_schema::<ListResourcesResult>());

        registry
    })
}

/// Generate a JSON schema for the given type
pub fn generate_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap_or_else(|_| Value::Null)
}

/// Validate the given value against the JSON schema for the type
pub fn validate<T: JsonSchema>(value: &Value) -> Result<(), Error> {
    let schema = generate_schema::<T>();
    validate_against_schema(value, &schema)
}

/// Validate the given value against a specific JSON schema
pub fn validate_against_schema(value: &Value, schema: &Value) -> Result<(), Error> {
    let compiled = JSONSchema::compile(schema)
        .map_err(|e| Error::Protocol(format!("Invalid schema: {}", e)))?;

    let validation = compiled.validate(value);
    if let Err(errors) = validation {
        let error_msgs: Vec<String> = errors.map(|e| format!("{}", e)).collect();

        return Err(Error::InvalidParams(format!(
            "Schema validation failed: {}",
            error_msgs.join(", ")
        )));
    }

    Ok(())
}

/// Get the parameter schema for a specific method
pub fn get_param_schema(method: &str) -> Option<Value> {
    param_schema_registry().get(method).cloned()
}

/// Get the result schema for a specific method
pub fn get_result_schema(method: &str) -> Option<Value> {
    result_schema_registry().get(method).cloned()
}

/// Validate parameters for a specific method
pub fn validate_method_params(method: &str, params: &Value) -> Result<(), Error> {
    if let Some(schema) = get_param_schema(method) {
        validate_against_schema(params, &schema)
    } else {
        // No schema found for this method - can't validate
        Ok(())
    }
}

/// Validate result for a specific method
pub fn validate_method_result(method: &str, result: &Value) -> Result<(), Error> {
    if let Some(schema) = get_result_schema(method) {
        validate_against_schema(result, &schema)
    } else {
        // No schema found for this method - can't validate
        Ok(())
    }
}

/// Generate schemas for all supported types
pub fn generate_all_schemas() -> Result<HashMap<String, Value>, Error> {
    let mut schemas = HashMap::new();

    // Include all parameter schemas
    for (method, schema) in param_schema_registry().iter() {
        schemas.insert(format!("{}_params", method), schema.clone());
    }

    // Include all result schemas
    for (method, schema) in result_schema_registry().iter() {
        schemas.insert(format!("{}_result", method), schema.clone());
    }

    Ok(schemas)
}
