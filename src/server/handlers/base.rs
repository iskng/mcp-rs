// //! Base handler trait
// //!
// //! This module defines the base handler trait that provides
// //! access to the service provider and common utilities.

// use async_trait::async_trait;
// use std::sync::Arc;

// use crate::server::services::ServiceProvider;

// /// Base handler trait for all domain handlers
// #[async_trait]
// pub trait BaseHandler: Send + Sync {
//     /// Get the service provider
//     fn service_provider(&self) -> &Arc<ServiceProvider>;

//     /// Get client ID or generate a new one if missing
//     async fn ensure_client_id(&self, client_id: Option<String>) -> String {
//         match client_id {
//             Some(id) => id,
//             None => uuid::Uuid::new_v4().to_string(),
//         }
//     }
// }

// /// Common handler state
// #[derive(Clone)]
// pub struct HandlerState {
//     /// Service provider
//     service_provider: Arc<ServiceProvider>,
// }

// impl HandlerState {
//     /// Create a new handler state
//     pub fn new(service_provider: Arc<ServiceProvider>) -> Self {
//         Self { service_provider }
//     }

//     /// Get the service provider
//     pub fn service_provider(&self) -> &Arc<ServiceProvider> {
//         &self.service_provider
//     }
// }
