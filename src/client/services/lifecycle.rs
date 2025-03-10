//! MCP Client Lifecycle Management
//!
//! This module implements the lifecycle state machine for the MCP client protocol.
//! It tracks the current state, validates operations against that state, and
//! manages state transitions.

use std::collections::HashSet;
use tokio::sync::RwLock;
use tracing::debug;

use crate::protocol::{Error, Method};
use crate::protocol::{InitializeResult, JSONRPCMessage, JSONRPCNotification};

/// Represents the different states in the MCP protocol lifecycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    /// Initial state before any communication
    Created,

    /// Client has sent initialize request, waiting for server response
    Initializing,

    /// Server has responded to initialize, but client hasn't sent initialized notification
    ServerInitialized,

    /// Client has sent initialized notification, ready for normal operations
    Ready,

    /// Client is in the process of shutting down
    ShuttingDown,

    /// Connection is closed
    Closed,

    /// An error has occurred
    Error(String),
}

/// Manager for the client lifecycle state machine
pub struct LifecycleManager {
    /// Current state of the client
    state: RwLock<LifecycleState>,

    /// Server information from initialize response
    server_info: RwLock<Option<InitializeResult>>,

    /// Server capabilities
    capabilities: RwLock<HashSet<String>>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            state: RwLock::new(LifecycleState::Created),
            server_info: RwLock::new(None),
            capabilities: RwLock::new(HashSet::new()),
        }
    }

    /// Get the current lifecycle state
    pub async fn current_state(&self) -> LifecycleState {
        self.state.read().await.clone()
    }

    /// Check if the client is in the ready state
    pub async fn is_ready(&self) -> bool {
        *self.state.read().await == LifecycleState::Ready
    }

    /// Transition to a new state
    pub async fn transition_to(&self, new_state: LifecycleState) -> Result<(), Error> {
        let mut state = self.state.write().await;

        // Validate the state transition
        match (&*state, &new_state) {
            // Valid transitions from Created
            (LifecycleState::Created, LifecycleState::Initializing) => {
                debug!("Transitioning from Created to Initializing");
                *state = new_state;
                Ok(())
            }

            // Valid transitions from Initializing
            (LifecycleState::Initializing, LifecycleState::ServerInitialized) => {
                debug!("Transitioning from Initializing to ServerInitialized");
                *state = new_state;
                Ok(())
            }
            (LifecycleState::Initializing, LifecycleState::Error(_)) => {
                debug!("Transitioning from Initializing to Error");
                *state = new_state;
                Ok(())
            }

            // Valid transitions from ServerInitialized
            (LifecycleState::ServerInitialized, LifecycleState::Ready) => {
                debug!("Transitioning from ServerInitialized to Ready");
                *state = new_state;
                Ok(())
            }
            (LifecycleState::ServerInitialized, LifecycleState::Error(_)) => {
                debug!("Transitioning from ServerInitialized to Error");
                *state = new_state;
                Ok(())
            }

            // Valid transitions from Ready
            (LifecycleState::Ready, LifecycleState::ShuttingDown) => {
                debug!("Transitioning from Ready to ShuttingDown");
                *state = new_state;
                Ok(())
            }
            (LifecycleState::Ready, LifecycleState::Error(_)) => {
                debug!("Transitioning from Ready to Error");
                *state = new_state;
                Ok(())
            }

            // Valid transitions from ShuttingDown
            (LifecycleState::ShuttingDown, LifecycleState::Closed) => {
                debug!("Transitioning from ShuttingDown to Closed");
                *state = new_state;
                Ok(())
            }
            (LifecycleState::ShuttingDown, LifecycleState::Error(_)) => {
                debug!("Transitioning from ShuttingDown to Error");
                *state = new_state;
                Ok(())
            }

            // Error state can always transition to Closed
            (LifecycleState::Error(_), LifecycleState::Closed) => {
                debug!("Transitioning from Error to Closed");
                *state = new_state;
                Ok(())
            }

            // Any state can transition to Error
            (_, LifecycleState::Error(_)) => {
                debug!("Transitioning to Error state");
                *state = new_state;
                Ok(())
            }

            // Invalid transitions
            _ => Err(Error::Lifecycle(format!(
                "Invalid state transition from {:?} to {:?}",
                *state, new_state
            ))),
        }
    }

    /// Set the server info received from initialize response
    pub async fn set_server_info(&self, info: InitializeResult) -> Result<(), Error> {
        // Store the server info
        {
            let mut server_info = self.server_info.write().await;
            *server_info = Some(info.clone());
        }

        // Extract capabilities
        {
            let mut capabilities = self.capabilities.write().await;
            capabilities.clear();

            // Add capabilities based on fields present in the server capabilities
            let caps = &info.capabilities;

            // Resources capability
            if let Some(resources) = &caps.resources {
                if resources.subscribe.unwrap_or(false) {
                    capabilities.insert("resources/subscribe".to_string());
                }
                if resources.list_changed.unwrap_or(false) {
                    capabilities.insert("notifications/resources/list_changed".to_string());
                }
            }

            // Prompts capability
            if let Some(prompts) = &caps.prompts {
                if prompts.list_changed.unwrap_or(false) {
                    capabilities.insert("notifications/prompts/list_changed".to_string());
                }
            }

            // Tools capability
            if let Some(tools) = &caps.tools {
                if tools.list_changed.unwrap_or(false) {
                    capabilities.insert("notifications/tools/list_changed".to_string());
                }
            }

            // Logging capability
            if caps.logging.is_some() {
                capabilities.insert("logging".to_string());
            }

            debug!("Server capabilities: {:?}", capabilities);
        }

        Ok(())
    }

    /// Get server information
    pub async fn server_info(&self) -> Option<InitializeResult> {
        self.server_info.read().await.clone()
    }

    /// Check if the server has a specific capability
    pub async fn has_capability(&self, capability: &str) -> bool {
        let capabilities = self.capabilities.read().await;
        capabilities.contains(capability)
    }

    /// Validate if an operation is allowed in the current state
    pub async fn validate_operation(&self, operation: &str) -> Result<(), Error> {
        let state = self.state.read().await;

        match *state {
            LifecycleState::Ready => Ok(()),
            LifecycleState::Error(ref msg) => Err(Error::Lifecycle(format!(
                "Cannot perform {} - client is in error state: {}",
                operation, msg
            ))),
            LifecycleState::Closed => Err(Error::Lifecycle(format!(
                "Cannot perform {} - client is closed",
                operation
            ))),
            _ => Err(Error::Lifecycle(format!(
                "Cannot perform {} - client is not ready (state: {:?})",
                operation, *state
            ))),
        }
    }

    /// Validate if a request is allowed in the current state
    pub async fn validate_request(&self, method: &Method) -> Result<(), Error> {
        // Special case for initialize which is allowed in Created state
        if method == &Method::Initialize {
            let state = self.state.read().await;
            if *state == LifecycleState::Created {
                return Ok(());
            }
        }

        // For other methods, client must be ready
        self.validate_operation(&format!("request {}", method))
            .await
    }

    /// Validate if a notification is allowed in the current state
    pub async fn validate_notification(&self, method: &Method) -> Result<(), Error> {
        // Special case for initialized notification which is allowed in ServerInitialized state
        if method == &Method::NotificationsInitialized {
            let state = self.state.read().await;
            if *state == LifecycleState::ServerInitialized {
                return Ok(());
            }
        }

        // For other notifications, client must be ready
        self.validate_operation(&format!("notification {}", method))
            .await
    }

    /// Create an initialized notification message
    pub fn create_initialized_notification() -> JSONRPCMessage {
        JSONRPCMessage::Notification(JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: Method::NotificationsInitialized,
            params: None,
        })
    }

    /// Set the client to error state with a message
    pub async fn set_error_state(&self, message: String) -> Result<(), Error> {
        self.transition_to(LifecycleState::Error(message)).await
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}
