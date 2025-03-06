use crate::errors::Error;
use crate::server::tools::process_manager::ToolProcessManager;
use crate::types::protocol::Role;
use crate::types::tools::{
    Tool,
    CallToolParams,
    CallToolResult,
    TextContent,
    Content,
    ContentAnnotations,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{ Mutex, mpsc };
use tracing::info;

use super::process_manager::ToolOutput;

/// Handler type for in-process tools
type ToolHandler = Arc<dyn (Fn(CallToolParams) -> Result<CallToolResult, Error>) + Send + Sync>;

/// External tool configuration, including optional annotations
pub struct ExternalToolConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Optional annotations to apply to the output
    pub annotations: Option<ContentAnnotations>,
}

impl Default for ExternalToolConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            annotations: None,
        }
    }
}

/// Tool definition variants to properly represent different tool types
pub enum ToolDefinition {
    /// External tool executed as a subprocess
    External {
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        annotations: Option<ContentAnnotations>,
    },
    /// In-process tool executed directly
    InProcess {
        tool: Tool,
        handler: ToolHandler,
    },
}

impl ToolDefinition {
    /// Get the tool metadata regardless of type
    pub fn get_tool(&self) -> &Tool {
        match self {
            ToolDefinition::External { tool, .. } => tool,
            ToolDefinition::InProcess { tool, .. } => tool,
        }
    }
}

pub struct ToolRegistry {
    tools: Arc<Mutex<HashMap<String, ToolDefinition>>>,
    process_manager: Arc<ToolProcessManager>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(HashMap::new())),
            process_manager: Arc::new(ToolProcessManager::new()),
        }
    }

    /// Register an external tool with the registry
    pub async fn register_external_tool(
        &self,
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        annotations: Option<ContentAnnotations>
    ) -> Result<(), Error> {
        let name = tool.name.clone();
        let mut tools = self.tools.lock().await;

        // Prevent duplicate tools
        if tools.contains_key(&name) {
            return Err(Error::Tool(format!("Tool already exists: {}", name)));
        }

        tools.insert(name, ToolDefinition::External {
            tool,
            command,
            args,
            env,
            annotations,
        });
        Ok(())
    }

    /// Register an external tool with the registry using a config object
    pub async fn register_external_tool_with_config(
        &self,
        tool: Tool,
        config: ExternalToolConfig
    ) -> Result<(), Error> {
        self.register_external_tool(
            tool,
            config.command,
            config.args,
            config.env,
            config.annotations
        ).await
    }

    /// Register an in-process tool with a function handler
    pub async fn register_in_process_tool(
        &self,
        tool: Tool,
        handler: impl (Fn(CallToolParams) -> Result<CallToolResult, Error>) + Send + Sync + 'static
    ) -> Result<(), Error> {
        let mut tools = self.tools.lock().await;

        // Store the tool definition with its handler
        tools.insert(tool.name.clone(), ToolDefinition::InProcess {
            tool,
            handler: Arc::new(handler),
        });

        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.lock().await;
        info!(
            "LISTING TOOLS CALLED: Available tools in registry: {}",
            tools.keys().cloned().collect::<Vec<String>>().join(", ")
        );
        tools
            .values()
            .map(|def| def.get_tool().clone())
            .collect()
    }

    /// Execute a tool using CallToolParams and return a CallToolResult
    pub async fn execute_tool_with_params(
        &self,
        params: CallToolParams
    ) -> Result<CallToolResult, Error> {
        let tools = self.tools.lock().await;
        let definition = tools
            .get(&params.name)
            .ok_or_else(|| Error::Tool(format!("Tool not found: {}", params.name)))?;

        // Handle based on tool type
        match definition {
            ToolDefinition::InProcess { handler, .. } => {
                // Call the handler function directly
                handler(params.clone())
            }
            ToolDefinition::External { command, args, env, annotations, .. } => {
                // Prepare parameters as environment variables
                let mut tool_env = env.clone();
                for (key, value) in &params.arguments {
                    tool_env.insert(format!("PARAM_{}", key.to_uppercase()), value.to_string());
                }

                // Also pass parameters as JSON
                tool_env.insert(
                    "TOOL_PARAMETERS".to_string(),
                    serde_json::to_string(&params.arguments)?
                );

                // Convert args to &str array
                let args_ref: Vec<&str> = args
                    .iter()
                    .map(|s| s.as_str())
                    .collect();

                // Execute the tool
                let tool_id = format!("{}_{}", params.name, uuid::Uuid::new_v4());
                let receiver = self.process_manager.spawn_process(
                    &tool_id,
                    command,
                    &args_ref,
                    tool_env
                ).await?;

                // Process the output with any provided annotations
                self.process_tool_output(receiver, annotations.clone()).await
            }
        }
    }

    /// Process the output from a tool execution to create a CallToolResult
    async fn process_tool_output(
        &self,
        mut receiver: mpsc::Receiver<ToolOutput>,
        tool_annotations: Option<ContentAnnotations>
    ) -> Result<CallToolResult, Error> {
        let mut stdout_content = String::new();
        let mut stderr_content = String::new();
        let mut is_error = false;

        while let Some(output) = receiver.recv().await {
            match output.output_type {
                super::process_manager::ToolOutputType::Stdout => {
                    stdout_content.push_str(&output.content);
                }
                super::process_manager::ToolOutputType::Stderr => {
                    stderr_content.push_str(&output.content);
                    is_error = true;
                }
            }
        }

        // If we have stderr content and no stdout content, use stderr as stdout
        if stdout_content.is_empty() && !stderr_content.is_empty() {
            stdout_content = stderr_content;
            stderr_content = String::new();
        }

        // Use provided annotations or create default ones for external tools
        let annotations = tool_annotations.unwrap_or_else(|| {
            ContentAnnotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.8),
            }
        });

        let content = if !stdout_content.is_empty() {
            // Try to parse stdout as JSON, fall back to text if it fails
            match serde_json::from_str::<serde_json::Value>(&stdout_content) {
                Ok(json_value) => {
                    // Try to convert JSON to Content
                    if
                        let Ok(content_value) = serde_json::from_value::<Content>(
                            json_value.clone()
                        )
                    {
                        // If it's already a valid Content type, use it
                        vec![content_value]
                    } else if json_value.is_object() && json_value.get("type").is_some() {
                        // If it has a "type" field but isn't a valid Content, wrap it as Text
                        vec![
                            Content::Text(TextContent {
                                content_type: "text".to_string(),
                                text: json_value.to_string(),
                                annotations: Some(annotations.clone()),
                            })
                        ]
                    } else {
                        // For other JSON, convert it to a string and use as text
                        vec![
                            Content::Text(TextContent {
                                content_type: "text".to_string(),
                                text: json_value.to_string(),
                                annotations: Some(annotations.clone()),
                            })
                        ]
                    }
                }
                Err(_) => {
                    // For non-JSON, use as plain text
                    vec![
                        Content::Text(TextContent {
                            content_type: "text".to_string(),
                            text: stdout_content,
                            annotations: Some(annotations.clone()),
                        })
                    ]
                }
            }
        } else if !stderr_content.is_empty() {
            // Use stderr content as text
            vec![
                Content::Text(TextContent {
                    content_type: "text".to_string(),
                    text: stderr_content,
                    annotations: Some(annotations),
                })
            ]
        } else {
            // Default empty content
            vec![
                Content::Text(TextContent {
                    content_type: "text".to_string(),
                    text: String::new(),
                    annotations: None,
                })
            ]
        };

        Ok(CallToolResult {
            content,
            is_error,
        })
    }

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: HashMap<String, serde_json::Value>
    ) -> Result<CallToolResult, Error> {
        // Convert parameters to CallToolParams
        let params = CallToolParams {
            name: tool_name.to_string(),
            arguments: parameters,
        };

        // Use the common execute_tool_with_params method
        self.execute_tool_with_params(params).await
    }

    /// Check if a tool exists in the registry
    pub async fn has_tool(&self, tool_name: &str) -> bool {
        let tools = self.tools.lock().await;
        tools.contains_key(tool_name)
    }

    /// Get a tool definition by name
    pub async fn get_tool(&self, tool_name: &str) -> Option<Tool> {
        let tools = self.tools.lock().await;
        tools.get(tool_name).map(|def| def.get_tool().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tools::{ ToolBuilder, ToolParameterType };

    #[tokio::test]
    async fn test_in_process_tool() {
        // Create a simple calculator tool
        let calc_tool = ToolBuilder::new("calculator", "Performs basic arithmetic")
            .add_parameter(
                crate::types::tools::ToolParameterBuilder
                    ::new("a", ToolParameterType::Number)
                    .required(true)
                    .build()
            )
            .add_parameter(
                crate::types::tools::ToolParameterBuilder
                    ::new("b", ToolParameterType::Number)
                    .required(true)
                    .build()
            )
            .add_parameter(
                crate::types::tools::ToolParameterBuilder
                    ::new("operation", ToolParameterType::String)
                    .enum_values(vec!["add", "subtract", "multiply", "divide"])
                    .required(true)
                    .build()
            )
            .build();

        // Create a registry
        let registry = ToolRegistry::new();

        // Register the calculator tool
        registry
            .register_in_process_tool(calc_tool, |params: CallToolParams| {
                let a = params.arguments
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| Error::InvalidParams("Missing parameter 'a'".to_string()))?;

                let b = params.arguments
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| Error::InvalidParams("Missing parameter 'b'".to_string()))?;

                let operation = params.arguments
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .ok_or_else(||
                        Error::InvalidParams("Missing parameter 'operation'".to_string())
                    )?;

                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b == 0.0 {
                            return Err(Error::InvalidParams("Cannot divide by zero".to_string()));
                        }
                        a / b
                    }
                    _ => {
                        return Err(
                            Error::InvalidParams(format!("Unknown operation: {}", operation))
                        );
                    }
                };

                Ok(CallToolResult {
                    content: vec![
                        Content::Text(TextContent {
                            content_type: "text".to_string(),
                            text: serde_json::json!({ "result": result }).to_string(),
                            annotations: None,
                        })
                    ],
                    is_error: false,
                })
            }).await
            .unwrap();

        // Execute the tool
        let params = CallToolParams {
            name: "calculator".to_string(),
            arguments: [
                ("a".to_string(), serde_json::json!(2)),
                ("b".to_string(), serde_json::json!(3)),
                ("operation".to_string(), serde_json::json!("add")),
            ]
                .into_iter()
                .collect(),
        };

        let result = registry.execute_tool_with_params(params).await.unwrap();
        assert!(!result.is_error);

        // Extract the result value from the content
        let content_text = match &result.content[0] {
            Content::Text(text_content) => &text_content.text,
            _ => panic!("Expected Text content"),
        };

        // Parse the JSON from the text string
        let json_value: serde_json::Value = serde_json::from_str(content_text).expect("Valid JSON");
        let result_value = json_value
            .get("result")
            .and_then(|v| v.as_f64())
            .unwrap();
        assert_eq!(result_value, 5.0);
    }
}
