//! Example server with transport and message handlers
//!
//! This example shows how to set up an MCP server with handlers
//! for different message types, such as initialization and requests.

use async_trait::async_trait;
use mcp_rs::protocol::Error;
use mcp_rs::server::Server;
use mcp_rs::server::handlers::{ ResourceHandler, ToolHandler };
use mcp_rs::server::services::resources::ResourceRegistry;
use mcp_rs::server::services::tools::tool_registry::ToolRegistry;
use mcp_rs::protocol::{ CallToolParams, CallToolResult, CallToolResultBuilder, ToolBuilder };
use mcp_rs::transport::TransportType;
use mcp_rs::protocol::Tool;
use mcp_rs::protocol::Role;
use serde::{ Deserialize, Serialize };
use std::sync::Arc;
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
    let calculator_tool = (Calculator {
        a: 0.0, // Default values
        b: 0.0,
        operation: Operation::Add,
    }).to_tool();

    tool_registry
        .register_in_process_tool(calculator_tool, |params: CallToolParams| {
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

            // Create a schema-compliant response using the type-safe builder
            let operation_text = format!("{:?}", typed_params.operation).to_lowercase();
            let result_text = format!(
                "The result of {} {} {} is {}",
                typed_params.a,
                operation_text,
                typed_params.b,
                result_value
            );

            // Optional: Add annotations for priority or audience if needed
            let annotations = CallToolResultBuilder::annotations(
                Some(vec![Role::Assistant]),
                Some(0.9)
            );

            Ok(
                CallToolResult::builder()
                    .add_text(result_text)
                    .add_text_with_annotations(
                        "This result is important for calculation purposes",
                        annotations
                    )
                    .build()
            )
        }).await
        .expect("Failed to register calculator tool");

    let tool_handler = ToolHandler::new(tool_registry);
    // 5. Set up resource registry
    let resource_registry = Arc::new(ResourceRegistry::new(true, true));

    // 6. Register example resources
    // Static text resource
    let markdown_content =
        r#"# MCP Resource Example
    
## Markdown Content
    
This is an example of a static resource served by the MCP server.
    
* Bullet point 1
* Bullet point 2
    
```python
def hello_world():
    print("Hello from MCP resource!")
```
    "#;

    // Create a markdown resource
    let md_resource = {
        let resource = Resource {
            uri: "resource:docs/example.md".to_string(),
            name: "Example Documentation".to_string(),
            description: Some("Example markdown documentation resource".to_string()),
            mime_type: Some("text/markdown".to_string()),
        };

        struct StaticTextResource {
            resource: Resource,
            content: String,
        }

        #[async_trait]
        impl mcp_rs::server::resources::ResourceProvider for StaticTextResource {
            fn metadata(&self) -> Resource {
                self.resource.clone()
            }

            async fn content(&self) -> Result<ResourceContent, Error> {
                Ok(ResourceContent::Text {
                    uri: self.resource.uri.clone(),
                    text: self.content.clone(),
                    mime_type: self.resource.mime_type.clone().unwrap_or("text/plain".to_string()),
                })
            }
        }

        StaticTextResource {
            resource,
            content: markdown_content.to_string(),
        }
    };

    // Register the markdown resource
    resource_registry
        .register_resource(md_resource).await
        .expect("Failed to register markdown resource");

    // Create a JSON resource
    let json_resource = {
        let resource = Resource {
            uri: "resource:data/example.json".to_string(),
            name: "Example JSON Data".to_string(),
            description: Some("Example JSON data resource".to_string()),
            mime_type: Some("application/json".to_string()),
        };

        struct StaticTextResource {
            resource: Resource,
            content: String,
        }

        #[async_trait]
        impl mcp_rs::server::resources::ResourceProvider for StaticTextResource {
            fn metadata(&self) -> Resource {
                self.resource.clone()
            }

            async fn content(&self) -> Result<ResourceContent, Error> {
                Ok(ResourceContent::Text {
                    uri: self.resource.uri.clone(),
                    text: self.content.clone(),
                    mime_type: self.resource.mime_type.clone().unwrap_or("text/plain".to_string()),
                })
            }
        }

        StaticTextResource {
            resource,
            content: r#"{
                "name": "MCP Example",
                "version": "1.0.0",
                "description": "A sample JSON resource",
                "items": [
                    {"id": 1, "value": "First item"},
                    {"id": 2, "value": "Second item"},
                    {"id": 3, "value": "Third item"}
                ]
            }"#.to_string(),
        }
    };

    // Register the JSON resource
    resource_registry
        .register_resource(json_resource).await
        .expect("Failed to register JSON resource");

    // 7. Create server and register handlers
    let mut server = Server::with_config(app_state.clone(), config);

    // Add resource handler
    let resource_handler = ResourceHandler::new(resource_registry);
    server.register_request_handler(RequestType::ListResources, resource_handler.clone());
    server.register_request_handler(RequestType::ReadResource, resource_handler.clone());
    server.register_request_handler(RequestType::SubscribeResource, resource_handler.clone());
    server.register_request_handler(RequestType::UnsubscribeResource, resource_handler);

    server.register_domain_handler(
        vec![RequestType::ListTools, RequestType::CallTool],
        tool_handler
    );
    // Add default handlers (including tool handler)
    server = server.with_default_handlers();

    // 8. Add transport and start server
    let server_handle = server.with_transport(TransportType::sse("127.0.0.1:8090")).start().await?;

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("---> Received shutdown signal");

    // Clean shutdown
    server_handle.shutdown().await?;
    println!("---> Server stopped");

    Ok(())
}
