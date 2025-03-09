//! MCP Client Notification Handling
//!
//! This module implements the notification routing system for the MCP client protocol.
//! It dispatches incoming notifications to registered handlers based on the method.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{ mpsc, Mutex, RwLock };
use tracing::{ debug, error, info, warn };

use crate::protocol::{
    ClientNotification,
    Error,
    JSONRPCNotification,
    LoggingMessageNotification,
    ProgressNotification,
    ResourceListChangedNotification,
    ResourceUpdatedNotification,
    ServerNotification,
    PromptListChangedNotification,
    ToolListChangedNotification,
};

/// Type alias for a notification handler function
pub type NotificationHandlerFn = Box<
    dyn (Fn(JSONRPCNotification) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>) +
        Send +
        Sync
>;

/// Router for MCP protocol notifications
pub struct NotificationRouter {
    /// Registered notification handlers by method
    handlers: RwLock<HashMap<String, Vec<NotificationHandlerFn>>>,

    /// Channel for broadcasting notifications to subscribers
    broadcast_tx: mpsc::Sender<JSONRPCNotification>,

    /// Receiver for the broadcast channel
    broadcast_rx: Mutex<Option<mpsc::Receiver<JSONRPCNotification>>>,
}

impl NotificationRouter {
    /// Create a new notification router
    pub fn new() -> Self {
        let (broadcast_tx, broadcast_rx) = mpsc::channel(100);

        Self {
            handlers: RwLock::new(HashMap::new()),
            broadcast_tx,
            broadcast_rx: Mutex::new(Some(broadcast_rx)),
        }
    }

    /// Register a handler for a specific notification method
    pub async fn register_handler(
        &self,
        method: String,
        handler: NotificationHandlerFn
    ) -> Result<(), Error> {
        let mut handlers = self.handlers.write().await;

        handlers.entry(method.clone()).or_insert_with(Vec::new).push(handler);

        debug!("Registered handler for notification method: {}", method);
        Ok(())
    }

    /// Handle a notification by dispatching to registered handlers
    pub async fn handle_notification(
        &self,
        notification: JSONRPCNotification
    ) -> Result<(), Error> {
        // Get the method from the notification
        let method = &notification.method;

        // Send to broadcast channel for subscribers
        if let Err(e) = self.broadcast_tx.send(notification.clone()).await {
            warn!("Failed to broadcast notification: {}", e);
        }

        // Get handlers for this method
        let handlers = self.handlers.read().await;
        let method_handlers = handlers.get(method);

        // If we have handlers for this method, invoke them
        if let Some(handlers) = method_handlers {
            debug!("Found {} handlers for notification method: {}", handlers.len(), method);

            // Call each handler
            for handler in handlers {
                // Use the handler function to create a future
                let fut = handler(notification.clone());

                // Spawn the future as a separate task
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                        error!("Error in notification handler: {}", e);
                    }
                });
            }
        } else {
            debug!("No handlers found for notification method: {}", method);
        }

        Ok(())
    }

    /// Route a notification to registered handlers (alias for handle_notification)
    pub async fn route_notification(
        &self,
        notification: &JSONRPCNotification
    ) -> Result<(), Error> {
        self.handle_notification(notification.clone()).await
    }

    /// Get a receiver for all notifications
    pub async fn subscribe(&self) -> mpsc::Receiver<JSONRPCNotification> {
        let (tx, rx) = mpsc::channel(100);

        // Register a handler that forwards all notifications to this receiver
        self.register_handler(
            "*".to_string(),
            Box::new(move |notification| {
                let tx = tx.clone();
                Box::pin(async move {
                    if let Err(e) = tx.send(notification).await {
                        warn!("Failed to forward notification to subscriber: {}", e);
                    }
                    Ok(())
                })
            })
        ).await.expect("Failed to register notification handler");

        rx
    }

    /// Subscribe to notifications with a filter
    pub async fn subscribe_to<F, T>(&self, filter: F) -> mpsc::Receiver<T>
        where F: Fn(&JSONRPCNotification) -> Option<T> + Send + Sync + 'static, T: Send + 'static
    {
        // Create a channel for filtered notifications
        let (tx, rx) = mpsc::channel(100);

        // Clone the broadcast receiver
        let mut notifications = self.subscribe().await;

        // Create an Arc around the filter for sharing
        let filter = Arc::new(filter);

        // Spawn a task to filter and forward notifications
        tokio::spawn(async move {
            while let Some(notification) = notifications.recv().await {
                // Apply the filter to each notification
                if let Some(typed) = filter(&notification) {
                    if let Err(e) = tx.send(typed).await {
                        warn!("Failed to forward filtered notification to subscriber: {}", e);
                        break;
                    }
                }
            }
        });

        rx
    }

    /// Parse a notification into a ServerNotification
    pub fn parse_server_notification(
        notification: &JSONRPCNotification
    ) -> Result<ServerNotification, Error> {
        // Check the method to determine the notification type
        match notification.method.as_str() {
            "notifications/progress" => {
                // Parse progress notification
                let progress_notification = serde_json
                    ::from_value::<ProgressNotification>(serde_json::to_value(notification)?)
                    .map_err(|e| {
                        Error::Other(format!("Failed to parse progress notification: {}", e))
                    })?;

                Ok(ServerNotification::Progress(progress_notification))
            }
            "notifications/resources/list_changed" => {
                // Parse resource list changed notification
                let params = serde_json
                    ::from_value::<ResourceListChangedNotification>(
                        serde_json::to_value(notification)?
                    )
                    .map_err(|e| {
                        Error::Other(
                            format!("Failed to parse resource list changed notification: {}", e)
                        )
                    })?;

                Ok(ServerNotification::ResourceListChanged(params))
            }
            "notifications/resources/updated" => {
                // Parse resource updated notification
                let params = serde_json
                    ::from_value::<ResourceUpdatedNotification>(serde_json::to_value(notification)?)
                    .map_err(|e| {
                        Error::Other(
                            format!("Failed to parse resource updated notification: {}", e)
                        )
                    })?;

                Ok(ServerNotification::ResourceUpdated(params))
            }
            "notifications/prompts/list_changed" => {
                Ok(
                    ServerNotification::PromptListChanged(
                        serde_json
                            ::from_value::<PromptListChangedNotification>(
                                serde_json::to_value(notification)?
                            )
                            .map_err(|e| {
                                Error::Other(
                                    format!("Failed to parse prompt list changed notification: {}", e)
                                )
                            })?
                    )
                )
            }
            "notifications/tools/list_changed" => {
                Ok(
                    ServerNotification::ToolListChanged(
                        serde_json
                            ::from_value::<ToolListChangedNotification>(
                                serde_json::to_value(notification)?
                            )
                            .map_err(|e| {
                                Error::Other(
                                    format!("Failed to parse tool list changed notification: {}", e)
                                )
                            })?
                    )
                )
            }
            "notifications/logging/message" => {
                Ok(
                    ServerNotification::LoggingMessage(
                        serde_json
                            ::from_value::<LoggingMessageNotification>(
                                serde_json::to_value(notification)?
                            )
                            .map_err(|e| {
                                Error::Other(
                                    format!("Failed to parse logging message notification: {}", e)
                                )
                            })?
                    )
                )
            }
            _ => {
                // Unknown notification type
                Err(Error::Other(format!("Unknown notification method: {}", notification.method)))
            }
        }
    }
}

impl Default for NotificationRouter {
    fn default() -> Self {
        Self::new()
    }
}
