// src/tools/message_parser.rs
//! Message Parser for Tool Communication
//!
//! This module provides a buffered message parser for reading structured messages
//! from tool processes, handling partial reads and message boundaries correctly.

use std::io::Error as IoError;
use tokio::io::{ AsyncBufReadExt, BufReader };

/// A parser for reading line-delimited messages from an async reader
pub struct MessageParser<R> {
    /// The buffered reader
    reader: BufReader<R>,
    /// Internal buffer for partial messages
    buffer: String,
}

impl<R: tokio::io::AsyncRead + Unpin> MessageParser<R> {
    /// Create a new message parser for the given reader
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buffer: String::new(),
        }
    }

    /// Read the next complete message, waiting for a newline delimiter
    pub async fn next_message(&mut self) -> Result<Option<String>, IoError> {
        loop {
            // Check if we have a complete message in the buffer
            if let Some(pos) = self.buffer.find('\n') {
                let message = self.buffer[..pos].to_string();
                self.buffer = self.buffer[pos + 1..].to_string();
                return Ok(Some(message));
            }

            // Read more data
            let bytes_read = self.reader.read_line(&mut self.buffer).await?;
            if bytes_read == 0 {
                // EOF
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    // Return remaining data as a message
                    let message = std::mem::take(&mut self.buffer);
                    return Ok(Some(message));
                }
            }
        }
    }

    /// Read multiple messages until EOF or the provided limit is reached
    pub async fn read_messages(&mut self, limit: Option<usize>) -> Result<Vec<String>, IoError> {
        let mut messages = Vec::new();
        let limit = limit.unwrap_or(usize::MAX);

        while messages.len() < limit {
            match self.next_message().await? {
                Some(message) => messages.push(message),
                None => {
                    break;
                }
            }
        }

        Ok(messages)
    }

    /// Read all available messages without waiting
    pub async fn read_available(&mut self) -> Result<Vec<String>, IoError> {
        let mut messages = Vec::new();

        // First check the buffer for any complete messages
        while let Some(pos) = self.buffer.find('\n') {
            let message = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 1..].to_string();
            messages.push(message);
        }

        // Then try to read more without blocking
        match
            tokio::time::timeout(std::time::Duration::from_millis(1), self.reader.fill_buf()).await
        {
            Ok(Ok(buf)) => {
                if !buf.is_empty() {
                    // More data available, process it
                    let mut read_messages = self.read_messages(None).await?;
                    messages.append(&mut read_messages);
                }
            }
            _ => (), // Timeout or error, just return what we have
        }

        Ok(messages)
    }
}
