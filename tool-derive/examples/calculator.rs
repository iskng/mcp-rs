//! Example server with transport and message handlers
//!
//! This example shows how to set up an MCP server with handlers
//! for different message types, such as initialization and requests.

use mcp_rs::protocol::Error;
use mcp_rs::protocol::tools::ToToolSchema;
use mcp_rs::protocol::{ CallToolParams, CallToolResult, PromptMessage };
use mcp_rs::server::Server;
use mcp_rs::server::services::prompts::{
    Prompt,
    PromptArgument,
    create_assistant_text_message,
    create_user_text_message,
};
use mcp_rs::server::transport::sse::{ SseServerOptions, SseServerTransport };
use mcp_rs::prompt;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::Arc;
use tool_derive::Tool;
use tracing::Level;
use async_trait::async_trait;
use serde_json::Value;

// Define an enum for the calculator operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

// Define a Calculator struct using our derive macro
#[derive(Tool, Debug, Clone, Serialize, Deserialize)]
#[tool(description = "Performs basic arithmetic on US EAST server")]
pub struct Calculator {
    #[param(description = "First operand", required = true)]
    pub a: f64,

    #[param(description = "Second operand", required = true)]
    pub b: f64,

    #[param(
        description = "Operation to perform",
        required = true,
        enum_values = "Add,Subtract,Multiply,Divide"
    )]
    pub operation: Operation,
}

// Add a Calculator help prompt struct
struct CalculatorHelpPrompt;

#[async_trait]
impl Prompt for CalculatorHelpPrompt {
    async fn render(
        &self,
        _arguments: Option<HashMap<String, Value>>
    ) -> Result<Vec<PromptMessage>, Error> {
        Ok(
            vec![
                create_user_text_message("How do I use the calculator?"),
                create_assistant_text_message(
                    "The calculator tool allows you to perform basic arithmetic operations. \
                You can use it by calling the 'calculator' tool with the following parameters:\n\
                - a: First operand (required)\n\
                - b: Second operand (required)\n\
                - operation: One of 'Add', 'Subtract', 'Multiply', or 'Divide' (required)\n\n\
                For example, to add 5 and 3, you would set a=5, b=3, and operation=Add."
                )
            ]
        )
    }

    fn get_name(&self) -> &str {
        "calculator-help"
    }

    fn get_description(&self) -> Option<&str> {
        Some("Provides help information about the calculator tool")
    }

    fn get_arguments(&self) -> Vec<PromptArgument> {
        vec![] // No arguments needed
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Use tracing for better logging
    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();
    println!("---> Starting MCP server");

    // Create a simple SSE transport
    let transport = SseServerTransport::new(SseServerOptions {
        bind_address: "127.0.0.1:8090".to_string(),
        ..Default::default()
    });

    // Create a server with a calculator tool
    let server_result = Server::builder()
        .with_server_name("MCP-RS Example Server")
        .with_server_version("0.1.0")
        .with_instructions(
            "This is an example server that demonstrates the Tool derive macro and Prompt Manager."
        )
        .register_in_process_tool(
            // Use the derived to_tool_schema() method to generate a Tool from our Calculator struct
            (Calculator {
                a: 0.0, // Default values
                b: 0.0,
                operation: Operation::Add,
            }).to_tool_schema(),
            |params: CallToolParams| {
                // Deserialize directly into our type-safe struct
                let typed_params: Calculator = serde_json
                    ::from_value(serde_json::to_value(params.arguments).unwrap())
                    .map_err(|e| Error::InvalidParams(format!("Invalid parameters: {}", e)))?;

                // Process based on operation enum
                let result_value = match typed_params.operation {
                    Operation::Add => typed_params.a + typed_params.b,
                    Operation::Subtract => typed_params.a - typed_params.b,
                    Operation::Multiply => typed_params.a * typed_params.b,
                    Operation::Divide => {
                        if typed_params.b == 0.0 {
                            return Err(Error::InvalidParams("Cannot divide by zero".to_string()));
                        }
                        typed_params.a / typed_params.b
                    }
                };

                // Create a result using the type-safe builder
                let operation_text = format!("{:?}", typed_params.operation).to_lowercase();
                let result_text = format!(
                    "The result of {} {} {} is {}",
                    typed_params.a,
                    operation_text,
                    typed_params.b,
                    result_value
                );

                // Return a text result
                Ok(CallToolResult::text(result_text))
            }
        )
        // Register the calculator help prompt
        .register_prompt(Arc::new(CalculatorHelpPrompt))
        // Register a welcome prompt using the macro
        .register_prompt(
            prompt!(
                "welcome",
                "A welcome message for users of the calculator server",
                vec![PromptArgument {
                    name: "name".to_string(),
                    description: Some("The name of the user".to_string()),
                    required: false,
                }],
                |arguments: Option<HashMap<String, Value>>| async move {
                    let name = match &arguments {
                        Some(args) => {
                            args.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("friend")
                        }
                        None => "friend",
                    };

                    Ok(
                        vec![
                            create_user_text_message("Hello!"),
                            create_assistant_text_message(
                                &format!("Welcome to the Calculator Server, {}! This server provides a calculator tool and prompt templates. \
                            Try the 'calculator-help' prompt to learn how to use the calculator.", name)
                            )
                        ]
                    )
                }
            )
        )
        // Register a simple examples prompt
        .register_prompt(
            prompt!(
                "examples",
                "Shows examples of using the calculator",
                vec![],
                |_: Option<HashMap<String, Value>>| async move {
                    Ok(
                        vec![
                            create_user_text_message("Can you show me some calculator examples?"),
                            create_assistant_text_message(
                                "Here are some examples of using the calculator tool:\n\n\
                         1. Addition: a=10, b=5, operation=Add → Result: 15\n\
                         2. Subtraction: a=10, b=5, operation=Subtract → Result: 5\n\
                         3. Multiplication: a=10, b=5, operation=Multiply → Result: 50\n\
                         4. Division: a=10, b=5, operation=Divide → Result: 2"
                            )
                        ]
                    )
                }
            )
        )
        .with_transport(transport)
        .build().await;

    match server_result {
        Ok(mut server) => {
            // Create and register another prompt after server creation
            server.register_prompt(
                prompt!(
                    "about",
                    "Information about this server",
                    vec![],
                    |_: Option<HashMap<String, Value>>| async move {
                        Ok(
                            vec![
                                create_user_text_message("What is this server about?"),
                                create_assistant_text_message(
                                    "This is a demonstration server for the MCP-RS library. It showcases:\n\n\
                             1. The Tool derive macro for easily creating tools\n\
                             2. The Prompt Manager for managing prompt templates\n\
                             3. The SSE transport for communication\n\n\
                             Feel free to explore the available tools and prompts!"
                                )
                            ]
                        )
                    }
                )
            ).await?;

            // Start the server
            if let Err(e) = server.start().await {
                eprintln!("Error starting server: {}", e);
                return Ok(());
            }

            println!("Server started on http://127.0.0.1:8090");
            println!("Available prompts: calculator-help, welcome, examples, about");
            println!("Available tools: calculator");

            // Wait for Ctrl+C
            println!("Press Ctrl+C to stop the server");
            tokio::signal::ctrl_c().await?;

            // Shutdown server
            server.shutdown().await;
            println!("Server shut down");
        }
        Err(e) => {
            eprintln!("Error building server: {}", e);
        }
    }

    Ok(())
}
