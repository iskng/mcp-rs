//! Type-safe definitions for the Model Context Protocol (MCP) method identifiers.
//! This module provides structured representation of all methods defined in the protocol.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Represents the methods defined in the Model Context Protocol (MCP).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Core initialization
    #[serde(rename = "initialize")]
    Initialize,

    /// Simple ping to check connection
    #[serde(rename = "ping")]
    Ping,

    /// Notification that initialization is complete
    #[serde(rename = "notifications/initialized")]
    NotificationsInitialized,

    /// Progress update notification
    #[serde(rename = "notifications/progress")]
    NotificationsProgress,

    /// Request cancellation notification
    #[serde(rename = "notifications/cancelled")]
    NotificationsCancelled,

    /// List available resources
    #[serde(rename = "resources/list")]
    ResourcesList,

    /// List resource templates
    #[serde(rename = "resources/templates/list")]
    ResourcesTemplatesList,

    /// Read a specific resource
    #[serde(rename = "resources/read")]
    ResourcesRead,

    /// Subscribe to resource updates
    #[serde(rename = "resources/subscribe")]
    ResourcesSubscribe,

    /// Unsubscribe from resource updates
    #[serde(rename = "resources/unsubscribe")]
    ResourcesUnsubscribe,

    /// Notification of resource list changes
    #[serde(rename = "notifications/resources/list_changed")]
    NotificationsResourcesListChanged,

    /// Notification of resource updates
    #[serde(rename = "notifications/resources/updated")]
    NotificationsResourcesUpdated,

    /// List available prompts
    #[serde(rename = "prompts/list")]
    PromptsList,

    /// Get a specific prompt
    #[serde(rename = "prompts/get")]
    PromptsGet,

    /// Notification of prompt list changes
    #[serde(rename = "notifications/prompts/list_changed")]
    NotificationsPromptsListChanged,

    /// List available tools
    #[serde(rename = "tools/list")]
    ToolsList,

    /// Call a tool
    #[serde(rename = "tools/call")]
    ToolsCall,

    /// Notification of tool list changes
    #[serde(rename = "notifications/tools/list_changed")]
    NotificationsToolsListChanged,

    /// Create a message from LLM
    #[serde(rename = "sampling/createMessage")]
    SamplingCreateMessage,

    /// Set logging level
    #[serde(rename = "logging/setLevel")]
    LoggingSetLevel,

    /// Logging message notification
    #[serde(rename = "notifications/logging/message")]
    NotificationsLoggingMessage,

    /// Get completion options
    #[serde(rename = "completion/complete")]
    CompletionComplete,

    /// List roots (directories/files)
    #[serde(rename = "roots/list")]
    RootsList,

    /// Notification of roots list changes
    #[serde(rename = "notifications/roots/list_changed")]
    NotificationsRootsListChanged,

    /// Special method to subscribe to all notifications
    #[serde(rename = "notifications/*")]
    NotificationsAll,
}

impl Method {
    /// Get the string representation of the method
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Initialize => "initialize",
            Method::Ping => "ping",
            Method::NotificationsInitialized => "notifications/initialized",
            Method::NotificationsProgress => "notifications/progress",
            Method::NotificationsCancelled => "notifications/cancelled",
            Method::ResourcesList => "resources/list",
            Method::ResourcesTemplatesList => "resources/templates/list",
            Method::ResourcesRead => "resources/read",
            Method::ResourcesSubscribe => "resources/subscribe",
            Method::ResourcesUnsubscribe => "resources/unsubscribe",
            Method::NotificationsResourcesListChanged => "notifications/resources/list_changed",
            Method::NotificationsResourcesUpdated => "notifications/resources/updated",
            Method::PromptsList => "prompts/list",
            Method::PromptsGet => "prompts/get",
            Method::NotificationsPromptsListChanged => "notifications/prompts/list_changed",
            Method::ToolsList => "tools/list",
            Method::ToolsCall => "tools/call",
            Method::NotificationsToolsListChanged => "notifications/tools/list_changed",
            Method::SamplingCreateMessage => "sampling/createMessage",
            Method::LoggingSetLevel => "logging/setLevel",
            Method::NotificationsLoggingMessage => "notifications/logging/message",
            Method::CompletionComplete => "completion/complete",
            Method::RootsList => "roots/list",
            Method::NotificationsRootsListChanged => "notifications/roots/list_changed",
            Method::NotificationsAll => "notifications/*",
        }
    }

    /// Check if this method is a notification
    pub fn is_notification(&self) -> bool {
        matches!(
            self,
            Method::NotificationsInitialized
                | Method::NotificationsCancelled
                | Method::NotificationsProgress
                | Method::NotificationsResourcesListChanged
                | Method::NotificationsResourcesUpdated
                | Method::NotificationsPromptsListChanged
                | Method::NotificationsToolsListChanged
                | Method::NotificationsLoggingMessage
                | Method::NotificationsRootsListChanged
        )
    }

    /// Check if this method is a request that requires a response
    pub fn is_request(&self) -> bool {
        !self.is_notification()
    }

    /// Check if this method is related to resources
    pub fn is_resource_method(&self) -> bool {
        matches!(
            self,
            Method::ResourcesList
                | Method::ResourcesTemplatesList
                | Method::ResourcesRead
                | Method::ResourcesSubscribe
                | Method::ResourcesUnsubscribe
                | Method::NotificationsResourcesListChanged
                | Method::NotificationsResourcesUpdated
        )
    }

    /// Check if this method is related to prompts
    pub fn is_prompt_method(&self) -> bool {
        matches!(
            self,
            Method::PromptsList | Method::PromptsGet | Method::NotificationsPromptsListChanged
        )
    }

    /// Check if this method is related to tools
    pub fn is_tool_method(&self) -> bool {
        matches!(
            self,
            Method::ToolsList | Method::ToolsCall | Method::NotificationsToolsListChanged
        )
    }

    /// Check if this method is related to logging
    pub fn is_logging_method(&self) -> bool {
        matches!(
            self,
            Method::LoggingSetLevel | Method::NotificationsLoggingMessage
        )
    }

    /// Check if this method is related to roots
    pub fn is_roots_method(&self) -> bool {
        matches!(
            self,
            Method::RootsList | Method::NotificationsRootsListChanged
        )
    }

    /// Determines if the method is client-to-server, server-to-client, or bidirectional
    pub fn direction(&self) -> MethodDirection {
        match self {
            // Client to server only
            Method::Initialize
            | Method::ResourcesList
            | Method::ResourcesTemplatesList
            | Method::ResourcesRead
            | Method::ResourcesSubscribe
            | Method::ResourcesUnsubscribe
            | Method::PromptsList
            | Method::PromptsGet
            | Method::ToolsList
            | Method::ToolsCall
            | Method::LoggingSetLevel
            | Method::CompletionComplete
            | Method::NotificationsInitialized
            | Method::NotificationsRootsListChanged => MethodDirection::ClientToServer,

            // Server to client only
            Method::SamplingCreateMessage
            | Method::RootsList
            | Method::NotificationsResourcesListChanged
            | Method::NotificationsResourcesUpdated
            | Method::NotificationsPromptsListChanged
            | Method::NotificationsToolsListChanged
            | Method::NotificationsLoggingMessage => MethodDirection::ServerToClient,

            // Bidirectional
            Method::Ping
            | Method::NotificationsProgress
            | Method::NotificationsCancelled
            | Method::NotificationsAll => MethodDirection::Bidirectional,
        }
    }

    /// Add this method
    pub fn matches(&self, other: &Method) -> bool {
        match (self, other) {
            // Special case: NotificationsAll matches any notification
            (&Method::NotificationsAll, other) => other.is_notification(),

            // Default case: exact match
            _ => self == other,
        }
    }
}

/// Indicates the direction of a method in the protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodDirection {
    /// Method is sent from client to server
    ClientToServer,
    /// Method is sent from server to client
    ServerToClient,
    /// Method can be sent in either direction
    Bidirectional,
}

impl Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Attempts to parse a string into a Method
impl std::str::FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "initialize" => Ok(Method::Initialize),
            "ping" => Ok(Method::Ping),
            "notifications/initialized" => Ok(Method::NotificationsInitialized),
            "notifications/progress" => Ok(Method::NotificationsProgress),
            "notifications/cancelled" => Ok(Method::NotificationsCancelled),
            "resources/list" => Ok(Method::ResourcesList),
            "resources/templates/list" => Ok(Method::ResourcesTemplatesList),
            "resources/read" => Ok(Method::ResourcesRead),
            "resources/subscribe" => Ok(Method::ResourcesSubscribe),
            "resources/unsubscribe" => Ok(Method::ResourcesUnsubscribe),
            "notifications/resources/list_changed" => Ok(Method::NotificationsResourcesListChanged),
            "notifications/resources/updated" => Ok(Method::NotificationsResourcesUpdated),
            "prompts/list" => Ok(Method::PromptsList),
            "prompts/get" => Ok(Method::PromptsGet),
            "notifications/prompts/list_changed" => Ok(Method::NotificationsPromptsListChanged),
            "tools/list" => Ok(Method::ToolsList),
            "tools/call" => Ok(Method::ToolsCall),
            "notifications/tools/list_changed" => Ok(Method::NotificationsToolsListChanged),
            "sampling/createMessage" => Ok(Method::SamplingCreateMessage),
            "logging/setLevel" => Ok(Method::LoggingSetLevel),
            "notifications/logging/message" => Ok(Method::NotificationsLoggingMessage),
            "completion/complete" => Ok(Method::CompletionComplete),
            "roots/list" => Ok(Method::RootsList),
            "notifications/roots/list_changed" => Ok(Method::NotificationsRootsListChanged),
            _ => Err(format!("Unknown method: {}", s)),
        }
    }
}

/// Module that provides serde helper functions for serializing Method as an
/// inline string rather than a struct when used within other structures
pub mod method_as_string {
    use super::Method;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(method.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Method, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Method>().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_serialization() {
        let method = Method::Initialize;
        let serialized = serde_json::to_string(&method).unwrap();
        assert_eq!(serialized, "\"initialize\"");

        let method = Method::NotificationsResourcesListChanged;
        let serialized = serde_json::to_string(&method).unwrap();
        assert_eq!(serialized, "\"notifications/resources/list_changed\"");
    }

    #[test]
    fn test_method_deserialization() {
        let deserialized: Method = serde_json::from_str("\"initialize\"").unwrap();
        assert_eq!(deserialized, Method::Initialize);

        let deserialized: Method =
            serde_json::from_str("\"notifications/resources/list_changed\"").unwrap();
        assert_eq!(deserialized, Method::NotificationsResourcesListChanged);
    }

    #[test]
    fn test_direction() {
        assert_eq!(
            Method::Initialize.direction(),
            MethodDirection::ClientToServer
        );
        assert_eq!(
            Method::SamplingCreateMessage.direction(),
            MethodDirection::ServerToClient
        );
        assert_eq!(Method::Ping.direction(), MethodDirection::Bidirectional);
    }

    #[test]
    fn test_is_notification() {
        assert!(Method::NotificationsInitialized.is_notification());
        assert!(!Method::Initialize.is_notification());
    }

    #[test]
    fn test_categorization() {
        assert!(Method::ResourcesList.is_resource_method());
        assert!(Method::PromptsGet.is_prompt_method());
        assert!(Method::ToolsCall.is_tool_method());
        assert!(Method::LoggingSetLevel.is_logging_method());
        assert!(Method::RootsList.is_roots_method());
    }

    #[test]
    fn test_from_str() {
        assert_eq!("initialize".parse::<Method>().unwrap(), Method::Initialize);
        assert!("unknown_method".parse::<Method>().is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(Method::Initialize.to_string(), "initialize");
        assert_eq!(
            Method::NotificationsResourcesListChanged.to_string(),
            "notifications/resources/list_changed"
        );
    }
}
