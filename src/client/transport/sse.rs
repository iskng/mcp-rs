//! SSE Client Transport
//!
//! This module implements the client-side transport for the SSE (Server-Sent Events)
//! protocol. It allows an MCP client to connect to an SSE server and receive real-time
//! updates via a persistent HTTP connection.

use async_trait::async_trait;
use eventsource_client::Client;
use futures_util::stream::StreamExt;
use log::{ error, info, trace, warn };
use reqwest::{ Client as HttpClient, header };
use tracing::debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{ Mutex, mpsc, oneshot };
use eventsource_client as es;
use futures::stream::Stream;
use futures::TryStreamExt;
use crate::client::transport::{ Transport, ConnectionStatus };
use tokio::sync::broadcast;

use crate::protocol::Error;
use crate::protocol::JSONRPCMessage;
use crate::client::transport::state::TransportStateChannel;

/// Default timeout for HTTP requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Default retry delay for reconnecting
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(2);
/// Buffer size for message channel
const CHANNEL_BUFFER_SIZE: usize = 100;

/// Types of events that can be received from the SSE stream
#[derive(Debug, Clone)]
enum SseEventType {
    /// Server is providing the message endpoint URL
    Endpoint(String),

    /// Server is confirming connection
    Connected(String),

    /// JSON-RPC message received
    Message(String),

    /// Error message received
    #[allow(dead_code)]
    Error(String),

    /// Keep-alive message
    KeepAlive,

    /// Unknown event type
    Unknown(String, String),
}

impl SseEventType {
    /// Create from EventSource event
    fn from_sse_event(event: es::Event) -> Self {
        match event.event_type.as_str() {
            "endpoint" => SseEventType::Endpoint(event.data),
            "connected" => SseEventType::Connected(event.data),
            "message" => SseEventType::Message(event.data),
            "error" => SseEventType::Error(event.data),
            "keep-alive" => SseEventType::KeepAlive,
            _ => SseEventType::Unknown(event.event_type, event.data),
        }
    }
}

/// Options for the SSE client transport
#[derive(Clone, Debug)]
pub struct SseOptions {
    /// Authentication token
    pub auth_token: Option<String>,
    /// Request timeout
    pub timeout: Duration,
    /// Retry delay for reconnection
    pub retry_delay: Duration,
    /// Maximum retry delay
    pub max_retry_delay: Duration,
    /// Backoff multiplier
    pub backoff_factor: u32,
    /// Custom headers
    pub custom_headers: Option<header::HeaderMap>,
    /// Client ID (auto-generated if None)
    pub client_id: Option<String>,
}

impl Default for SseOptions {
    fn default() -> Self {
        Self {
            auth_token: None,
            timeout: DEFAULT_TIMEOUT,
            retry_delay: DEFAULT_RETRY_DELAY,
            max_retry_delay: Duration::from_secs(60),
            backoff_factor: 2,
            custom_headers: None,
            client_id: None,
        }
    }
}

/// Client-side implementation of the SSE transport
pub struct SseTransport {
    /// URL for the SSE endpoint
    sse_url: String,
    /// HTTP client for sending messages (not for SSE)
    http_client: HttpClient,
    /// Channel for incoming messages
    rx: Arc<Mutex<mpsc::Receiver<JSONRPCMessage>>>,
    /// Sender for the message channel
    tx: Arc<mpsc::Sender<JSONRPCMessage>>,
    /// Options for the transport
    options: SseOptions,
    /// Transport state channel
    state: TransportStateChannel,
    /// Status broadcaster
    status_tx: Arc<broadcast::Sender<ConnectionStatus>>,
    /// Task handle for the SSE connection
    _task_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Shutdown signal sender
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl SseTransport {
    /// Create a new SSE transport
    pub async fn new(base_url: &str) -> Result<Self, Error> {
        Self::with_options(base_url, SseOptions::default()).await
    }

    /// Create a new SSE transport with custom options
    pub async fn with_options(base_url: &str, options: SseOptions) -> Result<Self, Error> {
        let sse_url = format!("{}/sse", base_url.trim_end_matches('/'));
        // No longer needed since we've removed the cached_messages_url field
        let _messages_url = format!("{}/message", base_url.trim_end_matches('/'));

        // Create the HTTP client with custom headers
        let mut headers = header::HeaderMap::new();

        // Add authorization if provided
        if let Some(token) = &options.auth_token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue
                    ::from_str(&format!("Bearer {}", token))
                    .map_err(|e| Error::Transport(format!("Invalid auth token: {}", e)))?
            );
        }

        // Add custom headers if provided
        if let Some(custom_headers) = &options.custom_headers {
            for (name, value) in custom_headers.iter() {
                headers.insert(name, value.clone());
            }
        }

        // Create the client
        let http_client = reqwest::Client
            ::builder()
            .default_headers(headers)
            .timeout(options.timeout)
            .build()
            .map_err(|e| Error::Transport(format!("Failed to create HTTP client: {}", e)))?;

        // Create a channel for messages
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        // Create a broadcast channel for status updates
        let (status_tx, _) = broadcast::channel(16);

        // Create the transport state channel
        let state = TransportStateChannel::new();

        Ok(Self {
            sse_url,
            http_client,
            rx: Arc::new(Mutex::new(rx)),
            tx: Arc::new(tx),
            options,
            state,
            status_tx: Arc::new(status_tx),
            _task_handle: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        })
    }

    /// Start a background task that connects to the SSE endpoint
    async fn start_sse_connection(&self) -> Result<(), Error> {
        // Create channels for event processing
        let (event_tx, event_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (processor_shutdown_tx, processor_shutdown_rx) = oneshot::channel();
        let (stream_shutdown_tx, stream_shutdown_rx) = oneshot::channel();

        // Store the shutdown sender
        let mut shutdown_guard = self.shutdown_tx.lock().await;
        *shutdown_guard = Some(stream_shutdown_tx);
        drop(shutdown_guard); // Release the lock explicitly

        // Create clones for the tasks
        let sse_url = self.sse_url.clone();
        let tx = self.tx.clone();
        let state = self.state.clone();
        let status_tx = self.status_tx.clone();

        // Start the event processing task
        let _process_handle = tokio::spawn(async move {
            process_sse_events(
                event_rx,
                tx,
                state,
                sse_url,
                processor_shutdown_rx,
                status_tx
            ).await;
        });

        // Create client builder
        let mut client_builder = es::ClientBuilder
            ::for_url(&self.sse_url)
            .map_err(|e| Error::Transport(format!("Invalid SSE URL: {}", e)))?;

        // Configure authentication if provided
        if let Some(token) = &self.options.auth_token {
            client_builder = client_builder
                .header("Authorization", &format!("Bearer {}", token))
                .map_err(|e|
                    Error::Transport(format!("Failed to add Authorization header: {}", e))
                )?;
        }

        // Add custom headers if provided
        if let Some(custom_headers) = &self.options.custom_headers {
            for (name, value) in custom_headers.iter() {
                client_builder = client_builder
                    .header(name.as_str(), value.to_str().unwrap_or(""))
                    .map_err(|e| Error::Transport(format!("Failed to add header: {}", e)))?;
            }
        }

        // Configure reconnection
        let reconnect_options = es::ReconnectOptions
            ::reconnect(true)
            .retry_initial(true)
            .delay(self.options.retry_delay)
            .backoff_factor(self.options.backoff_factor)
            .delay_max(self.options.max_retry_delay)
            .build();

        client_builder = client_builder.reconnect(reconnect_options);

        // Build the client
        let client = client_builder.build();

        // Clone values for the streaming task
        let event_tx_clone = event_tx.clone();
        let state_clone = self.state.clone();
        // Cannot clone oneshot::Sender, so create a new one
        let (stream_complete_tx, stream_complete_rx) = oneshot::channel();

        // Start the SSE streaming task
        let stream_handle = tokio::spawn(async move {
            let stream = client
                .stream()
                .map_err(|e| Error::Transport(format!("SSE stream error: {}", e)));

            // Only extract and forward events
            if
                let Err(e) = receive_sse_stream(
                    stream,
                    event_tx_clone,
                    state_clone.clone(),
                    stream_shutdown_rx
                ).await
            {
                error!("Error in SSE stream: {}", e);
            }

            // Signal the processor to shut down when stream ends
            let _ = stream_complete_tx.send(());
        });

        // Handle stream completion in a separate task
        tokio::spawn(async move {
            if stream_complete_rx.await.is_ok() {
                let _ = processor_shutdown_tx.send(());
            }
        });

        // Store the combined task handle
        let mut task_handle_guard = self._task_handle.lock().await;
        *task_handle_guard = Some(stream_handle);
        drop(task_handle_guard); // Release the lock explicitly

        Ok(())
    }
}

/// Receives events from the SSE stream and forwards them to the event channel
async fn receive_sse_stream<S>(
    mut stream: S,
    event_tx: mpsc::Sender<SseEventType>,
    state: TransportStateChannel,
    mut shutdown_rx: oneshot::Receiver<()>
) -> Result<(), Error>
    where S: Stream<Item = Result<es::SSE, Error>> + Unpin
{
    // Start with disconnected state
    state.reset();
    debug!("Initial connection status set to disconnected");

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                info!("Received shutdown signal, stopping SSE stream");
                // Update state to disconnected
                state.reset();
                let _ = event_tx.send(SseEventType::Error("SSE stream shutdown".to_string())).await;
                break;
            }

            next_result = stream.next() => {
                match next_result {
                    Some(Ok(sse_event)) => {
                        // Process the event based on its type
                        match sse_event {
                            // This is the connection established event
                            es::SSE::Connected(connection) => {
                                // Log connection established
                                info!("Connected to SSE endpoint: status={}", connection.response().status());
                                
                                // Update state
                                state.update(|s| {
                                    s.has_connected = true;
                                });

                                // Log the updated state for debugging
                                let current = state.current();
                                debug!("State after SSE connection: has_endpoint={}, has_connected={}, endpoint_url={:?}, session_id={:?}", 
                                    current.has_endpoint, current.has_connected, current.endpoint_url, current.session_id);
                                
                                // Send a Connected event to the processor
                                let _ = event_tx.send(SseEventType::Connected(format!("Connection established with status {}", connection.response().status()))).await;
                            },
                            // This is a regular SSE event
                            es::SSE::Event(event) => {
                                debug!("Received SSE event: type={}, data={}", event.event_type, event.data);
                                
                                // Convert directly from SSE event to our type
                                let event_type = SseEventType::from_sse_event(event);
                                
                                // Send the event to the processor
                                let _ = event_tx.send(event_type).await;
                            },
                            // This is a comment line in the SSE stream
                            es::SSE::Comment(comment) => {
                                debug!("Received SSE comment: {}", comment);
                                // We don't process comments
                            }
                        }
                    },
                    Some(Err(e)) => {
                        error!("Error in SSE stream: {}", e);
                        state.reset();
                        
                        // Log the updated state for debugging
                        let current = state.current();
                        debug!("State after SSE error: has_endpoint={}, has_connected={}", 
                            current.has_endpoint, current.has_connected);
                            
                        let _ = event_tx.send(SseEventType::Error(e.to_string())).await;
                        break;
                    },
                    None => {
                        info!("SSE stream ended");
                        state.reset();
                        
                        // Log the updated state for debugging
                        let current = state.current();
                        debug!("State after SSE stream end: has_endpoint={}, has_connected={}", 
                            current.has_endpoint, current.has_connected);
                            
                        let _ = event_tx.send(SseEventType::Error("SSE stream ended".to_string())).await;
                        break;
                    }
                }
            }
        }
    }

    // Make sure state is reset on exit
    state.reset();

    // Log the final state for debugging
    let current = state.current();
    debug!(
        "Final state on SSE stream exit: has_endpoint={}, has_connected={}",
        current.has_endpoint,
        current.has_connected
    );

    Ok(())
}

/// Processes events from the SSE stream
async fn process_sse_events(
    mut event_rx: mpsc::Receiver<SseEventType>,
    tx: Arc<mpsc::Sender<JSONRPCMessage>>,
    state: TransportStateChannel,
    sse_url: String,
    mut _shutdown_rx: oneshot::Receiver<()>,
    status_tx: Arc<broadcast::Sender<ConnectionStatus>>
) {
    // Start with disconnected state
    state.reset();
    debug!("Initial connection status set to disconnected");

    // Process events in a loop
    while let Some(event) = event_rx.recv().await {
        match event {
            SseEventType::Endpoint(endpoint) => {
                debug!("Processing endpoint URL: {}", endpoint);

                // Create the full endpoint URL
                let base_url = sse_url
                    .rsplit_once('/')
                    .map(|(base, _)| base)
                    .unwrap_or(&sse_url);
                let full_endpoint_url = format!("{}{}", base_url, endpoint);
                debug!("Base URL for endpoint: {}", base_url);
                debug!("Full endpoint URL constructed: {}", full_endpoint_url);

                // Extract session ID from endpoint URL if available
                let mut session_id = None;
                if let Some(session_id_param) = endpoint.find("session_id=") {
                    let session_id_start = session_id_param + "session_id=".len();
                    let session_id_value = endpoint[session_id_start..]
                        .split('&')
                        .next()
                        .unwrap_or_default();

                    session_id = Some(session_id_value.to_string());
                    debug!("Extracted session ID: {}", session_id_value);
                }

                // Update the state with the endpoint URL
                state.update(|s| {
                    s.has_endpoint = true;
                    s.endpoint_url = Some(full_endpoint_url);
                    s.session_id = session_id;
                });

                // Notify of connection status if both endpoint and connected are true
                if state.current().has_connected {
                    debug!(
                        "Endpoint received and already connected - setting transport to fully connected"
                    );
                    let _ = status_tx.send(ConnectionStatus::Connected);
                } else {
                    debug!(
                        "Endpoint received but waiting for connected event - not fully connected yet"
                    );
                }
            }
            SseEventType::Connected(data) => {
                info!("Server confirmed connection: {}", data);

                // Update the state
                state.update(|s| {
                    s.has_connected = true;
                });

                // Only notify if we have both endpoint and connected
                if state.current().has_endpoint {
                    debug!(
                        "Connected event received and have endpoint - setting transport to fully connected"
                    );
                    let _ = status_tx.send(ConnectionStatus::Connected);
                } else {
                    debug!(
                        "Connected event received but waiting for endpoint - not fully connected yet"
                    );
                }
            }
            SseEventType::Message(data) => {
                info!("Processing message event with data: {}", data);

                // Try to parse the message as JSON RPC message
                match serde_json::from_str::<JSONRPCMessage>(&data) {
                    Ok(message) => {
                        if let Err(e) = tx.send(message).await {
                            error!("Failed to forward message to channel: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC message: {}", e);
                    }
                }
            }
            SseEventType::Error(_) => {
                // We don't necessarily disconnect on errors
                // Let the transport handle this
            }
            SseEventType::KeepAlive => {
                trace!("Received keep-alive");
            }
            SseEventType::Unknown(type_name, data) => {
                debug!("Unhandled event type: {} with data: {}", type_name, data);
            }
        }
    }

    // Set disconnected on exit
    state.reset();
    let _ = status_tx.send(ConnectionStatus::Disconnected);
}

#[async_trait]
impl Transport for SseTransport {
    /// Start the transport
    async fn start(&self) -> Result<(), Error> {
        debug!("Starting SSE transport");

        // Reset state to disconnected
        self.state.reset();

        // Start the SSE connection
        self.start_sse_connection().await?;

        // Wait up to 5 seconds for connection
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);

        // Clone the state channel - now it will share the same sender
        let mut state_rx = self.state.receiver();

        while start_time.elapsed() < timeout {
            // Check connection status directly
            let current_state = self.state.current();
            let is_connected = current_state.is_connected();

            debug!(
                "Waiting for connection, current status: connected={}, has_endpoint={}, has_connected={}",
                is_connected,
                current_state.has_endpoint,
                current_state.has_connected
            );

            // Also check endpoint URL directly
            let endpoint_url = current_state.endpoint_url.clone().unwrap_or_default();
            debug!("Current endpoint URL: {}", if endpoint_url.is_empty() {
                "EMPTY"
            } else {
                &endpoint_url
            });

            if is_connected && !endpoint_url.is_empty() {
                info!("SSE transport fully connected and ready with URL: {}", endpoint_url);
                return Ok(());
            }

            // Wait for the state to change or timeout
            let wait_result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                state_rx.changed()
            ).await;

            match wait_result {
                Ok(Ok(_)) => {
                    debug!("Transport state changed, checking connection status");
                    // We'll check the new state on the next loop iteration
                }
                Ok(Err(e)) => {
                    error!("Error waiting for state change: {}", e);
                    // Continue in the loop to check again
                }
                Err(_) => {
                    // Timeout waiting for change, just continue the loop
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }

        // If we get here, we timed out - log detailed state
        let current_state = self.state.current();
        let is_connected = current_state.is_connected();
        let endpoint_url = current_state.endpoint_url.clone().unwrap_or_default();

        error!(
            "Connection timeout - status: connected={}, has_endpoint={}, has_connected={}, endpoint_url={}",
            is_connected,
            current_state.has_endpoint,
            current_state.has_connected,
            if endpoint_url.is_empty() {
                "EMPTY"
            } else {
                &endpoint_url
            }
        );

        // Return error
        Err(Error::Transport("Timeout waiting for SSE connection".to_string()))
    }

    /// Send a message to the server
    async fn send(&self, message: &JSONRPCMessage) -> Result<(), Error> {
        // Quick check if connected
        if !self.state.is_connected() {
            error!("Cannot send message - transport not connected");
            return Err(Error::Transport("Not connected to server".to_string()));
        }

        // Get the endpoint URL directly from the TransportState
        let current_state = self.state.current();
        let endpoint_url = match &current_state.endpoint_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => {
                error!("Cannot send message - no endpoint URL available in transport state");
                return Err(Error::Transport("No endpoint URL available".to_string()));
            }
        };

        // Log the request
        debug!("Sending message to endpoint: {}", endpoint_url);

        // Send the message as JSON
        let response = self.http_client
            .post(&endpoint_url)
            .json(message)
            .send().await
            .map_err(|e| Error::Transport(format!("Failed to send message: {}", e)))?;

        // Check for success status code
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Server returned error: {} - {}", status, error_text);
            return Err(Error::Transport(format!("Server error: {} - {}", status, error_text)));
        }

        debug!("Message sent successfully");
        Ok(())
    }

    async fn receive(&self) -> Result<(Option<String>, JSONRPCMessage), Error> {
        // We need to lock the receiver channel to receive from it
        let mut rx_guard = self.rx.lock().await;
        match rx_guard.recv().await {
            Some(message) => {
                // Get session ID from state
                let session_id = self.state.current().session_id.clone();
                Ok((session_id, message))
            }
            None => Err(Error::Transport("Message channel closed".to_string())),
        }
    }

    fn is_connected(&self) -> bool {
        self.state.is_connected()
    }

    fn subscribe_state(
        &self
    ) -> tokio::sync::watch::Receiver<crate::client::transport::state::TransportState> {
        self.state.receiver()
    }

    async fn close(&self) -> Result<(), Error> {
        debug!("Closing SSE transport");

        // Send shutdown signal
        let mut shutdown_guard = self.shutdown_tx.lock().await;
        if let Some(tx) = shutdown_guard.take() {
            debug!("Sending shutdown signal to SSE task");
            let _ = tx.send(());
        }
        drop(shutdown_guard);

        // Set to disconnected
        self.state.reset();
        let _ = self.status_tx.send(ConnectionStatus::Disconnected);

        // Wait a bit for resources to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        debug!("SSE transport closed");
        Ok(())
    }

    fn subscribe_status(&self) -> broadcast::Receiver<ConnectionStatus> {
        self.status_tx.subscribe()
    }
}
