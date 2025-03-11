//! MCP Client Lifecycle Management
//!
//! This module implements the lifecycle state machine for the MCP client protocol.
//! It tracks the current state, validates operations against that state, and
//! manages state transitions.

use std::collections::HashSet;
use tokio::sync::RwLock;
use tracing::debug;

use crate::protocol::{ Error, Method };
use crate::protocol::{ InitializeResult, JSONRPCMessage, JSONRPCNotification };

/// Represents the different states in the MCP protocol lifecycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    /// Client is in the initialization phase
    Initialization,

    /// Client is in normal operation mode
    Operation,

    /// Client is in the process of shutting down or closed
    Shutdown,

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
            state: RwLock::new(LifecycleState::Initialization),
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
        *self.state.read().await == LifecycleState::Operation
    }

    /// Transition to a new state
    pub async fn transition_to(&self, new_state: LifecycleState) -> Result<(), Error> {
        let mut state = self.state.write().await;

        // Validate the state transition
        match (&*state, &new_state) {
            // Valid transitions from Initialization
            (LifecycleState::Initialization, LifecycleState::Operation) => {
                debug!("Transitioning from Initialization to Operation");
                *state = new_state;
                Ok(())
            }
            (LifecycleState::Initialization, LifecycleState::Error(_)) => {
                debug!("Transitioning from Initialization to Error");
                *state = new_state;
                Ok(())
            }

            // Valid transitions from Operation
            (LifecycleState::Operation, LifecycleState::Shutdown) => {
                debug!("Transitioning from Operation to Shutdown");
                *state = new_state;
                Ok(())
            }
            (LifecycleState::Operation, LifecycleState::Error(_)) => {
                debug!("Transitioning from Operation to Error");
                *state = new_state;
                Ok(())
            }

            // Valid transitions from Shutdown
            (LifecycleState::Shutdown, LifecycleState::Error(_)) => {
                debug!("Transitioning from Shutdown to Error");
                *state = new_state;
                Ok(())
            }

            // Error state can always transition to Shutdown
            (LifecycleState::Error(_), LifecycleState::Shutdown) => {
                debug!("Transitioning from Error to Shutdown");
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
            _ =>
                Err(
                    Error::Lifecycle(
                        format!("Invalid state transition from {:?} to {:?}", *state, new_state)
                    )
                ),
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
            LifecycleState::Operation => Ok(()),
            LifecycleState::Error(ref msg) =>
                Err(
                    Error::Lifecycle(
                        format!("Cannot perform {} - client is in error state: {}", operation, msg)
                    )
                ),
            LifecycleState::Shutdown =>
                Err(Error::Lifecycle(format!("Cannot perform {} - client is closed", operation))),
            _ =>
                Err(
                    Error::Lifecycle(
                        format!(
                            "Cannot perform {} - client is not ready (state: {:?})",
                            operation,
                            *state
                        )
                    )
                ),
        }
    }

    /// Validate if a request is allowed in the current state
    pub async fn validate_request(&self, method: &Method) -> Result<(), Error> {
        // Special case for initialize which is allowed in Initialization state
        if method == &Method::Initialize {
            let state = self.state.read().await;
            if *state == LifecycleState::Initialization {
                return Ok(());
            }
        }

        // For other methods, client must be ready
        self.validate_operation(&format!("request {}", method)).await
    }

    /// Validate if a notification is allowed in the current state
    pub async fn validate_notification(&self, method: &Method) -> Result<(), Error> {
        // Special case for initialized notification which is allowed in Initialization state
        if method == &Method::NotificationsInitialized {
            let state = self.state.read().await;
            if *state == LifecycleState::Initialization {
                return Ok(());
            }
        }

        // For all other notifications, validate like regular operations
        self.validate_operation("notification").await
    }

    /// Create the initialized notification
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

    /// Validates if a capability is supported based on server capabilities
    pub async fn validate_capability(
        &self,
        category: &str,
        capability: Option<&str>
    ) -> Result<(), Error> {
        // Get the current state and server info
        let state = self.current_state().await;

        // Can only validate capabilities in Operation state
        if state != LifecycleState::Operation {
            return Err(
                Error::Protocol(
                    format!(
                        "Cannot validate capabilities in {:?} state, client must be Ready",
                        state
                    )
                )
            );
        }

        // Get server info
        let server_info_guard = self.server_info.read().await;
        let server_info = match server_info_guard.as_ref() {
            Some(info) => info,
            None => {
                return Err(
                    Error::Protocol("Server info not available, client not initialized".to_string())
                );
            }
        };

        // Get the capabilities
        let capabilities = &server_info.capabilities;

        // Check if the capability category exists
        let category_supported = match category {
            "prompts" => capabilities.prompts.is_some(),
            "resources" => capabilities.resources.is_some(),
            "tools" => capabilities.tools.is_some(),
            "logging" => capabilities.logging.is_some(),
            "experimental" => capabilities.experimental.is_some(),
            _ => false,
        };

        if !category_supported {
            return Err(Error::Protocol(format!("Server does not support {} capability", category)));
        }

        // If a specific capability is requested, check that too
        if let Some(specific) = capability {
            let specific_supported = match (category, specific) {
                ("resources", "subscribe") =>
                    capabilities.resources
                        .as_ref()
                        .and_then(|r| r.subscribe)
                        .unwrap_or(false),
                ("resources", "listChanged") =>
                    capabilities.resources
                        .as_ref()
                        .and_then(|r| r.list_changed)
                        .unwrap_or(false),
                ("prompts", "listChanged") =>
                    capabilities.prompts
                        .as_ref()
                        .and_then(|p| p.list_changed)
                        .unwrap_or(false),
                ("tools", "listChanged") =>
                    capabilities.tools
                        .as_ref()
                        .and_then(|t| t.list_changed)
                        .unwrap_or(false),
                _ => false,
            };

            if !specific_supported {
                return Err(
                    Error::Protocol(
                        format!("Server does not support {}/{} capability", category, specific)
                    )
                );
            }
        }

        Ok(())
    }

    /// Validates if an operation is allowed based on method and current state
    pub async fn validate_method(&self, method: &Method) -> Result<(), Error> {
        let current_state = self.current_state().await;
        tracing::info!("Validating method {} in state {:?}", method, current_state);

        // Special case for Initialize method
        match method {
            Method::Initialize => {
                match current_state {
                    LifecycleState::Initialization => {
                        tracing::debug!("Initialize method allowed in state {:?}", current_state);
                        return Ok(());
                    }
                    _ => {
                        let msg = format!(
                            "Invalid state for Initialize method: {:?}. Must be Initialization.",
                            current_state
                        );
                        tracing::warn!("{}", msg);
                        return Err(Error::Protocol(msg));
                    }
                }
            }
            Method::NotificationsInitialized => {
                match current_state {
                    LifecycleState::Initialization => {
                        tracing::debug!(
                            "NotificationsInitialized allowed in state {:?}",
                            current_state
                        );
                        return Ok(());
                    }
                    _ => {
                        let msg = format!(
                            "Invalid state for NotificationsInitialized: {:?}. Must be Initialization.",
                            current_state
                        );
                        tracing::warn!("{}", msg);
                        return Err(Error::Protocol(msg));
                    }
                }
            }
            // Ping is always allowed
            Method::Ping => {
                tracing::debug!("Ping allowed in any state");
                return Ok(());
            }
            // All other methods require Ready state
            _ => {
                if current_state == LifecycleState::Operation {
                    tracing::debug!("Method {} allowed in state Ready", method);
                    return Ok(());
                } else {
                    let msg = format!(
                        "Invalid state for method {}: {:?}. Must be Ready.",
                        method,
                        current_state
                    );
                    tracing::warn!("{}", msg);
                    return Err(Error::Protocol(msg));
                }
            }
        }
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}
