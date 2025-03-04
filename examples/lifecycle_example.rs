//! MCP Lifecycle Management Example
//!
//! This example demonstrates the MCP lifecycle management system, showing how
//! clients and servers transition through the different phases of the protocol.

use mcp_rs::lifecycle::*;
use mcp_rs::types::InitializeRequestParams;
use mcp_rs::{
    Error,
    client::ClientSession,
    server::Server,
    transport::{ sse::SseTransport, sse_server::{ SseServerOptions, SseServerTransport } },
    types::initialize::{ ClientCapabilities, Implementation, InitializeResult },
    utils::validation::ValidationConfig,
};

use std::net::SocketAddr;
use std::sync::{ Arc, Mutex };
use std::time::Duration;
use tokio::time::sleep;
use tracing::{ Level, debug, info };

/// Custom server context for lifecycle events
#[derive(Clone)]
struct ServerAppContext {
    startup_time: std::time::Instant,
    request_count: Arc<Mutex<u64>>,
    in_shutdown: Arc<Mutex<bool>>,
}

impl ServerAppContext {
    fn new() -> Self {
        Self {
            startup_time: std::time::Instant::now(),
            request_count: Arc::new(Mutex::new(0)),
            in_shutdown: Arc::new(Mutex::new(false)),
        }
    }

    fn increment_request_count(&self) {
        let mut count = self.request_count.lock().unwrap();
        *count += 1;
    }

    fn get_request_count(&self) -> u64 {
        let count = self.request_count.lock().unwrap();
        *count
    }
}

impl LifecycleContext for ServerAppContext {
    fn on_startup(&mut self) -> Result<(), Error> {
        info!("Server is starting up");
        Ok(())
    }

    fn on_initialized(
        &mut self,
        protocol_version: ProtocolVersion,
        capabilities: &NegotiatedCapabilities
    ) -> Result<(), Error> {
        info!("Server initialized with protocol version {}", protocol_version);
        info!("Negotiated capabilities: {:#?}", capabilities);
        Ok(())
    }

    fn before_request(&mut self, context: &RequestContext) -> Result<(), Error> {
        self.increment_request_count();
        debug!(
            "Processing request {} ({}): {}",
            self.get_request_count(),
            context.request_id,
            context.method
        );
        Ok(())
    }

    fn after_request<T>(
        &mut self,
        context: &RequestContext,
        result: &Result<T, Error>
    ) -> Result<(), Error> {
        match result {
            Ok(_) => debug!("Request {} completed successfully", context.request_id),
            Err(e) => debug!("Request {} failed: {}", context.request_id, e),
        }
        Ok(())
    }

    fn on_shutdown(&mut self) -> Result<(), Error> {
        let mut shutdown = self.in_shutdown.lock().unwrap();
        *shutdown = true;
        info!(
            "Server is shutting down after {} seconds of operation",
            self.startup_time.elapsed().as_secs()
        );
        info!("Processed {} requests", self.get_request_count());
        Ok(())
    }
}

/// Custom client context for lifecycle events
#[derive(Clone)]
struct ClientAppContext {
    startup_time: std::time::Instant,
    connected: Arc<Mutex<bool>>,
}

impl ClientAppContext {
    fn new() -> Self {
        Self {
            startup_time: std::time::Instant::now(),
            connected: Arc::new(Mutex::new(false)),
        }
    }

    fn set_connected(&self, connected: bool) {
        let mut conn = self.connected.lock().unwrap();
        *conn = connected;
    }
}

impl LifecycleContext for ClientAppContext {
    fn on_startup(&mut self) -> Result<(), Error> {
        info!("Client is starting up");
        Ok(())
    }

    fn on_initialized(
        &mut self,
        protocol_version: ProtocolVersion,
        capabilities: &NegotiatedCapabilities
    ) -> Result<(), Error> {
        self.set_connected(true);
        info!("Client initialized with protocol version {}", protocol_version);
        info!("Negotiated capabilities: {:#?}", capabilities);
        Ok(())
    }

    fn before_request(&mut self, _context: &RequestContext) -> Result<(), Error> {
        Ok(())
    }

    fn after_request<T>(
        &mut self,
        _context: &RequestContext,
        _result: &Result<T, Error>
    ) -> Result<(), Error> {
        Ok(())
    }

    fn on_shutdown(&mut self) -> Result<(), Error> {
        self.set_connected(false);
        info!(
            "Client is shutting down after {} seconds of operation",
            self.startup_time.elapsed().as_secs()
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Set up logging
    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();

    info!("Starting MCP Lifecycle Example");

    // Start server in a separate task
    let server_handle = tokio::spawn(run_server());

    // Wait for server to start
    sleep(Duration::from_millis(500)).await;

    // Start client
    let client_result = run_client().await;

    // Wait for server to process client shutdown
    sleep(Duration::from_millis(500)).await;

    // Request server shutdown
    let server_result = server_handle.await??;

    // Return client result
    client_result?;
    server_result?;

    info!("Example completed successfully");
    Ok(())
}

/// Run the MCP server with lifecycle management
async fn run_server() -> Result<Result<(), Error>, Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting MCP server");

    // Create server context with lifecycle hooks
    let context = ServerAppContext::new();

    // Create lifecycle manager
    let lifecycle = LifecycleManager::new(context.clone())
        .with_init_timeout(Duration::from_secs(30))
        .with_shutdown_timeout(Duration::from_secs(10));

    // Start the lifecycle
    lifecycle.start().await?;

    // Create and configure the server
    let mut server = Server::new(context).with_validation_config(ValidationConfig {
        validate_requests: true,
        validate_responses: true,
    });

    // Configure SSE server options
    let bind_addr: SocketAddr = "127.0.0.1:8000".parse()?;
    let sse_options = SseServerOptions {
        bind_address: bind_addr,
        auth_token: None,
        connection_timeout: Duration::from_secs(30),
        keep_alive_interval: 30,
        allowed_origins: Some(vec!["*".to_string()]),
        require_auth: false,
    };

    // Create SSE transport
    let mut sse_transport = SseServerTransport::with_options(sse_options);

    // Register basic methods
    server.register_initialize_handler(|state, params| {
        let state = state.clone();
        Box::pin(async move {
            info!("Received initialize request from client");
            state.increment_request_count();

            // Return a proper initialize result
            Ok(InitializeResult {
                protocol_version: params.protocol_version,
                server_info: Some(Implementation {
                    name: "MCP Lifecycle Example Server".to_string(),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                }),
                capabilities: mcp_rs::types::initialize::ServerCapabilities {
                    logging: Some(mcp_rs::types::initialize::LoggingCapabilities {}),
                    resources: Some(mcp_rs::types::initialize::ResourceCapabilities {
                        list_changed: true,
                        subscribe: true,
                    }),
                    tools: Some(mcp_rs::types::initialize::ToolCapabilities {
                        list_changed: true,
                    }),
                    prompts: Some(mcp_rs::types::initialize::PromptCapabilities {
                        list_changed: true,
                    }),
                    experimental: None,
                },
                instructions: Some("This is an example MCP lifecycle server".to_string()),
            })
        })
    });

    // Start the SSE transport
    sse_transport.start(None).await?;

    info!("SSE server started on {}", bind_addr);
    info!("SSE endpoint: http://{}/sse", bind_addr);
    info!("Messages endpoint: http://{}/messages/", bind_addr);

    // Run the server with the SSE transport (this will block until the server is shut down)
    let result = server.run(sse_transport).await;

    // Log and return the result
    match &result {
        Ok(_) => info!("Server shut down gracefully"),
        Err(e) => info!("Server shut down with error: {}", e),
    }

    Ok(result)
}

/// Run the MCP client with lifecycle management
async fn run_client() -> Result<(), Error> {
    info!("Starting MCP client");

    // Create client context with lifecycle hooks
    let context = ClientAppContext::new();

    // Create SSE transport for the client
    let server_url = "http://127.0.0.1:8000";
    let transport = SseTransport::new(server_url).await?;
    info!("Connected to SSE endpoint: {}/sse", server_url);

    // Create client session
    let mut session = ClientSession::new(transport).with_validation_config(ValidationConfig {
        validate_requests: true,
        validate_responses: true,
    });

    // Start the session (this connects to the server)
    session.start().await?;
    info!("Client session started");

    // Initialize the connection (this transitions through the protocol initialization phase)
    info!("Initializing connection to server...");
    let init_result = session.initialize(InitializeRequestParams {
        protocol_version: "2024-11-05".to_string(),
        client_info: Some(Implementation {
            name: "MCP Lifecycle Example Client".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
        capabilities: ClientCapabilities {
            roots: None,
            sampling: None,
            experimental: None,
        },
    }).await?;

    // Log initialization result
    info!(
        "Connected to server: {}",
        init_result.server_info
            .map(|info| info.name)
            .unwrap_or_else(|| "Unknown Server".to_string())
    );
    if let Some(instructions) = init_result.instructions {
        info!("Server instructions: {}", instructions);
    }
    info!("Server capabilities: {:?}", init_result.capabilities);

    // Simulate some client activity
    info!("Client is now in the operational phase");
    for i in 1..=3 {
        info!("Simulating client activity {}/3", i);
        sleep(Duration::from_secs(1)).await;
    }

    // Close the client session (this transitions to the shutdown phase)
    info!("Closing client session...");
    session.close().await?;
    info!("Client session closed");

    Ok(())
}
