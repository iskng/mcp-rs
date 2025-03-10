//! Client protocol handlers
//!
//! This module contains handlers for different aspects of the MCP protocol.
//! Each handler is responsible for a specific domain of the protocol.

pub mod completion;
pub mod composite;
pub mod handshake;
pub mod prompts;
pub mod resources;
pub mod route_handler;
pub mod tools;

// Re-export the main handler types
pub use completion::CompletionHandler;
pub use composite::CompositeClientHandler;
pub use handshake::HandshakeHandler;
pub use prompts::PromptHandler;
pub use resources::ResourceHandler;
pub use route_handler::RouteHandler;
pub use tools::ToolHandler;
