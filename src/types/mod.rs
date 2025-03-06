//! MCP Type Definitions
//!
//! This module re-exports various type definitions used in the MCP protocol, organizing
//! them into submodules for clarity and ease of use. It includes types for initialization,
//! prompts, resources, and tools.

pub mod initialize;
pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod tools;

// Re-export common types from each module
pub use initialize::{
    ClientCapabilities,
    Implementation,
    InitializeRequestParams,
    InitializeResult,
    ServerCapabilities,
};
pub use prompts::{ ContentPart, PromptMessage, Prompt, PromptArgument };
pub use protocol::{
    Message as ProtocolMessage,
    Request,
    Response,
    Notification,
    ResponseOutcome,
    ErrorData,
    NotificationMessage,
    RequestMessage,
    ResponseMessage,
    success_response,
    error_response,
};
pub use resources::{ ListResourcesParams, ListResourcesResult, Resource };
pub use tools::{ CallToolParams, CallToolResult, Tool, ToolParameter };
