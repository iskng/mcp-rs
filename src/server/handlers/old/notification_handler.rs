//! Handler for notification messages

use async_trait::async_trait;
use tracing::{ info, debug };

use crate::errors::Error;
use crate::types::protocol::{ Message, NotificationMessage };
use crate::server::MessageHandler;

/// Handler for notification messages
pub struct NotificationHandler;

impl NotificationHandler {
    /// Create a new instance of the notification handler
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl MessageHandler for NotificationHandler {
    async fn handle(&self, client_id: &str, message: &Message) -> Result<Option<Message>, Error> {
        match message {
            Message::Notification(notif) => {
                info!("Processing notification '{}' from client {}", notif.method, client_id);

                // Parse into typed notification
                match Message::parse_notification(notif) {
                    Ok(typed_notif) => {
                        debug!("Parsed notification: {:?}", typed_notif);

                        // For specific notifications, we would have additional handling
                        match typed_notif {
                            NotificationMessage::Initialized { meta } => {
                                info!("Client {} initialized with meta {:?}", client_id, meta);
                                // Handle initialization notification
                            }
                            NotificationMessage::Progress { progress, progress_token, total } => {
                                info!(
                                    "Progress update from client {}: {}/{:?} (progress_token: {:?})",
                                    client_id,
                                    progress,
                                    total,
                                    progress_token
                                );
                                // Handle progress notification
                            }
                            NotificationMessage::ResourcesListChanged { meta } => {
                                info!(
                                    "Resources list changed for client {} with meta {:?}",
                                    client_id,
                                    meta
                                );
                                // Handle resources list changed notification
                            }
                            NotificationMessage::ResourcesUpdated { uri } => {
                                info!("Resource updated for client {}: {} ", client_id, uri);
                                // Handle resource updated notification
                            }
                            NotificationMessage::PromptsListChanged { meta } => {
                                info!(
                                    "Prompts list changed for client {} with meta {:?}",
                                    client_id,
                                    meta
                                );
                                // Handle prompts list changed notification
                            }
                            NotificationMessage::ToolsListChanged { meta } => {
                                info!(
                                    "Tools list changed for client {} with meta {:?}",
                                    client_id,
                                    meta
                                );
                                // Handle tools list changed notification
                            }
                            NotificationMessage::CancelRequest { request_id, reason } => {
                                info!(
                                    "Client {} cancelled request {:?} (reason: {:?})",
                                    client_id,
                                    request_id,
                                    reason
                                );
                                // Handle cancel request notification
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Error parsing notification: {}", e);
                        // Continue with generic handling if parsing fails
                    }
                }

                // For now, we don't send any responses to notifications
                Ok(None)
            }
            _ => {
                // This handler only handles notifications
                Err(Error::Transport("Non-notification message received".into()))
            }
        }
    }
}
