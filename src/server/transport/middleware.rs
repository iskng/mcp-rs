use axum::http::Request;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{Mutex, mpsc};
use tower::{Layer, Service};
use uuid::Uuid;

/// Represents a client session with its message channel
#[derive(Debug, Clone)]
pub struct ClientSession {
    /// Unique identifier for this session
    pub session_id: String,
    /// Optional client identifier (assigned by application)
    pub client_id: Option<String>,
    /// Sender for messages to this client
    message_sender: Option<Arc<mpsc::Sender<String>>>,
    /// Additional session data
    data: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl ClientSession {
    /// Create a new client session with a generated session ID
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            client_id: None,
            message_sender: None,
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new client session with a specific session ID
    pub fn with_session_id(session_id: String) -> Self {
        Self {
            session_id,
            client_id: None,
            message_sender: None,
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the client ID for this session
    pub fn set_client_id(&mut self, client_id: String) {
        self.client_id = Some(client_id);
    }

    /// Set the message sender for this session
    pub fn set_message_sender(&mut self, sender: mpsc::Sender<String>) {
        self.message_sender = Some(Arc::new(sender));
    }

    /// Send a message to this client
    pub async fn send_message(&self, message: impl Into<String>) -> Result<(), String> {
        match &self.message_sender {
            Some(sender) => sender.send(message.into()).await.map_err(|e| e.to_string()),
            None => Err("No message sender configured for this session".to_string()),
        }
    }

    //TODO: We should validate before sending
    /// Send a JSON-serializable message to this client
    pub async fn send_json<T: serde::Serialize>(&self, message: &T) -> Result<(), String> {
        let json = serde_json::to_string(message).map_err(|e| e.to_string())?;
        self.send_message(json).await
    }

    /// Store a value in the session
    pub async fn set_value<T: serde::Serialize>(
        &self,
        key: &str,
        value: T,
    ) -> Result<(), serde_json::Error> {
        let value_json = serde_json::to_value(value)?;
        let mut data = self.data.lock().await;
        data.insert(key.to_string(), value_json);
        Ok(())
    }

    /// Retrieve a value from the session
    pub async fn get_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let data = self.data.lock().await;
        data.get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
}

/// Store for managing client sessions
#[derive(Debug, Clone, Default)]
pub struct ClientSessionStore {
    sessions: Arc<Mutex<HashMap<String, ClientSession>>>,
}

impl ClientSessionStore {
    /// Create a new empty session store
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a session by ID, or create one if it doesn't exist
    pub async fn get_or_create_session(&self, session_id: Option<String>) -> ClientSession {
        let mut sessions = self.sessions.lock().await;

        match session_id {
            Some(id) => {
                if let Some(session) = sessions.get(&id) {
                    session.clone()
                } else {
                    let session = ClientSession::with_session_id(id.clone());
                    sessions.insert(id, session.clone());
                    session
                }
            }
            None => {
                let session = ClientSession::new();
                sessions.insert(session.session_id.clone(), session.clone());
                session
            }
        }
    }

    /// Store a session
    pub async fn store_session(&self, session: ClientSession) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session.session_id.clone(), session);
    }

    /// Remove a session by ID
    pub async fn remove_session(&self, session_id: &str) -> Option<ClientSession> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id)
    }

    /// Find a session by client ID
    pub async fn find_session_by_client_id(&self, client_id: &str) -> Option<ClientSession> {
        let sessions = self.sessions.lock().await;

        // Find the first session with the matching client ID
        for session in sessions.values() {
            if session.client_id.as_deref() == Some(client_id) {
                return Some(session.clone());
            }
        }

        None
    }
}

/// Layer that adds session management to routes
#[derive(Clone)]
pub struct ClientSessionLayer {
    store: Arc<ClientSessionStore>,
}

impl ClientSessionLayer {
    /// Create a new session layer with default store
    pub fn new() -> Self {
        Self {
            store: Arc::new(ClientSessionStore::new()),
        }
    }

    /// Create a new session layer with a specific store
    pub fn with_store(store: impl Into<Arc<ClientSessionStore>>) -> Self {
        Self {
            store: store.into(),
        }
    }
}

impl<S> Layer<S> for ClientSessionLayer {
    type Service = SessionMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        SessionMiddleware {
            service,
            store: self.store.clone(),
        }
    }
}

/// Middleware that processes sessions
///
/// This middleware:
/// 1. Extracts the session ID from query parameters
/// 2. Retrieves or creates the session using its internal store
/// 3. Adds the session to request Extensions for handlers to access
/// 4. Persists any changes to the session after the request completes
#[derive(Clone)]
pub struct SessionMiddleware<S> {
    service: S,
    store: Arc<ClientSessionStore>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for SessionMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        // Clone to move into the future
        let mut service = self.service.clone();
        let store = self.store.clone();

        Box::pin(async move {
            // Log the request URI for debugging
            tracing::debug!("Session middleware processing request to: {}", req.uri());

            // Extract session_id from query parameters
            let session_id = req.uri().query().and_then(|q| {
                tracing::trace!("Parsing query parameters: {}", q);
                url::form_urlencoded::parse(q.as_bytes())
                    .find(|(key, _)| key == "session_id")
                    .map(|(_, value)| value.to_string())
            });

            tracing::debug!(
                "Session middleware: processing request with session_id: {:?}",
                session_id
            );

            // Get or create session from our internal store
            let session = if let Some(id) = session_id {
                store.get_or_create_session(Some(id)).await
            } else {
                // No session_id in query, check if there's a session in request already
                if req.extensions().get::<ClientSession>().is_none() {
                    tracing::debug!(
                        "No session_id in query and no session in request, creating new session"
                    );
                    store.get_or_create_session(None).await
                } else {
                    tracing::debug!(
                        "Session already in request extensions, using existing session"
                    );
                    req.extensions().get::<ClientSession>().unwrap().clone()
                }
            };

            tracing::debug!("Using session with ID: {}", session.session_id);

            // Add session to request extensions
            req.extensions_mut().insert(session.clone());

            // Also add the store to extensions for handlers that need it
            req.extensions_mut().insert(store.clone());

            // Process the request with the inner service
            tracing::debug!("Forwarding request to inner service");
            let response = service.call(req).await?;

            // Store any updates to the session
            store.store_session(session).await;
            tracing::debug!("Session stored after request completion");

            Ok(response)
        })
    }
}

/// Extension trait for Request to easily access the session
pub trait RequestSessionExt {
    /// Get the client session from the request
    fn session(&self) -> Option<ClientSession>;
}

impl<B> RequestSessionExt for Request<B> {
    fn session(&self) -> Option<ClientSession> {
        self.extensions().get::<ClientSession>().cloned()
    }
}
