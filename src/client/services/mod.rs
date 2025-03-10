//! Client services
//!
//! This module contains service implementations that manage client state
//! and provide shared functionality to the handlers.

pub mod lifecycle;
pub mod notification;
pub mod progress;
pub mod request;
pub mod service_provider;
pub mod subscription;

// Re-export the main service types
pub use lifecycle::LifecycleManager;
pub use notification::NotificationRouter;
pub use progress::ProgressTracker;
pub use request::RequestManager;
pub use service_provider::ServiceProvider;
pub use subscription::SubscriptionManager;
