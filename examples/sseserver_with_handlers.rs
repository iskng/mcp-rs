//! Example server with transport and message handlers
//!
//! This example shows how to set up an MCP server with handlers
//! for different message types, such as initialization and requests.

use mcp_rs::errors::Error;
use mcp_rs::server::Server;
use mcp_rs::server::tools::tool_registry::ToolRegistry;
use mcp_rs::server::handlers::{ ResourceHandler, ToolHandler };
use mcp_rs::server::resources::ResourceRegistry;
use mcp_rs::types::protocol::RequestType;
use mcp_rs::transport::TransportType;
use mcp_rs::types::resources::{ Resource, ResourceContent };
use mcp_rs::types::tools::{
    ToolBuilder,
    ToolParameterBuilder,
    ToolParameterType,
    CallToolParams,
    CallToolResult,
};
use async_trait::async_trait;

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
