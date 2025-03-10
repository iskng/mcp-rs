//! Example server with transport and message handlers
//!
//! This example shows how to set up an MCP server with handlers
//! for different message types, such as initialization and requests.

use mcp_rs::protocol::Error;
use mcp_rs::protocol::tools::ToToolSchema;
use mcp_rs::protocol::{CallToolParams, CallToolResult};
use mcp_rs::server::Server;
use mcp_rs::server::transport::sse::{SseServerOptions, SseServerTransport};
use serde::{Deserialize, Serialize};
use tool_derive::Tool;
use tracing::Level;

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

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Use tracing for better logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
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
        .with_instructions("This is an example server that demonstrates the Tool derive macro.")
        .register_in_process_tool(
            // Use the derived to_tool_schema() method to generate a Tool from our Calculator struct
            (Calculator {
                a: 0.0, // Default values
                b: 0.0,
                operation: Operation::Add,
            })
            .to_tool_schema(),
            |params: CallToolParams| {
                // Deserialize directly into our type-safe struct
                let typed_params: Calculator =
                    serde_json::from_value(serde_json::to_value(params.arguments).unwrap())
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
                    typed_params.a, operation_text, typed_params.b, result_value
                );

                // Return a text result
                Ok(CallToolResult::text(result_text))
            },
        )
        .with_transport(transport)
        .build()
        .await;

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
