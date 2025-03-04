/// Resource lifecycle state
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

/// Base resource structure
#[derive(Clone)]
pub struct Resource {
    /// Resource URI
    pub uri: String,

    /// Resource name
    pub name: String,

    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Resource mime type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Resource implementation
    #[serde(skip)]
    pub implementation: Arc<Mutex<Option<Box<dyn ResourceImpl>>>>,

    /// Resource lifecycle state
    #[serde(skip)]
    pub lifecycle_state: Arc<Mutex<ResourceLifecycleState>>,

    /// Subscribers to resource changes
    #[serde(skip)]
    pub subscribers: Arc<Mutex<Vec<mpsc::Sender<ResourceChangeNotification>>>>,
}

impl Resource {
    /// Create a new resource
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

    /// Set the resource implementation
    pub async fn set_implementation(
        &self,
        implementation: Box<dyn ResourceImpl>
    ) -> Result<(), Error> {
        let mut impl_guard = self.implementation.lock().await;
        *impl_guard = Some(implementation);

        // Update lifecycle state
        let mut state_guard = self.lifecycle_state.lock().await;
        *state_guard = ResourceLifecycleState::Ready;

        Ok(())
    }

    /// Read the resource content
    pub async fn read(&self) -> Result<Vec<u8>, Error> {
        // Check lifecycle state
        {
            let state_guard = self.lifecycle_state.lock().await;
            if *state_guard == ResourceLifecycleState::Closed {
                return Err(Error::Resource("Resource is closed".to_string()));
            }
        }

        // Get implementation
        let impl_guard = self.implementation.lock().await;
        match &*impl_guard {
            Some(implementation) => implementation.read().await,
            None => Err(Error::Resource("Resource has no implementation".to_string())),
        }
    }

    /// Write to the resource
    pub async fn write(&self, content: &[u8]) -> Result<(), Error> {
        // Check lifecycle state
        {
            let state_guard = self.lifecycle_state.lock().await;
            if *state_guard == ResourceLifecycleState::Closed {
                return Err(Error::Resource("Resource is closed".to_string()));
            }
        }

        // Get implementation
        let mut impl_guard = self.implementation.lock().await;
        match &mut *impl_guard {
            Some(implementation) => {
                let result = implementation.write(content).await;

                // Update lifecycle state
                if result.is_ok() {
                    let mut state_guard = self.lifecycle_state.lock().await;
                    *state_guard = ResourceLifecycleState::Modified;

                    // Notify subscribers
                    self.notify_change().await;
                }

                result
            }
            None => Err(Error::Resource("Resource has no implementation".to_string())),
        }
    }

    /// Close the resource
    pub async fn close(&self) -> Result<(), Error> {
        // Check lifecycle state
        {
            let state_guard = self.lifecycle_state.lock().await;
            if *state_guard == ResourceLifecycleState::Closed {
                return Ok(());
            }
        }

        // Update lifecycle state
        {
            let mut state_guard = self.lifecycle_state.lock().await;
            *state_guard = ResourceLifecycleState::Closing;
        }

        // Get implementation
        let mut impl_guard = self.implementation.lock().await;
        let result = match &mut *impl_guard {
            Some(implementation) => implementation.close().await,
            None => Ok(()),
        };

        // Update lifecycle state
        {
            let mut state_guard = self.lifecycle_state.lock().await;
            *state_guard = ResourceLifecycleState::Closed;
        }

        // Clear subscribers
        {
            let mut subscribers_guard = self.subscribers.lock().await;
            subscribers_guard.clear();
        }

        result
    }

    /// Subscribe to resource changes
    pub async fn subscribe(&self) -> mpsc::Receiver<ResourceChangeNotification> {
        let (tx, rx) = mpsc::channel(10);

        // Add subscriber
        {
            let mut subscribers_guard = self.subscribers.lock().await;
            subscribers_guard.push(tx);
        }

        rx
    }

    /// Notify subscribers of a change
    async fn notify_change(&self) {
        let notification = ResourceChangeNotification {
            uri: self.uri.clone(),
            timestamp: std::time::SystemTime::now().into(),
        };

        let mut subscribers_guard = self.subscribers.lock().await;
        subscribers_guard.retain(|tx| { tx.try_send(notification.clone()).is_ok() });
    }
}
