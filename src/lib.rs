//! MCP Rust Library
//!
//! This crate provides a Rust implementation of the Model Context Protocol (MCP),
//! enabling seamless integration between AI applications and external data sources
//! or tools. It includes both client and server components, supporting various
//! transports like STDIO and SSE, with a focus on type safety, performance, and
//! extensibility.

// Re-export core components
pub mod client;
pub mod errors;
pub mod lifecycle;
pub mod messages;
pub mod server;
pub mod server_session;
pub mod tools;
pub mod transport;
pub mod types;
pub mod utils;
// Re-export commonly used items
pub use errors::Error;
pub use lifecycle::LifecycleManager;
pub use transport::Transport;
pub use transport::sse::SseTransport;
pub use transport::stdio::StdioTransport;
