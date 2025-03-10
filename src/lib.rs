//! MCP Rust Library
//!
//! This crate provides a Rust implementation of the Model Context Protocol (MCP),
//! enabling seamless integration between AI applications and external data sources
//! or tools. It includes both client and server components, supporting various
//! transports like STDIO and SSE, with a focus on type safety, performance, and
//! extensibility.

// Re-export core components
pub mod client;
pub mod protocol;
pub mod server;
// pub mod server_session;

pub mod utils;
// Re-export commonly used items
pub use server::transport::sse::SseServerTransport;
pub use server::transport::stdio::StdioTransport;
