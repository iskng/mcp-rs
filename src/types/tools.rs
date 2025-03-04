//! MCP Tool Types
//!
//! This module defines types related to tools in the MCP protocol, including
//! `Tool` and `CallToolParams`, which allow the AI application to perform actions
//! or computations via the server.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a parameter for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolParameterType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "array")]
    Array,
}

/// Represents a parameter for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Name of the parameter
    pub name: String,
    /// Type of the parameter
    #[serde(rename = "type")]
    pub type_name: ToolParameterType,
    /// Description of the parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the parameter is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Schema for the parameter (for complex types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Enum values (for string parameters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Represents a tool in the MCP protocol
#[derive(Debug, Clone)]
pub struct Tool {
    /// Unique name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// Parameters that the tool accepts
    pub parameters: Option<Vec<ToolParameter>>,
    /// Type of the return value
    pub return_type: Option<String>,
    /// Schema for the return value
    pub return_schema: Option<serde_json::Value>,
    /// Whether the tool execution is streaming
    pub is_streaming: Option<bool>,
    /// Whether the tool can be cancelled
    pub is_cancellable: Option<bool>,
    /// Maximum execution time in seconds
    pub timeout_seconds: Option<u64>,
}

/// Parameters for listing tools
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListToolsParams {
    /// Optional pagination token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    /// Optional limit on the number of tools to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

/// Result of listing tools
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListToolsResult {
    /// List of tools
    pub tools: Vec<Tool>,
    /// Token for the next page (if there are more tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Parameters for calling a tool
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CallToolParams {
    /// Name of the tool to call (field used by Python/JS clients)
    #[serde(default)]
    pub name: String,
    /// Arguments to pass to the tool (field used by Python/JS clients)
    #[serde(default)]
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Text content for a message or tool result
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct TextContent {
    /// The type of content (always "text" for TextContent)
    #[serde(rename = "type")]
    pub content_type: String,

    /// The text content
    pub text: String,

    /// Optional annotations (not used in this implementation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
}

/// Result of calling a tool
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CallToolResult {
    /// Content of the tool call result
    pub content: Vec<serde_json::Value>,

    /// Flag indicating whether this is an error (using camelCase for Python compatibility)
    #[serde(rename = "isError")]
    pub is_error: bool,
}

// Instead of directly implementing JsonSchema for Tool, create a helper struct
// that can be used for JsonSchema derivation
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ToolSchema {
    /// Unique name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// Input schema for the tool
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    /// Type of the return value
    #[serde(rename = "returnType", skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    /// Schema for the return value
    #[serde(rename = "returnSchema", skip_serializing_if = "Option::is_none")]
    pub return_schema: Option<serde_json::Value>,
    /// Whether the tool execution is streaming
    #[serde(rename = "isStreaming", skip_serializing_if = "Option::is_none")]
    pub is_streaming: Option<bool>,
    /// Whether the tool can be cancelled
    #[serde(rename = "isCancellable", skip_serializing_if = "Option::is_none")]
    pub is_cancellable: Option<bool>,
    /// Maximum execution time in seconds
    #[serde(rename = "timeoutSeconds", skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

// Add a conversion from Tool to ToolSchema
impl From<&Tool> for ToolSchema {
    fn from(tool: &Tool) -> Self {
        // Build inputSchema from parameters
        let input_schema = if let Some(params) = &tool.parameters {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for param in params {
                // Create schema for this parameter
                let mut schema = match &param.schema {
                    Some(schema) => schema.clone(),
                    None => {
                        // Create default schema based on type_name
                        let type_str = match param.type_name {
                            ToolParameterType::String => "string",
                            ToolParameterType::Number => "number",
                            ToolParameterType::Boolean => "boolean",
                            ToolParameterType::Object => "object",
                            ToolParameterType::Array => "array",
                        };
                        serde_json::json!({ "type": type_str })
                    }
                };

                // Add enum values if present
                if let Some(enum_values) = &param.enum_values {
                    if !enum_values.is_empty() {
                        if let Some(obj) = schema.as_object_mut() {
                            obj.insert(
                                "enum".to_string(),
                                serde_json::Value::Array(
                                    enum_values
                                        .iter()
                                        .map(|v| serde_json::Value::String(v.clone()))
                                        .collect(),
                                ),
                            );
                        }
                    }
                }

                // Add default value if present
                if let Some(default_value) = &param.default {
                    if let Some(obj) = schema.as_object_mut() {
                        obj.insert("default".to_string(), default_value.clone());
                    }
                }

                // Add to properties map
                properties.insert(param.name.clone(), schema);

                // Add to required array if parameter is required
                if param.required.unwrap_or(false) {
                    required.push(param.name.clone());
                }
            }

            // Create the inputSchema object
            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            })
        } else {
            // Add minimal inputSchema if no parameters
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        };

        ToolSchema {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema,
            return_type: tool.return_type.clone(),
            return_schema: tool.return_schema.clone(),
            is_streaming: tool.is_streaming,
            is_cancellable: tool.is_cancellable,
            timeout_seconds: tool.timeout_seconds,
        }
    }
}

// Implement Serialize for Tool to properly handle the inputSchema field
impl Serialize for Tool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ToolSchema::from(self).serialize(serializer)
    }
}

// Implement Deserialize for Tool
impl<'de> Deserialize<'de> for Tool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ToolHelper {
            name: String,
            description: String,
            #[serde(rename = "inputSchema")]
            input_schema: Option<serde_json::Value>,
            #[serde(rename = "returnType")]
            return_type: Option<String>,
            #[serde(rename = "returnSchema")]
            return_schema: Option<serde_json::Value>,
            #[serde(rename = "isStreaming")]
            is_streaming: Option<bool>,
            #[serde(rename = "isCancellable")]
            is_cancellable: Option<bool>,
            #[serde(rename = "timeoutSeconds")]
            timeout_seconds: Option<u64>,
        }

        let helper = ToolHelper::deserialize(deserializer)?;

        // Convert inputSchema back to parameters
        let parameters = if let Some(input_schema) = helper.input_schema {
            if let Some(properties) = input_schema.get("properties").and_then(|p| p.as_object()) {
                let required: Vec<String> = input_schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let mut params = Vec::new();
                for (name, schema) in properties {
                    let type_name = schema
                        .get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| {
                            match t {
                                "string" => ToolParameterType::String,
                                "number" => ToolParameterType::Number,
                                "boolean" => ToolParameterType::Boolean,
                                "object" => ToolParameterType::Object,
                                "array" => ToolParameterType::Array,
                                _ => ToolParameterType::String, // Default to string for unknown types
                            }
                        })
                        .unwrap_or(ToolParameterType::String);

                    let enum_values = schema.get("enum").and_then(|e| e.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    });

                    let default = schema.get("default").cloned();

                    params.push(ToolParameter {
                        name: name.clone(),
                        type_name,
                        description: None, // Description is not stored in JSON Schema
                        required: Some(required.contains(name)),
                        schema: Some(schema.clone()),
                        default,
                        enum_values,
                    });
                }

                Some(params)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Tool {
            name: helper.name,
            description: helper.description,
            parameters,
            return_type: helper.return_type,
            return_schema: helper.return_schema,
            is_streaming: helper.is_streaming,
            is_cancellable: helper.is_cancellable,
            timeout_seconds: helper.timeout_seconds,
        })
    }
}

// Fix the JsonSchema implementation to avoid using 'gen' as parameter name
impl JsonSchema for Tool {
    fn schema_name() -> String {
        "Tool".to_string()
    }

    fn json_schema(
        schema_generator: &mut schemars::r#gen::SchemaGenerator,
    ) -> schemars::schema::Schema {
        <ToolSchema as JsonSchema>::json_schema(schema_generator)
    }
}

/// Builder for tool parameter
pub struct ToolParameterBuilder {
    name: String,
    type_name: ToolParameterType,
    description: Option<String>,
    required: Option<bool>,
    schema: Option<serde_json::Value>,
    default: Option<serde_json::Value>,
    enum_values: Option<Vec<String>>,
}

impl ToolParameterBuilder {
    /// Create a new parameter builder with the given name and type
    pub fn new(name: impl Into<String>, type_name: ToolParameterType) -> Self {
        Self {
            name: name.into(),
            type_name,
            description: None,
            required: None,
            schema: None,
            default: None,
            enum_values: None,
        }
    }

    /// Set the description for this parameter
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark this parameter as required or optional
    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    /// Set a custom JSON schema for this parameter
    pub fn schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Set a default value for this parameter
    pub fn default<T: Serialize>(mut self, value: T) -> Result<Self, serde_json::Error> {
        self.default = Some(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set enum values for string parameters
    pub fn enum_values(mut self, values: Vec<impl Into<String>>) -> Self {
        self.enum_values = Some(values.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Build the parameter
    pub fn build(self) -> ToolParameter {
        ToolParameter {
            name: self.name,
            type_name: self.type_name,
            description: self.description,
            required: self.required,
            schema: self.schema,
            default: self.default,
            enum_values: self.enum_values,
        }
    }
}

/// Builder for creating tools
pub struct ToolBuilder {
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
    return_type: Option<String>,
    return_schema: Option<serde_json::Value>,
    is_streaming: Option<bool>,
    is_cancellable: Option<bool>,
    timeout_seconds: Option<u64>,
}

impl ToolBuilder {
    /// Create a new tool builder with the given name and description
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
            return_type: None,
            return_schema: None,
            is_streaming: None,
            is_cancellable: None,
            timeout_seconds: None,
        }
    }

    /// Add a parameter to this tool
    pub fn add_parameter(mut self, parameter: ToolParameter) -> Self {
        self.parameters.push(parameter);
        self
    }

    /// Starts building a parameter with the given name and type.
    /// This returns a tuple of (ToolBuilder, ToolParameterBuilder) to allow for fluent parameter definition
    /// while preserving the tool builder context.
    pub fn parameter(
        self,
        name: impl Into<String>,
        type_name: ToolParameterType,
    ) -> (Self, ToolParameterBuilder) {
        (self, ToolParameterBuilder::new(name, type_name))
    }

    /// Add the parameter built by the parameter builder
    pub fn with_parameter(mut self, parameter_builder: ToolParameterBuilder) -> Self {
        self.parameters.push(parameter_builder.build());
        self
    }

    /// Set the return type for this tool
    pub fn return_type(mut self, return_type: impl Into<String>) -> Self {
        self.return_type = Some(return_type.into());
        self
    }

    /// Set the return schema for this tool
    pub fn return_schema(mut self, schema: serde_json::Value) -> Self {
        self.return_schema = Some(schema);
        self
    }

    /// Set whether the tool streams results
    pub fn streaming(mut self, is_streaming: bool) -> Self {
        self.is_streaming = Some(is_streaming);
        self
    }

    /// Set whether the tool can be cancelled
    pub fn cancellable(mut self, is_cancellable: bool) -> Self {
        self.is_cancellable = Some(is_cancellable);
        self
    }

    /// Set the timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// Build the tool
    pub fn build(self) -> Tool {
        Tool {
            name: self.name,
            description: self.description,
            parameters: if self.parameters.is_empty() {
                None
            } else {
                Some(self.parameters)
            },
            return_type: self.return_type,
            return_schema: self.return_schema,
            is_streaming: self.is_streaming,
            is_cancellable: self.is_cancellable,
            timeout_seconds: self.timeout_seconds,
        }
    }
}

// Helper traits and implementations to make the builder pattern more ergonomic

/// Helper trait for creating tool parameters with a specific type
pub trait IntoToolParameter {
    fn string_param(self, name: impl Into<String>) -> ToolParameterBuilder;
    fn number_param(self, name: impl Into<String>) -> ToolParameterBuilder;
    fn boolean_param(self, name: impl Into<String>) -> ToolParameterBuilder;
    fn object_param(self, name: impl Into<String>) -> ToolParameterBuilder;
    fn array_param(self, name: impl Into<String>) -> ToolParameterBuilder;
}

impl IntoToolParameter for &ToolBuilder {
    fn string_param(self, name: impl Into<String>) -> ToolParameterBuilder {
        ToolParameterBuilder::new(name, ToolParameterType::String)
    }

    fn number_param(self, name: impl Into<String>) -> ToolParameterBuilder {
        ToolParameterBuilder::new(name, ToolParameterType::Number)
    }

    fn boolean_param(self, name: impl Into<String>) -> ToolParameterBuilder {
        ToolParameterBuilder::new(name, ToolParameterType::Boolean)
    }

    fn object_param(self, name: impl Into<String>) -> ToolParameterBuilder {
        ToolParameterBuilder::new(name, ToolParameterType::Object)
    }

    fn array_param(self, name: impl Into<String>) -> ToolParameterBuilder {
        ToolParameterBuilder::new(name, ToolParameterType::Array)
    }
}

// Extension trait for ToolBuilder
pub trait ToolBuilderExt {
    fn add_string_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder);
    fn add_number_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder);
    fn add_boolean_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder);
    fn add_object_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder);
    fn add_array_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder);
}

impl ToolBuilderExt for ToolBuilder {
    fn add_string_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder) {
        let param_builder = ToolParameterBuilder::new(name, ToolParameterType::String);
        (Box::new(self), param_builder)
    }

    fn add_number_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder) {
        let param_builder = ToolParameterBuilder::new(name, ToolParameterType::Number);
        (Box::new(self), param_builder)
    }

    fn add_boolean_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder) {
        let param_builder = ToolParameterBuilder::new(name, ToolParameterType::Boolean);
        (Box::new(self), param_builder)
    }

    fn add_object_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder) {
        let param_builder = ToolParameterBuilder::new(name, ToolParameterType::Object);
        (Box::new(self), param_builder)
    }

    fn add_array_parameter(self, name: impl Into<String>) -> (Box<Self>, ToolParameterBuilder) {
        let param_builder = ToolParameterBuilder::new(name, ToolParameterType::Array);
        (Box::new(self), param_builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_builder() {
        let tool = ToolBuilder::new("calculator", "Performs basic calculations")
            .add_parameter(
                ToolParameterBuilder::new("operation", ToolParameterType::String)
                    .description("Operation to perform")
                    .required(true)
                    .enum_values(vec!["add", "subtract", "multiply", "divide"])
                    .default("add")
                    .unwrap()
                    .build(),
            )
            .add_parameter(
                ToolParameterBuilder::new("a", ToolParameterType::Number)
                    .description("First operand")
                    .required(true)
                    .build(),
            )
            .add_parameter(
                ToolParameterBuilder::new("b", ToolParameterType::Number)
                    .description("Second operand")
                    .required(true)
                    .build(),
            )
            .return_type("object")
            .return_schema(json!({
                "type": "object",
                "properties": {
                    "result": { "type": "number" }
                },
                "required": ["result"]
            }))
            .streaming(false)
            .cancellable(false)
            .timeout(30)
            .build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, "Performs basic calculations");
        assert_eq!(tool.parameters.as_ref().unwrap().len(), 3);
        assert_eq!(tool.is_streaming, Some(false));
        assert_eq!(tool.is_cancellable, Some(false));
        assert_eq!(tool.timeout_seconds, Some(30));
    }

    #[test]
    fn test_tool_builder_with_extension_trait() {
        let (builder_box, param_builder) =
            ToolBuilder::new("calculator", "Performs basic calculations")
                .add_string_parameter("operation");

        let tool = (*builder_box)
            .with_parameter(
                param_builder
                    .description("Operation to perform")
                    .required(true)
                    .enum_values(vec!["add", "subtract", "multiply", "divide"])
                    .default("add")
                    .unwrap(),
            )
            .build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.parameters.as_ref().unwrap().len(), 1);
        assert_eq!(tool.parameters.as_ref().unwrap()[0].name, "operation");
    }
}
