//! Prompt Handler
//!
//! This module provides handlers for prompt-related operations in the MCP protocol.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::client::Client;
use crate::client::services::ServiceProvider;
use crate::client::transport::DirectIOTransport;
use crate::protocol::{Error, GetPromptResult, ListPromptsResult};

/// Handler trait for prompt operations
#[async_trait]
pub trait PromptHandler: Send + Sync {
    /// List prompts from the server
    async fn list_prompts(&self) -> Result<ListPromptsResult, Error>;

    /// Get a prompt by name with optional arguments
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, Value>>,
    ) -> Result<GetPromptResult, Error>;
}

/// Default implementation of the prompt handler
pub struct DefaultPromptHandler<T: DirectIOTransport + 'static> {
    /// The underlying client
    client: Arc<Client<T>>,

    /// Service provider for accessing services
    service_provider: Arc<ServiceProvider>,
}

impl<T: DirectIOTransport + 'static> DefaultPromptHandler<T> {
    /// Create a new prompt handler
    pub fn new(client: Arc<Client<T>>, service_provider: Arc<ServiceProvider>) -> Self {
        Self {
            client,
            service_provider,
        }
    }
}

#[async_trait]
impl<T: DirectIOTransport + 'static> PromptHandler for DefaultPromptHandler<T> {
    async fn list_prompts(&self) -> Result<ListPromptsResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, Value>>,
    ) -> Result<GetPromptResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }
}
