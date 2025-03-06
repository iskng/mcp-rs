//! Server-side WebSocket transport implementation using Axum
//!
//! This module provides a WebSocket server implementation built on Axum that:
//! - Accepts WebSocket connections from clients at the configured endpoint
//! - Routes messages between clients and the MCP server
//! - Supports bidirectional real-time communication
//! - Provides client tracking and management

use crate::errors::Error;
use crate::types::protocol::Message;
use crate::transport::Transport;
use async_trait::async_trait;
use axum::{
    Extension,
    Router,
    extract::ws::{ Message as AxumWsMessage, WebSocket, WebSocketUpgrade },
    response::IntoResponse,
    routing::get,
};
use futures_util::{ SinkExt, StreamExt };
use http::HeaderValue;
use scopeguard;
use serde_json::{ self };
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, SystemTime };
use tokio::sync::{ broadcast, mpsc, oneshot };
use tower_http::cors::{ Any, CorsLayer };
use uuid::Uuid;

/// Maximum number of concurrent clients
const MAX_CLIENTS: usize = 100;

/// Default connection timeout
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);
/// Default heartbeat interval
const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
/// Default reconnection delay
const DEFAULT_RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Configuration options for the WebSocket server
#[derive(Debug, Clone)]
pub struct WebSocketServerOptions {
    /// Address to bind the server to
    pub bind_address: SocketAddr,
    /// Path for the WebSocket endpoint
    pub websocket_path: String,
    /// Optional authentication token to validate requests
    pub auth_token: Option<String>,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Whether to require authentication
    pub require_auth: bool,
    /// CORS allowed origins
    pub allowed_origins: Option<Vec<String>>,
}

impl Default for WebSocketServerOptions {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:9000".parse().unwrap(),
            websocket_path: "/ws".to_string(),
            auth_token: None,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            require_auth: false,
            allowed_origins: None,
        }
    }
}

/// A client connection
#[derive(Debug)]
struct ClientConnection {
    id: String,
    connected_at: SystemTime,
    last_activity: SystemTime,
    sender: broadcast::Sender<String>, // Using String to avoid Clone bound issues
}

/// Shared application state for WebSocket server
#[derive(Clone)]
struct AppState {
    clients: Arc<Mutex<HashMap<String, ClientConnection>>>,
    message_tx: mpsc::Sender<(String, Result<Message, Error>)>,
}

/// Server-side implementation of the WebSocket transport using Axum
pub struct WebSocketServerTransport {
    /// Map of client IDs to connection info
    clients: Arc<Mutex<HashMap<String, ClientConnection>>>,
    /// Channel for incoming messages from clients
    message_rx: mpsc::Receiver<(String, Result<Message, Error>)>,
    /// Channel for sending messages to specific clients
    message_tx: mpsc::Sender<(String, Result<Message, Error>)>,
    /// Channel to notify when the server should shut down
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Server options
    options: WebSocketServerOptions,
    /// Server handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Is the transport connected
    connected: bool,
}

impl WebSocketServerTransport {
    /// Create a new WebSocket server transport with default options
    pub fn new() -> Self {
        Self::with_options(WebSocketServerOptions::default())
    }

    /// Create a new WebSocket server transport with the specified options
    pub fn with_options(options: WebSocketServerOptions) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);

        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            message_rx,
            message_tx,
            shutdown_tx: None,
            options,
            server_handle: None,
            connected: false,
        }
    }

    /// Start the WebSocket server
    pub async fn start(&mut self) -> Result<(), Error> {
        if self.connected {
            return Err(Error::Transport("WebSocket server already running".to_string()));
        }

        // Notify Starting lifecycle event
        tracing::info!("WebSocket server transport starting");

        // Create the app state
        let app_state = AppState {
            clients: self.clients.clone(),
            message_tx: self.message_tx.clone(),
        };

        // Create a CORS layer
        let cors = match &self.options.allowed_origins {
            Some(origins) => {
                let mut layer = CorsLayer::new();
                for origin in origins {
                    layer = layer.allow_origin(origin.parse::<HeaderValue>().unwrap());
                }
                layer.allow_methods(Any).allow_headers(Any).max_age(Duration::from_secs(86400))
            }
            None => CorsLayer::permissive(),
        };

        // Build the router
        let app = Router::new()
            .route(&self.options.websocket_path, get(ws_handler))
            .layer(Extension(app_state))
            .layer(cors);

        // Create a shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // Start the server
        let bind_address = self.options.bind_address;
        let server_handle = tokio::spawn(async move {
            tracing::info!("Starting WebSocket server on {}", bind_address);

            // Build our application with the specified routes
            let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
            tracing::info!("Listening on {}", bind_address);

            // Start the Axum server
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                    tracing::info!("WebSocket server shutting down");
                }).await
                .unwrap_or_else(|e| {
                    tracing::error!("Server error: {}", e);
                })
        });

        self.server_handle = Some(server_handle);
        self.connected = true;
        tracing::info!(
            "WebSocket server started at ws://{}{}",
            self.options.bind_address,
            self.options.websocket_path
        );

        // Notify Started lifecycle event
        tracing::info!("WebSocket server transport started");

        Ok(())
    }

    /// Get the number of connected clients
    pub fn connected_clients(&self) -> usize {
        self.clients.lock().unwrap().len()
    }
}

/// WebSocket connection handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>
) -> impl IntoResponse {
    // Upgrade the connection to WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let clients = state.clients.clone();
    let message_tx = state.message_tx.clone();

    // Check if we've reached maximum clients
    {
        let clients_map = clients.lock().unwrap();
        if clients_map.len() >= MAX_CLIENTS {
            // Too many clients connected
            tracing::warn!("Maximum client connections reached");
            return;
        }
    }

    // Generate a client ID
    let client_id = Uuid::new_v4().to_string();

    // Create broadcast channel for this client
    let (sender, _) = broadcast::channel(100);

    // Register the client
    {
        let mut clients_map = clients.lock().unwrap();
        clients_map.insert(client_id.clone(), ClientConnection {
            id: client_id.clone(),
            connected_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            sender: sender.clone(),
        });
    }

    tracing::info!("New WebSocket client connected: {}", client_id);

    // Split the socket
    let (mut sender_socket, mut receiver_socket) = socket.split();

    // Clone for cleanup
    let clients_for_cleanup = clients.clone();
    let client_id_for_cleanup = client_id.clone();

    // Ensure client is removed when connection is closed
    let _cleanup = scopeguard::guard((), move |_| {
        let mut clients_map = clients_for_cleanup.lock().unwrap();
        if clients_map.remove(&client_id_for_cleanup).is_some() {
            tracing::info!("WebSocket client disconnected: {}", client_id_for_cleanup);
        }
    });

    // Create receiver for messages to send to this client
    let mut client_receiver = {
        let clients_map = clients.lock().unwrap();
        if let Some(client) = clients_map.get(&client_id) {
            client.sender.subscribe()
        } else {
            return; // Client already disconnected
        }
    };

    // Process incoming messages from the client
    let client_id_clone = client_id.clone();
    let message_tx_clone = message_tx.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(result) = receiver_socket.next().await {
            match result {
                Ok(msg) => {
                    // Handle message from client
                    match msg {
                        AxumWsMessage::Text(text) => {
                            match serde_json::from_str::<Message>(&text) {
                                Ok(message) => {
                                    // Forward message to MCP server
                                    if
                                        message_tx_clone
                                            .send((client_id_clone.clone(), Ok(message))).await
                                            .is_err()
                                    {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse message: {}", e);
                                    // Send error to MCP server
                                    if
                                        message_tx_clone
                                            .send((
                                                client_id_clone.clone(),
                                                Err(Error::Json(e)),
                                            )).await
                                            .is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                        }
                        AxumWsMessage::Binary(data) => {
                            // Try to parse binary data as JSON message
                            if let Ok(text) = std::str::from_utf8(&data) {
                                match serde_json::from_str::<Message>(text) {
                                    Ok(message) => {
                                        if
                                            message_tx_clone
                                                .send((client_id_clone.clone(), Ok(message))).await
                                                .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to parse binary message: {}", e);
                                        if
                                            message_tx_clone
                                                .send((
                                                    client_id_clone.clone(),
                                                    Err(Error::Json(e)),
                                                )).await
                                                .is_err()
                                        {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        AxumWsMessage::Close(_) => {
                            break;
                        }
                        // Ignore other message types like Ping/Pong
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    // Forward messages from server to client
    let send_task = tokio::spawn(async move {
        while let Ok(json) = client_receiver.recv().await {
            if sender_socket.send(AxumWsMessage::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = receive_task => {}
        _ = send_task => {}
    }
}

// Implement the Transport trait for WebSocketServerTransport
#[async_trait]
impl Transport for WebSocketServerTransport {
    /// Start the transport - this initializes the websocket server
    async fn start(&mut self) -> Result<(), Error> {
        // Call the existing start method if we're not already connected
        if !self.connected {
            self.start().await?;
        }

        Ok(())
    }

    async fn receive(&mut self) -> Result<(Option<String>, Message), Error> {
        // Check if the server is running, if not return an error (start() should have been called)
        if !self.connected {
            return Err(
                Error::Transport(
                    "WebSocket server not started yet - call start() first".to_string()
                )
            );
        }

        // Wait for a message from the client using the message channel
        match self.message_rx.try_recv() {
            Ok((client_id, result)) => {
                match result {
                    Ok(message) => Ok((Some(client_id), message)),
                    Err(e) => Err(e),
                }
            }
            Err(_) => {
                // No message available, wait a bit and return a timeout error
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                Err(Error::Timeout("No messages available".to_string()))
            }
        }
    }

    async fn send_to(&mut self, client_id: &str, message: &Message) -> Result<(), Error> {
        if !self.connected {
            return Err(Error::Transport("WebSocket server not running".to_string()));
        }

        // Serialize the message
        let json = serde_json::to_string(message).map_err(Error::Json)?;

        // Find the specific client
        let clients_map = self.clients.lock().unwrap();
        if let Some(client) = clients_map.get(client_id) {
            // Send the message to only this client
            if client.sender.send(json).is_err() {
                return Err(
                    Error::Transport(format!("Failed to send message to client: {}", client_id))
                );
            }
            Ok(())
        } else {
            Err(Error::Transport(format!("Client not found: {}", client_id)))
        }
    }

    async fn send(&mut self, message: &Message) -> Result<(), Error> {
        if !self.connected {
            return Err(Error::Transport("WebSocket server not running".to_string()));
        }

        // Serialize the message
        let json = serde_json::to_string(message).map_err(Error::Json)?;

        // Send to all clients
        {
            let clients_map = self.clients.lock().unwrap();
            if clients_map.is_empty() {
                return Err(Error::Transport("No connected clients".to_string()));
            }

            // Broadcast to all clients
            for (_, client) in clients_map.iter() {
                let _ = client.sender.send(json.clone());
            }
        }

        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn close(&mut self) -> Result<(), Error> {
        if !self.connected {
            return Ok(());
        }

        // Notify Closing lifecycle event
        tracing::info!("WebSocket server transport closing");

        // Trigger server shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for server to shut down
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }

        // Clear all clients
        {
            let mut clients_map = self.clients.lock().unwrap();
            clients_map.clear();
        }

        self.connected = false;
        tracing::info!("WebSocket server stopped");

        // Notify Closed lifecycle event
        tracing::info!("WebSocket server transport closed");

        Ok(())
    }
}
