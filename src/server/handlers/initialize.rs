//! Initialize handler for the server
//!
//! This module contains the initialize handler trait and implementation for handling
//! initialization requests from clients.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::protocol::PROTOCOL_VERSION;
use crate::protocol::{
    Implementation, InitializeRequest, InitializeResult, ResourcesCapability, ServerCapabilities,
    ToolsCapability, errors::Error,
};
use crate::server::services::ServiceProvider;
use crate::server::transport::middleware::ClientSession;

/// Initialize handler trait for initialization operations
#[async_trait]
pub trait InitializeHandler: Send + Sync {
    /// Handle initialize request
    async fn handle_initialize(
        &self,
        request: &InitializeRequest,
        session: &ClientSession,
    ) -> Result<InitializeResult, Error>;
}

/// Default implementation of the initialize handler
pub struct DefaultInitializeHandler {
    /// Service provider
    service_provider: Arc<ServiceProvider>,
}

impl DefaultInitializeHandler {
    /// Create a new initialize handlermplementation: Imple
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        Self { service_provider }
    }
}

#[async_trait]
impl InitializeHandler for DefaultInitializeHandler {
    async fn handle_initialize(
        &self,
        _request: &InitializeRequest,
        session: &ClientSession,
    ) -> Result<InitializeResult, Error> {
        // Log the initialization
        if let Some(id) = &session.client_id {
            tracing::info!("Initialize request from client {}", id);
        } else {
            tracing::info!("Initialize request from unknown client");
        }

        // Get capabilities from registries
        let resource_registry = self.service_provider.resource_registry();
        let resource_capabilities = Some(resource_registry.capabilities().clone());

        // Get tool capabilities
        let tool_registry = self.service_provider.tool_registry();
        let tool_capabilities = Some(tool_registry.capabilities().clone());

        // Create server info
        let server_info = Implementation {
            name: "MCP Server".to_string(),
            version: "0.1.0".to_string(),
        };

        // Create response
        Ok(InitializeResult {
            server_info,
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ServerCapabilities {
                resources: resource_capabilities,
                prompts: None, // Not implemented
                tools: tool_capabilities,
                logging: None,      // Not implemented
                experimental: None, // No experimental features
            },
            instructions: None,
            _meta: None,
        })
    }
}

/// Builder for creating customized initialize handlers
pub struct InitializeHandlerBuilder {
    service_provider: Arc<ServiceProvider>,
    server_name_override: Option<String>,
    server_version_override: Option<String>,
    protocol_version_override: Option<String>,
    resource_capabilities_override: Option<ResourcesCapability>,
    tool_capabilities_override: Option<ToolsCapability>,
    experimental_capabilities: Option<HashMap<String, serde_json::Value>>,
    instructions: Option<String>,
}

impl InitializeHandlerBuilder {
    /// Create a new builder with default values from service provider
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        Self {
            service_provider,
            server_name_override: None,
            server_version_override: None,
            protocol_version_override: None,
            resource_capabilities_override: None,
            tool_capabilities_override: None,
            experimental_capabilities: None,
            instructions: None,
        }
    }

    /// Override the server name
    pub fn with_server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name_override = Some(name.into());
        self
    }

    /// Override the server version
    pub fn with_server_version(mut self, version: impl Into<String>) -> Self {
        self.server_version_override = Some(version.into());
        self
    }

    /// Override the protocol version
    pub fn with_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.protocol_version_override = Some(version.into());
        self
    }

    /// Override resource capabilities
    pub fn with_resource_capabilities(mut self, list_changed: bool, subscribe: bool) -> Self {
        self.resource_capabilities_override = Some(ResourcesCapability {
            list_changed: Some(list_changed),
            subscribe: Some(subscribe),
        });
        self
    }

    /// Override tool capabilities
    pub fn with_tool_capabilities(mut self, list_changed: bool) -> Self {
        self.tool_capabilities_override = Some(ToolsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Add experimental capabilities
    pub fn with_experimental_capabilities(
        mut self,
        capabilities: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.experimental_capabilities = Some(capabilities);
        self
    }

    /// Add instructions for the client
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Build the configured initialize handler
    pub fn build(self) -> Box<dyn InitializeHandler> {
        Box::new(ConfigurableInitializeHandler {
            service_provider: self.service_provider,
            server_name_override: self.server_name_override,
            server_version_override: self.server_version_override,
            protocol_version_override: self.protocol_version_override,
            resource_capabilities_override: self.resource_capabilities_override,
            tool_capabilities_override: self.tool_capabilities_override,
            experimental_capabilities: self.experimental_capabilities,
            instructions: self.instructions,
        })
    }
}

/// A configurable initialize handler with options from the builder
struct ConfigurableInitializeHandler {
    service_provider: Arc<ServiceProvider>,
    server_name_override: Option<String>,
    server_version_override: Option<String>,
    protocol_version_override: Option<String>,
    resource_capabilities_override: Option<ResourcesCapability>,
    tool_capabilities_override: Option<ToolsCapability>,
    experimental_capabilities: Option<HashMap<String, serde_json::Value>>,
    instructions: Option<String>,
}

#[async_trait]
impl InitializeHandler for ConfigurableInitializeHandler {
    async fn handle_initialize(
        &self,
        request: &InitializeRequest,
        session: &ClientSession,
    ) -> Result<InitializeResult, Error> {
        // Log the initialization
        if let Some(id) = &session.client_id {
            tracing::info!("Initialize request from client {}", id);
        } else {
            tracing::info!("Initialize request from unknown client");
        }
        tracing::info!("Initialize request in configurable handler: {:?}", request);
        // Get capabilities from registries (with overrides)
        let resource_capabilities = self.resource_capabilities_override.clone().or_else(|| {
            let resource_registry = self.service_provider.resource_registry();
            Some(resource_registry.capabilities().clone())
        });

        // Get tool capabilities (with overrides)
        let tool_capabilities = self.tool_capabilities_override.clone().or_else(|| {
            let tool_registry = self.service_provider.tool_registry();
            Some(tool_registry.capabilities().clone())
        });

        // Create server info (with overrides)
        let server_info = Implementation {
            name: self
                .server_name_override
                .clone()
                .unwrap_or_else(|| "MCP Server".to_string()),
            version: self
                .server_version_override
                .clone()
                .unwrap_or_else(|| "0.1.0".to_string()),
        };

        // Create response
        Ok(InitializeResult {
            server_info,
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ServerCapabilities {
                resources: resource_capabilities,
                prompts: None, // Not implemented
                tools: tool_capabilities,
                logging: None, // Not implemented
                experimental: self.experimental_capabilities.clone(),
            },
            instructions: self.instructions.clone(),
            _meta: None,
        })
    }
}
