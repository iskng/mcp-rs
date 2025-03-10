//! Server message handlers organized by domain
//!
//! This module contains handler traits and implementations for processing
//! client messages and dispatching them to appropriate domain-specific handlers.

// Server handler interface
mod route_handler;
pub use route_handler::RouteHandler;

// Specialized handlers
mod handshake;
pub use handshake::{DefaultHandshakeHandler, HandshakeHandler, PingResult};

// Initialize handler
mod initialize;
pub use initialize::{DefaultInitializeHandler, InitializeHandler, InitializeHandlerBuilder};

// Tool handlers
mod tools;
pub use tools::{DefaultToolHandler, ToolHandler};

// Resource handlers
mod resources;
pub use resources::{DefaultResourceHandler, ResourceHandler};

// Composite implementation - concrete implementation
pub mod composite;
pub use composite::CompositeServerHandler;

// We'll implement Axum integration later when needed
// mod axum_integration;
// pub use axum_integration::{ handle_client_message, create_handler };
