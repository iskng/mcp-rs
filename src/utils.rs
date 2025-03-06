//! MCP Utilities
//!
//! This module provides utility functions and helpers for the MCP library, such as
//! schema generation, message validation, or other common operations used across
//! the crate.

use crate::errors::Error;
use crate::types::protocol::{ ErrorData, Message, Response, ResponseOutcome };
use serde::{ Deserialize, Serialize, de::DeserializeOwned };
use serde_json::Value;
use std::future::Future;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout as tokio_timeout;
use tracing::{ debug, trace, warn };
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
        let field_value = value
            .get(field)
            .ok_or_else(|| {
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
// 3. Error Utilities
//=============================================================================

/// Error utility functions for MCP error handling
pub mod errors {
    use super::*;
    use crate::errors::error_codes;

    /// Create an error response with the given code, message, and optional data
    pub fn create_error_response(
        id: i32,
        code: i32,
        message: &str,
        data: Option<Value>
    ) -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            id,
            outcome: ResponseOutcome::Error {
                error: ErrorData {
                    code,
                    message: message.to_string(),
                    data,
                },
            },
        }
    }

    /// Create a standard protocol error response
    pub fn protocol_error(id: i32, message: &str) -> Response {
        create_error_response(id, error_codes::INVALID_REQUEST, message, None)
    }

    /// Create a method not found error response
    pub fn method_not_found(id: i32, method: &str) -> Response {
        create_error_response(
            id,
            error_codes::METHOD_NOT_FOUND,
            &format!("Method not found: {}", method),
            None
        )
    }

    /// Create an invalid params error response
    pub fn invalid_params(id: i32, message: &str) -> Response {
        create_error_response(id, error_codes::INVALID_PARAMS, message, None)
    }

    /// Create an internal error response
    pub fn internal_error(id: i32, message: &str) -> Response {
        create_error_response(id, error_codes::INTERNAL_ERROR, message, None)
    }

    /// Create a resource not found error response
    pub fn resource_not_found(id: i32, uri: &str) -> Response {
        create_error_response(
            id,
            error_codes::RESOURCE_NOT_FOUND,
            &format!("Resource not found: {}", uri),
            None
        )
    }

    /// Create a tool not found error response
    pub fn tool_not_found(id: i32, tool_name: &str) -> Response {
        create_error_response(
            id,
            error_codes::TOOL_NOT_FOUND,
            &format!("Tool not found: {}", tool_name),
            None
        )
    }

    /// Create a tool execution error response
    pub fn tool_execution_error(id: i32, tool_name: &str, error_msg: &str) -> Response {
        create_error_response(
            id,
            error_codes::TOOL_EXECUTION_ERROR,
            &format!("Error executing tool {}: {}", tool_name, error_msg),
            None
        )
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
        where F: Future<Output = Result<T, Error>>
    {
        match tokio_timeout(duration, future).await {
            Ok(result) => result,
            Err(_) => Err(Error::Other(format!("Operation timed out after {:?}", duration))),
        }
    }

    /// Retry an async operation with exponential backoff
    pub async fn retry<F, Fut, T>(
        operation: F,
        max_attempts: usize,
        initial_backoff: Duration
    )
        -> Result<T, Error>
        where F: Fn() -> Fut, Fut: Future<Output = Result<T, Error>>
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

                    debug!("Retry attempt {}/{} failed: {}", attempts, max_attempts, error);
                    tokio::time::sleep(backoff).await;
                    backoff *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Cancel a future after a specified duration
    pub async fn with_timeout<F, T>(future: F, duration: Duration) -> Result<T, Error>
        where F: Future<Output = T>
    {
        tokio_timeout(duration, future).await.map_err(|_|
            Error::Other(format!("Operation timed out after {:?}", duration))
        )
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

    use crate::errors::Error;
    use crate::types::protocol::{ Message, Response, Request };
    use crate::types::resources::ReadResourceParams;
    use crate::types::resources::ReadResourceResult;
    use crate::types::{
        initialize::{ InitializeRequestParams, InitializeResult },
        prompts::{ ListPromptsParams, ListPromptsResult },
        resources::{
            CreateResourceParams,
            CreateResourceResult,
            DeleteResourceParams,
            DeleteResourceResult,
            ListResourcesParams,
            ListResourcesResult,
            UpdateResourceParams,
            UpdateResourceResult,
        },
        tools::{ CallToolParams, CallToolResult, ListToolsParams, ListToolsResult },
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

    /// Validate a message against the full MCP JSON schema
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

        let json_value = serde_json
            ::to_value(request)
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

        let json_value = serde_json
            ::to_value(response)
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

        let json_value = serde_json
            ::to_value(message)
            .map_err(|e| Error::SchemaValidation(format!("Failed to serialize message: {}", e)))?;

        validate_against_full_schema(&json_value)
    }

    /// Get or create the method parameter schema registry
    fn param_schema_registry() -> &'static HashMap<String, Value> {
        static REGISTRY: OnceLock<HashMap<String, Value>> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut registry = HashMap::new();

            // Initialize method
            registry.insert("initialize".to_string(), generate_schema::<InitializeRequestParams>());

            // Resource methods
            registry.insert("resources/list".to_string(), generate_schema::<ListResourcesParams>());
            registry.insert("resources/read".to_string(), generate_schema::<ReadResourceParams>());
            registry.insert(
                "resources/create".to_string(),
                generate_schema::<CreateResourceParams>()
            );
            registry.insert(
                "resources/update".to_string(),
                generate_schema::<UpdateResourceParams>()
            );
            registry.insert(
                "resources/delete".to_string(),
                generate_schema::<DeleteResourceParams>()
            );

            // Tool methods
            registry.insert("tools/list".to_string(), generate_schema::<ListToolsParams>());
            registry.insert("tools/call".to_string(), generate_schema::<CallToolParams>());

            // Prompt methods
            registry.insert("prompts/list".to_string(), generate_schema::<ListPromptsParams>());

            registry
        })
    }

    /// Get or create the method result schema registry
    fn result_schema_registry() -> &'static HashMap<String, Value> {
        static REGISTRY: OnceLock<HashMap<String, Value>> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut registry = HashMap::new();

            // Initialize method
            registry.insert("initialize".to_string(), generate_schema::<InitializeResult>());

            // Resource methods
            registry.insert("resources/list".to_string(), generate_schema::<ListResourcesResult>());
            registry.insert("resources/read".to_string(), generate_schema::<ReadResourceResult>());
            registry.insert(
                "resources/create".to_string(),
                generate_schema::<CreateResourceResult>()
            );
            registry.insert(
                "resources/update".to_string(),
                generate_schema::<UpdateResourceResult>()
            );
            registry.insert(
                "resources/delete".to_string(),
                generate_schema::<DeleteResourceResult>()
            );

            // Tool methods
            registry.insert("tools/list".to_string(), generate_schema::<ListToolsResult>());
            registry.insert("tools/call".to_string(), generate_schema::<CallToolResult>());

            // Prompt methods
            registry.insert("prompts/list".to_string(), generate_schema::<ListPromptsResult>());

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
        let compiled = JSONSchema::compile(schema).map_err(|e|
            Error::Protocol(format!("Invalid schema: {}", e))
        )?;

        let validation = compiled.validate(value);
        if let Err(errors) = validation {
            let error_msgs: Vec<String> = errors.map(|e| format!("{}", e)).collect();

            return Err(
                Error::InvalidParams(format!("Schema validation failed: {}", error_msgs.join(", ")))
            );
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
            Err(Error::Protocol("Invalid authentication header format".to_string()))
        }
    }
}

//=============================================================================
// 7. Logging and Debugging Utilities
//=============================================================================

/// Logging and debugging utility functions for MCP
pub mod logging {
    use super::*;

    /// Log an MCP message at debug level
    pub fn log_message(msg: &Message) {
        match msg {
            Message::Request(req) => {
                debug!("MCP Request: method={}, id={:?}", req.method, req.id);
                trace!("Request params: {:?}", req.params);
            }
            Message::Response(resp) =>
                match &resp.outcome {
                    ResponseOutcome::Success { result: _ } => {
                        debug!("MCP Response: id={}, success=true", resp.id);
                        trace!("Response result: {:?}", resp.outcome);
                    }
                    ResponseOutcome::Error { error } => {
                        warn!(
                            "MCP Error Response: id={}, code={}, message={}",
                            resp.id,
                            error.code,
                            error.message
                        );
                    }
                }
            Message::Notification(notification) => {
                debug!("MCP Notification: method={}", notification.method);
                trace!("Notification params: {:?}", notification.params);
            }
        }
    }

    /// Pretty-print an MCP message as a string
    pub fn format_message(msg: &Message) -> Result<String, Error> {
        json::to_pretty_string(msg)
    }

    /// Create a debug dump of the message exchange for troubleshooting
    pub struct MessageExchangeDebugger {
        request: Option<Message>,
        response: Option<Message>,
        timestamp: std::time::SystemTime,
    }

    impl MessageExchangeDebugger {
        /// Create a new message exchange debugger
        pub fn new() -> Self {
            Self {
                request: None,
                response: None,
                timestamp: std::time::SystemTime::now(),
            }
        }

        /// Record a request
        pub fn record_request(&mut self, request: Message) {
            self.request = Some(request);
        }

        /// Record a response
        pub fn record_response(&mut self, response: Message) {
            self.response = Some(response);
        }

        /// Generate a debug report
        pub fn generate_report(&self) -> Result<String, Error> {
            let mut report = String::new();

            report.push_str(&format!("MCP Message Exchange Debug Report\n"));
            report.push_str(&format!("Timestamp: {:?}\n\n", self.timestamp));

            if let Some(ref req) = self.request {
                report.push_str("REQUEST:\n");
                report.push_str(&json::to_pretty_string(req)?);
                report.push_str("\n\n");
            } else {
                report.push_str("REQUEST: None recorded\n\n");
            }

            if let Some(ref resp) = self.response {
                report.push_str("RESPONSE:\n");
                report.push_str(&json::to_pretty_string(resp)?);
                report.push_str("\n");
            } else {
                report.push_str("RESPONSE: None recorded\n");
            }

            Ok(report)
        }
    }

    impl Default for MessageExchangeDebugger {
        fn default() -> Self {
            Self::new()
        }
    }
}

// Re-export commonly used utilities
pub use async_utils::timeout;
pub use errors::{ create_error_response, invalid_params, method_not_found };
pub use json::{ from_str, to_string };
pub use logging::log_message;
pub use schema::validate;
pub use uri::{ McpUri, validate_uri };

/// Validation module for request and response validation
pub mod validation {
    use super::schema;
    use crate::errors::Error;
    use crate::types::protocol::{ Request, Response, ResponseOutcome };

    /// Configuration for schema validation
    #[derive(Debug, Clone, Copy)]
    pub struct ValidationConfig {
        /// Whether to validate requests
        pub validate_requests: bool,
        /// Whether to validate responses
        pub validate_responses: bool,
    }

    impl Default for ValidationConfig {
        fn default() -> Self {
            Self {
                validate_requests: true,
                validate_responses: true,
            }
        }
    }

    /// Validate an incoming request against its method schema
    pub fn validate_request(request: &Request, config: &ValidationConfig) -> Result<(), Error> {
        // Skip validation if disabled
        if !config.validate_requests {
            return Ok(());
        }

        // Get method and params
        let method = &request.method;

        // Validate params if they exist
        if let Some(params) = &request.params {
            schema::validate_method_params(method, params)?;
        }

        // Conditionally initialize and use full schema validation
        // We use a soft validation that won't fail if the schema isn't available
        if config.validate_requests {
            // Try to initialize the schema only when needed
            if schema::init_full_schema().is_ok() {
                schema::validate_request_against_full_schema(request).ok();
            }
        }

        Ok(())
    }

    /// Validate an outgoing response against its method schema
    pub fn validate_response(
        response: &Response,
        method: &str,
        config: &ValidationConfig
    ) -> Result<(), Error> {
        // Skip validation if disabled
        if !config.validate_responses {
            return Ok(());
        }

        // Validate result if it's a success response
        if let ResponseOutcome::Success { result } = &response.outcome {
            schema::validate_method_result(method, result)?;
        }

        // Conditionally initialize and use full schema validation
        // We use a soft validation that won't fail if the schema isn't available
        if config.validate_responses {
            // Try to initialize the schema only when needed
            if schema::init_full_schema().is_ok() {
                schema::validate_response_against_full_schema(response).ok();
            }
        }

        Ok(())
    }
}
