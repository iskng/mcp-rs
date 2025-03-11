//! MCP Client Resource Operations
//!
//! This module provides domain-specific operations for working with resources.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::client::clientsession::ClientSession;
use crate::protocol::{Error, ListResourcesResult, ReadResourceParams, ReadResourceResult};

/// Extension trait for resource operations on ClientSession
#[async_trait]
pub trait ResourceOperations {
    /// List all resources, optionally filtered by type
    async fn list_resources_by_type(
        &self,
        resource_type: Option<String>,
    ) -> Result<ListResourcesResult, Error>;

    /// Get a resource by ID
    async fn get_resource(&self, uri: &str) -> Result<ReadResourceResult, Error>;

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

    /// Wait for a resource to be created or updated
    async fn wait_for_resource_update(&self, uri: &str) -> Result<ReadResourceResult, Error>;
}

#[async_trait]
impl ResourceOperations for ClientSession {
    async fn list_resources_by_type(
        &self,
        resource_type: Option<String>,
    ) -> Result<ListResourcesResult, Error> {
        // Create filter parameters based on resource type
        let params = if let Some(resource_type) = resource_type {
            Some(serde_json::json!({
                "filter": {
                    "resourceType": resource_type
                }
            }))
        } else {
            None
        };

        // Call the list resources API
        self.list_resources(params).await
    }

    async fn get_resource(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        let params = ReadResourceParams {
            uri: uri.to_string(),
        };

        self.read_resource(params).await
    }

    async fn create_resource_from_template(
        &self,
        template_id: &str,
        name: &str,
        properties: Option<HashMap<String, Value>>,
    ) -> Result<ReadResourceResult, Error> {
        // Create the parameters
        let mut params = serde_json::json!({
            "templateId": template_id,
            "name": name
        });

        // Add properties if specified
        if let Some(props) = properties {
            if let Some(obj) = params.as_object_mut() {
                obj.insert(
                    "properties".to_string(),
                    Value::Object(props.into_iter().collect()),
                );
            }
        }

        // Call the create resource API
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

        // Call the update resource API
        self.update_resource(params).await
    }

    async fn wait_for_resource_update(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        // Subscribe to resource updates
        let mut subscription = self.subscribe_resource_updates().await;

        // Loop until we get an update for this resource
        while let Some(notification) = subscription.next().await {
            if notification.params.uri == uri {
                return self.get_resource(uri).await;
            }
        }

        Err(Error::Other(format!(
            "Subscription ended without update for resource {}",
            uri
        )))
    }
}
