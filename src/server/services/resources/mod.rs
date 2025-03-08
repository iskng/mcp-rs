// Re-export resource registry module and its contents
pub mod resource_registry;

// Re-export public types from resource_registry
pub use resource_registry::{ResourceProvider, ResourceRegistry, TemplateResourceProvider};

use crate::protocol::Annotations;
use crate::protocol::Error;
use crate::protocol::Resource;

use async_trait::async_trait;

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// Resource lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLifecycleState {
    /// Resource is created but not fully initialized
    Created,

    /// Resource is initialized and ready for use
    Ready,

    /// Resource has been modified
    Modified,

    /// Resource is being closed
    Closing,

    /// Resource is closed
    Closed,
}

/// Trait for resource implementations
#[async_trait]
pub trait ResourceImpl: Send + Sync + 'static {
    /// Get the resource content
    async fn read(&self) -> Result<Vec<u8>, Error>;

    /// Write content to the resource
    async fn write(&mut self, content: &[u8]) -> Result<(), Error>;

    /// Close the resource and release any associated resources
    async fn close(&mut self) -> Result<(), Error>;
}

/// Resource change notification
#[derive(Clone)]
pub struct ResourceChangeNotification {
    /// URI of the resource that changed
    pub uri: String,

    /// Timestamp of the change
    pub timestamp: std::time::SystemTime,
}

/// Resource instance
pub struct ResourceInstance {
    /// Resource URI
    pub uri: String,

    /// Resource name
    pub name: String,

    /// Resource description
    pub description: Option<String>,

    /// Resource mime type
    pub mime_type: Option<String>,

    /// Resource implementation
    pub implementation: Arc<Mutex<Option<Box<dyn ResourceImpl>>>>,

    /// Resource lifecycle state
    pub lifecycle_state: Arc<Mutex<ResourceLifecycleState>>,

    /// Subscribers to resource changes
    pub subscribers: Arc<Mutex<Vec<mpsc::Sender<ResourceChangeNotification>>>>,
}

// Internal module for memory resource implementation
mod memory_resource {
    use super::*;

    pub struct MemoryResourceImpl {
        content: Vec<u8>,
    }

    impl MemoryResourceImpl {
        pub fn new(content: Vec<u8>) -> Self {
            Self { content }
        }

        pub fn from_string(content: String) -> Self {
            Self {
                content: content.into_bytes(),
            }
        }
    }

    #[async_trait]
    impl ResourceImpl for MemoryResourceImpl {
        async fn read(&self) -> Result<Vec<u8>, Error> {
            Ok(self.content.clone())
        }

        async fn write(&mut self, content: &[u8]) -> Result<(), Error> {
            self.content = content.to_vec();
            Ok(())
        }

        async fn close(&mut self) -> Result<(), Error> {
            // Nothing to do for memory resources
            Ok(())
        }
    }
}

// Internal module for file resource implementation
mod file_resource {
    use super::*;
    use tokio::fs;

    pub struct FileResourceImpl {
        path: std::path::PathBuf,
    }

    impl FileResourceImpl {
        pub fn new(path: std::path::PathBuf) -> Self {
            Self { path }
        }
    }

    #[async_trait]
    impl ResourceImpl for FileResourceImpl {
        async fn read(&self) -> Result<Vec<u8>, Error> {
            fs::read(&self.path)
                .await
                .map_err(|e| Error::Resource(format!("Failed to read file: {}", e)))
        }

        async fn write(&mut self, content: &[u8]) -> Result<(), Error> {
            fs::write(&self.path, content)
                .await
                .map_err(|e| Error::Resource(format!("Failed to write file: {}", e)))
        }

        async fn close(&mut self) -> Result<(), Error> {
            // Nothing special needed for file resources
            Ok(())
        }
    }
}

// Re-export implementations
pub use self::file_resource::FileResourceImpl;
pub use self::memory_resource::MemoryResourceImpl;

// Implement ResourceInstance methods
impl ResourceInstance {
    pub fn new(uri: String, name: String) -> Self {
        Self {
            uri,
            name,
            description: None,
            mime_type: None,
            implementation: Arc::new(Mutex::new(None)),
            lifecycle_state: Arc::new(Mutex::new(ResourceLifecycleState::Created)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn set_implementation(
        &self,
        implementation: Box<dyn ResourceImpl>,
    ) -> Result<(), Error> {
        let mut impl_guard = self.implementation.lock().await;
        *impl_guard = Some(implementation);

        let mut state_guard = self.lifecycle_state.lock().await;
        *state_guard = ResourceLifecycleState::Ready;

        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<u8>, Error> {
        let impl_guard = self.implementation.lock().await;
        if let Some(impl_ref) = &*impl_guard {
            impl_ref.read().await
        } else {
            Err(Error::Resource(
                "Resource implementation not set".to_string(),
            ))
        }
    }

    pub async fn write(&self, content: &[u8]) -> Result<(), Error> {
        let mut impl_guard = self.implementation.lock().await;
        if let Some(impl_ref) = &mut *impl_guard {
            let result = impl_ref.write(content).await;
            if result.is_ok() {
                let mut state_guard = self.lifecycle_state.lock().await;
                *state_guard = ResourceLifecycleState::Modified;
                self.notify_change().await;
            }
            result
        } else {
            Err(Error::Resource(
                "Resource implementation not set".to_string(),
            ))
        }
    }

    pub async fn close(&self) -> Result<(), Error> {
        {
            let mut state_guard = self.lifecycle_state.lock().await;
            *state_guard = ResourceLifecycleState::Closing;
        }

        let mut impl_guard = self.implementation.lock().await;
        if let Some(impl_ref) = &mut *impl_guard {
            let result = impl_ref.close().await;
            if result.is_ok() {
                let mut state_guard = self.lifecycle_state.lock().await;
                *state_guard = ResourceLifecycleState::Closed;
            }
            result
        } else {
            // If there's no implementation, just mark it as closed
            let mut state_guard = self.lifecycle_state.lock().await;
            *state_guard = ResourceLifecycleState::Closed;
            Ok(())
        }
    }

    pub async fn subscribe(&self) -> mpsc::Receiver<ResourceChangeNotification> {
        let (tx, rx) = mpsc::channel(10);

        let mut subscribers = self.subscribers.lock().await;
        subscribers.push(tx);

        rx
    }

    async fn notify_change(&self) {
        let notification = ResourceChangeNotification {
            uri: self.uri.clone(),
            timestamp: std::time::SystemTime::now(),
        };

        let subscribers = self.subscribers.lock().await;
        for subscriber in subscribers.iter() {
            let _ = subscriber.send(notification.clone()).await;
        }
    }

    pub fn to_resource(&self) -> Resource {
        Resource {
            uri: self.uri.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            mime_type: self.mime_type.clone(),
            size: None,
            annotations: None,
        }
    }
}
