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
pub mod server;
pub mod protocol;
pub mod types;
// pub mod server_session;

pub mod transport;

pub mod utils;
// Re-export commonly used items
pub use errors::Error;
pub use transport::Transport;
pub use transport::sse_server::SseServerTransport;
pub use transport::stdio::StdioTransport;
