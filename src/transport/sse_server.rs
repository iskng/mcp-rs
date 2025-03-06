//! Server-side implementation of the SSE transport
//!
//! This module provides an SSE server implementation that:
//! - Accepts SSE connections from clients at the `/sse` endpoint
//! - Accepts HTTP POST requests from clients at the `/message` endpoint
//! - Routes messages between clients and the MCP server

use crate::errors::Error;
use crate::types::protocol::Message;
use crate::transport::connection_manager::ConnectionManager;
pub use crate::transport::connection_manager::MessageEventHandler;
use async_stream;

use axum::{
    Router,
    extract::{ Query, State },
    http::StatusCode,
    response::{ IntoResponse, Sse, sse::Event },
    routing::{ get, post },
};

use http::Method;
use serde_json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{ Duration, SystemTime };
use tokio::sync::mpsc;
use tower_http::cors::{ Any, CorsLayer };
use uuid;
use crate::transport::TransportMessageHandler;

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

/// Message processing event types for the event-based architecture
#[derive(Debug, Clone)]
pub enum MessageEvent {
    /// A new message was received from a client
    MessageReceived(Option<String>, Message),

    /// A message was sent to a client
    MessageSent(String, Message),

    /// A message processing error occurred
    MessageError(Option<String>, Error),
}

/// Application state for the SSE server
struct AppState {
    /// Connection manager
    connection_manager: Arc<ConnectionManager>,

    /// Authentication token (if required)
    auth_token: Option<String>,

    /// Whether authentication is required
    require_auth: bool,

    /// Allowed origins for CORS
    allowed_origins: Option<Vec<String>>,

    /// Server reference for direct handler calls
    server: Option<Arc<dyn TransportMessageHandler + Send + Sync>>,
}

impl AppState {
    fn new(
        connection_manager: Arc<ConnectionManager>,
        auth_token: Option<String>,
        require_auth: bool,
        allowed_origins: Option<Vec<String>>
    ) -> Self {
        Self {
            connection_manager,
            auth_token,
            require_auth,
            allowed_origins,
            server: None,
        }
    }
}

/// Create a transport error
fn transport_error<S: Into<String>>(message: S) -> Error {
    Error::Transport(message.into())
}

/// Server-side implementation of the SSE transport
#[derive(Clone)]
pub struct SseServerTransport {
    /// Options for SSE server
    options: SseServerOptions,

    /// Connection manager
    connection_manager: Arc<ConnectionManager>,

    /// Server socket address
    server_addr: Option<SocketAddr>,

    /// Application state
    app_state: Arc<tokio::sync::RwLock<Option<Arc<AppState>>>>,

    /// Message handler (if registered)
    message_handler: Option<Arc<dyn TransportMessageHandler + Send + Sync>>,

    /// Server task handle
    server_handle: Arc<tokio::sync::RwLock<Option<tokio::task::JoinHandle<Result<(), Error>>>>>,
}

impl SseServerTransport {
    /// Create a new SSE server transport with the given options
    pub fn new(options: SseServerOptions) -> Self {
        let connection_timeout = options.connection_timeout;
        Self {
            options: options.clone(),
            connection_manager: Arc::new(ConnectionManager::new(connection_timeout)),
            server_addr: None,
            app_state: Arc::new(tokio::sync::RwLock::new(None)),
            message_handler: None,
            server_handle: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Get the current app state
    async fn get_app_state(&self) -> Option<Arc<AppState>> {
        let guard = self.app_state.read().await;
        guard.clone()
    }

    /// Set the app state
    async fn set_app_state(&self, state: Arc<AppState>) {
        let mut guard = self.app_state.write().await;
        *guard = Some(state);
    }

    /// Register message handler
    pub fn register_message_handler<H>(&mut self, handler: H)
        where H: TransportMessageHandler + Send + Sync + 'static
    {
        self.message_handler = Some(Arc::new(handler));
    }

    /// Register server message handler
    pub async fn configure_server(
        &mut self,
        server: Arc<dyn TransportMessageHandler + Send + Sync>
    ) {
        tracing::info!("Configuring server handler");

        // Store it for direct access
        self.message_handler = Some(server.clone());

        // Update app state if it exists, or create it if it doesn't
        if let Some(app_state) = self.get_app_state().await {
            // Create a new AppState with the server included
            let new_app_state = Arc::new(AppState {
                connection_manager: app_state.connection_manager.clone(),
                auth_token: app_state.auth_token.clone(),
                require_auth: app_state.require_auth,
                allowed_origins: app_state.allowed_origins.clone(),
                server: Some(server),
            });

            // Replace the old app state
            self.set_app_state(new_app_state).await;
        } else {
            // Create a new AppState
            let new_app_state = Arc::new(
                AppState::new(
                    self.connection_manager.clone(),
                    self.options.auth_token.clone(),
                    self.options.require_auth,
                    self.options.allowed_origins.clone()
                )
            );

            // Update the AppState with the server
            let app_state_with_server = Arc::new(AppState {
                connection_manager: new_app_state.connection_manager.clone(),
                auth_token: new_app_state.auth_token.clone(),
                require_auth: new_app_state.require_auth,
                allowed_origins: new_app_state.allowed_origins.clone(),
                server: Some(server),
            });

            // Set the new app state
            self.set_app_state(app_state_with_server).await;
            tracing::info!("Created new AppState during server configuration");
        }
    }

    /// Start the SSE server transport
    pub async fn start(&mut self) -> Result<(), Error> {
        if self.server_handle.read().await.is_some() {
            // Already started
            tracing::warn!("SSE server already started");
            return Ok(());
        }

        let addr_str = self.options.bind_address.clone();
        let addr = addr_str.parse::<SocketAddr>().map_err(|e| {
            tracing::error!("Failed to parse bind address: {}", e);
            transport_error(format!("Invalid bind address: {}", e))
        })?;

        self.server_addr = Some(addr);

        // Create a new connection manager
        let connection_manager = self.connection_manager.clone();

        // Get message handler
        let message_handler = self.message_handler.clone();

        // Create app state
        let app_state = Arc::new(AppState {
            connection_manager: connection_manager.clone(),
            auth_token: self.options.auth_token.clone(),
            require_auth: self.options.require_auth,
            allowed_origins: self.options.allowed_origins.clone(),
            server: message_handler,
        });

        tracing::debug!("Created new AppState for server start");

        // Update the app state
        self.set_app_state(app_state.clone()).await;

        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
            .allow_origin(Any);

        let app = Router::new()
            .route("/sse", get(handle_sse_connection))
            .route("/message", post(handle_client_message))
            .layer(cors)
            .with_state(app_state);

        let server_addr = addr.clone();
        let server_handle = tokio::spawn(async move {
            tracing::info!("Starting SSE server on {}", addr_str);
            let listener = tokio::net::TcpListener
                ::bind(server_addr).await
                .map_err(|e| {
                    transport_error(format!("Failed to bind to {}: {}", server_addr, e))
                })?;

            axum
                ::serve(listener, app.into_make_service()).await
                .map_err(|e| { transport_error(format!("Server error: {}", e)) })?;

            Ok(())
        });

        let mut handle = self.server_handle.write().await;
        *handle = Some(server_handle);

        Ok(())
    }

    /// Close the transport
    pub async fn close(&mut self) -> Result<(), Error> {
        tracing::info!("Closing SSE server transport");

        // Abort the server task if it exists
        let server_handle_opt = {
            let mut server_handle_guard = self.server_handle.write().await;
            server_handle_guard.take()
        };

        if let Some(handle) = server_handle_opt {
            handle.abort();
        }

        // Shutdown the connection manager
        self.connection_manager.shutdown().await?;

        tracing::info!("SSE server transport closed");

        Ok(())
    }
}

/// Handle an SSE connection
async fn handle_sse_connection(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Generate client ID and session ID
    let client_id = uuid::Uuid::new_v4().to_string();
    let session_id = uuid::Uuid::new_v4().to_string();

    tracing::info!("New SSE connection: client_id = {client_id}, session_id = {session_id}");

    // Create the session-specific message endpoint URI
    let session_uri = format!("/message?session_id={}", session_id);

    // Register client with connection manager
    let client = state.connection_manager.register_client(client_id.clone(), session_id.clone());

    // Create the SSE stream
    let stream =
        async_stream::stream! {
        let endpoint_event = Event::default().event("endpoint").data(session_uri.clone());
        yield Ok::<_, Infallible>(endpoint_event);

        let keep_alive_interval = Duration::from_secs(30);
        let mut interval = tokio::time::interval(keep_alive_interval);
        let mut message_rx = client.message_sender.subscribe();

        loop {
            tokio::select! {
                msg = message_rx.recv() => {
                    match msg {
                        Ok(message) => {
                            let json_data = serde_json::to_string(&message).unwrap_or_else(|e| format!("{{\"error\":\"Serialization error: {}\"}}", e));
                            let event = Event::default().event("message").data(json_data);
                            yield Ok::<_, Infallible>(event);
                        }
                        Err(e) => {
                            tracing::error!("Error receiving client message: {}", e);
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    let event = Event::default().event("keep-alive").data(format!("{{\"time\":{}}}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs()));
                    yield Ok::<_, Infallible>(event);
                    state.connection_manager.update_client_activity(&client_id);
                }
            }
        }
    };

    Sse::new(stream).into_response()
}

/// Handle a message from a client
async fn handle_client_message(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
    body: String
) -> Result<String, StatusCode> {
    // Extract session_id from query params
    let session_id = params.get("session_id").ok_or(StatusCode::BAD_REQUEST)?;

    // Get client by session ID
    let client = state.connection_manager
        .get_client_by_session(session_id).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Update activity and await the result
    state.connection_manager.update_client_activity(&client.client_id);

    let message = serde_json::from_str::<Message>(&body).map_err(|e| {
        tracing::error!("Failed to parse message: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Process the message with the server handler if available
    if let Some(server) = &state.server {
        match server.handle_message(&client.client_id, &message).await {
            Ok(Some(response)) => {
                // Got a response, send it directly to the client
                if
                    let Some(client_obj) = state.connection_manager.get_client(
                        &client.client_id
                    ).await
                {
                    if let Err(e) = client_obj.message_sender.send(response) {
                        tracing::error!("Failed to send response to client: {}", e);
                    }
                }
            }
            Ok(None) => {
                // No response needed
                tracing::debug!("No response needed for message from client {}", client.client_id);
            }
            Err(e) => {
                tracing::error!("Error processing message from client {}: {}", client.client_id, e);
            }
        }
    } else {
        tracing::error!("No server handler registered");
    }

    // Generate unique ID for acknowledgement
    let id = uuid::Uuid::new_v4().to_string();
    Ok(format!("{{\"id\":\"{}\"}}", id))
}
