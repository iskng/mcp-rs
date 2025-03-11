//! STDIO Client Transport
//!
//! This module implements the client-side transport for the STDIO protocol, enabling
//! communication via standard input and output streams. It is particularly useful
//! for local subprocess communication in CLI-based MCP clients.

use async_trait::async_trait;
use log::{ debug, error, info, warn };
use std::sync::Arc;
use tokio::io::{ AsyncBufReadExt, AsyncWriteExt, BufReader };
use tokio::sync::{ broadcast, mpsc, oneshot, Mutex };
use tokio::time::Duration;

use crate::client::transport::{ ConnectionStatus, Transport };
use crate::client::transport::state::TransportStateChannel;
use crate::protocol::Error;
use crate::protocol::JSONRPCMessage;

/// Buffer size for message channel
const CHANNEL_BUFFER_SIZE: usize = 100;

/// A transport implementation that uses standard input and output
pub struct StdioTransport {
    /// Channel for incoming messages
    rx: Arc<Mutex<mpsc::Receiver<JSONRPCMessage>>>,
    /// Sender for the message channel
    tx: Arc<mpsc::Sender<JSONRPCMessage>>,
    /// Sender for outgoing messages
    outgoing_tx: Arc<broadcast::Sender<JSONRPCMessage>>,
    /// Transport state channel
    state: TransportStateChannel,
    /// Status broadcaster
    status_tx: Arc<broadcast::Sender<ConnectionStatus>>,
    /// Task handle for the STDIO connection
    _task_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Shutdown signal sender
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl StdioTransport {
    /// Create a new STDIO transport
    pub async fn new() -> Result<Self, Error> {
        // Create channels for messages
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (outgoing_tx, _) = broadcast::channel(CHANNEL_BUFFER_SIZE);

        // Create a broadcast channel for status updates
        let (status_tx, _) = broadcast::channel(16);

        // Create the transport state channel
        let state = TransportStateChannel::new();

        let transport = Self {
            rx: Arc::new(Mutex::new(rx)),
            tx: Arc::new(tx),
            outgoing_tx: Arc::new(outgoing_tx),
            state,
            status_tx: Arc::new(status_tx),
            _task_handle: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        };

        Ok(transport)
    }

    /// Start a background task that handles stdin/stdout communication
    async fn start_stdio_connection(&self) -> Result<(), Error> {
        // Create channels for stream shutdown
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        // Store the shutdown sender
        let mut shutdown_guard = self.shutdown_tx.lock().await;
        *shutdown_guard = Some(shutdown_tx);
        drop(shutdown_guard); // Release the lock explicitly

        // Create clones for the tasks
        let tx = self.tx.clone();
        let state = self.state.clone();
        let status_tx = self.status_tx.clone();

        // Create a receiver for outgoing messages
        let mut outgoing_rx = self.outgoing_tx.subscribe();

        // Create a task to handle stdin/stdout
        let handle = tokio::spawn(async move {
            // Initialize stdin/stdout
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut stdout = tokio::io::stdout();

            // Initialize a buffer for reading
            let mut line = String::new();

            // Set the state to connected
            state.update(|s| {
                s.has_connected = true;
                s.has_endpoint = true;
                s.endpoint_url = Some("stdio://local".to_string());
                s.session_id = Some("stdio-session".to_string());
            });

            // Notify we're connected
            let _ = status_tx.send(ConnectionStatus::Connected);

            // Process messages
            loop {
                // Use tokio::select to handle both reading and shutdown
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Received shutdown signal, stopping STDIO stream");
                        break;
                    }
                    line_result = reader.read_line(&mut line) => {
                        match line_result {
                            Ok(0) => {
                                // EOF
                                warn!("End of input stream reached");
                                break;
                            }
                            Ok(_) => {
                                // Try to parse the input as a JSON-RPC message
                                match serde_json::from_str::<JSONRPCMessage>(&line) {
                                    Ok(message) => {
                                        debug!("Received message: {}", line.trim());
                                        if let Err(e) = tx.send(message).await {
                                            error!("Failed to forward message to channel: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse JSON-RPC message: {} - Input: {}", e, line.trim());
                                    }
                                }
                                line.clear();
                            }
                            Err(e) => {
                                error!("Error reading from stdin: {}", e);
                                break;
                            }
                        }
                    }
                    msg_result = outgoing_rx.recv() => {
                        if let Ok(message) = msg_result {
                            // Serialize the message
                            match serde_json::to_string(&message) {
                                Ok(json) => {
                                    // Write the message to stdout
                                    if let Err(e) = stdout.write_all(json.as_bytes()).await {
                                        error!("Failed to write message to stdout: {}", e);
                                        continue;
                                    }
                                    if let Err(e) = stdout.write_all(b"\n").await {
                                        error!("Failed to write newline to stdout: {}", e);
                                        continue;
                                    }
                                    if let Err(e) = stdout.flush().await {
                                        error!("Failed to flush stdout: {}", e);
                                        continue;
                                    }
                                    debug!("Message sent successfully");
                                }
                                Err(e) => {
                                    error!("Failed to serialize message: {}", e);
                                }
                            }
                        } else {
                            // Channel closed
                            warn!("Outgoing message channel closed");
                            break;
                        }
                    }
                }
            }

            // Reset state on exit
            state.reset();
            let _ = status_tx.send(ConnectionStatus::Disconnected);
            debug!("STDIO stream handler exited");
        });

        // Store the task handle
        let mut task_handle_guard = self._task_handle.lock().await;
        *task_handle_guard = Some(handle);
        drop(task_handle_guard);

        Ok(())
    }
}

#[async_trait]
impl Transport for StdioTransport {
    /// Start the transport
    async fn start(&self) -> Result<(), Error> {
        debug!("Starting STDIO transport");

        // Reset state to disconnected
        self.state.reset();

        // Start the STDIO connection
        self.start_stdio_connection().await?;

        // Wait up to 2 seconds for connection
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(2);

        // Clone the state channel
        let mut state_rx = self.state.receiver();

        while start_time.elapsed() < timeout {
            // Check connection status directly
            if self.state.is_connected() {
                info!("STDIO transport fully connected and ready");
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

        // If we get here, we timed out
        error!("Connection timeout waiting for STDIO connection");
        Err(Error::Transport("Timeout waiting for STDIO connection".to_string()))
    }

    /// Send a message to the server
    async fn send(&self, message: &JSONRPCMessage) -> Result<(), Error> {
        // Quick check if connected
        if !self.state.is_connected() {
            error!("Cannot send message - transport not connected");
            return Err(Error::Transport("Not connected to server".to_string()));
        }

        // Send the message through the channel to the I/O task
        self.outgoing_tx
            .send(message.clone())
            .map_err(|_| Error::Transport("Failed to send message to output channel".to_string()))?;

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
        debug!("Closing STDIO transport");

        // Send shutdown signal
        let mut shutdown_guard = self.shutdown_tx.lock().await;
        if let Some(tx) = shutdown_guard.take() {
            debug!("Sending shutdown signal to STDIO task");
            let _ = tx.send(());
        }
        drop(shutdown_guard);

        // Set to disconnected
        self.state.reset();
        let _ = self.status_tx.send(ConnectionStatus::Disconnected);

        // Wait a bit for resources to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        debug!("STDIO transport closed");
        Ok(())
    }

    fn subscribe_status(&self) -> broadcast::Receiver<ConnectionStatus> {
        self.status_tx.subscribe()
    }
}
