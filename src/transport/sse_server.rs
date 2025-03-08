//! Server-side implementation of the SSE transport
//!
//! This module provides an SSE server implementation that:
//! - Accepts SSE connections from clients at the `/sse` endpoint
//! - Accepts HTTP POST requests from clients at the `/message` endpoint
//! - Routes messages between clients and the MCP server

use crate::protocol::{ JSONRPCMessage, JSONRPCMessage as Message, errors::Error };
use crate::server::handlers::RouteHandler;
use crate::transport::{ Transport };
use crate::transport::middleware::{ ClientSession, ClientSessionLayer, ClientSessionStore };
use async_trait::async_trait;
use async_stream;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    extract::{ Extension, Query, State },
    http::{ Request, StatusCode },
    middleware::Next,
    response::{ sse::{ Event, KeepAlive, Sse }, IntoResponse },
    routing::{ get, post },
    Router,
};
use tower::ServiceBuilder;
use tower_http::cors::{ Any, CorsLayer };
use tokio::sync::mpsc;
use uuid;
use tokio::sync::RwLock;

/// Configuration options for the SSE server
#[derive(Debug, Clone)]
pub struct SseServerOptions {
    /// Address to bind the server to
    pub bind_address: String,
    /// Optional authentication token to validate requests
    pub auth_token: Option<String>,
    /// Connection timeout in seconds
    pub connection_timeout: std::time::Duration,
    /// Keep-alive interval in seconds
    pub keep_alive_interval: u64,
    /// CORS allowed origins
    pub allowed_origins: Option<Vec<String>>,
    /// Whether to require authentication
    pub require_auth: bool,
    /// Message transmitter channel
    pub message_tx: Option<mpsc::Sender<(Option<String>, Result<Message, Error>)>>,
}

impl Default for SseServerOptions {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8090".to_string(),
            auth_token: None,
            connection_timeout: std::time::Duration::from_secs(30),
            keep_alive_interval: 30,
            allowed_origins: None,
            require_auth: false,
            message_tx: None,
        }
    }
}

/// Connection events emitted by the SSE server
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Client connected
    ClientConnected(String),
    /// Client disconnected
    ClientDisconnected(String),
    /// Client activity (heartbeat, message, etc.)
    ClientActivity(String),
    /// Client property changed
    ClientPropertyChanged(String, String, String),
}

/// Type for connection event handlers
pub type ConnectionEventHandler = Box<dyn Fn(ConnectionEvent) + Send + Sync + 'static>;

/// Message event types
pub enum MessageEvent {
    /// A new message was received from a client
    MessageReceived(Option<String>, serde_json::Value),

    /// A message was sent to a client
    MessageSent(String, serde_json::Value),

    /// A message processing error occurred
    MessageError(Option<String>, Error),
}

/// Application state for the SSE server
struct AppState {
    /// Authentication token (if required)
    auth_token: Option<String>,

    /// Whether authentication is required
    require_auth: bool,

    /// Allowed origins for CORS
    allowed_origins: Option<Vec<String>>,

    /// Server handler for processing messages
    route_handler: Option<Arc<dyn RouteHandler + Send + Sync>>,

    /// Client session store
    session_store: Arc<ClientSessionStore>,
}

impl AppState {
    fn new(
        auth_token: Option<String>,
        require_auth: bool,
        allowed_origins: Option<Vec<String>>
    ) -> Self {
        Self {
            auth_token,
            require_auth,
            allowed_origins,
            route_handler: None,
            session_store: Arc::new(ClientSessionStore::new()),
        }
    }

    /// Set server handler
    fn with_route_handler(mut self, handler: Arc<dyn RouteHandler + Send + Sync>) -> Self {
        self.route_handler = Some(handler);
        self
    }
}

/// Create a transport error
fn transport_error<S: Into<String>>(message: S) -> Error {
    Error::Transport(message.into())
}

/// SSE server transport implementation
pub struct SseServerTransport {
    /// Options for SSE server
    options: SseServerOptions,

    /// Server socket address
    server_addr: Option<SocketAddr>,

    /// Application state shared with server
    app_state: Arc<RwLock<Option<Arc<crate::server::server::AppState>>>>,

    /// Server task handle
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<Result<(), Error>>>>>,
}

impl SseServerTransport {
    /// Create a new SSE server transport with default options
    pub fn new(options: SseServerOptions) -> Self {
        Self {
            options,
            server_addr: None,
            app_state: Arc::new(RwLock::new(None)),
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new SSE server transport with app_state
    pub fn with_app_state(
        options: SseServerOptions,
        app_state: Arc<crate::server::server::AppState>
    ) -> Self {
        let transport = Self::new(options);
        // Initialize the app_state immediately
        tokio::spawn({
            let app_state_lock = transport.app_state.clone();
            let app_state = app_state.clone();
            async move {
                let mut guard = app_state_lock.write().await;
                *guard = Some(app_state);
                tracing::info!("Initialized SseServerTransport with app_state");
            }
        });
        transport
    }

    /// Get the current app state
    async fn get_app_state(&self) -> Option<Arc<crate::server::server::AppState>> {
        let guard = self.app_state.read().await;
        guard.clone()
    }

    /// Start the SSE server
    pub async fn start(&mut self) -> Result<(), Error> {
        // Check if we have app state
        let app_state = match self.get_app_state().await {
            Some(state) => state,
            None => {
                return Err(Error::Protocol("No app state available for transport".to_string()));
            }
        };

        // Extract the route handler from app state
        let route_handler = app_state.route_handler.clone();

        tracing::info!("Starting SSE server with route handler from app state");

        // Parse server address
        let addr_str = self.options.bind_address.clone();
        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| transport_error(format!("Invalid bind address '{}': {}", addr_str, e)))?;

        // Get the session store from app state
        let session_store = app_state.session_store.clone();
        tracing::info!("Configuring SSE router with session store from app state");

        let app = Router::new()
            .route("/", get(handle_status))
            .route("/sse", get(handle_sse_connection))
            .route("/message", post(handle_client_message))
            .with_state(app_state)
            // Add the store as an extension first
            .layer(Extension(session_store.clone()))
            // Add the session layer that will extract sessions from query params
            .layer(ClientSessionLayer::with_store(session_store))
            // Add CORS support
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));

        // Start the server
        let server_addr = addr.clone();
        let server_handle = tokio::spawn(async move {
            tracing::info!("Starting SSE server on {}", addr);
            let listener = tokio::net::TcpListener
                ::bind(addr).await
                .map_err(|e| transport_error(format!("Failed to bind to {}: {}", addr, e)))?;

            axum::serve(listener, app.into_make_service()).await.map_err(|e|
                transport_error(format!("Server error: {}", e))
            )
        });

        // Store server information
        self.server_addr = Some(server_addr);
        {
            let mut server_handle_guard = self.server_handle.write().await;
            *server_handle_guard = Some(server_handle);
        }

        tracing::info!("SSE server started on {}", addr_str);
        Ok(())
    }

    /// Close the SSE server
    pub async fn close(&mut self) -> Result<(), Error> {
        tracing::info!("Shutting down SSE server transport");

        // Abort the server task if it exists
        let server_handle_opt = {
            let mut server_handle_guard = self.server_handle.write().await;
            server_handle_guard.take()
        };

        if let Some(handle) = server_handle_opt {
            handle.abort();
        }

        tracing::info!("SSE server transport closed");

        Ok(())
    }
}

#[async_trait]
impl Transport for SseServerTransport {
    async fn start(&mut self) -> Result<(), Error> {
        // Call the SseServerTransport's start method
        SseServerTransport::start(self).await
    }

    async fn close(&mut self) -> Result<(), Error> {
        // Call the SseServerTransport's close method
        SseServerTransport::close(self).await
    }

    async fn is_connected(&self) -> bool {
        // Consider the SSE transport connected if it has a server handle
        let handle_guard = self.server_handle.read().await;
        handle_guard.is_some()
    }

    async fn send_to(&mut self, client_id: &str, message: &JSONRPCMessage) -> Result<(), Error> {
        // Get the app state to access the session store
        let app_state = match self.get_app_state().await {
            Some(state) => state,
            None => {
                return Err(Error::Protocol("No app state available".to_string()));
            }
        };

        // Find the session by client ID using the store from app state
        if let Some(session) = app_state.session_store.find_session_by_client_id(client_id).await {
            // Serialize the message to JSON
            let json = serde_json
                ::to_string(message)
                .map_err(|e| Error::Protocol(format!("Failed to serialize message: {}", e)))?;

            // Send the message to the client
            session
                .send_message(json).await
                .map_err(|e|
                    Error::Protocol(
                        format!("Failed to send message to client {}: {}", client_id, e)
                    )
                )?;

            tracing::debug!("Message sent to client {}", client_id);
            Ok(())
        } else {
            Err(Error::Protocol(format!("Client with ID {} not found", client_id)))
        }
    }
    /// Set the app state
    /// fn set_app_state(&mut self, app_state: Arc<AppState>);
    async fn set_app_state(&mut self, state: Arc<crate::server::server::AppState>) {
        let mut guard = self.app_state.write().await;
        *guard = Some(state);
    }
}

/// Handle an SSE connection
async fn handle_sse_connection(
    Extension(store): Extension<Arc<ClientSessionStore>>,
    State(state): State<Arc<crate::server::server::AppState>>
) -> impl IntoResponse {
    // Generate session ID
    let session_id = uuid::Uuid::new_v4().to_string();

    // Add more detailed logging
    tracing::info!("New SSE connection established! session_id = {}", session_id);

    // Create the session-specific message endpoint URI
    let session_uri = format!("/message?session_id={}", session_id);

    // Get or create a session using the supplied store
    let mut session = store.get_or_create_session(Some(session_id.clone())).await;
    tracing::info!("Created and stored new session: {}", session_id);

    // Create the SSE stream
    let stream =
        async_stream::stream! {
        // Send the endpoint URL as the first event
        let endpoint_event = Event::default().event("endpoint").data(session_uri.clone());
        yield Ok::<_, Infallible>(endpoint_event);
        tracing::debug!("Sent endpoint event for session: {}", session_id);

        let keep_alive_interval = Duration::from_secs(30);
        let mut interval = tokio::time::interval(keep_alive_interval);

        // Create a new channel for this session
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Set the message sender in the session
        session.set_message_sender(tx);
        store.store_session(session).await;
        tracing::debug!("Stored session with sender: {}", session_id);

        // Send a welcome message to indicate successful connection
        yield Ok::<_, Infallible>(Event::default().event("connected").data("Connection established"));

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    if let Some(message) = msg {
                        tracing::trace!("Sending message to client: {}", session_id);
                        yield Ok::<_, Infallible>(Event::default().data(message));
                    } else {
                        // Channel closed, end the stream
                        tracing::debug!("Channel closed for session: {}", session_id);
                        break;
                    }
                }
                _ = interval.tick() => {
                    // Send keep-alive event
                    tracing::trace!("Sending keep-alive to: {}", session_id);
                    yield Ok::<_, Infallible>(Event::default().event("keep-alive").data(""));
                }
            }
        }

        // Clean up the session when the connection is closed
        if let Some(_) = store.remove_session(&session_id).await {
            tracing::debug!("Removed session: {}", session_id);
        } else {
            tracing::warn!("Session not found for removal: {}", session_id);
        }
    };

    // Set Content-Type and return the stream
    Sse::new(stream).keep_alive(
        KeepAlive::new().interval(Duration::from_secs(30)).text("keep-alive")
    )
}

/// Handle a client message
async fn handle_client_message(
    Extension(store): Extension<Arc<ClientSessionStore>>,
    Extension(mut session): Extension<ClientSession>,
    State(state): State<Arc<crate::server::server::AppState>>,
    body: String
) -> Result<String, StatusCode> {
    // Set up detailed error logging
    tracing::info!(
        "Handling client message from session: {} (client_id: {:?})",
        session.session_id,
        session.client_id
    );

    // Log all request components for debugging
    tracing::debug!("Request body: {}", body);
    tracing::debug!(
        "Session data: session_id={}, client_id={:?}",
        session.session_id,
        session.client_id
    );

    // Parse the incoming message as a JSON-RPC message
    let message: JSONRPCMessage = match serde_json::from_str(&body) {
        Ok(msg) => {
            tracing::debug!("Successfully parsed JSON-RPC message: {:?}", msg);
            msg
        }
        Err(e) => {
            tracing::error!("Failed to parse JSON-RPC message: {}, body: {}", e, body);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Get client_id from session or generate one if not present
    if session.client_id.is_none() {
        // First message from this session, generate a client ID
        let id = uuid::Uuid::new_v4().to_string();
        session.client_id = Some(id.clone());
        tracing::info!("Assigned client_id {} to session {}", id, session.session_id);

        // Update the session in the store
        store.store_session(session.clone()).await;
        tracing::debug!("Updated session in store with new client_id");
    }

    tracing::info!(
        "Processing message from client: {:?}, session ID: {}",
        session.client_id,
        session.session_id
    );

    // Wrap in a closure to handle any errors the same way
    let handle_result = state.route_handler.as_ref().handle_message(message, &session).await;

    // Process the message with the handler
    match handle_result {
        Ok(Some(response)) => {
            // Send the response back to the client
            tracing::debug!("Got response from handler: {:?}", response);
            match serde_json::to_string(&response) {
                Ok(json) => {
                    tracing::debug!("Serialized response: {}", json);
                    match session.send_message(json.clone()).await {
                        Ok(_) => {
                            return Ok(json);
                        }
                        Err(e) => {
                            tracing::error!("Failed to send message to client: {}", e);
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        }
                    };
                }
                Err(e) => {
                    tracing::error!("Failed to serialize response: {}", e);

                    // Create a proper JSON-RPC error response
                    let error_json = format!(
                        r#"{{"jsonrpc":"2.0","error":{{"code":-32700,"message":"Failed to serialize response: {}"}}}}"#,
                        e.to_string().replace('"', "\\\"")
                    );

                    return Ok(error_json);
                }
            }
        }
        Ok(None) => {
            // No response needed
            tracing::debug!("No response needed for client");
            return Ok("".to_string());
        }
        Err(e) => {
            tracing::error!("Error processing message from client: {}", e);

            // Return a more helpful error message
            let error_json = format!(
                r#"{{"jsonrpc":"2.0","error":{{"code":-32000,"message":"{}"}}}}"#,
                e.to_string().replace('"', "\\\"")
            );

            return Ok(error_json);
        }
    }
}

// Add a simple status handler to verify the server is running
async fn handle_status() -> impl IntoResponse {
    tracing::info!("Status endpoint called");
    "MCP SSE Server is running"
}
