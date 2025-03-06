//! Example server with transport and message handlers
//!
//! This example shows how to set up an MCP server with handlers
//! for different message types, such as initialization and requests.

use mcp_rs::errors::Error;
use mcp_rs::server::Server;
use mcp_rs::server::tools::tool_registry::ToolRegistry;
use mcp_rs::transport::TransportType;
use mcp_rs::types::tools::{
    ToolBuilder,
    ToolParameterBuilder,
    ToolParameterType,
    CallToolParams,
    CallToolResult,
};

use std::sync::Arc;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Use tracing for better logging
    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();
    println!("---> Starting MCP server");

    // 1. Create server configuration
    let config = mcp_rs::server::server::ServerConfig {
        name: "MCP-RS Example Server".to_string(),
        version: "0.1.0".to_string(),
    };

    // 2. Create application state
    #[derive(Clone)]
    struct AppState {
        _name: String,
    }

    let app_state = AppState {
        _name: "ExampleServer".to_string(),
    };

    // 3. Set up the tool registry
    let tool_registry = Arc::new(ToolRegistry::new());

    // 4. Register a calculator tool
    let calculator_tool = ToolBuilder::new("calculator", "Performs basic arithmetic")
        .add_parameter(
            ToolParameterBuilder::new("a", ToolParameterType::Number).required(true).build()
        )
        .add_parameter(
            ToolParameterBuilder::new("b", ToolParameterType::Number).required(true).build()
        )
        .add_parameter(
            ToolParameterBuilder::new("operation", ToolParameterType::String)
                .enum_values(
                    vec![
                        "add".to_string(),
                        "subtract".to_string(),
                        "multiply".to_string(),
                        "divide".to_string()
                    ]
                )
                .required(true)
                .build()
        )
        .build();

    // Register the calculator tool
    tool_registry
        .register_in_process_tool(calculator_tool, |params: CallToolParams| {
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
                .ok_or_else(|| Error::InvalidParams("Missing parameter 'operation'".to_string()))?;

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
                    return Err(Error::InvalidParams(format!("Unknown operation: {}", operation)));
                }
            };

            // Create a properly formatted TextContent object
            Ok(CallToolResult {
                content: vec![
                    serde_json::json!({
                    "type": "text",
                    "text": format!("The result of {} {} {} is {}", a, operation, b, result)
                })
                ],
                is_error: false,
            })
        }).await
        .expect("Failed to register calculator tool");

    // 6. Create and configure the server with method chaining
    let server = Server::with_config(app_state.clone(), config)
        .with_tool_registry(tool_registry)
        .with_default_handlers();

    // 7. Add transport and start server
    let server_handle = server.with_transport(TransportType::sse("127.0.0.1:8090")).start().await?;

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("---> Received shutdown signal");

    // Clean shutdown
    server_handle.shutdown().await?;
    println!("---> Server stopped");

    Ok(())
}
