//! Service Provider
//!
//! This module provides a centralized access point for all client services.

use std::sync::Arc;

use crate::client::services::{
    lifecycle::LifecycleManager, notification::NotificationRouter, progress::ProgressTracker,
    request::RequestManager, subscription::SubscriptionManager,
};

/// Service provider for accessing all client services
pub struct ServiceProvider {
    /// Lifecycle manager for tracking client state
    lifecycle_manager: Arc<LifecycleManager>,

    /// Notification router for handling notifications
    notification_router: Arc<NotificationRouter>,

    /// Progress tracker for handling progress notifications
    progress_tracker: Arc<ProgressTracker>,

    /// Request manager for tracking requests
    request_manager: Arc<RequestManager>,

    /// Subscription manager for handling subscriptions
    subscription_manager: Arc<SubscriptionManager>,
}

impl ServiceProvider {
    /// Create a new service provider with default services
    pub fn new() -> Self {
        // Create services
        let lifecycle_manager = Arc::new(LifecycleManager::new());
        let notification_router = Arc::new(NotificationRouter::new());
        let request_manager = Arc::new(RequestManager::default());
        let progress_tracker = Arc::new(ProgressTracker::new());

        // Create subscription manager after other services
        let subscription_manager = Arc::new(SubscriptionManager::new(notification_router.clone()));

        // Initialize progress tracker with notification router
        // This is a placeholder until we implement this properly
        // progress_tracker.init(notification_router.clone()).await.unwrap();

        Self {
            lifecycle_manager,
            notification_router,
            progress_tracker,
            request_manager,
            subscription_manager,
        }
    }

    /// Get a reference to the lifecycle manager
    pub fn lifecycle_manager(&self) -> Arc<LifecycleManager> {
        self.lifecycle_manager.clone()
    }

    /// Get a reference to the notification router
    pub fn notification_router(&self) -> Arc<NotificationRouter> {
        self.notification_router.clone()
    }

    /// Get a reference to the progress tracker
    pub fn progress_tracker(&self) -> Arc<ProgressTracker> {
        self.progress_tracker.clone()
    }

    /// Get a reference to the request manager
    pub fn request_manager(&self) -> Arc<RequestManager> {
        self.request_manager.clone()
    }

    /// Get a reference to the subscription manager
    pub fn subscription_manager(&self) -> Arc<SubscriptionManager> {
        self.subscription_manager.clone()
    }
}

impl Default for ServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
