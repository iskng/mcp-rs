//! Simple Server with SSE Transport Example
//!
//! This example demonstrates how to create a minimal MCP server
//! with the redesigned transport interface.

use mcp_rs::{
    protocol::{
        Error,
        Tool,
        CallToolParams,
        CallToolResult,
        tools::{ ToolBuilder, CallToolResultBuilder },
        ToolsCapability,
        ResourcesCapability,
    },
    server::{
        Server,
        handlers::composite::CompositeServerHandler,
        services::{
            ServiceProvider,
            resources::resource_registry::ResourceRegistry,
            tools::tool_registry::ToolRegistry,
        },
    },
    transport::sse_server::{ SseServerTransport, SseServerOptions },
};
use std::sync::Arc;
use serde_json::json;
use std::collections::HashMap;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    tracing_subscriber::fmt().init();

    // Create a simple sse transport
    let transport = SseServerTransport::new(SseServerOptions {
        bind_address: "127.0.0.1:8090".to_string(),
        ..Default::default()
    });

    // Create a server with a calculator tool
    let server_result = Server::builder()
        .with_server_name("Example Server")
        .with_server_version("0.1.0")
        .with_instructions("This is an example server with a calculator tool.")
        .register_in_process_tool(
            ToolBuilder::new("calculator", "Basic calculator")
                .add_number_parameter("a", "First number", true)
                .add_number_parameter("b", "Second number", true)
                .add_enum_parameter(
                    "operation",
                    "Operation to perform",
                    &["add", "subtract", "multiply", "divide"],
                    true
                )
                .build(),
            |params: CallToolParams| {
                // Extract parameters
                let args = params.arguments.ok_or_else(||
                    Error::Protocol("Missing arguments".to_string())
                )?;

                let a = args
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| Error::Protocol("Missing parameter 'a'".to_string()))?;

                let b = args
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| Error::Protocol("Missing parameter 'b'".to_string()))?;

                let operation = args
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Protocol("Missing parameter 'operation'".to_string()))?;

                // Perform calculation
                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b == 0.0 {
                            return Err(Error::Protocol("Division by zero".to_string()));
                        }
                        a / b
                    }
                    _ => {
                        return Err(Error::Protocol(format!("Unknown operation: {}", operation)));
                    }
                };

                // Create the result text
                let result_text = format!("The result of {} {} {} is {}", a, operation, b, result);

                // Use the helper method to create a text result
                Ok(CallToolResult::text(result_text))
            }
        )
        .with_transport(transport)
        .build().await;

    match server_result {
        Ok(mut server) => {
            // Start the server
            if let Err(e) = server.start().await {
                eprintln!("Error starting server: {}", e);
                return Ok(());
            }

            println!("Server started on http://127.0.0.1:8090");

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
