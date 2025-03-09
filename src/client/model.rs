//! Client model types
//!
//! This module defines additional types needed for the client implementation
//! that mirror or extend the protocol types.

use serde::{ Deserialize, Serialize };
use serde_json::Value;
use std::collections::HashMap;

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}
