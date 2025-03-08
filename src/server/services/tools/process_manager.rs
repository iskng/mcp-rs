//! Process Manager for Tool Execution
//!
//! This module implements the process management functionality for executing tools
//! as external processes, capturing their output, and managing their lifecycle.

use crate::protocol::Error;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::{Mutex, mpsc, oneshot};

/// Type of tool output (stdout or stderr)
#[derive(Debug, Clone)]
pub enum ToolOutputType {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
}

/// Output from a tool process
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Type of output (stdout or stderr)
    pub output_type: ToolOutputType,
    /// Content of the output
    pub content: String,
}

/// A running tool process
struct ToolProcess {
    /// The child process
    child: Child,
    /// The stdin handle for writing to the process
    stdin: Option<tokio::process::ChildStdin>,
    /// Channel for cancelling the process
    cancellation_tx: Option<oneshot::Sender<()>>,
}

/// Manager for spawning and interacting with tool processes
pub struct ToolProcessManager {
    /// Map of running processes by ID
    processes: Arc<Mutex<HashMap<String, ToolProcess>>>,
}

impl ToolProcessManager {
    /// Create a new tool process manager
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a new process for a tool
    pub async fn spawn_process(
        &self,
        tool_id: &str,
        command: &str,
        args: &[&str],
        env: HashMap<String, String>,
    ) -> Result<mpsc::Receiver<ToolOutput>, Error> {
        let mut command = TokioCommand::new(command);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in env {
            command.env(key, value);
        }

        // Spawn the process
        let mut child = command
            .spawn()
            .map_err(|e| Error::Tool(format!("Failed to spawn process: {}", e)))?;

        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Tool("Failed to capture stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Tool("Failed to capture stderr".to_string()))?;

        // Create communication channels
        let (output_tx, output_rx) = mpsc::channel(100);
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Store the process
        let mut processes = self.processes.lock().await;
        processes.insert(
            tool_id.to_string(),
            ToolProcess {
                child,
                stdin,
                cancellation_tx: Some(cancel_tx),
            },
        );

        // Process stdout/stderr in background tasks
        self.process_output(stdout, output_tx.clone(), ToolOutputType::Stdout);
        self.process_output(stderr, output_tx.clone(), ToolOutputType::Stderr);

        // Handle cancellation
        let processes_ref = self.processes.clone();
        let tool_id = tool_id.to_string();
        tokio::spawn(async move {
            let _ = cancel_rx.await;
            let mut processes = processes_ref.lock().await;
            if let Some(process) = processes.get_mut(&tool_id) {
                let _ = process.child.kill().await;
            }
            processes.remove(&tool_id);
        });

        Ok(output_rx)
    }

    /// Process output from a subprocess
    fn process_output<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
        &self,
        reader: R,
        tx: mpsc::Sender<ToolOutput>,
        output_type: ToolOutputType,
    ) {
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        tokio::spawn(async move {
            loop {
                line.clear();
                match buf_reader.read_line(&mut line).await {
                    Ok(0) => {
                        break;
                    } // EOF
                    Ok(_) => {
                        let output = ToolOutput {
                            output_type: output_type.clone(),
                            content: line.clone(),
                        };

                        if tx.send(output).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });
    }

    /// Send input to a running process
    pub async fn send_input(&self, tool_id: &str, input: &str) -> Result<(), Error> {
        let mut processes = self.processes.lock().await;
        let process = processes
            .get_mut(tool_id)
            .ok_or_else(|| Error::Tool(format!("Process not found: {}", tool_id)))?;

        if let Some(stdin) = &mut process.stdin {
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(|e| Error::Tool(format!("Failed to write to process: {}", e)))?;
            stdin
                .flush()
                .await
                .map_err(|e| Error::Tool(format!("Failed to flush stdin: {}", e)))?;
            Ok(())
        } else {
            Err(Error::Tool("Process stdin not available".to_string()))
        }
    }

    /// Cancel a running process
    pub async fn cancel_process(&self, tool_id: &str) -> Result<(), Error> {
        let mut processes = self.processes.lock().await;
        if let Some(process) = processes.get_mut(tool_id) {
            if let Some(tx) = process.cancellation_tx.take() {
                let _ = tx.send(());
            }
            processes.remove(tool_id);
            Ok(())
        } else {
            Err(Error::Tool(format!("Process not found: {}", tool_id)))
        }
    }
}
