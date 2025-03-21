# MCP-rs: Rust Implementation of the Model Context Protocol

MCP-rs is a type-safe, efficient, and ergonomic Rust implementation of the [Model Context Protocol (MCP)](https://github.com/microsoft/mcp), designed to enable seamless integration between AI applications and external data sources or tools.

## Current Status

This repository contains a working implementation of the MCP protocol with the following components:

- **Rust Server**: A fully functional MCP server with support for tools, resources, and lifecycle management
- **Python Client**: A Python client that can connect to the server, list tools, and call them
- **Calculator Tool**: A sample tool that performs arithmetic operations (add, subtract, multiply, divide)
- **Server-Sent Events (SSE) Transport**: Real-time communication between client and server
- **WebSocket Transport**: Alternative transport for bidirectional communication
- **Tool Derive Macro**: A macro for easily creating tools from Rust structs
- **Prompt Management**: Support for creating and managing prompt templates

The `server_test` example is fully functional and works as a Cursor tool, allowing you to run an MCP server and interact with it using the Python client.

## Features

- **Type-Safe**: Leverages Rust's strong type system to ensure protocol correctness at compile time
- **Async-First**: Built on Tokio for high-performance, non-blocking I/O operations
- **Multiple Transports**: Supports STDIO, Server-Sent Events (SSE), and WebSocket transports
- **Schema Validation**: Automatic JSON Schema generation for protocol types
- **Extensible**: Easy to add new message handlers, tools, resources, and prompts
- **Python Compatibility**: Works with the standard MCP Python library
- **Derive Macros**: Generate tool schemas automatically from Rust structs
- **Prompt Templates**: Create and manage reusable prompt templates

## Quick Start

### Running the Server

```bash
# Run the server_test example
cargo run --example server_test
```

This will start an MCP server on port 8090 with the calculator tool registered.

### Running the Python Client

```bash
# Navigate to the Python client directory
cd python_client

# Run the client script
./run_client.sh
```

The Python client will:
1. Connect to the MCP server
2. Initialize the connection
3. List available tools
4. Test the calculator tool with various operations

### Creating Your Own MCP Server

```rust
use mcp_rs::{ServerSession, Transport, sse_server::SseServerTransport};
use mcp_rs::types::tools::{Tool, ToolBuilder, ToolParameterType};

#[tokio::main]
async fn main() {
    // Create a new server with application state
    let mut server = ServerSession::new(());
    
    // Register a calculator tool
    server.register_tool_builder(
        server
            .build_tool("calculator", "Performs arithmetic operations")
            .add_parameter(/* ... */)
            // ... add more parameters
            .return_type("number")
            .timeout(30)
    );
    
    // Register handler for the calculator tool
    server.register_handler("tools/call", |state, params| {
        // Handle tool calls
    });
    
    // Start the server with SSE transport
    let transport = SseServerTransport::new();
    server.run(transport).await;
}
```

### Using the Tool Derive Macro

The library provides a convenient derive macro to create tools from Rust structs:

```rust
use tool_derive::Tool;
use serde::{Serialize, Deserialize};

#[derive(Tool, Debug, Clone, Serialize, Deserialize)]
#[tool(description = "Performs basic arithmetic")]
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

// Register the tool using the derived to_tool_schema() method
server.register_in_process_tool(
    calculator.to_tool_schema(),
    |params| {
        // Handle the tool call with type-safe parameters
    }
)
```

### Working with Prompts

The library includes a prompt management system for creating reusable prompt templates:

```rust
// Using the prompt macro to create a prompt template
server.register_prompt(
    prompt!(
        "welcome",
        "A welcome message for users",
        vec![PromptArgument {
            name: "name".to_string(),
            description: Some("The name of the user".to_string()),
            required: false,
        }],
        |arguments: Option<HashMap<String, Value>>| async move {
            let name = match &arguments {
                Some(args) => args.get("name").and_then(|v| v.as_str()).unwrap_or("friend"),
                None => "friend",
            };

            Ok(vec![
                create_user_text_message("Hello!"),
                create_assistant_text_message(&format!("Welcome, {}!", name))
            ])
        }
    )
)
```

## Examples

The repository includes several examples to demonstrate different MCP features:

- **calculator.rs**: Demonstrates the Tool derive macro and prompt management system
- **server_test**: A complete MCP server with a calculator tool and SSE transport
- **lifecycle_example**: Demonstrates the lifecycle management features
- **websocket_transport_example**: Shows how to use WebSocket transport
- **tool_execution**: Illustrates implementing and registering tools
- **mcp-server-example**: A standalone MCP server implementation

To run an example:

```bash
cargo run --example calculator
```

## Python Client

The repository includes a Python client in the `python_client` directory. This client:

- Uses the standard MCP Python library
- Connects to the MCP server using SSE transport
- Lists available tools
- Calls the calculator tool with different operations

To run the Python client:

```bash
cd python_client
./run_client.sh
```

## Documentation

For detailed documentation, see the `docs` directory or run:

```bash
cargo doc --open
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 