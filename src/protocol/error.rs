/// Enhanced error type for the Model Context Protocol
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Capability error
    #[error("Capability error: {0}")]
    Capability(String),

    /// Version error
    #[error("Version error: {0}")]
    Version(String),

    /// Request timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Lifecycle error
    #[error("Lifecycle error: {0}")]
    Lifecycle(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Request cancelled
    #[error("Request cancelled: {0}")]
    Cancelled(String),

    /// Method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Invalid params
    #[error("Invalid params: {0}")]
    InvalidParams(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

/// Convert from JSON-RPC error to MCP error
pub fn rpc_error_to_error(error: &JSONRPCError) -> Error {
    // Convert based on standard JSON-RPC error codes
    match error.error.code {
        -32700 => Error::Parse(error.error.message.clone()),
        -32600 => Error::Protocol(error.error.message.clone()),
        -32601 => Error::MethodNotFound(error.error.message.clone()),
        -32602 => Error::InvalidParams(error.error.message.clone()),
        -32603 => Error::Internal(error.error.message.clone()),
        -32000 => Error::Capability(error.error.message.clone()),
        -32001 => Error::Version(error.error.message.clone()),
        -32002 => Error::Lifecycle(error.error.message.clone()),
        -32003 => Error::Authentication(error.error.message.clone()),
        -32800 => Error::Cancelled(error.error.message.clone()),
        _ => Error::Other(format!("Unknown error: {}", error.error.message)),
    }
}

/// Convert from MCP error to JSON-RPC error code
pub fn error_to_rpc_code(error: &Error) -> i32 {
    match error {
        Error::Parse(_) => -32700,
        Error::Protocol(_) => -32600,
        Error::MethodNotFound(_) => -32601,
        Error::InvalidParams(_) => -32602,
        Error::Internal(_) => -32603,
        Error::Capability(_) => -32000,
        Error::Version(_) => -32001,
        Error::Lifecycle(_) => -32002,
        Error::Authentication(_) => -32003,
        Error::Cancelled(_) => -32800,
        Error::Transport(_) => -32000, // Map transport to server error
        Error::Timeout(_) => -32000, // Map timeout to server error
        Error::Io(_) => -32603, // Map IO to internal error
        Error::Json(_) => -32700, // Map JSON to parse error
        Error::Other(_) => -32603, // Map other to internal error
    }
}
