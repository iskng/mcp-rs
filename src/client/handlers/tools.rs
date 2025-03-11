//! Tool Handler
//!
//! This module provides handlers for tool-related operations in the MCP protocol.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::client::client::Client;
use crate::client::services::ServiceProvider;
use crate::client::transport::DirectIOTransport;
use crate::protocol::{ CallToolParams, CallToolResult, Error, ListToolsResult };

/// Handler trait for tool operations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// List tools from the server
    async fn list_tools(&self) -> Result<ListToolsResult, Error>;

    /// Call a tool on the server
    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error>;

    /// Find a tool by name
    async fn find_tool_by_name(
        &self,
        name: &str
    ) -> Result<crate::client::services::tools::ToolInfo, Error>;

    /// Call a tool and wait for its completion
    async fn call_tool_and_wait(
        &self,
        name: &str,
        args: Value,
        timeout: Option<Duration>
    ) -> Result<CallToolResult, Error>;

    /// Call a tool with string arguments
    async fn call_tool_with_string_args(
        &self,
        name: &str,
        args: HashMap<String, String>
    ) -> Result<CallToolResult, Error>;

    /// Call a tool by name
    async fn call_tool_by_name(&self, name: &str, args: Value) -> Result<CallToolResult, Error>;
}

/// Default implementation of the tool handler
pub struct DefaultToolHandler<T: DirectIOTransport + 'static> {
    /// The underlying client
    client: Arc<Client<T>>,

    /// Service provider for accessing services
    service_provider: Arc<ServiceProvider>,
}

impl<T: DirectIOTransport + 'static> DefaultToolHandler<T> {
    /// Create a new tool handler
    pub fn new(client: Arc<Client<T>>, service_provider: Arc<ServiceProvider>) -> Self {
        Self {
            client,
            service_provider,
        }
    }
}

#[async_trait]
impl<T: DirectIOTransport + 'static> ToolHandler for DefaultToolHandler<T> {
    async fn list_tools(&self) -> Result<ListToolsResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other("Not implemented in this handler yet".to_string()))
    }

    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other("Not implemented in this handler yet".to_string()))
    }

    async fn find_tool_by_name(
        &self,
        name: &str
    ) -> Result<crate::client::services::tools::ToolInfo, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other("Not implemented in this handler yet".to_string()))
    }

    async fn call_tool_and_wait(
        &self,
        name: &str,
        args: Value,
        timeout: Option<Duration>
    ) -> Result<CallToolResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other("Not implemented in this handler yet".to_string()))
    }

    async fn call_tool_with_string_args(
        &self,
        name: &str,
        args: HashMap<String, String>
    ) -> Result<CallToolResult, Error> {
        // Create a JSON value object from the string args
        let mut args_value = serde_json::Map::new();
        for (key, value) in args {
            args_value.insert(key, Value::String(value));
        }

        // Call the tool with the args
        self.call_tool_and_wait(name, Value::Object(args_value), None).await
    }

    async fn call_tool_by_name(&self, name: &str, args: Value) -> Result<CallToolResult, Error> {
        // Create the parameters
        let params = CallToolParams {
            name: name.to_string(),
            arguments: if args.is_object() {
                Some(args.as_object().unwrap().clone().into_iter().collect())
            } else {
                None
            },
        };

        // Call the tool
        self.call_tool(params).await
    }
}
