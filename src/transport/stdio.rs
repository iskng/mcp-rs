//! STDIO Transport
//!
//! This module implements the STDIO transport for the MCP library, enabling
//! communication via standard input and output streams. It is particularly useful
//! for local subprocess communication in CLI-based MCP servers.

use crate::errors::Error;
use crate::messages::Message;
use crate::transport::Transport;
use crate::lifecycle::{ LifecycleEvent, LifecycleManager };
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{ self, AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout };
use tokio::process;
use tokio::sync::mpsc;
use tracing;
use std::sync::Arc;

/// Channels used for process communication
struct StdioChannels {
    /// Receiver for incoming messages
    message_rx: mpsc::Receiver<Result<Message, Error>>,
    /// Sender for outgoing messages
    outgoing_tx: mpsc::Sender<Message>,
    /// Task handle for reader
    reader_task: tokio::task::JoinHandle<()>,
    /// Task handle for writer
    writer_task: tokio::task::JoinHandle<()>,
}

/// A transport implementation that uses standard input and output
pub struct StdioTransport {
    /// Child process
    process: Option<process::Child>,

    /// Process command
    command: String,

    /// Process arguments
    args: Vec<String>,

    /// Process environment variables
    env: Option<HashMap<String, String>>,

    /// Process communication channels
    channels: Option<StdioChannels>,

    /// Is the transport connected
    connected: bool,

    /// Lifecycle event handlers
    lifecycle_handlers: Vec<Box<dyn Fn(LifecycleEvent) + Send + Sync>>,

    /// Process exit timeout
    exit_timeout: Duration,

    /// Standard input for direct I/O mode
    stdin: Option<Stdin>,

    /// Standard output for direct I/O mode
    stdout: Option<Stdout>,

    /// Buffered reader for stdin
    reader: Option<BufReader<Stdin>>,

    /// Lifecycle manager for handling lifecycle events
    lifecycle_manager: Arc<LifecycleManager>,
}

impl StdioTransport {
    /// Create a new STDIO transport
    pub fn new() -> Self {
        let stdin = io::stdin();
        let stdout = io::stdout();

        Self {
            process: None,
            command: String::new(),
            args: Vec::new(),
            env: None,
            channels: None,
            connected: true,
            lifecycle_handlers: Vec::new(),
            exit_timeout: Duration::from_secs(5),
            stdin: Some(stdin),
            stdout: Some(stdout),
            reader: Some(BufReader::new(io::stdin())),
            lifecycle_manager: Arc::new(LifecycleManager::new()),
        }
    }

    /// Start the process and establish communication channels
    async fn start_process(&mut self) -> Result<(), Error> {
        // Notify starting
        tracing::info!("STDIO transport starting");

        let mut command = tokio::process::Command::new(&self.command);
        command.args(&self.args);

        // Set environment
        if let Some(env) = &self.env {
            for (key, value) in env {
                command.env(key, value);
            }
        }

        // Configure stdio
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());

        // Spawn process
        let mut child = command
            .spawn()
            .map_err(|e| Error::Transport(format!("Failed to spawn process: {}", e)))?;

        // Get stdio handles
        let stdin = child.stdin
            .take()
            .ok_or_else(|| Error::Transport("Failed to get stdin handle".to_string()))?;
        let stdout = child.stdout
            .take()
            .ok_or_else(|| Error::Transport("Failed to get stdout handle".to_string()))?;

        // Create channels
        let (message_tx, message_rx) = mpsc::channel(100);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(100);

        // Start reader and writer tasks
        let reader_task = tokio::spawn(stdio_reader(stdout, message_tx.clone()));
        let writer_task = tokio::spawn(stdio_writer(stdin, outgoing_rx));

        // Store everything
        self.process = Some(child);
        self.channels = Some(StdioChannels {
            message_rx,
            outgoing_tx,
            reader_task,
            writer_task,
        });

        self.connected = true;

        // Notify started
        tracing::info!("STDIO transport started");

        Ok(())
    }

    /// Send a lifecycle event notification
    fn notify_lifecycle(&self, event: LifecycleEvent) {
        // Clone the event before passing it to the lifecycle manager
        let event_clone = event.clone();
        self.lifecycle_manager.notify_event(event_clone);

        // Also notify the legacy handlers
        for handler in &self.lifecycle_handlers {
            handler(event.clone());
        }
    }
}

/// Process stdout from a child process and forward messages
async fn stdio_reader(
    stdout: process::ChildStdout,
    tx: mpsc::Sender<Result<Message, Error>>
) -> () {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF
                break;
            }
            Ok(_) => {
                // Parse the JSON message
                match serde_json::from_str::<Message>(&line) {
                    Ok(message) => {
                        if tx.send(Ok(message)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(Error::Json(e))).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Err(Error::Io(e))).await;
                break;
            }
        }
    }
}

/// Process outgoing messages and write to stdin
async fn stdio_writer(mut stdin: process::ChildStdin, mut rx: mpsc::Receiver<Message>) -> () {
    while let Some(message) = rx.recv().await {
        match serde_json::to_string(&message) {
            Ok(json) => {
                // Write the JSON followed by a newline
                if stdin.write_all(json.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(b"\n").await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
            Err(_) => {
                // Ignore serialization errors
                continue;
            }
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    /// Start the transport - for stdio this initializes the process
    async fn start(&mut self) -> Result<(), Error> {
        // Check if we're already started
        if self.connected {
            return Ok(());
        }

        // If we have a command, start the process
        if !self.command.is_empty() {
            self.start_process().await?;
        } else {
            // Otherwise just mark as connected and use stdin/stdout directly
            self.connected = true;
            tracing::info!("STDIO transport ready (direct I/O mode)");
        }

        Ok(())
    }

    async fn receive(&mut self) -> Result<(Option<String>, Message), Error> {
        if !self.connected {
            return Err(Error::Transport("Transport is not connected".to_string()));
        }

        let mut line = String::new();

        if let Some(reader) = &mut self.reader {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF
                    self.connected = false;
                    Err(Error::Transport("EOF reached".to_string()))
                }
                Ok(_) => {
                    // Parse the JSON message
                    serde_json
                        ::from_str(&line)
                        .map_err(Error::Json)
                        .map(|msg| (None, msg)) // StdioTransport has no client ID concept
                }
                Err(e) => {
                    self.connected = false;
                    Err(Error::Io(e))
                }
            }
        } else if let Some(channels) = &mut self.channels {
            match channels.message_rx.recv().await {
                Some(result) => result.map(|msg| (None, msg)), // StdioTransport has no client ID concept
                None => {
                    self.connected = false;
                    Err(Error::Transport("Channel closed".to_string()))
                }
            }
        } else {
            Err(Error::Transport("Transport is not properly initialized".to_string()))
        }
    }

    async fn send(&mut self, message: &Message) -> Result<(), Error> {
        if !self.connected {
            return Err(Error::Transport("Transport is not connected".to_string()));
        }

        // Serialize the message to JSON
        let json = serde_json::to_string(message).map_err(Error::Json)?;

        if let Some(channels) = &mut self.channels {
            // Send through process channels
            channels.outgoing_tx
                .send(message.clone()).await
                .map_err(|_| Error::Transport("Failed to send message".to_string()))?;
        } else if let Some(stdout) = &mut self.stdout {
            // Write directly to stdout
            stdout.write_all(json.as_bytes()).await.map_err(Error::Io)?;
            stdout.write_all(b"\n").await.map_err(Error::Io)?;
            stdout.flush().await.map_err(Error::Io)?;
        } else {
            return Err(Error::Transport("Transport is not properly initialized".to_string()));
        }

        Ok(())
    }

    async fn send_to(&mut self, _client_id: &str, message: &Message) -> Result<(), Error> {
        // For STDIO transport, there's only one client, so send_to behaves the same as send
        self.send(message).await
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn close(&mut self) -> Result<(), Error> {
        if !self.connected {
            return Ok(());
        }

        // Notify closing
        tracing::info!("STDIO transport closing");

        // Take channels
        let channels = self.channels.take();

        // Close channels if they exist
        if let Some(channels) = channels {
            // Close outgoing channel
            drop(channels.outgoing_tx);

            // Wait for writer task to complete
            if let Err(e) = channels.writer_task.await {
                tracing::error!("Error joining writer task: {}", e);
            }
        }

        // Take the process
        let mut process = self.process.take();

        // Try graceful shutdown first
        if let Some(ref mut child) = process {
            // Terminate process
            tracing::debug!("Terminating child process");

            #[cfg(unix)]
            {
                if let Err(e) = child.kill().await {
                    tracing::error!("Error killing process: {}", e);
                }
            }

            #[cfg(windows)]
            {
                if let Err(e) = child.kill().await {
                    tracing::error!("Error killing process: {}", e);
                }
            }

            // Wait for process to exit
            match tokio::time::timeout(self.exit_timeout, child.wait()).await {
                Ok(Ok(_)) => {
                    tracing::debug!("Child process exited successfully");
                }
                Ok(Err(e)) => {
                    tracing::error!("Error waiting for child process: {}", e);
                }
                Err(_) => {
                    tracing::error!("Timeout waiting for child process to exit");

                    // Force kill
                    #[cfg(unix)]
                    {
                        if let Err(e) = child.kill().await {
                            tracing::error!("Error force killing process: {}", e);
                        }
                    }

                    #[cfg(windows)]
                    {
                        if let Err(e) = child.kill().await {
                            tracing::error!("Error force killing process: {}", e);
                        }
                    }
                }
            }
        }

        self.connected = false;

        // Notify closed
        tracing::info!("STDIO transport closed");

        Ok(())
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}
