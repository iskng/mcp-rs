//! MCP Client Subscription Management
//!
//! This module provides a typed subscription system for MCP notifications.
//! It allows clients to subscribe to specific types of notifications and
//! receive them in a type-safe manner.

use futures::stream::Stream;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ Context, Poll };
use tokio::sync::{ broadcast, mpsc, oneshot };
use tracing::{ debug, warn };

use crate::client::services::notification::NotificationRouter;
use crate::protocol::{
    Error,
    JSONRPCNotification,
    Method,
    ProgressNotification,
    ResourceListChangedNotification,
    ResourceUpdatedNotification,
};

/// A token that cancels a subscription when dropped
pub struct CancelToken {
    cancel_tx: Option<oneshot::Sender<()>>,
    subscription_id: String,
}

impl fmt::Debug for CancelToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelToken").field("subscription_id", &self.subscription_id).finish()
    }
}

impl Drop for CancelToken {
    fn drop(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            if tx.send(()).is_err() {
                debug!("Failed to send cancellation signal - receiver dropped");
            }
        }
    }
}

/// A subscription to a specific type of notification
pub struct Subscription<T> {
    /// Receiver for the notification messages
    rx: mpsc::Receiver<T>,

    /// Token that cancels the subscription when dropped
    _cancel_token: CancelToken,
}

impl<T> Subscription<T> {
    /// Create a new subscription
    pub(crate) fn new(rx: mpsc::Receiver<T>, cancel_token: CancelToken) -> Self {
        Self {
            rx,
            _cancel_token: cancel_token,
        }
    }

    /// Get the next notification, or None if the subscription has ended
    pub async fn next(&mut self) -> Option<T> {
        self.rx.recv().await
    }

    /// Convert the subscription into a stream
    pub fn into_stream(self) -> SubscriptionStream<T> {
        SubscriptionStream {
            rx: self.rx,
            _cancel_token: self._cancel_token,
        }
    }
}

/// A stream that yields notifications from a subscription
pub struct SubscriptionStream<T> {
    /// Receiver for the notification messages
    rx: mpsc::Receiver<T>,

    /// Token that cancels the subscription when dropped
    _cancel_token: CancelToken,
}

impl<T> Stream for SubscriptionStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_recv(cx)
    }
}

/// Manager for handling subscriptions to different types of notifications
pub struct SubscriptionManager {
    /// Router for notifications
    notification_router: Arc<NotificationRouter>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new(notification_router: Arc<NotificationRouter>) -> Self {
        Self {
            notification_router,
        }
    }

    /// Subscribe to all notifications
    pub async fn subscribe_all(&self) -> Subscription<JSONRPCNotification> {
        // Create a channel for notifications
        let (tx, rx) = mpsc::channel(100);

        // Create a cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Generate a subscription ID
        let subscription_id = format!("all-{}", uuid::Uuid::new_v4());

        // Get a clone of the sender for the task
        let tx_clone = tx.clone();

        // Register a handler for all notifications
        self.notification_router
            .register_handler(
                Method::NotificationsAll,
                Box::new(move |notification| {
                    let tx = tx_clone.clone();
                    Box::pin(async move {
                        if let Err(e) = tx.send(notification).await {
                            warn!("Failed to send notification to subscriber: {}", e);
                            return Err(Error::Other("Subscriber channel closed".to_string()));
                        }
                        Ok(())
                    })
                })
            ).await
            .expect("Failed to register notification handler");

        // Create a task that will close the channel when the cancellation token is dropped
        tokio::spawn(async move {
            // Wait for cancellation or channel closure
            let _ = cancel_rx.await;

            // Close the sender by dropping it
            drop(tx);
        });

        // Create the subscription
        Subscription::new(rx, CancelToken {
            cancel_tx: Some(cancel_tx),
            subscription_id,
        })
    }

    /// Subscribe to progress notifications
    pub async fn subscribe_progress(&self) -> Subscription<ProgressNotification> {
        // Create a channel for notifications
        let (tx, rx) = mpsc::channel(100);

        // Create a cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Generate a subscription ID
        let subscription_id = format!("progress-{}", uuid::Uuid::new_v4());

        // Get a clone of the sender for the task
        let tx_clone = tx.clone();

        // Register a handler for progress notifications
        self.notification_router
            .register_handler(
                Method::NotificationsProgress,
                Box::new(move |notification| {
                    let tx = tx_clone.clone();
                    Box::pin(async move {
                        // Parse the notification into a progress notification
                        match
                            serde_json::from_value::<ProgressNotification>(
                                notification.params
                                    .clone()
                                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
                            )
                        {
                            Ok(progress) => {
                                if let Err(e) = tx.send(progress).await {
                                    warn!("Failed to send progress notification to subscriber: {}", e);
                                    return Err(
                                        Error::Other("Subscriber channel closed".to_string())
                                    );
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse progress notification: {}", e);
                            }
                        }

                        Ok(())
                    })
                })
            ).await
            .expect("Failed to register notification handler");

        // Create a task that will close the channel when the cancellation token is dropped
        tokio::spawn(async move {
            // Wait for cancellation or channel closure
            let _ = cancel_rx.await;

            // Close the sender by dropping it
            drop(tx);
        });

        // Create the subscription
        Subscription::new(rx, CancelToken {
            cancel_tx: Some(cancel_tx),
            subscription_id,
        })
    }

    /// Subscribe to resource updated notifications
    pub async fn subscribe_resource_updates(&self) -> Subscription<ResourceUpdatedNotification> {
        // Create a channel for notifications
        let (tx, rx) = mpsc::channel(100);

        // Create a cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Generate a subscription ID
        let subscription_id = format!("resource-updates-{}", uuid::Uuid::new_v4());

        // Get a clone of the sender for the task
        let tx_clone = tx.clone();

        // Register a handler for resource updated notifications
        self.notification_router
            .register_handler(
                Method::NotificationsResourcesUpdated,
                Box::new(move |notification| {
                    let tx = tx_clone.clone();
                    Box::pin(async move {
                        // Parse the notification into a resource updated notification
                        match
                            serde_json::from_value::<ResourceUpdatedNotification>(
                                notification.params
                                    .clone()
                                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
                            )
                        {
                            Ok(update) => {
                                if let Err(e) = tx.send(update).await {
                                    warn!("Failed to send resource updated notification to subscriber: {}", e);
                                    return Err(
                                        Error::Other("Subscriber channel closed".to_string())
                                    );
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse resource updated notification: {}", e);
                            }
                        }

                        Ok(())
                    })
                })
            ).await
            .expect("Failed to register notification handler");

        // Create a task that will close the channel when the cancellation token is dropped
        tokio::spawn(async move {
            // Wait for cancellation or channel closure
            let _ = cancel_rx.await;

            // Close the sender by dropping it
            drop(tx);
        });

        // Create the subscription
        Subscription::new(rx, CancelToken {
            cancel_tx: Some(cancel_tx),
            subscription_id,
        })
    }

    /// Subscribe to resource list changed notifications
    pub async fn subscribe_resource_list_changes(
        &self
    ) -> Subscription<ResourceListChangedNotification> {
        // Create a channel for notifications
        let (tx, rx) = mpsc::channel(100);

        // Create a cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Generate a subscription ID
        let subscription_id = format!("resource-list-changes-{}", uuid::Uuid::new_v4());

        // Get a clone of the sender for the task
        let tx_clone = tx.clone();

        // Register a handler for resource list changed notifications
        self.notification_router
            .register_handler(
                Method::NotificationsResourcesListChanged,
                Box::new(move |notification| {
                    let tx = tx_clone.clone();
                    Box::pin(async move {
                        // Parse the notification into a resource list changed notification
                        match
                            serde_json::from_value::<ResourceListChangedNotification>(
                                notification.params
                                    .clone()
                                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
                            )
                        {
                            Ok(list_change) => {
                                if let Err(e) = tx.send(list_change).await {
                                    warn!("Failed to send resource list changed notification to subscriber: {}", e);
                                    return Err(
                                        Error::Other("Subscriber channel closed".to_string())
                                    );
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse resource list changed notification: {}", e);
                            }
                        }

                        Ok(())
                    })
                })
            ).await
            .expect("Failed to register notification handler");

        // Create a task that will close the channel when the cancellation token is dropped
        tokio::spawn(async move {
            // Wait for cancellation or channel closure
            let _ = cancel_rx.await;

            // Close the sender by dropping it
            drop(tx);
        });

        // Create the subscription
        Subscription::new(rx, CancelToken {
            cancel_tx: Some(cancel_tx),
            subscription_id,
        })
    }

    /// Subscribe to filtered notifications
    pub async fn subscribe_filtered<F, T>(&self, method: Method, filter: F) -> Subscription<T>
        where F: Fn(JSONRPCNotification) -> Option<T> + Send + Sync + 'static, T: Send + 'static
    {
        // Create a channel for notifications
        let (tx, rx) = mpsc::channel(100);

        // Create a cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Generate a subscription ID
        let subscription_id = format!("filtered-{}-{}", method, uuid::Uuid::new_v4());

        // Clone a filter function to move into the handler closure
        let filter = Arc::new(filter);
        let filter_clone = filter.clone();
        let tx_clone = tx.clone();
        // Register a handler for the specified method
        self.notification_router
            .register_handler(
                method.clone(),
                Box::new(move |notification| {
                    let tx = tx.clone();
                    let filter = filter_clone.clone();
                    Box::pin(async move {
                        // Apply the filter function
                        if let Some(typed_notification) = filter(notification) {
                            if let Err(e) = tx.send(typed_notification).await {
                                warn!("Failed to send filtered notification to subscriber: {}", e);
                                return Err(Error::Other("Subscriber channel closed".to_string()));
                            }
                        }

                        Ok(())
                    })
                })
            ).await
            .expect("Failed to register notification handler");

        // Create a task that will close the channel when the cancellation token is dropped
        tokio::spawn(async move {
            // Wait for cancellation or channel closure
            let _ = cancel_rx.await;

            // Close the sender by dropping it
            drop(tx_clone);
        });

        // Create the subscription
        Subscription::new(rx, CancelToken {
            cancel_tx: Some(cancel_tx),
            subscription_id,
        })
    }

    /// Create a subscription from any receiver
    pub fn create_subscription<T>(
        &self,
        mut broadcast_rx: broadcast::Receiver<T>,
        subscription_id: String
    ) -> Subscription<T>
        where T: Clone + Send + 'static
    {
        // Convert broadcast to mpsc channel
        let (tx, rx) = mpsc::channel(100);

        // Create a cancel token
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        // Spawn a task that forwards from broadcast to mpsc
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Wait for cancellation
                    _ = &mut cancel_rx => break,

                    // Forward messages
                    result = broadcast_rx.recv() => {
                        match result {
                            Ok(item) => {
                                if tx.send(item).await.is_err() {
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                // Just continue if we lagged
                                continue;
                            }
                        }
                    }
                }
            }
        });

        // Create the subscription
        Subscription::new(rx, CancelToken {
            cancel_tx: Some(cancel_tx),
            subscription_id,
        })
    }
}
