//! SSE Client Transport
//!
//! This module implements the client-side transport for the SSE (Server-Sent Events)
//! protocol. It allows an MCP client to connect to an SSE server and receive real-time
//! updates via a persistent HTTP connection.

use async_trait::async_trait;
use futures_util::stream::StreamExt;
use reqwest::{ Client as HttpClient, ClientBuilder, header };
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tokio::sync::{ Mutex, mpsc, oneshot };
use url::Url;
use futures::future::{ self, Either };
use serde_json::Value;
use log::{ info, debug, warn, error };
use tokio::sync::Mutex as AsyncMutex;
use futures::future::poll_fn;
use url::form_urlencoded;

use crate::errors::Error;
use crate::messages::Message;
use crate::transport::Transport;
use crate::lifecycle::{ LifecycleEvent, LifecycleManager };

/// Default timeout for HTTP requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Default retry delay for reconnecting
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(2);
/// Maximum number of reconnect attempts
const MAX_RECONNECT_ATTEMPTS: usize = 5;
/// Buffer size for message channel
const CHANNEL_BUFFER_SIZE: usize = 100;

/// Options for the SSE client transport
#[derive(Clone, Debug)]
pub struct SseOptions {
    /// Authentication token
    pub auth_token: Option<String>,
    /// Request timeout
    pub timeout: Duration,
    /// Retry delay for reconnection
    pub retry_delay: Duration,
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
            custom_headers: None,
            client_id: None,
        }
    }
}

/// Client-side implementation of the SSE transport
pub struct SseTransport {
    /// Base URL for the SSE server
    base_url: String,
    /// URL for the SSE events endpoint
    events_url: String,
    /// URL for the messages endpoint
    messages_url: String,
    /// HTTP client
    http_client: HttpClient,
    /// Channel for incoming messages
    rx: mpsc::Receiver<Message>,
    /// Options for the transport
    options: SseOptions,
    /// Session ID (if assigned by server)
    session_id: Arc<Mutex<String>>,
    /// Whether the transport is connected
    connected: Arc<Mutex<bool>>,
    /// Whether the transport is ready to send/receive messages
    is_ready: Arc<AtomicBool>,
    /// Handle to the background task
    _task_handle: tokio::task::JoinHandle<()>,
    /// Lifecycle manager for handling lifecycle events
    lifecycle_manager: Arc<crate::lifecycle::LifecycleManager>,
    /// Shutdown signal sender
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl SseTransport {
    /// Create a new SSE transport
    pub async fn new(base_url: &str) -> Result<Self, Error> {
        Self::with_options(base_url, SseOptions::default()).await
    }

    /// Create a new SSE transport with custom options
    pub async fn with_options(base_url: &str, options: SseOptions) -> Result<Self, Error> {
        // Verify the base URL is valid
        let base_url = match Url::parse(base_url) {
            Ok(url) => url,
            Err(e) => {
                return Err(Error::Transport(format!("Invalid base URL: {}", e)));
            }
        };

        // Create the endpoints
        let events_url = base_url
            .join("sse")
            .map_err(|e| Error::Transport(format!("Failed to create events URL: {}", e)))?
            .to_string();

        let messages_url = base_url
            .join("message")
            .map_err(|e| Error::Transport(format!("Failed to create messages URL: {}", e)))?
            .to_string();

        // Create the HTTP client with specified timeout
        let http_client = ClientBuilder::new()
            .timeout(options.timeout)
            .build()
            .map_err(|e| Error::Transport(format!("Failed to create HTTP client: {}", e)))?;

        // Create a channel for incoming messages
        let (tx, rx) = mpsc::channel(100);

        // Create the session ID
        let session_id = Arc::new(Mutex::new(String::new()));

        // Create the connected flag
        let connected = Arc::new(Mutex::new(false));

        // Create the is_ready flag
        let is_ready = Arc::new(AtomicBool::new(false));

        // Create the lifecycle manager
        let lifecycle_manager = Arc::new(crate::lifecycle::LifecycleManager::new());

        // Create a shutdown signal
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // Clone values for the task
        let events_url_clone = events_url.clone();
        let tx_clone = tx.clone();
        let session_id_clone = session_id.clone();
        let connected_clone = connected.clone();
        let is_ready_clone = is_ready.clone();
        let http_client_clone = http_client.clone();
        let lifecycle_manager_clone = lifecycle_manager.clone();
        let messages_url_clone = messages_url.clone();

        // Spawn a task to handle the SSE connection
        let task_handle = tokio::spawn(async move {
            // Create a task-local shutdown signal
            let mut shutdown_rx = shutdown_rx;

            // Enter the connection loop
            loop {
                // Check if we should shut down
                if shutdown_rx.try_recv().is_ok() {
                    tracing::info!("Received shutdown signal, closing SSE connection");
                    break;
                }

                // Try to connect to the SSE endpoint
                match
                    Self::connect_to_sse(
                        http_client_clone.clone(),
                        events_url_clone.clone(),
                        tx_clone.clone(),
                        session_id_clone.clone(),
                        connected_clone.clone(),
                        is_ready_clone.clone(),
                        messages_url_clone.clone(),
                        lifecycle_manager_clone.clone()
                    ).await
                {
                    Ok(_) => {
                        tracing::info!("SSE connection ended normally");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("SSE connection error: {}", e);
                        // Short delay before retry
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }

            // Ensure we mark as disconnected
            {
                let mut connected_guard = connected_clone.lock().await;
                *connected_guard = false;
            }

            // Mark is_ready as false since we're no longer connected
            is_ready_clone.store(false, std::sync::atomic::Ordering::Release);
            tracing::info!("SSE connection task ending");

            // Notify final closed state
            info!("SSE transport closed");
        });

        Ok(Self {
            base_url: base_url.to_string(),
            events_url,
            messages_url,
            http_client,
            rx,
            options,
            session_id,
            connected,
            is_ready,
            _task_handle: task_handle,
            lifecycle_manager,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Register a lifecycle handler
    pub fn register_lifecycle_handler<F>(&self, handler: F)
        where F: Fn(LifecycleEvent) + Send + Sync + 'static
    {
        self.lifecycle_manager.register_event_handler(handler);
    }

    /// Notify lifecycle events
    async fn notify_lifecycle_event(
        lifecycle_manager: &Arc<crate::lifecycle::LifecycleManager>,
        event: LifecycleEvent
    ) {
        lifecycle_manager.notify_event(event);
    }

    /// Connect to the SSE endpoint and process messages
    async fn connect_to_sse(
        http_client: HttpClient,
        events_url: String,
        sender: mpsc::Sender<Message>,
        session_id: Arc<Mutex<String>>,
        connected: Arc<Mutex<bool>>,
        is_ready: Arc<AtomicBool>,
        messages_url: String,
        lifecycle_manager: Arc<crate::lifecycle::LifecycleManager>
    ) -> Result<(), Error> {
        let mut retries = 0;
        let max_retries = 5;
        let mut retry_delay = std::time::Duration::from_millis(1000);

        // Store a clone of messages_url for later use
        let messages_url_clone = messages_url.clone();

        loop {
            tracing::info!("Connecting to SSE endpoint: {}", events_url);

            // Reset ready state on new connection attempt
            is_ready.store(false, std::sync::atomic::Ordering::Release);

            // Create a request with headers (no client ID - the server will assign one)
            let req = http_client
                .get(&events_url)
                .header("Accept", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .build()
                .map_err(|e| Error::Transport(format!("Failed to build SSE request: {}", e)))?;

            // Send the request and check the response
            let response = match http_client.execute(req).await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        let error = format!(
                            "SSE connection failed with status {}: {}",
                            status,
                            text
                        );
                        tracing::error!("{}", error);

                        // Increase retries and delay before next attempt
                        retries += 1;
                        if retries >= max_retries {
                            return Err(Error::Transport(error));
                        }

                        tracing::info!(
                            "Retrying SSE connection in {:?} (attempt {}/{})",
                            retry_delay,
                            retries,
                            max_retries
                        );
                        tokio::time::sleep(retry_delay).await;
                        retry_delay = std::cmp::min(
                            retry_delay * 2,
                            std::time::Duration::from_secs(30)
                        );
                        continue;
                    }
                    resp
                }
                Err(e) => {
                    let error = format!("Failed to connect to SSE endpoint: {}", e);
                    tracing::error!("{}", error);

                    // Increase retries and delay before next attempt
                    retries += 1;
                    if retries >= max_retries {
                        return Err(Error::Transport(error));
                    }

                    tracing::info!(
                        "Retrying SSE connection in {:?} (attempt {}/{})",
                        retry_delay,
                        retries,
                        max_retries
                    );
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = std::cmp::min(
                        retry_delay * 2,
                        std::time::Duration::from_secs(30)
                    );
                    continue;
                }
            };

            // Update connected status
            *connected.lock().await = true;
            tracing::info!("Successfully connected to SSE endpoint!");

            // Set the transport as ready immediately - we no longer wait for endpoint event
            is_ready.store(true, std::sync::atomic::Ordering::Release);

            // Process the response stream
            match
                Self::process_sse_stream(
                    response,
                    sender.clone(),
                    session_id.clone(),
                    is_ready.clone()
                ).await
            {
                Ok(_) => {
                    tracing::info!("SSE stream processed successfully");
                    break; // Exit loop if stream ended gracefully
                }
                Err(e) => {
                    tracing::error!("Error processing SSE stream: {}", e);

                    // Update connected status
                    *connected.lock().await = false;

                    // Notify error
                    error!("SSE transport error occurred");

                    // Increase retries and delay before next attempt
                    retries += 1;
                    if retries >= max_retries {
                        return Err(e);
                    }

                    tracing::info!(
                        "Retrying SSE connection in {:?} (attempt {}/{})",
                        retry_delay,
                        retries,
                        max_retries
                    );
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = std::cmp::min(
                        retry_delay * 2,
                        std::time::Duration::from_secs(30)
                    );
                }
            }
        }

        Ok(())
    }

    async fn process_sse_stream(
        response: reqwest::Response,
        sender: mpsc::Sender<Message>,
        session_id: Arc<Mutex<String>>,
        is_ready: Arc<AtomicBool>
    ) -> Result<(), Error> {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut event_type = String::new();
        let mut event_data = String::new();

        tracing::debug!("Starting to process SSE stream");

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // Convert bytes to string and append to buffer
                    let chunk_str = std::str
                        ::from_utf8(&chunk)
                        .map_err(|e| {
                            Error::Transport(format!("Invalid UTF-8 in SSE stream: {}", e))
                        })?;

                    buffer.push_str(chunk_str);

                    // Process complete lines in the buffer
                    let mut pos = 0;
                    while let Some(next_newline) = buffer[pos..].find('\n') {
                        let line_end = pos + next_newline;
                        let line = buffer[pos..line_end].trim();
                        pos = line_end + 1;

                        tracing::debug!("Received SSE line: {}", line);

                        if line.is_empty() {
                            // Empty line marks the end of an event
                            if !event_type.is_empty() && !event_data.is_empty() {
                                // Process the complete event - only for message events
                                if event_type == "message" {
                                    Self::process_message_event(&event_data, &sender).await?;
                                } else if event_type == "error" {
                                    tracing::error!("Received error event: {}", event_data);
                                }

                                // Reset event state
                                event_type.clear();
                                event_data.clear();
                            }
                        } else if let Some(data) = line.strip_prefix("data:") {
                            // Append to event data
                            if !event_data.is_empty() {
                                event_data.push('\n');
                            }
                            event_data.push_str(data.trim());
                        } else if let Some(event) = line.strip_prefix("event:") {
                            // Set event type
                            event_type = event.trim().to_string();
                        }
                    }

                    // Remove processed content from buffer
                    if pos < buffer.len() {
                        buffer = buffer[pos..].to_string();
                    } else {
                        buffer.clear();
                    }
                }
                Err(e) => {
                    return Err(Error::Transport(format!("Error reading SSE stream: {}", e)));
                }
            }
        }

        tracing::info!("SSE stream ended");
        Ok(())
    }

    async fn process_message_event(
        event_data: &str,
        sender: &mpsc::Sender<Message>
    ) -> Result<(), Error> {
        tracing::debug!("Processing message event: {}", event_data);

        // Try to parse the message from JSON
        match serde_json::from_str::<Message>(event_data) {
            Ok(message) => {
                // Send the message to the receiver
                if let Err(e) = sender.send(message).await {
                    tracing::error!("Failed to send message to channel: {}", e);
                    return Err(Error::Transport("Failed to process message".to_string()));
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to parse message from event data: {}", e);
                Err(Error::Transport(format!("Invalid message format: {}", e)))
            }
        }
    }

    // Message filtering based on ID
    async fn handle_sse_event(
        event_type: &str,
        event_data: &str,
        sender: &mpsc::Sender<Message>,
        pending_requests: &Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Message, Error>>>>>
    ) -> Result<(), Error> {
        if event_type != "message" {
            return Ok(());
        }

        // Parse message
        let message: Message = serde_json::from_str(event_data)?;

        // If message is a response, check if we have a pending request for it
        if let Message::Response(response) = &message {
            // Convert i32 ID to u64 for HashMap lookup
            let id = response.id as u64;

            // Check if we have a pending request for this ID
            let mut pending = pending_requests.lock().await;
            if let Some(sender) = pending.remove(&id) {
                // Send response to the waiting call
                let _ = sender.send(Ok(message.clone()));
                return Ok(());
            }
        }

        // Otherwise, forward the message
        sender
            .send(message).await
            .map_err(|e| Error::Transport(format!("Failed to forward message: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl Transport for SseTransport {
    /// Start the transport - for the client this is a no-op as initialization
    /// happens in the constructor
    async fn start(&mut self) -> Result<(), Error> {
        // Check if we're already connected
        let connected = *self.connected.lock().await;
        if connected {
            return Ok(());
        }

        // Otherwise, just log that we're ready
        tracing::info!("SSE transport ready");
        Ok(())
    }

    async fn receive(&mut self) -> Result<(Option<String>, Message), Error> {
        // Wait for the transport to be ready
        if !self.is_ready.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Transport("Transport not ready".to_string()));
        }

        // Receive a message from the channel
        match self.rx.recv().await {
            Some(msg) => Ok((None, msg)),
            None => Err(Error::Transport("Channel closed".to_string())),
        }
    }

    async fn send(&mut self, message: &Message) -> Result<(), Error> {
        self.send_to("", message).await
    }

    async fn send_to(&mut self, _client_id: &str, message: &Message) -> Result<(), Error> {
        // Wait for the transport to be ready
        if !self.is_ready.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Transport("Transport not ready".to_string()));
        }

        // Serialize the message to JSON
        let message_json = serde_json
            ::to_string(message)
            .map_err(|e| Error::Transport(format!("Failed to serialize message: {}", e)))?;

        // Send the message to the server via HTTP POST
        let response = self.http_client
            .post(&self.messages_url)
            .header("Content-Type", "application/json")
            .body(message_json)
            .send().await
            .map_err(|e| Error::Transport(format!("Failed to send message: {}", e)))?;

        // Check if the response is successful
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(
                Error::Transport(
                    format!("Failed to send message, received status {}: {}", status, text)
                )
            );
        }

        Ok(())
    }

    async fn is_connected(&self) -> bool {
        let connected = self.connected.lock().await;
        *connected
    }

    async fn close(&mut self) -> Result<(), Error> {
        // Check if already closed
        if !self.is_connected().await {
            return Ok(());
        }

        // Send the shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for the task to finish
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(())
    }
}
