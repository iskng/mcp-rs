//! Connection manager for SSE server transport
//!
//! This module provides the connection management functionality for the SSE server,
//! handling client connections, sessions, and message routing.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, SystemTime };
use std::sync::atomic::{ AtomicU64, Ordering };

use tokio::sync::{ broadcast, RwLock };

use crate::errors::Error;
use crate::types::protocol::Message;
use crate::transport::sse_server::{ ConnectionEvent, MessageEvent };

/// Gets the current time in milliseconds
fn now_millis() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Connection event handler type
pub type ConnectionEventHandler = Box<dyn Fn(ConnectionEvent) + Send + Sync + 'static>;

/// Message event handler type
pub type MessageEventHandler = Box<
    dyn (Fn(MessageEvent) -> Result<(), Error>) + Send + Sync + 'static
>;

/// Client connection information
pub struct ClientConnection {
    /// Client ID
    pub client_id: String,
    /// Session ID
    pub session_id: String,
    /// Connection time
    pub connected_at: SystemTime,
    /// Last activity timestamp (stored as millis since epoch for atomic updates)
    pub last_activity: Arc<AtomicU64>,
    /// Message sender channel for sending events to the client
    pub message_sender: broadcast::Sender<Message>,
    /// Client properties
    pub properties: HashMap<String, String>,
}

impl Clone for ClientConnection {
    fn clone(&self) -> Self {
        Self {
            client_id: self.client_id.clone(),
            session_id: self.session_id.clone(),
            connected_at: self.connected_at,
            last_activity: self.last_activity.clone(),
            message_sender: self.message_sender.clone(),
            properties: self.properties.clone(),
        }
    }
}

/// Connection manager for the SSE server
pub struct ConnectionManager {
    /// Primary map - client_id to client connection
    connections: Arc<RwLock<HashMap<String, ClientConnection>>>,
    /// Session lookup - session_id to client_id mapping
    sessions: Arc<RwLock<HashMap<String, String>>>,
    /// Connection timeout
    connection_timeout: Duration,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(connection_timeout: Duration) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            connection_timeout,
        }
    }

    /// Register a client
    pub fn register_client(&self, client_id: String, session_id: String) -> ClientConnection {
        // Create message channel
        let (sender, _) = broadcast::channel(100);

        // Create client connection
        let client = ClientConnection {
            client_id: client_id.clone(),
            session_id: session_id.clone(),
            connected_at: SystemTime::now(),
            last_activity: Arc::new(AtomicU64::new(now_millis())),
            message_sender: sender,
            properties: HashMap::new(),
        };

        let client_clone = client.clone();
        let self_clone = self.clone();
        let client_id_clone = client_id.clone();

        // Store client and session mapping
        tokio::spawn(async move {
            let mut connections = self_clone.connections.write().await;
            connections.insert(client_id_clone.clone(), client_clone);

            let mut sessions = self_clone.sessions.write().await;
            sessions.insert(session_id, client_id_clone.clone());
        });

        client
    }

    /// Get a client by ID
    pub async fn get_client(&self, client_id: &str) -> Option<ClientConnection> {
        let connections = self.connections.read().await;
        connections.get(client_id).cloned()
    }

    /// Get a client by session ID
    pub async fn get_client_by_session(&self, session_id: &str) -> Option<ClientConnection> {
        let sessions = self.sessions.read().await;
        if let Some(client_id) = sessions.get(session_id) {
            let connections = self.connections.read().await;
            connections.get(client_id).cloned()
        } else {
            None
        }
    }

    /// Update client activity
    pub fn update_client_activity(&self, client_id: &str) {
        let self_clone = self.clone();
        let client_id = client_id.to_string();

        tokio::spawn(async move {
            if let Some(client) = self_clone.get_client(&client_id).await {
                client.last_activity.store(now_millis(), Ordering::SeqCst);
            }
        });
    }

    /// Unregister a client
    pub async fn unregister_client(&self, client_id: &str) -> Result<(), Error> {
        // Get client
        let client = match self.get_client(client_id).await {
            Some(client) => client,
            None => {
                return Err(Error::Transport(format!("Client not found: {}", client_id)));
            }
        };

        let session_id = client.session_id.clone();

        // Remove from connections and sessions
        {
            let mut connections = self.connections.write().await;
            connections.remove(client_id);

            let mut sessions = self.sessions.write().await;
            sessions.remove(&session_id);
        }

        Ok(())
    }

    /// Check if a client is connected
    pub async fn is_client_connected(&self, client_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(client_id)
    }

    /// Get a list of connected client IDs
    pub async fn connected_clients(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Run maintenance tasks
    pub async fn run_maintenance(&self) {
        let now = now_millis();
        let timeout_millis = self.connection_timeout.as_millis() as u64;
        let mut clients_to_remove = Vec::new();

        // Check for inactive clients
        {
            let connections = self.connections.read().await;
            for (client_id, client) in connections.iter() {
                let last_activity = client.last_activity.load(Ordering::SeqCst);
                if now - last_activity > timeout_millis {
                    clients_to_remove.push(client_id.clone());
                }
            }
        }

        // Remove inactive clients
        for client_id in clients_to_remove {
            let _ = self.unregister_client(&client_id).await;
        }
    }

    /// Shutdown and clean up all connections
    pub async fn shutdown(&self) -> Result<(), Error> {
        // Get all clients
        let client_ids = self.connected_clients().await;

        // Disconnect each client
        for client_id in client_ids {
            let _ = self.unregister_client(&client_id).await;
        }

        Ok(())
    }

    /// Clone the connection manager
    pub fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            sessions: self.sessions.clone(),
            connection_timeout: self.connection_timeout,
        }
    }
}
