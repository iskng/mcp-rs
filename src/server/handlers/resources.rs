//! Resource handler for the server
//!
//! This module contains the resource handler trait and implementation for handling
//! resource-related operations like listing, reading, and subscribing to resources.

use async_trait::async_trait;
use std::sync::Arc;

use crate::protocol::Error;
use crate::protocol::{
    ListResourceTemplatesRequest, ListResourceTemplatesResult, ListResourcesRequest,
    ListResourcesResult, PaginatedRequestParams, ReadResourceRequest, ReadResourceResult,
    RootsListChangedNotification, SubscribeRequest, UnsubscribeRequest,
};
use crate::server::services::ServiceProvider;
use crate::server::transport::middleware::ClientSession;

/// Resource handler trait for resource-related operations
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    /// Handle list resources request
    async fn handle_list_resources(
        &self,
        request: &ListResourcesRequest,
        session: &ClientSession,
    ) -> Result<ListResourcesResult, Error>;

    /// Handle read resource request
    async fn handle_read_resource(
        &self,
        request: &ReadResourceRequest,
        session: &ClientSession,
    ) -> Result<ReadResourceResult, Error>;

    /// Handle list resource templates request
    async fn handle_list_templates(
        &self,
        request: &ListResourceTemplatesRequest,
        session: &ClientSession,
    ) -> Result<ListResourceTemplatesResult, Error>;

    /// Handle subscribe request
    async fn handle_subscribe(
        &self,
        request: &SubscribeRequest,
        session: &ClientSession,
    ) -> Result<(), Error>;

    /// Handle unsubscribe request
    async fn handle_unsubscribe(
        &self,
        request: &UnsubscribeRequest,
        session: &ClientSession,
    ) -> Result<(), Error>;

    /// Handle roots list changed notification
    async fn handle_roots_list_changed(
        &self,
        notification: &RootsListChangedNotification,
        session: &ClientSession,
    ) -> Result<(), Error>;
}

/// Default implementation of the resource handler
pub struct DefaultResourceHandler {
    /// Service provider
    service_provider: Arc<ServiceProvider>,
}

impl DefaultResourceHandler {
    /// Create a new resource handler
    pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
        Self { service_provider }
    }
}

#[async_trait]
impl ResourceHandler for DefaultResourceHandler {
    async fn handle_list_resources(
        &self,
        request: &ListResourcesRequest,
        session: &ClientSession,
    ) -> Result<ListResourcesResult, Error> {
        // Log the request
        if let Some(id) = &session.client_id {
            tracing::debug!("List resources request from client {}", id);
        } else {
            tracing::debug!("List resources request from unknown client");
        }

        // Extract parameters from the optional params
        let params = request
            .params
            .as_ref()
            .unwrap_or(&(PaginatedRequestParams { cursor: None }));

        // Get the resource registry from the service provider
        let resource_registry = self.service_provider.resource_registry();

        // Call the resource registry
        resource_registry.list_resources(params).await
    }

    async fn handle_read_resource(
        &self,
        request: &ReadResourceRequest,
        session: &ClientSession,
    ) -> Result<ReadResourceResult, Error> {
        // Log the request
        if let Some(id) = &session.client_id {
            tracing::debug!("Read resource request from client {}", id);
        } else {
            tracing::debug!("Read resource request from unknown client");
        }

        // Get the resource registry from the service provider
        let resource_registry = self.service_provider.resource_registry();

        // Call the resource registry
        let content = resource_registry.read_resource(&request.params.uri).await?;
        let result = ReadResourceResult {
            contents: vec![content],
            _meta: None,
        };
        Ok(result)
    }

    async fn handle_list_templates(
        &self,
        request: &ListResourceTemplatesRequest,
        session: &ClientSession,
    ) -> Result<ListResourceTemplatesResult, Error> {
        // Log the request
        if let Some(id) = &session.client_id {
            tracing::debug!("List templates request from client {}", id);
        } else {
            tracing::debug!("List templates request from unknown client");
        }

        let params = request
            .params
            .as_ref()
            .unwrap_or(&(PaginatedRequestParams { cursor: None }));
        // Get the resource registry from the service provider
        let resource_registry = self.service_provider.resource_registry();

        // Call the resource registry
        resource_registry.list_templates(params).await
    }

    async fn handle_subscribe(
        &self,
        request: &SubscribeRequest,
        session: &ClientSession,
    ) -> Result<(), Error> {
        // Require a client ID for subscriptions
        let client_id = session
            .client_id
            .as_ref()
            .ok_or_else(|| Error::Resource("Missing client ID".into()))?;

        // Log the request
        tracing::debug!("Subscribe request from client {}", client_id);

        // Get the resource registry from the service provider
        let resource_registry = self.service_provider.resource_registry();

        // Call the resource registry
        resource_registry
            .subscribe(client_id, &request.params.uri)
            .await
    }

    async fn handle_unsubscribe(
        &self,
        request: &UnsubscribeRequest,
        session: &ClientSession,
    ) -> Result<(), Error> {
        // Require a client ID for unsubscribing
        let client_id = session
            .client_id
            .as_ref()
            .ok_or_else(|| Error::Resource("Missing client ID".into()))?;

        // Log the request
        tracing::debug!("Unsubscribe request from client {}", client_id);

        // Get the resource registry from the service provider
        let resource_registry = self.service_provider.resource_registry();

        // Call the resource registry
        resource_registry
            .unsubscribe(client_id, &request.params.uri)
            .await
    }

    async fn handle_roots_list_changed(
        &self,
        notification: &RootsListChangedNotification,
        session: &ClientSession,
    ) -> Result<(), Error> {
        // Log the notification
        if let Some(id) = &session.client_id {
            tracing::debug!("Roots list changed notification from client {}", id);
        } else {
            tracing::debug!("Roots list changed notification from unknown client");
        }

        // Currently we don't do anything with these notifications
        // But we could update a cache or inform other clients

        Ok(())
    }
}
