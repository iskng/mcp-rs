//! Transport message handler implementation

use std::sync::Arc;

use tracing::debug;

use crate::errors::Error;
use crate::types::protocol::Message;
use crate::server::server::Server;

/// Server message handler for processing messages from transports
pub struct ServerMessageHandler<S> {
    /// Reference to the server
    server: Arc<Server<S>>,
}

impl<S> ServerMessageHandler<S> where S: Clone + Send + Sync + 'static {
    /// Create a new server message handler
    pub fn new(server: Arc<Server<S>>) -> Self {
        Self { server }
    }
}

#[async_trait::async_trait]
impl<S> crate::transport::TransportMessageHandler
    for ServerMessageHandler<S>
    where S: Clone + Send + Sync + 'static
{
    async fn handle_message(
        &self,
        client_id: &str,
        message: &Message
    ) -> Result<Option<Message>, Error> {
        // Process the message using the server and directly return the response
        debug!("Processing message from client {}: {:?}", client_id, message);
        self.server.process_message(client_id, message).await
    }
}
