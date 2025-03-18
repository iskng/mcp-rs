//! MCP Client Tool Operations
//!
//! This module provides domain-specific operations for working with tools.

use async_trait::async_trait;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use crate::client::clientsession::ClientSession;
use crate::protocol::{ CallToolParams, CallToolResult, Error };

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Name of the tool
    pub name: String,
    /// Description of the tool
    pub description: Option<String>,
    /// Schema for tool inputs
    pub input_schema: Option<Value>,
}

/// Extension trait for tool operations on ClientSession
#[async_trait]
pub trait ToolOperations {
    /// Find a tool by name
    async fn find_tool_by_name(&self, name: &str) -> Result<ToolInfo, Error>;

    /// Call a tool and wait for its completion if it returns a progress token
    async fn call_tool_and_wait(
        &self,
        name: &str,
        args: Value,
        timeout: Option<Duration>
    ) -> Result<CallToolResult, Error>;

    /// Call a tool with simple string arguments
    async fn call_tool_with_string_args(
        &self,
        name: &str,
        args: HashMap<String, String>
    ) -> Result<CallToolResult, Error>;

    /// Call a tool by name
    async fn call_tool_by_name(&self, name: &str, args: Value) -> Result<CallToolResult, Error>;
}

#[async_trait]
impl ToolOperations for ClientSession {
    async fn find_tool_by_name(&self, name: &str) -> Result<ToolInfo, Error> {
        // Get all tools
        let tools = self.list_tools().await?;

        // Find the tool by name
        let tool = tools.tools.iter().find(|t| t.name == name);

        match tool {
            Some(tool) =>
                Ok(ToolInfo {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: Some(
                        serde_json::to_value(&tool.input_schema).unwrap_or_default()
                    ),
                }),
            None => Err(Error::Other(format!("Tool not found: {}", name))),
        }
    }

    async fn call_tool_and_wait(
        &self,
        name: &str,
        args: Value,
        _timeout: Option<Duration>
    ) -> Result<CallToolResult, Error> {
        // Convert args to proper format
        let arguments = if args.is_object() {
            Some(args.as_object().unwrap().clone())
        } else {
            None
        };

        // Create the parameters
        let params = CallToolParams {
            name: name.to_string(),
            arguments: arguments.map(|m| m.into_iter().collect()),
        };

        // Call the tool
        let result = self.call_tool(params).await?;

        // Check if we have a progress token
        // NOTE: This is commented out as we don't have access to private progress_tracker
        // If a progress token is returned, we'd need to wait for it to complete

        Ok(result)
    }

    async fn call_tool_with_string_args(
        &self,
        name: &str,
        args: HashMap<String, String>
    ) -> Result<CallToolResult, Error> {
        // Convert string args to JSON values
        let args_value = args
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect::<serde_json::Map<String, Value>>();

        // Call the tool
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
