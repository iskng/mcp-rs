//! MCP Utilities
//!
//! This module provides utility functions and helpers for the MCP library, such as
//! schema generation, message validation, or other common operations used across
//! the crate.

use crate::protocol::errors::Error;
use crate::protocol::{
    ErrorData, JSONRPCMessage as Message, JSONRPCResponse as Response, Result as ResponseOutcome,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::future::Future;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, trace, warn};
use url::Url;

//=============================================================================
// 1. JSON Utilities
//=============================================================================

/// JSON utility functions for working with MCP data
pub mod json {
    use super::*;

    /// Serialize a value to a JSON string with pretty formatting
    pub fn to_pretty_string<T: Serialize>(value: &T) -> Result<String, Error> {
        serde_json::to_string_pretty(value).map_err(Error::Json)
    }

    /// Serialize a value to a compact JSON string
    pub fn to_string<T: Serialize>(value: &T) -> Result<String, Error> {
        serde_json::to_string(value).map_err(Error::Json)
    }

    /// Deserialize a JSON string to a value of type T
    pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, Error> {
        serde_json::from_str(s).map_err(Error::Json)
    }

    /// Extract a field from a JSON Value as a specific type
    pub fn extract_field<T: DeserializeOwned>(value: &Value, field: &str) -> Result<T, Error> {
        let field_value = value.get(field).ok_or_else(|| {
            Error::Protocol(format!("Field '{}' not found in JSON object", field))
        })?;

        serde_json::from_value(field_value.clone()).map_err(Error::Json)
    }

    /// Merge two JSON objects, with values from the second overriding the first
    pub fn merge_objects(base: &mut Value, overlay: &Value) -> Result<(), Error> {
        if !base.is_object() || !overlay.is_object() {
            return Err(Error::Protocol("Can only merge JSON objects".to_string()));
        }

        let base_map = base.as_object_mut().unwrap();
        let overlay_map = overlay.as_object().unwrap();

        for (key, value) in overlay_map {
            if let Some(existing) = base_map.get_mut(key) {
                if existing.is_object() && value.is_object() {
                    merge_objects(existing, value)?;
                } else {
                    *existing = value.clone();
                }
            } else {
                base_map.insert(key.clone(), value.clone());
            }
        }

        Ok(())
    }
}

//=============================================================================
// 2. URI Handling
//=============================================================================

/// URI utility functions for working with MCP resources
pub mod uri {
    use super::*;

    /// The scheme for MCP resource URIs
    pub const MCP_SCHEME: &str = "mcp";

    /// A typed representation of an MCP URI
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct McpUri {
        url: Url,
    }

    impl McpUri {
        /// Create a new MCP URI from the given path components
        pub fn new(components: &[&str]) -> Result<Self, Error> {
            let path = components.join("/");
            let uri_str = format!("{}://{}", MCP_SCHEME, path);
            Self::from_str(&uri_str)
        }

        /// Return the scheme of the URI
        pub fn scheme(&self) -> &str {
            self.url.scheme()
        }

        /// Return the path of the URI
        pub fn path(&self) -> &str {
            self.url.path()
        }

        /// Return the resource type from the URI
        pub fn resource_type(&self) -> Option<&str> {
            self.url.path_segments()?.next()
        }

        /// Return the resource ID from the URI
        pub fn resource_id(&self) -> Option<&str> {
            let mut segments = self.url.path_segments()?;
            segments.next()?; // Skip the type
            segments.next() // Get the ID
        }

        /// Check if this is a valid MCP URI
        pub fn is_valid(&self) -> bool {
            self.scheme() == MCP_SCHEME && self.resource_type().is_some()
        }
    }

    impl FromStr for McpUri {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match Url::parse(s) {
                Ok(url) => Ok(McpUri { url }),
                Err(e) => Err(Error::Resource(format!("Invalid URI: {}", e))),
            }
        }
    }

    impl ToString for McpUri {
        fn to_string(&self) -> String {
            self.url.to_string()
        }
    }

    /// Validate that a URI string is a valid MCP URI
    pub fn validate_uri(uri: &str) -> Result<(), Error> {
        let mcp_uri = McpUri::from_str(uri)?;
        if !mcp_uri.is_valid() {
            return Err(Error::Resource(format!("Invalid MCP URI format: {}", uri)));
        }
        Ok(())
    }
}

//=============================================================================
// 4. Async Utilities
//=============================================================================

/// Async utility functions for working with futures in MCP
pub mod async_utils {
    use super::*;

    /// Run a future with a timeout
    pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, Error>
    where
        F: Future<Output = Result<T, Error>>,
    {
        match tokio_timeout(duration, future).await {
            Ok(result) => result,
            Err(_) => Err(Error::Other(format!(
                "Operation timed out after {:?}",
                duration
            ))),
        }
    }

    /// Retry an async operation with exponential backoff
    pub async fn retry<F, Fut, T>(
        operation: F,
        max_attempts: usize,
        initial_backoff: Duration,
    ) -> Result<T, Error>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        let mut attempts = 0;
        let mut backoff = initial_backoff;

        loop {
            attempts += 1;
            match operation().await {
                Ok(value) => {
                    return Ok(value);
                }
                Err(error) => {
                    if attempts >= max_attempts {
                        return Err(error);
                    }

                    debug!(
                        "Retry attempt {}/{} failed: {}",
                        attempts, max_attempts, error
                    );
                    tokio::time::sleep(backoff).await;
                    backoff *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Cancel a future after a specified duration
    pub async fn with_timeout<F, T>(future: F, duration: Duration) -> Result<T, Error>
    where
        F: Future<Output = T>,
    {
        tokio_timeout(duration, future)
            .await
            .map_err(|_| Error::Other(format!("Operation timed out after {:?}", duration)))
    }
}

//=============================================================================
// 5. Schema Utilities
//=============================================================================

/// Schema utility functions for working with JSON schemas in MCP
pub mod schema {
    use jsonschema::JSONSchema;
    use schemars::JsonSchema;
    use schemars::schema_for;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    use crate::protocol::ReadResourceParams;
    use crate::protocol::ReadResourceResult;
    use crate::protocol::errors::Error;
    use crate::protocol::{
        CallToolParams, CallToolResult, InitializeResult, ListPromptsResult, ListResourcesResult,
        ListToolsResult,
    };
    use crate::protocol::{
        JSONRPCMessage as Message, JSONRPCRequest as Request, JSONRPCResponse as Response,
    };

    // Static reference to compiled full schema
    static FULL_SCHEMA: OnceLock<JSONSchema> = OnceLock::new();

    /// Initialize the full MCP schema from file
    pub fn init_full_schema() -> Result<(), Error> {
        if FULL_SCHEMA.get().is_some() {
            return Ok(());
        }

        // Try to use the embedded schema if available
        let schema_content = include_str!("../src/protocol/protocol.json");
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

    /// Validate a message against the full MCP JSON schema
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
    pub fn validate_request_against_full_schema(request: &Request) -> Result<(), Error> {
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
    pub fn validate_response_against_full_schema(response: &Response) -> Result<(), Error> {
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
    pub fn validate_message_against_full_schema(message: &Message) -> Result<(), Error> {
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

            registry.insert(
                "resources/read".to_string(),
                generate_schema::<ReadResourceParams>(),
            );

            registry.insert(
                "tools/call".to_string(),
                generate_schema::<CallToolParams>(),
            );

            registry
        })
    }

    /// Get or create the method result schema registry
    fn result_schema_registry() -> &'static HashMap<String, Value> {
        static REGISTRY: OnceLock<HashMap<String, Value>> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut registry = HashMap::new();

            // Initialize method
            registry.insert(
                "initialize".to_string(),
                generate_schema::<InitializeResult>(),
            );

            // Resource methods
            registry.insert(
                "resources/list".to_string(),
                generate_schema::<ListResourcesResult>(),
            );
            registry.insert(
                "resources/read".to_string(),
                generate_schema::<ReadResourceResult>(),
            );

            // Tool methods
            registry.insert(
                "tools/list".to_string(),
                generate_schema::<ListToolsResult>(),
            );
            registry.insert(
                "tools/call".to_string(),
                generate_schema::<CallToolResult>(),
            );

            // Prompt methods
            registry.insert(
                "prompts/list".to_string(),
                generate_schema::<ListPromptsResult>(),
            );

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
}

//=============================================================================
// 6. Authentication Utilities
//=============================================================================

/// Authentication utility functions for MCP (if required)
pub mod auth {
    use super::*;

    /// A simple API key for basic authentication
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ApiKey {
        pub key: String,
    }

    impl ApiKey {
        /// Create a new API key
        pub fn new(key: &str) -> Self {
            Self {
                key: key.to_string(),
            }
        }

        /// Validate the API key
        pub fn validate(&self, expected: &str) -> bool {
            self.key == expected
        }
    }

    /// Generate a simple authentication header for HTTP-based transports
    pub fn generate_auth_header(api_key: &ApiKey) -> String {
        format!("Bearer {}", api_key.key)
    }

    /// Parse an authentication header to extract the API key
    pub fn parse_auth_header(header: &str) -> Result<ApiKey, Error> {
        if let Some(key) = header.strip_prefix("Bearer ") {
            Ok(ApiKey::new(key))
        } else {
            Err(Error::Protocol(
                "Invalid authentication header format".to_string(),
            ))
        }
    }
}
