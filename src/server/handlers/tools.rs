//! Tool handler for the server
//!
//! This module contains the tool handler trait and implementation for handling
//! tool-related operations like listing tools and executing tools.

use async_trait::async_trait;
use std::sync::Arc;

use crate::protocol::Error;
use crate::protocol::{ CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult };
use crate::server::services::ServiceProvider;
use crate::server::transport::middleware::ClientSession;

/// Tool handler trait for tool-related operations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Handle list tools request
    async fn handle_list_tools(
        &self,
        request: &ListToolsRequest,
        session: &ClientSession
    ) -> Result<ListToolsResult, Error>;

    /// Handle call tool request
    async fn handle_call_tool(
        &self,
        request: &CallToolRequest,
        session: &ClientSession
    ) -> Result<CallToolResult, Error>;
}

/// Default implementation of the tool handler
pub struct DefaultToolHandler {
    /// Service provider
    service_provider: Arc<ServiceProvider>,
}

impl DefaultToolHandler {
    /// Create a new tool handler
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        Self { service_provider }
    }
}

#[async_trait]
impl ToolHandler for DefaultToolHandler {
    async fn handle_list_tools(
        &self,
        request: &ListToolsRequest,
        session: &ClientSession
    ) -> Result<ListToolsResult, Error> {
        // Log the request
        if let Some(id) = &session.client_id {
            tracing::debug!("List tools request from client {}", id);
        } else {
            tracing::debug!("List tools request from unknown client");
        }

        // Get the tool registry from the service provider
        let tool_registry = self.service_provider.tool_registry();

        // Extract pagination parameters
        let cursor = request.params.as_ref().and_then(|p| p.cursor.as_ref());

        // Use a reasonable default limit
        let limit = Some(50);

        // Fetch the list of tools with pagination
        let (tools, next_cursor) = tool_registry.list_tools(cursor, limit).await;

        // Return the result
        Ok(ListToolsResult {
            tools,
            next_cursor,
            _meta: None,
        })
    }

    async fn handle_call_tool(
        &self,
        request: &CallToolRequest,
        session: &ClientSession
    ) -> Result<CallToolResult, Error> {
        // Extract tool call parameters
        let params = request.params.clone();

        // Log the request
        if let Some(id) = &session.client_id {
            tracing::debug!("Call tool request from client {}: {}", id, params.name);
        } else {
            tracing::debug!("Call tool request from unknown client: {}", params.name);
        }

        // Get the tool registry from the service provider
        let tool_registry = self.service_provider.tool_registry();

        // Execute the tool with the provided parameters
        tool_registry.execute_tool_with_params(params).await
    }
}
