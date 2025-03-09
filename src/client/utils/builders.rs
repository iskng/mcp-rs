//! MCP Client Builder Utilities
//!
//! This module provides builder pattern implementations for MCP protocol types.

use serde_json::Value;
use std::collections::HashMap;

use crate::protocol::{
    CallToolParams,
    CompleteParams,
    CompleteArgument,
    GetPromptParams,
    Message,
    PromptMessage,
    ReadResourceParams,
    Reference,
    Role,
    TextContent,
};

/// Builder for CallToolParams
pub struct CallToolParamsBuilder {
    name: String,
    arguments: Option<HashMap<String, Value>>,
}

impl CallToolParamsBuilder {
    /// Create a new builder for CallToolParams
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    /// Set the arguments object
    pub fn with_args(mut self, args: HashMap<String, Value>) -> Self {
        self.arguments = Some(args);
        self
    }

    /// Add a string argument
    pub fn with_string_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let args = self.arguments.get_or_insert_with(HashMap::new);
        args.insert(key.into(), Value::String(value.into()));
        self
    }

    /// Add a number argument
    pub fn with_number_arg(mut self, key: impl Into<String>, value: f64) -> Self {
        let args = self.arguments.get_or_insert_with(HashMap::new);

        // Use from_f64 which handles the conversion properly
        if let Some(num) = serde_json::Number::from_f64(value) {
            args.insert(key.into(), Value::Number(num));
        } else {
            // Handle NaN or infinity values by using null
            args.insert(key.into(), Value::Null);
        }

        self
    }

    /// Add a boolean argument
    pub fn with_bool_arg(mut self, key: impl Into<String>, value: bool) -> Self {
        let args = self.arguments.get_or_insert_with(HashMap::new);
        args.insert(key.into(), Value::Bool(value));
        self
    }

    /// Build the parameters
    pub fn build(self) -> CallToolParams {
        CallToolParams {
            name: self.name,
            arguments: self.arguments,
        }
    }
}

/// Builder for GetPromptParams
pub struct GetPromptParamsBuilder {
    name: String,
    arguments: Option<HashMap<String, String>>,
}

impl GetPromptParamsBuilder {
    /// Create a new builder for GetPromptParams
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    /// Set all arguments at once
    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.arguments = Some(arguments);
        self
    }

    /// Add a string argument
    pub fn with_string_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let args = self.arguments.get_or_insert_with(HashMap::new);
        args.insert(key.into(), value.into());
        self
    }

    /// Build the parameters
    pub fn build(self) -> GetPromptParams {
        GetPromptParams {
            name: self.name,
            arguments: self.arguments,
        }
    }
}

/// Builder for ReadResourceParams
pub struct ReadResourceParamsBuilder {
    uri: String,
}

impl ReadResourceParamsBuilder {
    /// Create a new builder for ReadResourceParams
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
        }
    }

    /// Build the parameters
    pub fn build(self) -> ReadResourceParams {
        ReadResourceParams {
            uri: self.uri,
        }
    }
}

/// Builder for CompleteParams
pub struct CompleteParamsBuilder {
    argument_name: String,
    argument_value: String,
    reference_type: String,
    reference_value: String,
}

impl CompleteParamsBuilder {
    /// Create a new builder for CompleteParams
    pub fn new(argument_name: impl Into<String>) -> Self {
        Self {
            argument_name: argument_name.into(),
            argument_value: String::new(),
            reference_type: "ref/prompt".to_string(), // Default to prompt reference
            reference_value: String::new(),
        }
    }

    /// Set the argument value
    pub fn with_argument_value(mut self, value: impl Into<String>) -> Self {
        self.argument_value = value.into();
        self
    }

    /// Set a prompt reference
    pub fn with_prompt_reference(mut self, name: impl Into<String>) -> Self {
        self.reference_type = "ref/prompt".to_string();
        self.reference_value = name.into();
        self
    }

    /// Set a resource reference
    pub fn with_resource_reference(mut self, uri: impl Into<String>) -> Self {
        self.reference_type = "ref/resource".to_string();
        self.reference_value = uri.into();
        self
    }

    /// Build the parameters
    pub fn build(self) -> CompleteParams {
        let argument = CompleteArgument {
            name: self.argument_name,
            value: self.argument_value,
        };

        let reference = match self.reference_type.as_str() {
            "ref/prompt" =>
                Reference::Prompt(
                    serde_json
                        ::from_value(
                            serde_json::json!({
                "type": "ref/prompt",
                "name": self.reference_value
            })
                        )
                        .unwrap()
                ),
            "ref/resource" =>
                Reference::Resource(
                    serde_json
                        ::from_value(
                            serde_json::json!({
                "type": "ref/resource",
                "uri": self.reference_value
            })
                        )
                        .unwrap()
                ),
            _ => panic!("Unknown reference type"),
        };

        CompleteParams {
            argument,
            ref_: reference,
        }
    }
}
