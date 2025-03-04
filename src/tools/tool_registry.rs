use crate::errors::Error;
use crate::tools::process_manager::ToolProcessManager;
use crate::types::tools::{ Tool, CallToolParams, CallToolResult, TextContent };
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{ Mutex, mpsc };

use super::process_manager::ToolOutput;

/// Handler type for in-process tools
type ToolHandler = Arc<dyn (Fn(CallToolParams) -> Result<CallToolResult, Error>) + Send + Sync>;

/// Tool definition variants to properly represent different tool types
pub enum ToolDefinition {
    /// External tool executed as a subprocess
    External {
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
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

    /// Register a tool in the registry
    pub async fn register_tool(
        &self,
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>
    ) -> Result<(), Error> {
        let mut tools = self.tools.lock().await;
        tools.insert(tool.name.clone(), ToolDefinition::External {
            tool,
            command,
            args,
            env,
        });
        Ok(())
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

    /// Register an external tool that executes as a subprocess
    ///
    /// This is a convenience method that can be called directly instead of
    /// going through the Server.
    pub async fn register_external_tool(
        &self,
        tool: Tool,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>
    ) -> Result<(), Error> {
        println!(
            "DEBUG: Registering external tool directly in ToolRegistry: {} with command: {}",
            tool.name,
            command
        );
        self.register_tool(tool, command, args, env).await
    }

    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.lock().await;
        println!(
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
        let tool_name = &params.name;
        let tools = self.tools.lock().await;

        let definition = tools
            .get(tool_name)
            .ok_or_else(|| Error::Tool(format!("Tool not found: {}", tool_name)))?;

        match definition {
            ToolDefinition::InProcess { handler, .. } => {
                // Execute the in-process handler directly
                handler(params.clone())
            }
            ToolDefinition::External { .. } => {
                // For external tools, use the process executor
                drop(tools); // Release the lock before the async operation
                let receiver = self.execute_tool(tool_name, params.arguments).await?;
                self.process_tool_output(receiver).await
            }
        }
    }

    /// Process the output from a tool execution to create a CallToolResult
    async fn process_tool_output(
        &self,
        mut receiver: mpsc::Receiver<ToolOutput>
    ) -> Result<CallToolResult, Error> {
        let mut stdout_content = String::new();
        let mut stderr_content = String::new();

        // Process output messages
        while let Some(output) = receiver.recv().await {
            match output.output_type {
                super::process_manager::ToolOutputType::Stdout => {
                    stdout_content.push_str(&output.content);
                }
                super::process_manager::ToolOutputType::Stderr => {
                    stderr_content.push_str(&output.content);
                }
            }
        }

        // Determine if there's an error based on stderr content
        let is_error = !stderr_content.is_empty();

        // Prepare the result content
        let content = if is_error {
            // Return stderr as error text
            vec![
                serde_json
                    ::to_value(TextContent {
                        content_type: "text".to_string(),
                        text: stderr_content,
                        annotations: None,
                    })
                    .unwrap()
            ]
        } else {
            // Try to parse stdout as JSON, fall back to text if it fails
            match serde_json::from_str::<serde_json::Value>(&stdout_content) {
                Ok(json_value) => { vec![json_value] }
                Err(_) => {
                    vec![
                        serde_json
                            ::to_value(TextContent {
                                content_type: "text".to_string(),
                                text: stdout_content,
                                annotations: None,
                            })
                            .unwrap()
                    ]
                }
            }
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
    ) -> Result<mpsc::Receiver<ToolOutput>, Error> {
        let tools = self.tools.lock().await;
        let definition = tools
            .get(tool_name)
            .ok_or_else(|| Error::Tool(format!("Tool not found: {}", tool_name)))?;

        // Handle based on tool type
        match definition {
            ToolDefinition::InProcess { .. } => {
                return Err(
                    Error::Tool(
                        format!("Tool '{}' is an in-process tool and should be executed with execute_tool_with_params", tool_name)
                    )
                );
            }
            ToolDefinition::External { command, args, env, .. } => {
                // Prepare parameters as environment variables
                let mut tool_env = env.clone();
                for (key, value) in &parameters {
                    tool_env.insert(format!("PARAM_{}", key.to_uppercase()), value.to_string());
                }

                // Also pass parameters as JSON
                tool_env.insert(
                    "TOOL_PARAMETERS".to_string(),
                    serde_json::to_string(&parameters).map_err(|e| Error::Json(e))?
                );

                // Convert args to &str array
                let args_ref: Vec<&str> = args
                    .iter()
                    .map(|s| s.as_str())
                    .collect();

                // Execute the tool
                let tool_id = format!("{}_{}", tool_name, uuid::Uuid::new_v4());
                self.process_manager.spawn_process(&tool_id, command, &args_ref, tool_env).await
            }
        }
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
    use crate::types::tools::{ Tool, ToolBuilder, ToolParameterType };

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
                    content: vec![serde_json::json!({ "result": result })],
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

        let result_value = result.content[0]
            .get("result")
            .and_then(|v| v.as_f64())
            .unwrap();
        assert_eq!(result_value, 5.0);
    }
}
