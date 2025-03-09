//! MCP Client Progress Tracking
//!
//! This module provides functionality for tracking progress of long-running operations
//! through progress notifications.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{ broadcast, mpsc, Mutex, RwLock };
use tracing::{ debug, warn };

use crate::client::notification::NotificationRouter;
use crate::protocol::{ Error, ProgressNotification, ProgressParams, ProgressToken };

/// Status of a progress operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressStatus {
    /// Operation is in progress
    Running,

    /// Operation has completed successfully
    Completed,

    /// Operation has failed
    Failed,

    /// Operation was cancelled
    Cancelled,
}

/// Information about a progress operation
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// The progress token
    pub token: String,

    /// Current status of the operation
    pub status: ProgressStatus,

    /// Current progress percentage (0-100)
    pub percentage: Option<u32>,

    /// Human-readable message about the progress
    pub message: Option<String>,
}

impl From<ProgressNotification> for ProgressInfo {
    fn from(notification: ProgressNotification) -> Self {
        // Extract progress params
        let params = &notification.params;

        // Determine the status based on the progress value
        let status = if params.progress >= 100.0 {
            ProgressStatus::Completed
        } else {
            ProgressStatus::Running
        };

        // Convert the progress token to a string
        let token = match &params.progress_token {
            ProgressToken::String(s) => s.clone(),
            ProgressToken::Number(n) => n.to_string(),
        };

        Self {
            token,
            status,
            percentage: Some(params.progress as u32),
            message: None,
        }
    }
}

/// Handler for tracking progress operations
pub struct ProgressTracker {
    /// Active progress operations by token
    active_progress: RwLock<HashMap<String, ProgressInfo>>,

    /// Broadcast channel for progress updates
    progress_tx: broadcast::Sender<ProgressInfo>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        let (progress_tx, _) = broadcast::channel(100);

        Self {
            active_progress: RwLock::new(HashMap::new()),
            progress_tx,
        }
    }

    /// Initialize the progress tracker with a notification router
    pub async fn init(&self, notification_router: Arc<NotificationRouter>) -> Result<(), Error> {
        // Get a clone of the progress_tx for the handler
        let progress_tx = self.progress_tx.clone();

        // Get an Arc to self for the handler
        let tracker = Arc::new(self.clone());

        // Register a handler for progress notifications
        notification_router.register_handler(
            "notifications/progress".to_string(),
            Box::new(move |notification| {
                let progress_tx = progress_tx.clone();
                let tracker = tracker.clone();

                Box::pin(async move {
                    // Parse the notification as a progress notification
                    match
                        serde_json::from_value::<ProgressNotification>(
                            notification.params
                                .clone()
                                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
                        )
                    {
                        Ok(progress) => {
                            // Convert to ProgressInfo
                            let info = ProgressInfo::from(progress.clone());

                            // Update the active progress map
                            tracker.update_progress(info.clone()).await;

                            // Broadcast the progress update
                            if let Err(e) = progress_tx.send(info) {
                                debug!("Failed to broadcast progress update: {}", e);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse progress notification: {}", e);
                        }
                    }

                    Ok(())
                })
            })
        ).await?;

        Ok(())
    }

    /// Update progress information
    async fn update_progress(&self, info: ProgressInfo) {
        let mut active = self.active_progress.write().await;

        match info.status {
            // For completed, failed, or cancelled, remove from active tracking
            ProgressStatus::Completed | ProgressStatus::Failed | ProgressStatus::Cancelled => {
                active.remove(&info.token);
                debug!("Progress operation {} completed with status {:?}", info.token, info.status);
            }
            // For running, update or add to active tracking
            ProgressStatus::Running => {
                active.insert(info.token.clone(), info);
            }
        }
    }

    /// Get information about a specific progress operation
    pub async fn get_progress(&self, token: &str) -> Option<ProgressInfo> {
        let active = self.active_progress.read().await;
        active.get(token).cloned()
    }

    /// Get all active progress operations
    pub async fn get_all_progress(&self) -> Vec<ProgressInfo> {
        let active = self.active_progress.read().await;
        active.values().cloned().collect()
    }

    /// Subscribe to progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressInfo> {
        self.progress_tx.subscribe()
    }

    /// Subscribe to progress updates for a specific token
    pub fn subscribe_to_token(&self, token: String) -> mpsc::Receiver<ProgressInfo> {
        let (tx, rx) = mpsc::channel(100);

        // Create a broadcast receiver
        let mut broadcast_rx = self.progress_tx.subscribe();

        // Spawn a task to filter and forward progress updates
        tokio::spawn(async move {
            while let Ok(progress) = broadcast_rx.recv().await {
                if progress.token == token {
                    if tx.send(progress).await.is_err() {
                        // Channel closed, exit loop
                        break;
                    }
                }
            }
        });

        rx
    }

    /// Wait for a progress operation to complete
    pub async fn wait_for_completion(&self, token: &str) -> Result<ProgressInfo, Error> {
        let mut rx = self.subscribe_to_token(token.to_string());

        // First check if we already have a completed status
        if let Some(info) = self.get_progress(token).await {
            match info.status {
                ProgressStatus::Completed => {
                    return Ok(info);
                }
                ProgressStatus::Failed => {
                    return Err(
                        Error::Other(
                            format!(
                                "Operation {} failed: {}",
                                token,
                                info.message.unwrap_or_else(|| "No error message".to_string())
                            )
                        )
                    );
                }
                ProgressStatus::Cancelled => {
                    return Err(Error::Other(format!("Operation {} was cancelled", token)));
                }
                _ => {} // Continue waiting for completion
            }
        }

        // Wait for progress updates
        while let Some(info) = rx.recv().await {
            match info.status {
                ProgressStatus::Completed => {
                    return Ok(info);
                }
                ProgressStatus::Failed => {
                    return Err(
                        Error::Other(
                            format!(
                                "Operation {} failed: {}",
                                token,
                                info.message.unwrap_or_else(|| "No error message".to_string())
                            )
                        )
                    );
                }
                ProgressStatus::Cancelled => {
                    return Err(Error::Other(format!("Operation {} was cancelled", token)));
                }
                _ => {} // Continue waiting for completion
            }
        }

        // If we get here, the channel closed without completion
        Err(Error::Other(format!("Progress tracking for {} ended without completion", token)))
    }
}

impl Clone for ProgressTracker {
    fn clone(&self) -> Self {
        Self {
            active_progress: RwLock::new(HashMap::new()),
            progress_tx: self.progress_tx.clone(),
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
