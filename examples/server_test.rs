use mcp_rs::{
    client::Client,
    errors::Error,
    lifecycle::LifecycleEvent,
    messages::Message,
    server_session::{ ServerSession, ServerSessionOptions },
    transport::{
        sse_server::{ SseServerOptions, SseServerTransport },
        websocket_server::{ WebSocketServerOptions, WebSocketServerTransport },
    },
    types::{
        initialize::{
            LoggingCapabilities,
            PromptCapabilities,
            ResourceCapabilities,
            ToolCapabilities,
        },
        tools::{
            CallToolParams,
            CallToolResult,
            TextContent,
            Tool,
            ToolParameterBuilder,
            ToolParameterType,
        },
        ListResourcesParams,
        ListResourcesResult,
        Resource,
        ServerCapabilities,
    },
    utils::validation::ValidationConfig,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{ Arc, Mutex };
use tracing::{ Level, error, info };

/// Server state to manage resources and server status
#[derive(Clone)]
struct AppState {
    /// Map of resource URI to resource data
    resources: Arc<Mutex<HashMap<String, (Resource, String)>>>,
    /// Count of requests processed
    request_count: Arc<Mutex<u64>>,
    /// Connected clients
    connected_clients: Arc<Mutex<HashMap<String, String>>>,
    /// Transport lifecycle state
    transport_state: Arc<Mutex<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
            request_count: Arc::new(Mutex::new(0)),
            connected_clients: Arc::new(Mutex::new(HashMap::new())),
            transport_state: Arc::new(Mutex::new("uninitialized".to_string())),
        }
    }
}

/// Simple command-line arguments for server configuration
struct ServerConfig {
    transport: String,
    bind: String,
    validate: bool,
    verbose: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            transport: "sse".to_string(),
            bind: "127.0.0.1:8090".to_string(),
            validate: true,
            verbose: false,
        }
    }
}

// Constants
const SERVER_ADDR: &str = "127.0.0.1:8090";
const VENV_DIR: &str = "/tmp/mcp_venv";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a default server configuration
    let config = ServerConfig::default();

    // Set up logging
    let log_level = if config.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    // Create server state
    let state = AppState::default();

    // Create server session with custom options
    let mut server_session = ServerSession::new(state.clone(), ServerSessionOptions {
        name: "MCP-RS Example Server".to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        validation: ValidationConfig {
            validate_requests: config.validate,
            validate_responses: config.validate,
        },
        capabilities: ServerCapabilities {
            resources: Some(ResourceCapabilities {
                subscribe: true,
                list_changed: true,
            }),
            tools: Some(ToolCapabilities { list_changed: true }),
            prompts: Some(PromptCapabilities { list_changed: true }),
            logging: Some(LoggingCapabilities {}),
            experimental: None,
        },
        instructions: Some("Example MCP server ready for commands".to_string()),
    });

    // Register application-specific handlers
    register_handlers(&mut server_session);

    // Parse socket address
    let addr: SocketAddr = config.bind.parse()?;

    // Create a oneshot channel to signal when the server is ready
    let (server_ready_tx, server_ready_rx) = tokio::sync::oneshot::channel();

    // Start the server in a separate task
    let server_handle = tokio::spawn(async move {
        match config.transport.as_str() {
            "sse" => {
                info!("Starting server with SSE transport on {}", addr);

                // Create server transport with custom options
                // First, create a message channel for receiving client messages
                let (message_tx, _message_rx) = tokio::sync::mpsc::channel::<
                    (Option<String>, Result<Message, Error>)
                >(100);

                let mut transport = SseServerTransport::new(SseServerOptions {
                    bind_address: addr.to_string(),
                    auth_token: None,
                    connection_timeout: std::time::Duration::from_secs(30),
                    keep_alive_interval: 30,
                    allowed_origins: None,
                    require_auth: false,
                    message_tx: Some(message_tx),
                });

                // Register lifecycle handler for transport events directly
                let state_for_lifecycle = state.clone();
                transport.register_lifecycle_handler(move |event| {
                    handle_lifecycle_event(event, &state_for_lifecycle);
                });

                info!("Server started! Listening for connections...");

                // Signal that the server is ready
                let _ = server_ready_tx.send(addr);

                // Start the server session and return the result
                server_session.run(transport).await
            }
            "ws" => {
                info!("Starting server with WebSocket transport on {}", addr);

                // Create WebSocket server transport with options
                // addr is already a SocketAddr, no need to parse it
                let mut transport = WebSocketServerTransport::with_options(WebSocketServerOptions {
                    bind_address: addr,
                    websocket_path: "/ws".to_string(),
                    auth_token: None,
                    connection_timeout: std::time::Duration::from_secs(30),
                    heartbeat_interval: std::time::Duration::from_secs(30),
                    require_auth: false,
                    allowed_origins: None,
                });

                // Since lifecycle_handlers is private, we can't access it directly
                // We'll need to modify the WebSocketServerTransport to expose a method for this
                // For now, we'll just log that we can't register the handler
                info!(
                    "Note: Lifecycle handler registration is not available for WebSocketServerTransport"
                );

                info!("Server started! Listening for connections...");

                // Signal that the server is ready
                let _ = server_ready_tx.send(addr);

                // Run the server with WebSocket transport
                server_session.run(transport).await
            }
            _ => {
                let err = Error::Other(format!("Unsupported transport: {}", config.transport));
                return Err(err);
            }
        }
    });

    // Wait for the server to be ready
    match server_ready_rx.await {
        Ok(addr) => {
            info!("Server is ready on {}", addr);

            // Run Python client if requested (via environment variable)
            if std::env::var("RUN_PYTHON_CLIENT").unwrap_or_default() == "1" {
                run_python_client().await?;
            }
        }
        Err(e) => {
            return Err(format!("Failed to receive server ready signal: {}", e).into());
        }
    }

    // Wait for the server task to complete
    match server_handle.await {
        Ok(result) => {
            if let Err(e) = result {
                return Err(format!("Server error: {}", e).into());
            }
        }
        Err(e) => {
            return Err(format!("Server task error: {}", e).into());
        }
    }

    info!("Server stopped gracefully");
    Ok(())
}

/// Register application-specific method handlers
fn register_handlers(server: &mut ServerSession<AppState>) {
    // Register the calculator tool
    info!("Registering calculator tool");

    // Create the calculator tool definition using the ToolBuilder API
    let calc_tool = server
        .build_tool("calculator", "Performs arithmetic operations")
        .add_parameter(
            ToolParameterBuilder::new("operation", ToolParameterType::String)
                .description("Operation to perform (add, subtract, multiply, divide)")
                .required(true)
                .enum_values(vec!["add", "subtract", "multiply", "divide"])
                .default("add")
                .unwrap()
                .build()
        )
        .add_parameter(
            ToolParameterBuilder::new("a", ToolParameterType::Number)
                .description("First operand")
                .required(true)
                .build()
        )
        .add_parameter(
            ToolParameterBuilder::new("b", ToolParameterType::Number)
                .description("Second operand")
                .required(true)
                .build()
        )
        .return_type("number")
        .return_schema(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {
                        "type": "number",
                        "description": "The result of the calculation"
                    }
                },
                "required": ["result"]
            })
        )
        .streaming(false)
        .cancellable(false)
        .timeout(30)
        .build();

    // Register the calculator as an in-process tool
    server.register_in_process_tool(calc_tool, calculator_handler);

    // Resources list handler
    server.register_handler("resources/list", |state, params: ListResourcesParams| {
        let state = state.clone();
        Box::pin(async move {
            info!("Listing resources");

            // Increment request counter
            {
                let mut count = state.request_count.lock().unwrap();
                *count += 1;
            }

            // Get resources
            let resources = state.resources.lock().unwrap();
            let resources_list = resources
                .iter()
                .map(|(_, (resource, _))| resource.clone())
                .collect::<Vec<_>>();

            Ok(ListResourcesResult {
                resources: resources_list,
                next_page_token: None,
            })
        })
    });
}

/// Calculator tool implementation
fn calculator_handler(params: CallToolParams) -> Result<CallToolResult, Error> {
    // Extract operation and operands
    let operation = params.arguments
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("add");
    let a = params.arguments
        .get("a")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let b = params.arguments
        .get("b")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Perform calculation
    let result = match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b == 0.0 {
                return Err(Error::InvalidParams("Division by zero".to_string()));
            }
            a / b
        }
        _ => {
            return Err(Error::InvalidParams(format!("Unknown operation: {}", operation)));
        }
    };

    // Log the calculation
    info!("Calculator: {} {} {} = {}", a, operation, b, result);

    // Create a TextContent struct for the result
    let text_content = TextContent {
        content_type: "text".to_string(),
        text: format!("The result of {} {} {} is {}", a, operation, b, result),
        annotations: None,
    };

    // Return result
    Ok(CallToolResult {
        // Use content field for Python/JS clients as a Vec of content items
        content: vec![serde_json::to_value(text_content).unwrap()],
        // Set isError flag to false
        is_error: false,
    })
}

/// Handle transport lifecycle events
fn handle_lifecycle_event(event: LifecycleEvent, state: &AppState) {
    match event {
        LifecycleEvent::ClientConnected(client_id) => {
            info!("Client connected: {}", client_id);
            state.connected_clients
                .lock()
                .unwrap()
                .insert(client_id.clone(), "connected".to_string());

            // Update transport state
            *state.transport_state.lock().unwrap() = "client_connected".to_string();
        }
        LifecycleEvent::ClientDisconnected(client_id) => {
            info!("Client disconnected: {}", client_id);
            state.connected_clients.lock().unwrap().remove(&client_id);

            // Update transport state
            *state.transport_state.lock().unwrap() = "client_disconnected".to_string();
        }
    }
}

/// Run a Python client that connects to the MCP server
async fn run_python_client() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running Python client from python_client directory...");

    // Run the Python client script
    let output = std::process::Command
        ::new("bash")
        .arg("-c")
        .arg("cd python_client && ./run_client.sh --debug")
        .output()?;

    // Print stdout
    println!("\n--- Python Client Output (stdout) ---");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    // Print stderr if there is any
    if !output.stderr.is_empty() {
        println!("\n--- Python Client Output (stderr) ---");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // Check exit status
    if output.status.success() {
        info!("Python client completed successfully");
    } else {
        info!("Python client exited with status: {}", output.status);
    }

    Ok(())
}
