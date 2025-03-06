//! STDIO Transport
//!
//! This module implements the STDIO transport for the MCP library, enabling
//! communication via standard input and output streams. It is particularly useful
//! for local subprocess communication in CLI-based MCP servers.

use std::sync::Arc;

use crate::errors::Error;
use crate::types::protocol::Message;
use crate::transport::{ Transport, TransportMessageHandler };
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::{ AsyncBufReadExt, AsyncWriteExt };
use tracing;
use std::process::Stdio;
use tokio::process;
use tokio::sync::mpsc;

/// Channels used for process communication
struct StdioChannels {
    /// Receiver for incoming messages
    message_rx: mpsc::Receiver<Result<Message, Error>>,
    /// Sender for outgoing messages
    outgoing_tx: mpsc::Sender<Message>,
    /// Task handle for reader
    #[allow(dead_code)]
    reader_task: tokio::task::JoinHandle<()>,
    /// Task handle for writer
    #[allow(dead_code)]
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

    /// Standard input for direct I/O mode
    stdin: Option<tokio::io::Stdin>,

    /// Standard output for direct I/O mode
    stdout: Option<tokio::io::Stdout>,

    /// Buffered reader for stdin
    reader: Option<tokio::io::BufReader<tokio::io::Stdin>>,

    /// Message handler (if registered)
    message_handler: Option<Arc<dyn TransportMessageHandler + Send + Sync>>,
}

impl StdioTransport {
    /// Create a new STDIO transport
    pub fn new() -> Self {
        Self {
            process: None,
            command: String::new(),
            args: Vec::new(),
            env: None,
            channels: None,
            connected: false,
            stdin: None,
            stdout: None,
            reader: None,
            message_handler: None,
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

    /// Register a message handler
    pub fn register_message_handler<H>(&mut self, handler: H)
        where H: TransportMessageHandler + Send + Sync + 'static
    {
        self.message_handler = Some(Arc::new(handler));
    }
}

/// Process stdout from a child process and forward messages
async fn stdio_reader(
    stdout: process::ChildStdout,
    tx: mpsc::Sender<Result<Message, Error>>
) -> () {
    let mut reader = tokio::io::BufReader::new(stdout);
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
                        let _ = tx.send(Ok(message)).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(Error::Json(e))).await;
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
            // Otherwise just use stdin/stdout directly
            if self.stdin.is_none() {
                self.stdin = Some(tokio::io::stdin());
                self.stdout = Some(tokio::io::stdout());
                self.reader = Some(tokio::io::BufReader::new(tokio::io::stdin()));
            }
            self.connected = true;
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
            // Write directly to stdout using tokio async operations
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
        // First drop the channels if they exist, which will signal the tasks to shut down
        if let Some(channels) = self.channels.take() {
            // Abort reader and writer tasks
            channels.reader_task.abort();
            channels.writer_task.abort();
        }

        // Close child process if any
        if let Some(mut child) = self.process.take() {
            // Kill the process
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }
        }

        self.connected = false;
        tracing::info!("STDIO transport closed");
        Ok(())
    }
}

#[async_trait::async_trait]
impl TransportMessageHandler for StdioTransport {
    async fn handle_message(
        &self,
        client_id: &str,
        message: &Message
    ) -> Result<Option<Message>, Error> {
        if let Some(handler) = &self.message_handler {
            // Forward to the registered handler
            handler.handle_message(client_id, message).await
        } else {
            Err(Error::Transport("No message handler registered".to_string()))
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}
