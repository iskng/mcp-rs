// //! WebSocket Transport
// //!
// //! This module implements the WebSocket transport for the MCP library, enabling
// //! full-duplex communication over WebSockets. It is suitable for web-based
// //! and networked applications requiring real-time communication.

// use async_trait::async_trait;
// use futures::future::{ FutureExt };
// use futures::{ SinkExt, StreamExt };
// use std::sync::Arc;
// use std::time::Duration;
// use tokio::sync::{ Mutex, mpsc, oneshot };
// use tokio_tungstenite::{ connect_async, tungstenite::protocol::Message as WsMessage };
// use url::Url;

// use crate::protocol::Error;
// use crate::protocol::Message;
// use crate::client::transport::Transport;

// /// Default connection timeout
// const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);
// /// Default heartbeat interval
// const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
// /// Maximum reconnection attempts
// const MAX_RECONNECT_ATTEMPTS: usize = 5;
// /// Default reconnection delay
// const DEFAULT_RECONNECT_DELAY: Duration = Duration::from_secs(2);

// /// WebSocket transport options
// #[derive(Debug, Clone)]
// pub struct WebSocketOptions {
//     /// Connection timeout
//     pub connection_timeout: Duration,
//     /// Heartbeat interval
//     pub heartbeat_interval: Duration,
//     /// Reconnection delay
//     pub reconnect_delay: Duration,
//     /// Authentication token
//     pub auth_token: Option<String>,
//     /// Custom headers
//     pub headers: Option<Vec<(String, String)>>,
// }

// impl Default for WebSocketOptions {
//     fn default() -> Self {
//         Self {
//             connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
//             heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
//             reconnect_delay: DEFAULT_RECONNECT_DELAY,
//             auth_token: None,
//             headers: None,
//         }
//     }
// }

// /// WebSocket transport for MCP
// pub struct WebSocketTransport {
//     /// The WebSocket URL
//     url: String,
//     /// Receiver for incoming messages
//     rx: Arc<Mutex<mpsc::Receiver<Result<Message, Error>>>>,
//     /// Sender for outgoing messages
//     tx: mpsc::Sender<Message>,
//     /// Connection status
//     connected: Arc<Mutex<bool>>,
//     /// WebSocket options
//     options: WebSocketOptions,
//     /// New flag to track if the transport is ready for sending
//     ready: Arc<Mutex<bool>>,
//     /// Connection task
//     connection_task: Option<tokio::task::JoinHandle<()>>,
//     /// Signal to indicate the transport is shutting down
//     shutdown: Arc<Mutex<bool>>,
// }

// impl WebSocketTransport {
//     /// Create a new WebSocket transport with default options
//     pub async fn new(url: String) -> Result<Self, Error> {
//         Self::with_options(url, WebSocketOptions::default()).await
//     }

//     /// Create a new WebSocket transport with custom options
//     pub async fn with_options(url: String, options: WebSocketOptions) -> Result<Self, Error> {
//         // Validate URL
//         let _ = Url::parse(&url).map_err(|e|
//             Error::Transport(format!("Invalid WebSocket URL: {}", e))
//         )?;

//         // Create message channels
//         let (tx_in, rx_in) = mpsc::channel::<Result<Message, Error>>(100);
//         let (tx_out, rx_out) = mpsc::channel::<Message>(100);

//         let connected = Arc::new(Mutex::new(false));
//         let ready = Arc::new(Mutex::new(false));
//         let shutdown = Arc::new(Mutex::new(false));

//         // Notify Starting lifecycle event
//         tracing::info!("WebSocket transport starting");

//         let connected_clone = connected.clone();
//         let ready_clone = ready.clone();
//         let shutdown_clone = shutdown.clone();
//         let url_clone = url.clone();
//         let options_clone = options.clone();

//         // Start background task for WebSocket communication
//         let connection_task = tokio::spawn(async move {
//             tracing::info!("Starting WebSocket connection task");
//             Self::run_connection_loop(
//                 url_clone,
//                 options_clone,
//                 tx_in,
//                 rx_out,
//                 connected_clone,
//                 ready_clone,
//                 shutdown_clone
//             ).await;
//             tracing::info!("WebSocket connection task completed");
//         });

//         Ok(Self {
//             url,
//             rx: Arc::new(Mutex::new(rx_in)),
//             tx: tx_out,
//             connected,
//             options,
//             ready,
//             connection_task: Some(connection_task),
//             shutdown,
//         })
//     }

//     /// Run the WebSocket connection loop
//     async fn run_connection_loop(
//         url: String,
//         options: WebSocketOptions,
//         tx_in: mpsc::Sender<Result<Message, Error>>,
//         rx_out: mpsc::Receiver<Message>,
//         connected: Arc<Mutex<bool>>,
//         ready: Arc<Mutex<bool>>,
//         shutdown: Arc<Mutex<bool>>
//     ) {
//         // Create a new channel for reconnection signals
//         let (reconnect_tx, _reconnect_rx) = mpsc::channel::<()>(1);
//         let mut reconnect_attempts = 0;

//         // Instead of trying to handle complex reconnection logic with ownership issues,
//         // let's simplify and just retry connection in a loop when it fails
//         loop {
//             if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
//                 let err = Error::Transport(
//                     format!("Failed to connect after {} attempts", MAX_RECONNECT_ATTEMPTS)
//                 );
//                 let _ = tx_in.send(Err(err)).await;

//                 // Notify Error lifecycle event
//                 tracing::error!("WebSocket transport error occurred");
//                 break;
//             }

//             let result = Self::handle_connection(
//                 url.clone(),
//                 options.clone(),
//                 tx_in.clone(),
//                 rx_out,
//                 connected.clone(),
//                 ready.clone(),
//                 shutdown.clone()
//             ).await;

//             if result {
//                 // Connection ended normally, reset reconnect attempts
//                 reconnect_attempts = 0;
//             } else {
//                 // Connection failed, increment reconnect attempts
//                 reconnect_attempts += 1;

//                 // Notify Error lifecycle event
//                 tracing::error!("WebSocket transport error occurred");

//                 // Wait with exponential backoff before reconnecting
//                 let backoff =
//                     (2_u64).pow(reconnect_attempts.min(10) as u32) *
//                     (options.reconnect_delay.as_millis() as u64);
//                 tracing::info!(
//                     "Reconnecting in {} ms (attempt {}/{})",
//                     backoff,
//                     reconnect_attempts,
//                     MAX_RECONNECT_ATTEMPTS
//                 );
//                 tokio::time::sleep(Duration::from_millis(backoff)).await;
//             }

//             // We can't reconnect because rx_out was moved into handle_connection
//             // In a real implementation, we would need to create a new channel
//             // and integrate it with the client's message loop
//             break;
//         }

//         // Mark as disconnected
//         *connected.lock().await = false;
//         *ready.lock().await = false;

//         // Notify Closed lifecycle event
//         tracing::info!("WebSocket transport closed");
//     }

//     /// Handle a single WebSocket connection
//     /// Returns true if connection ended normally, false if it failed
//     async fn handle_connection(
//         url: String,
//         options: WebSocketOptions,
//         tx_in: mpsc::Sender<Result<Message, Error>>,
//         mut rx_out: mpsc::Receiver<Message>,
//         connected: Arc<Mutex<bool>>,
//         ready: Arc<Mutex<bool>>,
//         shutdown: Arc<Mutex<bool>>
//     ) -> bool {
//         tracing::info!("Connecting to WebSocket at {}", url);

//         // Check if we're shutting down
//         if *shutdown.lock().await {
//             tracing::info!("Shutdown signal received, aborting connection");
//             return false;
//         }

//         // First, explicitly set connected and ready to false at the start
//         {
//             let mut connected_guard = connected.lock().await;
//             *connected_guard = false;
//             tracing::debug!("Set connected flag to false during connection setup");
//         }

//         {
//             let mut ready_guard = ready.lock().await;
//             *ready_guard = false;
//             tracing::debug!("Set ready flag to false during connection setup");
//         }

//         let mut url_with_auth = url.clone();
//         if let Some(token) = &options.auth_token {
//             url_with_auth.push_str(&format!("?token={}", token));
//         }

//         match connect_async(&url_with_auth).await {
//             Ok((ws_stream, _)) => {
//                 tracing::info!("WebSocket connection established");

//                 // Set connected flag to true
//                 {
//                     let mut connected_guard = connected.lock().await;
//                     *connected_guard = true;
//                     tracing::debug!("Set connected flag to true after successful connection");
//                 }

//                 // Notify Started lifecycle event
//                 tracing::info!("WebSocket transport started");

//                 let (mut ws_sender, mut ws_receiver) = ws_stream.split();
//                 let tx_in_clone = tx_in.clone();
//                 let ready_clone = ready.clone();

//                 // Create a channel for the sender task to signal it's ready
//                 let (sender_ready_tx, sender_ready_rx) = oneshot::channel();

//                 let sender_handle = tokio::spawn(async move {
//                     tracing::info!("Starting WebSocket sender task");

//                     // Signal that the sender is ready
//                     if sender_ready_tx.send(()).is_err() {
//                         tracing::error!("Failed to signal sender ready");
//                         return;
//                     }

//                     while let Some(msg) = rx_out.recv().await {
//                         match serde_json::to_string(&msg) {
//                             Ok(json) => {
//                                 tracing::debug!("Sending message: {}", json);
//                                 if let Err(e) = ws_sender.send(WsMessage::Text(json.into())).await {
//                                     tracing::error!("WebSocket send error: {}", e);
//                                     break;
//                                 }
//                             }
//                             Err(e) => {
//                                 tracing::error!("JSON serialization error: {}", e);
//                                 continue;
//                             }
//                         }
//                     }
//                     tracing::warn!("WebSocket sender task ended");
//                 });

//                 let receiver_handle = tokio::spawn(async move {
//                     tracing::info!("Starting WebSocket receiver task");
//                     while let Some(result) = ws_receiver.next().await {
//                         match result {
//                             Ok(ws_msg) => {
//                                 if ws_msg.is_text() {
//                                     let text = ws_msg.to_text().unwrap();
//                                     tracing::debug!("Received raw message: {}", text);
//                                     match serde_json::from_str::<Message>(text) {
//                                         Ok(message) => {
//                                             tracing::debug!("Parsed message: {:?}", message);
//                                             if tx_in_clone.send(Ok(message)).await.is_err() {
//                                                 tracing::error!(
//                                                     "Failed to forward message to client"
//                                                 );
//                                                 break;
//                                             }
//                                         }
//                                         Err(e) => {
//                                             tracing::warn!("Failed to parse message: {}", e);
//                                             if tx_in_clone.send(Err(Error::Json(e))).await.is_err() {
//                                                 break;
//                                             }
//                                         }
//                                     }
//                                 } else if ws_msg.is_close() {
//                                     tracing::info!("WebSocket closed by server");
//                                     break;
//                                 }
//                             }
//                             Err(e) => {
//                                 tracing::error!("WebSocket receiver error: {}", e);
//                                 let _ = tx_in_clone.send(
//                                     Err(Error::Transport(e.to_string()))
//                                 ).await;
//                                 break;
//                             }
//                         }
//                     }
//                     tracing::warn!("WebSocket receiver task ended");
//                 });

//                 // Wait for the sender task to signal it's ready
//                 match
//                     tokio::time::timeout(tokio::time::Duration::from_secs(5), sender_ready_rx).await
//                 {
//                     Ok(Ok(())) => {
//                         tracing::info!("WebSocket sender task is ready");

//                         // Set ready flag to true
//                         let mut ready_guard = ready_clone.lock().await;
//                         *ready_guard = true;
//                         tracing::debug!("Set ready flag to true after sender signaled ready");
//                         drop(ready_guard); // Explicitly drop the guard to release the lock
//                     }
//                     Ok(Err(_)) => {
//                         tracing::error!("Sender task failed to signal ready");
//                         return false;
//                     }
//                     Err(_) => {
//                         tracing::error!("Timed out waiting for sender task to be ready");
//                         return false;
//                     }
//                 }

//                 tokio::select! {
//                     _ = sender_handle => {
//                         tracing::info!("Sender task completed");
//                         let mut ready_guard = ready.lock().await;
//                         *ready_guard = false;
//                         tracing::debug!("Set ready flag to false after sender task completed");
//                         false
//                     }
//                     _ = receiver_handle => {
//                         tracing::info!("Receiver task completed");
//                         let mut ready_guard = ready.lock().await;
//                         *ready_guard = false;
//                         tracing::debug!("Set ready flag to false after receiver task completed");
//                         false
//                     }
//                 }
//             }
//             Err(e) => {
//                 tracing::error!("WebSocket connection error: {}", e);
//                 let err = Error::Transport(format!("WebSocket connection error: {}", e));
//                 let _ = tx_in.send(Err(err)).await;

//                 // Ensure flags are set to false on error
//                 {
//                     let mut connected_guard = connected.lock().await;
//                     *connected_guard = false;
//                     tracing::debug!("Set connected flag to false after connection error");
//                 }

//                 {
//                     let mut ready_guard = ready.lock().await;
//                     *ready_guard = false;
//                     tracing::debug!("Set ready flag to false after connection error");
//                 }

//                 false
//             }
//         }
//     }
// }

// impl Drop for WebSocketTransport {
//     fn drop(&mut self) {
//         if let Some(handle) = self.connection_task.take() {
//             handle.abort();
//         }
//     }
// }

// #[async_trait]
// impl Transport for WebSocketTransport {
//     /// Start the transport - for websocket client, this is a no-op as connection
//     /// is established in the constructor
//     async fn start(&mut self) -> Result<(), Error> {
//         // Check if we're already connected
//         let connected = *self.connected.lock().await;
//         if connected {
//             return Ok(());
//         }

//         // Otherwise, just log that we're ready
//         tracing::info!("WebSocket transport ready");
//         Ok(())
//     }

//     async fn receive(&mut self) -> Result<(Option<String>, Message), Error> {
//         let mut rx = self.rx.lock().await;
//         rx.recv().await
//             .ok_or_else(|| Error::Transport("WebSocket receive channel closed".to_string()))?
//             .map(|msg| (None, msg)) // Client transport, so no client ID
//     }

//     async fn send(&mut self, message: &Message) -> Result<(), Error> {
//         tracing::debug!("WebSocketTransport.send called");

//         // First check connection without locking 'ready'
//         {
//             let connected = *self.connected.lock().await;
//             if !connected {
//                 tracing::error!("WebSocket is not connected");
//                 return Err(Error::Transport("WebSocket is not connected".to_string()));
//             }
//             tracing::debug!("WebSocket is connected");
//         }

//         // Check the ready flag first before waiting, using a short-lived lock
//         let start = std::time::Instant::now();
//         let timeout = std::time::Duration::from_secs(10); // 10 second timeout

//         let mut is_ready = false;
//         tracing::debug!("Checking if WebSocket is ready...");

//         // Loop but release the lock each time to avoid deadlock
//         while !is_ready {
//             // Short-lived lock to check ready state
//             {
//                 let ready_guard = self.ready.lock().await;
//                 is_ready = *ready_guard;
//                 if is_ready {
//                     tracing::debug!("WebSocket is ready for sending");
//                     break;
//                 }
//             }

//             // Check timeout without holding any locks
//             if start.elapsed() > timeout {
//                 tracing::error!("Timeout waiting for WebSocket to be ready");
//                 return Err(
//                     Error::Transport(
//                         "Timeout waiting for WebSocket to be ready for sending".to_string()
//                     )
//                 );
//             }

//             // Log and sleep without holding any locks
//             tracing::debug!("WebSocket not ready yet, waiting...");
//             tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//         }

//         tracing::debug!("Sending message via WebSocket channel: {:?}", message);
//         match self.tx.send(message.clone()).await {
//             Ok(_) => {
//                 tracing::debug!("Message sent to channel successfully");
//                 Ok(())
//             }
//             Err(e) => {
//                 tracing::error!("Failed to send message to WebSocket channel: {}", e);
//                 Err(Error::Transport("Failed to send message to WebSocket channel".to_string()))
//             }
//         }
//     }

//     async fn send_to(&mut self, _client_id: &str, message: &Message) -> Result<(), Error> {
//         // For client transports, send_to is the same as send since there's only one connection
//         self.send(message).await
//     }

//     async fn is_connected(&self) -> bool {
//         *self.connected.lock().await
//     }

//     async fn close(&mut self) -> Result<(), Error> {
//         tracing::info!("Closing WebSocket transport");

//         // Notify Closing lifecycle event
//         tracing::info!("WebSocket transport closing");

//         // Set shutdown flag first
//         let mut shutdown_guard = self.shutdown.lock().await;
//         *shutdown_guard = true;
//         drop(shutdown_guard);

//         // Abort the connection task
//         if let Some(handle) = self.connection_task.take() {
//             tracing::debug!("Aborting WebSocket connection task");
//             handle.abort();
//             tracing::debug!("WebSocket connection task aborted");
//         }

//         // Set connected and ready to false
//         *self.connected.lock().await = false;
//         *self.ready.lock().await = false;

//         // Notify Closed lifecycle event
//         tracing::info!("WebSocket transport closed");

//         Ok(())
//     }
// }
