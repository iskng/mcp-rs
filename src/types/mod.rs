//! MCP Type Definitions
//!
//! This module re-exports various type definitions used in the MCP protocol, organizing
//! them into submodules for clarity and ease of use. It includes types for initialization,
//! prompts, resources, and tools.

pub mod initialize;
pub mod prompts;
pub mod resources;
pub mod tools;

// Re-export common types from each module
pub use initialize::{
    ClientCapabilities, Implementation, InitializeRequestParams, InitializeResult,
    ServerCapabilities,
};
pub use prompts::{ContentPart, Message, Prompt, PromptArgument, Role};
pub use resources::{ListResourcesParams, ListResourcesResult, Resource};
pub use tools::{CallToolParams, CallToolResult, Tool, ToolParameter};
