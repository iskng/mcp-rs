//! Resource Handler
//!
//! This module provides handlers for resource-related operations in the MCP protocol.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::client::Client;
use crate::client::services::ServiceProvider;
use crate::client::services::subscription::Subscription;
use crate::client::transport::DirectIOTransport;
use crate::protocol::{
    Error, ListResourceTemplatesResult, ListResourcesResult, ReadResourceParams,
    ReadResourceResult, ResourceListChangedNotification, ResourceUpdatedNotification,
};

/// Handler trait for resource operations
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    /// List all resources from the server
    async fn list_resources(&self, params: Option<Value>) -> Result<ListResourcesResult, Error>;

    /// Read a resource from the server
    async fn read_resource(&self, params: ReadResourceParams) -> Result<ReadResourceResult, Error>;

    /// List resource templates from the server
    async fn list_resource_templates(&self) -> Result<ListResourceTemplatesResult, Error>;

    /// Create a resource on the server
    async fn create_resource(&self, params: Value) -> Result<ReadResourceResult, Error>;

    /// Update a resource on the server
    async fn update_resource(&self, params: Value) -> Result<ReadResourceResult, Error>;

    /// Delete a resource from the server
    async fn delete_resource(&self, uri: String) -> Result<(), Error>;

    /// Subscribe to resource updates
    async fn subscribe_resource_updates(&self) -> Subscription<ResourceUpdatedNotification>;

    /// Subscribe to resource list changes
    async fn subscribe_resource_list_changes(
        &self,
    ) -> Subscription<ResourceListChangedNotification>;

    /// Create a resource from a template
    async fn create_resource_from_template(
        &self,
        template_id: &str,
        name: &str,
        properties: Option<HashMap<String, Value>>,
    ) -> Result<ReadResourceResult, Error>;

    /// Update a resource's properties
    async fn update_resource_properties(
        &self,
        uri: &str,
        properties: HashMap<String, Value>,
    ) -> Result<ReadResourceResult, Error>;

    /// Wait for a resource to be updated
    async fn wait_for_resource_update(&self, uri: &str) -> Result<ReadResourceResult, Error>;
}

/// Default implementation of the resource handler
pub struct DefaultResourceHandler<T: DirectIOTransport + 'static> {
    /// The underlying client
    client: Arc<Client<T>>,

    /// Service provider for accessing services
    service_provider: Arc<ServiceProvider>,
}

impl<T: DirectIOTransport + 'static> DefaultResourceHandler<T> {
    /// Create a new resource handler
    pub fn new(client: Arc<Client<T>>, service_provider: Arc<ServiceProvider>) -> Self {
        Self {
            client,
            service_provider,
        }
    }
}

#[async_trait]
impl<T: DirectIOTransport + 'static> ResourceHandler for DefaultResourceHandler<T> {
    async fn list_resources(&self, params: Option<Value>) -> Result<ListResourcesResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn read_resource(&self, params: ReadResourceParams) -> Result<ReadResourceResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn list_resource_templates(&self) -> Result<ListResourceTemplatesResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn create_resource(&self, params: Value) -> Result<ReadResourceResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn update_resource(&self, params: Value) -> Result<ReadResourceResult, Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn delete_resource(&self, uri: String) -> Result<(), Error> {
        // Implementation will be completed in a subsequent PR
        Err(Error::Other(
            "Not implemented in this handler yet".to_string(),
        ))
    }

    async fn subscribe_resource_updates(&self) -> Subscription<ResourceUpdatedNotification> {
        // Get the subscription manager
        let subscription_manager = self.service_provider.subscription_manager();

        // Create the subscription
        subscription_manager.subscribe_resource_updates().await
    }

    async fn subscribe_resource_list_changes(
        &self,
    ) -> Subscription<ResourceListChangedNotification> {
        // Get the subscription manager
        let subscription_manager = self.service_provider.subscription_manager();

        // Create the subscription
        subscription_manager.subscribe_resource_list_changes().await
    }

    async fn create_resource_from_template(
        &self,
        template_id: &str,
        name: &str,
        properties: Option<HashMap<String, Value>>,
    ) -> Result<ReadResourceResult, Error> {
        // Create the parameters
        let params = serde_json::json!({
            "templateId": template_id,
            "name": name,
            "properties": properties
        });

        // Call the create resource method
        self.create_resource(params).await
    }

    async fn update_resource_properties(
        &self,
        uri: &str,
        properties: HashMap<String, Value>,
    ) -> Result<ReadResourceResult, Error> {
        // Create the parameters
        let params = serde_json::json!({
            "uri": uri,
            "properties": properties
        });

        // Call the update resource method
        self.update_resource(params).await
    }

    async fn wait_for_resource_update(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        // Subscribe to resource updates
        let mut subscription = self.subscribe_resource_updates().await;

        // Loop until we get an update for this resource
        while let Some(notification) = subscription.next().await {
            if notification.params.uri == uri {
                // Read the updated resource
                return self
                    .read_resource(ReadResourceParams {
                        uri: uri.to_string(),
                    })
                    .await;
            }
        }

        // If we get here, the subscription ended without an update
        Err(Error::Other(format!(
            "Subscription ended without update for resource {}",
            uri
        )))
    }
}
