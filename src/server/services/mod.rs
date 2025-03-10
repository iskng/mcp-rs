//! Service provider for server components
//!
//! The service provider is a central registry of services that can be accessed by handlers.
//! It provides access to registries, managers, and other application services.

pub mod resources;
pub mod tools;
use crate::server::services::resources::ResourceRegistry;
use crate::server::services::tools::ToolRegistry;
use std::sync::Arc;

/// Service provider for server handlers
#[derive(Clone)]
pub struct ServiceProvider {
    /// Resource registry
    resource_registry: Arc<ResourceRegistry>,

    /// Tool registry
    tool_registry: Arc<ToolRegistry>,
    // Additional services can be added here as needed
}

impl ServiceProvider {
    /// Create a new service provider
    pub fn new(resource_registry: Arc<ResourceRegistry>, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            resource_registry,
            tool_registry,
        }
    }

    /// Get the resource registry
    pub fn resource_registry(&self) -> &ResourceRegistry {
        &self.resource_registry
    }

    /// Get the tool registry
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }
}
