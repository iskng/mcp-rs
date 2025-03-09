//! Resource registry and handler implementation
//!
//! This module provides the Registry that manages resources and templates,
//! and the Handler that processes MCP protocol messages related to resources.
use crate::protocol::{
    Cursor,
    ListResourceTemplatesResult,
    ListResourcesResult,
    Resource,
    ResourceContentType as ResourceContent,
    ResourceTemplate,
    UriTemplate,
};
use crate::protocol::{ Error, PaginatedRequestParams, ResourcesCapability };
use crate::protocol::{ JSONRPCMessage as Message, JSONRPCNotification as Notification };
use crate::transport::Transport;

use async_trait::async_trait;
use serde_json::json;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{ debug, warn };

/// Provider for static resources
#[async_trait]
pub trait ResourceProvider: Send + Sync + 'static {
    /// Get the resource metadata
    fn metadata(&self) -> Resource;

    /// Get the resource content
    async fn content(&self) -> Result<ResourceContent, Error>;
}

/// Provider for template-based resources
#[async_trait]
pub trait TemplateResourceProvider: Send + Sync + 'static {
    /// Get the resource content with the provided parameters
    async fn content(&self, params: HashMap<String, String>) -> Result<ResourceContent, Error>;
}

/// Resource registry for managing resources
pub struct ResourceRegistry {
    /// Static resources by URI
    resources: RwLock<HashMap<String, Arc<dyn ResourceProvider>>>,

    /// Resource templates by URI template
    templates: RwLock<HashMap<String, (ResourceTemplate, Arc<dyn TemplateResourceProvider>)>>,

    /// Resource subscriptions by client ID
    subscriptions: RwLock<HashMap<String, HashSet<String>>>,

    /// Client subscriptions by resource URI
    subscribers: RwLock<HashMap<String, HashSet<String>>>,

    /// Resource capabilities
    capabilities: ResourcesCapability,
}

impl ResourceRegistry {
    /// Create a new resource registry
    pub fn new(supports_subscribe: bool, supports_list_changed: bool) -> Self {
        Self {
            resources: RwLock::new(HashMap::new()),
            templates: RwLock::new(HashMap::new()),
            subscriptions: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            capabilities: ResourcesCapability {
                list_changed: Some(supports_list_changed),
                subscribe: Some(supports_subscribe),
            },
        }
    }

    /// Register a static resource provider
    pub async fn register_resource<P>(&self, provider: P) -> Result<(), Error>
        where P: ResourceProvider + 'static
    {
        let metadata = provider.metadata();
        let uri = metadata.uri.clone();

        // Add to resources map
        let mut resources = self.resources.write().await;
        resources.insert(uri, Arc::new(provider));

        Ok(())
    }

    /// Register a template resource provider
    pub async fn register_template<P>(
        &self,
        template: ResourceTemplate,
        provider: P
    ) -> Result<(), Error>
        where P: TemplateResourceProvider + 'static
    {
        let uri_template = template.uri_template.clone();

        // Add to templates map
        let mut templates = self.templates.write().await;
        templates.insert(uri_template, (template, Arc::new(provider)));

        Ok(())
    }

    /// List all resources with optional filtering
    pub async fn list_resources(
        &self,
        params: &PaginatedRequestParams
    ) -> Result<ListResourcesResult, Error> {
        let resources = self.resources.read().await;

        let mut result = Vec::new();
        for provider in resources.values() {
            let metadata = provider.metadata();
            result.push(metadata);
        }

        // Sort by URI for consistency
        result.sort_by(|a, b| a.uri.cmp(&b.uri));

        // Implement pagination with cursor
        let (paginated_result, next_cursor) = if let Some(cursor) = &params.cursor {
            // Parse cursor - assuming it's a URI to start after
            let cursor_value = cursor.0.clone();
            let start_index = result
                .iter()
                .position(|r| r.uri > cursor_value)
                .unwrap_or(0);

            // Use a reasonable default page size
            const DEFAULT_PAGE_SIZE: usize = 50;
            let end_index = (start_index + DEFAULT_PAGE_SIZE).min(result.len());

            let next_cursor = if end_index < result.len() {
                Some(Cursor(result[end_index - 1].uri.clone()))
            } else {
                None
            };

            (result[start_index..end_index].to_vec(), next_cursor)
        } else {
            // First page with a reasonable default page size
            const DEFAULT_PAGE_SIZE: usize = 50;
            let end_index = DEFAULT_PAGE_SIZE.min(result.len());

            let next_cursor = if end_index < result.len() {
                Some(Cursor(result[end_index - 1].uri.clone()))
            } else {
                None
            };

            (result[..end_index].to_vec(), next_cursor)
        };

        Ok(ListResourcesResult {
            resources: paginated_result,
            next_cursor: next_cursor,
            _meta: None,
        })
    }

    /// List all resource templates with optional filtering
    pub async fn list_templates(
        &self,
        params: &PaginatedRequestParams
    ) -> Result<ListResourceTemplatesResult, Error> {
        let templates = self.templates.read().await;

        let mut result = Vec::new();
        for (_, (template, _)) in templates.iter() {
            result.push(template.clone());
        }

        // Sort by URI template for consistency
        result.sort_by(|a, b| a.uri_template.cmp(&b.uri_template));

        // Implement pagination with cursor
        let (paginated_result, next_cursor) = if let Some(cursor) = &params.cursor {
            // Parse cursor - assuming it's a URI template to start after
            let cursor_value = cursor.0.clone();
            let start_index = result
                .iter()
                .position(|r| r.uri_template > cursor_value)
                .unwrap_or(0);

            // Use a reasonable default page size
            const DEFAULT_PAGE_SIZE: usize = 50;
            let end_index = (start_index + DEFAULT_PAGE_SIZE).min(result.len());

            let next_cursor = if end_index < result.len() {
                Some(Cursor(result[end_index - 1].uri_template.clone()))
            } else {
                None
            };

            (result[start_index..end_index].to_vec(), next_cursor)
        } else {
            // First page with a reasonable default page size
            const DEFAULT_PAGE_SIZE: usize = 50;
            let end_index = DEFAULT_PAGE_SIZE.min(result.len());

            let next_cursor = if end_index < result.len() {
                Some(Cursor(result[end_index - 1].uri_template.clone()))
            } else {
                None
            };

            (result[..end_index].to_vec(), next_cursor)
        };

        Ok(ListResourceTemplatesResult {
            resource_templates: paginated_result,
            next_cursor: next_cursor,
            _meta: None,
        })
    }

    /// Read a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent, Error> {
        // First try static resources
        let resources = self.resources.read().await;
        if let Some(provider) = resources.get(uri) {
            return provider.content().await;
        }
        drop(resources);

        // If not found, try templates
        let templates = self.templates.read().await;
        for (template_uri, (_, provider)) in templates.iter() {
            let template_parser = UriTemplate::new(template_uri);
            if let Some(params) = template_parser.match_uri(uri) {
                return provider.content(params).await;
            }
        }
        debug!("Resource not found: {}", uri);
        Err(Error::Resource(format!("Resource not found: {}", uri)))
    }

    /// Subscribe to a resource
    pub async fn subscribe(&self, client_id: &str, uri: &str) -> Result<(), Error> {
        if !self.supports_subscribe() {
            return Err(Error::Resource(format!("Resource subscription not supported: {}", uri)));
        }

        // Verify resource exists or matches a template
        let resource_exists = {
            let resources = self.resources.read().await;
            resources.contains_key(uri)
        };

        if !resource_exists {
            let template_match = {
                let templates = self.templates.read().await;
                templates.iter().any(|(template_uri, _)| {
                    let template_parser = UriTemplate::new(template_uri);
                    template_parser.match_uri(uri).is_some()
                })
            };

            if !template_match {
                return Err(Error::Resource(format!("Resource not found: {}", uri)));
            }
        }

        // Add to client's subscriptions
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions
                .entry(client_id.to_string())
                .or_insert_with(HashSet::new)
                .insert(uri.to_string());
        }

        // Add client to resource's subscribers
        {
            let mut subscribers = self.subscribers.write().await;
            subscribers
                .entry(uri.to_string())
                .or_insert_with(HashSet::new)
                .insert(client_id.to_string());
        }

        debug!("Client {} subscribed to resource {}", client_id, uri);
        Ok(())
    }

    /// Unsubscribe from a resource
    pub async fn unsubscribe(&self, client_id: &str, uri: &str) -> Result<(), Error> {
        if !self.supports_subscribe() {
            return Err(Error::Resource("Resource subscription not supported".to_string()));
        }

        // Remove from client's subscriptions
        {
            let mut subscriptions = self.subscriptions.write().await;
            if let Some(client_subs) = subscriptions.get_mut(client_id) {
                client_subs.remove(uri);
            }
        }

        // Remove client from resource's subscribers
        {
            let mut subscribers = self.subscribers.write().await;
            if let Some(resource_subs) = subscribers.get_mut(uri) {
                resource_subs.remove(client_id);
            }
        }

        debug!("Client {} unsubscribed from resource {}", client_id, uri);
        Ok(())
    }

    /// Get subscribers for a resource
    pub async fn get_subscribers(&self, uri: &str) -> Vec<String> {
        let subscribers = self.subscribers.read().await;
        match subscribers.get(uri) {
            Some(subs) => subs.iter().cloned().collect(),
            None => Vec::new(),
        }
    }

    /// Notify that a resource has changed
    pub async fn notify_resource_changed(&self, uri: &str) -> Vec<String> {
        if !self.supports_subscribe() {
            return Vec::new();
        }

        debug!("Resource changed: {}", uri);
        self.get_subscribers(uri).await
    }

    /// Create a resource update notification
    pub fn create_update_notification(uri: &str) -> Notification {
        Notification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/resources/updated".to_string(),
            params: Some(json!({
                "uri": uri
            })),
        }
    }

    /// Create a resource list changed notification
    pub fn create_list_changed_notification() -> Notification {
        Notification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/resources/list_changed".to_string(),
            params: None,
        }
    }

    /// Unsubscribe all resources for a client (e.g., when client disconnects)
    pub async fn unsubscribe_all(&self, client_id: &str) {
        if !self.supports_subscribe() {
            return;
        }

        // Get all resources this client is subscribed to
        let subscribed_resources = {
            let subscriptions = self.subscriptions.read().await;
            match subscriptions.get(client_id) {
                Some(resources) => resources.clone(),
                None => {
                    return;
                }
            }
        };

        // Remove client from all resource subscribers
        {
            let mut subscribers = self.subscribers.write().await;
            for uri in &subscribed_resources {
                if let Some(resource_subs) = subscribers.get_mut(uri) {
                    resource_subs.remove(client_id);
                }
            }
        }

        // Remove client from subscriptions
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.remove(client_id);
        }

        debug!("Removed all subscriptions for client {}", client_id);
    }

    /// Get the resource capabilities
    pub fn capabilities(&self) -> &ResourcesCapability {
        &self.capabilities
    }

    /// Check if the registry supports subscriptions
    pub fn supports_subscribe(&self) -> bool {
        self.capabilities.subscribe.unwrap_or(false)
    }

    /// Check if the registry supports list changed notifications
    pub fn supports_list_changed(&self) -> bool {
        self.capabilities.list_changed.unwrap_or(false)
    }

    /// Notify clients that a resource has changed and send notifications
    pub async fn notify_resource_update(
        &self,
        uri: &str,
        transport: &mut Box<dyn Transport + Send + Sync>
    ) -> Result<(), Error> {
        let subscribers = self.get_subscribers(uri).await;

        if !subscribers.is_empty() {
            let notification = Self::create_update_notification(uri);
            let message = Message::Notification(notification);

            for client_id in subscribers {
                if let Err(e) = forward_message_to_client(transport, &client_id, &message).await {
                    warn!("Failed to notify client {} about resource update: {}", client_id, e);
                }
            }
        }

        Ok(())
    }

    /// Notify clients that the resource list has changed
    pub async fn notify_list_changed(
        &self,
        transport: &mut Box<dyn Transport + Send + Sync>
    ) -> Result<(), Error> {
        if !self.supports_list_changed() {
            return Ok(());
        }

        // For now, we'll just log this since Transport doesn't have a broadcast method
        warn!("notify_list_changed called, but no broadcast method available on Transport");

        // In a full implementation, we would iterate through all clients and send a notification
        // But for now, this is left as a placeholder
        // let notification = Self::create_list_changed_notification();
        // let message = Message::Notification(notification);

        Ok(())
    }
}

// Helper function to forward messages through Transport
async fn forward_message_to_client(
    transport: &mut Box<dyn Transport + Send + Sync>,
    client_id: &str,
    message: &Message
) -> Result<(), Error> {
    // The message is already a JSONRPCMessage (alias)
    // Send the message using the transport
    transport.send_to(client_id, message).await
}
