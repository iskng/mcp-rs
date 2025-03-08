//! Handler for request messages

use async_trait::async_trait;
use tracing::{ debug, info, warn };

use crate::errors::Error;
use crate::types::protocol::{ Message, RequestMessage, error_response, success_response };
use crate::server::MessageHandler;

/// Handler for request messages
pub struct RequestHandler;

impl RequestHandler {
    /// Create a new instance of the request handler
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl MessageHandler for RequestHandler {
    async fn handle(&self, client_id: &str, message: &Message) -> Result<Option<Message>, Error> {
        match message {
            Message::Request(req) => {
                // Special case for shutdown request
                if req.method == "shutdown" {
                    debug!("Received shutdown request from client {}", client_id);
                    return Ok(None);
                }

                // Parse into typed request
                match Message::parse_request(req) {
                    Ok(typed_req) => {
                        debug!("Parsed request: {:?}", typed_req);

                        // Handle the typed request
                        match typed_req {
                            RequestMessage::Ping { meta } => {
                                info!(
                                    "Ping request from client {} with meta {:?}",
                                    client_id,
                                    meta
                                );
                                // Return a simple pong response
                                return Ok(
                                    Some(
                                        Message::Response(
                                            success_response(req.id, serde_json::json!({}))
                                        )
                                    )
                                );
                            }
                            // Additional request types would be handled here
                            _ => {
                                warn!("Unhandled request type: {:?}", typed_req);
                                // Return a method not found error
                                return Ok(
                                    Some(
                                        Message::Response(
                                            error_response(
                                                req.id,
                                                -32601,
                                                &format!("Method not implemented: {}", req.method),
                                                None
                                            )
                                        )
                                    )
                                );
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Error parsing request: {}", e);
                        // Return a parse error
                        return Ok(
                            Some(
                                Message::Response(
                                    error_response(
                                        req.id,
                                        -32700,
                                        &format!("Parse error: {}", e),
                                        None
                                    )
                                )
                            )
                        );
                    }
                }
            }
            _ => {
                // This handler only handles requests
                Err(Error::Transport("Non-request message received".into()))
            }
        }
    }
}
