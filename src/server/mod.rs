//! Server module for message handling and business logic
//!
//! This module contains the core server logic, message handlers,
//! and related components.

use async_trait::async_trait;

use crate::errors::Error;
use crate::types::protocol::{ Message, MessageType, RequestType, NotificationType, ResponseType };
pub mod server;
pub mod handlers;

pub mod resources;

pub mod tools;

/// MessageHandler trait defines the interface for handling messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle a message from a client
    ///
    /// # Arguments
    /// * `client_id` - The ID of the client sending the message
    /// * `message` - The message to handle
    ///
    /// # Returns
    /// A `Result` indicating success or failure
    async fn handle(&self, client_id: &str, message: &Message) -> Result<Option<Message>, Error>;
}

/// Re-export the Server
pub use crate::server::server::Server;
