//! MCP Client Implementation
//!
//! This module provides a Rust client for the Model Control Protocol (MCP).
//! It implements the protocol specification and provides a high-level API for
//! interacting with MCP servers.

// Export submodules
pub mod client;
pub mod clientsession;
// pub mod handlers;
pub mod model;
pub mod services;
pub mod transport;
pub mod utils;

// Re-export key types for easier access
pub use client::{ Client, ClientBuilder, ClientConfig };
pub use clientsession::{
    ClientSession,
    ClientSessionBuilder,
    ClientSessionConfig,
    ClientSessionGuard,
};
pub use model::ClientInfo;
pub use services::{
    lifecycle::{ LifecycleManager, LifecycleState },
    notification::NotificationRouter,
    progress::{ ProgressInfo, ProgressStatus, ProgressTracker },
    request::RequestManager,
    subscription::{ Subscription, SubscriptionManager },
};

#[cfg(test)]
pub mod tests;
