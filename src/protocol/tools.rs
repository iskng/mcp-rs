use crate::protocol::{
    Annotations, CallToolParams, CallToolResult, Content, ImageContent, TextContent, Tool,
    ToolInputSchema,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Helper to create a tool
pub fn create_tool(
    name: &str,
    description: Option<&str>,
    properties: Option<HashMap<String, HashMap<String, Value>>>,
    required: Option<Vec<String>>,
) -> Tool {
    Tool {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        input_schema: ToolInputSchema {
            type_field: "object".to_string(),
            properties,
            required,
        },
    }
}

/// Helper to create a tool input schema
pub fn create_tool_input_schema(
    properties: Option<HashMap<String, HashMap<String, Value>>>,
    required: Option<Vec<String>>,
) -> ToolInputSchema {
    ToolInputSchema {
        type_field: "object".to_string(),
        properties,
        required,
    }
}

/// Helper to create a parameter schema for a string
pub fn create_string_param_schema(description: &str) -> HashMap<String, Value> {
    let mut schema = HashMap::new();
    schema.insert("type".to_string(), Value::String("string".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    schema
}

/// Helper to create a parameter schema for a number
pub fn create_number_param_schema(description: &str) -> HashMap<String, Value> {
    let mut schema = HashMap::new();
    schema.insert("type".to_string(), Value::String("number".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    schema
}

/// Helper to create a parameter schema for a boolean
pub fn create_boolean_param_schema(description: &str) -> HashMap<String, Value> {
    let mut schema = HashMap::new();
    schema.insert("type".to_string(), Value::String("boolean".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    schema
}

/// Helper to create a parameter schema for an array
pub fn create_array_param_schema(description: &str, items_type: &str) -> HashMap<String, Value> {
    let mut schema = HashMap::new();
    schema.insert("type".to_string(), Value::String("array".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );

    let mut items = serde_json::Map::new();
    items.insert("type".to_string(), Value::String(items_type.to_string()));

    schema.insert("items".to_string(), Value::Object(items));
    schema
}

/// Helper to create a parameter schema for an enum
pub fn create_enum_param_schema(description: &str, values: &[&str]) -> HashMap<String, Value> {
    let mut schema = HashMap::new();
    schema.insert("type".to_string(), Value::String("string".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );

    let values: Vec<Value> = values
        .iter()
        .map(|s| Value::String(s.to_string()))
        .collect();
    schema.insert("enum".to_string(), Value::Array(values));

    schema
}

/// Helper to create call tool params
pub fn create_call_tool_params(
    name: &str,
    arguments: Option<HashMap<String, Value>>,
) -> CallToolParams {
    CallToolParams {
        name: name.to_string(),
        arguments,
    }
}

/// Helper to create call tool result
pub fn create_call_tool_result(content: Vec<Content>, is_error: bool) -> CallToolResult {
    CallToolResult {
        content,
        is_error: if is_error { Some(true) } else { None },
        _meta: None,
    }
}

/// Helper to create a successful text result
pub fn create_text_tool_result(text: &str) -> CallToolResult {
    CallToolResult {
        content: vec![Content::Text(TextContent {
            type_field: "text".to_string(),
            text: text.to_string(),
            annotations: None,
        })],
        is_error: None,
        _meta: None,
    }
}

/// Helper to create an error text result
pub fn create_error_tool_result(error_message: &str) -> CallToolResult {
    CallToolResult {
        content: vec![Content::Text(TextContent {
            type_field: "text".to_string(),
            text: error_message.to_string(),
            annotations: None,
        })],
        is_error: Some(true),
        _meta: None,
    }
}

/// Builder for creating CallToolResult objects
pub struct CallToolResultBuilder {
    content: Vec<Content>,
    is_error: bool,
}

impl CallToolResultBuilder {
    /// Create a new empty result builder
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            is_error: false,
        }
    }

    /// Add text content to the result
    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.content.push(Content::Text(TextContent {
            type_field: "text".to_string(),
            text: text.into(),
            annotations: None,
        }));
        self
    }

    /// Add text content with annotations to the result
    pub fn add_text_with_annotations(
        mut self,
        text: impl Into<String>,
        annotations: Annotations,
    ) -> Self {
        self.content.push(Content::Text(TextContent {
            type_field: "text".to_string(),
            text: text.into(),
            annotations: Some(annotations),
        }));
        self
    }

    /// Add image content to the result
    pub fn add_image(mut self, data: &[u8], mime_type: impl Into<String>) -> Self {
        self.content.push(Content::Image(ImageContent {
            type_field: "image".to_string(),
            data: BASE64_STANDARD.encode(data),
            mime_type: mime_type.into(),
            annotations: None,
        }));
        self
    }

    /// Add image content with annotations to the result
    pub fn add_image_with_annotations(
        mut self,
        data: &[u8],
        mime_type: impl Into<String>,
        annotations: Annotations,
    ) -> Self {
        self.content.push(Content::Image(ImageContent {
            type_field: "image".to_string(),
            data: BASE64_STANDARD.encode(data),
            mime_type: mime_type.into(),
            annotations: Some(annotations),
        }));
        self
    }

    /// Mark the result as an error
    pub fn with_error(mut self) -> Self {
        self.is_error = true;
        self
    }

    /// Build the final CallToolResult
    pub fn build(self) -> CallToolResult {
        CallToolResult {
            content: self.content,
            is_error: if self.is_error { Some(true) } else { None },
            _meta: None,
        }
    }
}

impl Default for CallToolResultBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// ToolBuilder for fluent creation of Tool objects
pub struct ToolBuilder {
    name: String,
    description: Option<String>,
    properties: HashMap<String, HashMap<String, Value>>,
    required: Vec<String>,
}

impl ToolBuilder {
    /// Create a new tool builder
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Add a string parameter to the tool
    pub fn add_string_parameter(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name_str = name.into();
        let schema = create_string_param_schema(&description.into());

        self.properties.insert(name_str.clone(), schema);
        if required {
            self.required.push(name_str);
        }
        self
    }

    /// Add a number parameter to the tool
    pub fn add_number_parameter(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name_str = name.into();
        let schema = create_number_param_schema(&description.into());

        self.properties.insert(name_str.clone(), schema);
        if required {
            self.required.push(name_str);
        }
        self
    }

    /// Add a boolean parameter to the tool
    pub fn add_boolean_parameter(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name_str = name.into();
        let schema = create_boolean_param_schema(&description.into());

        self.properties.insert(name_str.clone(), schema);
        if required {
            self.required.push(name_str);
        }
        self
    }

    /// Add an array parameter to the tool
    pub fn add_array_parameter(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        items_type: impl Into<String>,
        required: bool,
    ) -> Self {
        let name_str = name.into();
        let schema = create_array_param_schema(&description.into(), &items_type.into());

        self.properties.insert(name_str.clone(), schema);
        if required {
            self.required.push(name_str);
        }
        self
    }

    /// Add an enum parameter to the tool
    pub fn add_enum_parameter(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        values: &[&str],
        required: bool,
    ) -> Self {
        let name_str = name.into();
        let schema = create_enum_param_schema(&description.into(), values);

        self.properties.insert(name_str.clone(), schema);
        if required {
            self.required.push(name_str);
        }
        self
    }

    /// Add a custom parameter schema to the tool
    pub fn add_parameter_schema(
        mut self,
        name: impl Into<String>,
        schema: HashMap<String, Value>,
        required: bool,
    ) -> Self {
        let name_str = name.into();
        self.properties.insert(name_str.clone(), schema);
        if required {
            self.required.push(name_str);
        }
        self
    }

    /// Build the final Tool object
    pub fn build(self) -> Tool {
        Tool {
            name: self.name,
            description: self.description,
            input_schema: ToolInputSchema {
                type_field: "object".to_string(),
                properties: if self.properties.is_empty() {
                    None
                } else {
                    Some(self.properties)
                },
                required: if self.required.is_empty() {
                    None
                } else {
                    Some(self.required)
                },
            },
        }
    }
}

/// Helper to create annotations with audience
pub fn create_audience_annotations(roles: Vec<crate::protocol::Role>) -> Annotations {
    Annotations {
        audience: Some(roles),
        priority: None,
    }
}

/// Helper to create annotations with priority
pub fn create_priority_annotations(priority: f64) -> Annotations {
    Annotations {
        audience: None,
        priority: Some(priority),
    }
}

/// Helper to create annotations with both audience and priority
pub fn create_annotations(
    audience: Option<Vec<crate::protocol::Role>>,
    priority: Option<f64>,
) -> Annotations {
    Annotations { audience, priority }
}

/// Extension functions for CallToolResult
impl CallToolResult {
    /// Create a new builder for constructing a CallToolResult
    pub fn builder() -> CallToolResultBuilder {
        CallToolResultBuilder::new()
    }

    /// Create a simple text result
    pub fn text(text: impl Into<String>) -> Self {
        create_text_tool_result(&text.into())
    }

    /// Create a simple error result
    pub fn error(error_message: impl Into<String>) -> Self {
        create_error_tool_result(&error_message.into())
    }
}

/// Implemented on structs that can be converted to a Tool
pub trait ToToolSchema {
    /// Convert this struct into a Tool schema
    fn to_tool_schema(&self) -> Tool;
}

/// Helper struct for serializing tool parameters together
#[derive(Debug, Clone)]
pub struct ToolParameters {
    pub parameters: HashMap<String, HashMap<String, Value>>,
    pub required: Vec<String>,
}

impl ToolParameters {
    /// Create a new empty parameters collection
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Add a string parameter
    pub fn add_string(&mut self, name: &str, description: &str, required: bool) -> &mut Self {
        let schema = create_string_param_schema(description);
        self.parameters.insert(name.to_string(), schema);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a number parameter
    pub fn add_number(&mut self, name: &str, description: &str, required: bool) -> &mut Self {
        let schema = create_number_param_schema(description);
        self.parameters.insert(name.to_string(), schema);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a boolean parameter
    pub fn add_boolean(&mut self, name: &str, description: &str, required: bool) -> &mut Self {
        let schema = create_boolean_param_schema(description);
        self.parameters.insert(name.to_string(), schema);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add an array parameter
    pub fn add_array(
        &mut self,
        name: &str,
        description: &str,
        items_type: &str,
        required: bool,
    ) -> &mut Self {
        let schema = create_array_param_schema(description, items_type);
        self.parameters.insert(name.to_string(), schema);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add an enum parameter
    pub fn add_enum(
        &mut self,
        name: &str,
        description: &str,
        values: &[&str],
        required: bool,
    ) -> &mut Self {
        let schema = create_enum_param_schema(description, values);
        self.parameters.insert(name.to_string(), schema);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Build a Tool from these parameters
    pub fn to_tool(&self, name: &str, description: &str) -> Tool {
        create_tool(
            name,
            Some(description),
            if self.parameters.is_empty() {
                None
            } else {
                Some(self.parameters.clone())
            },
            if self.required.is_empty() {
                None
            } else {
                Some(self.required.clone())
            },
        )
    }
}

impl Default for ToolParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for creating tool arguments
pub struct ToolArgumentsBuilder {
    arguments: HashMap<String, Value>,
}

impl ToolArgumentsBuilder {
    /// Create a new empty arguments builder
    pub fn new() -> Self {
        Self {
            arguments: HashMap::new(),
        }
    }

    /// Add a string argument
    pub fn add_string(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments
            .insert(name.into(), Value::String(value.into()));
        self
    }

    /// Add a number argument
    pub fn add_number(mut self, name: impl Into<String>, value: f64) -> Self {
        self.arguments.insert(
            name.into(),
            Value::Number(
                serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        self
    }

    /// Add a boolean argument
    pub fn add_boolean(mut self, name: impl Into<String>, value: bool) -> Self {
        self.arguments.insert(name.into(), Value::Bool(value));
        self
    }

    /// Add a null argument
    pub fn add_null(mut self, name: impl Into<String>) -> Self {
        self.arguments.insert(name.into(), Value::Null);
        self
    }

    /// Add a serializable argument
    pub fn add<T: Serialize>(
        mut self,
        name: impl Into<String>,
        value: &T,
    ) -> Result<Self, serde_json::Error> {
        let json_value = serde_json::to_value(value)?;
        self.arguments.insert(name.into(), json_value);
        Ok(self)
    }

    /// Build the final arguments map
    pub fn build(self) -> HashMap<String, Value> {
        self.arguments
    }

    /// Create tool call parameters
    pub fn to_call_params(self, tool_name: impl Into<String>) -> CallToolParams {
        CallToolParams {
            name: tool_name.into(),
            arguments: Some(self.arguments),
        }
    }
}

impl Default for ToolArgumentsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
