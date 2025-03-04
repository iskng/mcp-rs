//! MCP Resource Types
//!
//! This module defines types related to resources in the MCP protocol, such as
//! `Resource` and `ResourceTemplate`, which represent data sources or files that
//! can be accessed or manipulated by the AI application.

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::io;
use std::path::{ Path, PathBuf };

/// Represents a resource in the MCP protocol
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Resource {
    /// Unique identifier for the resource
    pub uri: String,
    /// Human-readable name for the resource
    pub name: String,
    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content (e.g., "text/plain")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Parameters for listing resources
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListResourcesParams {
    /// Optional filter for resource types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional pagination token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    /// Optional limit on the number of resources to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

/// Result of listing resources
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListResourcesResult {
    /// List of resources
    pub resources: Vec<Resource>,
    /// Token for the next page (if there are more resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Parameters for getting a resource's content
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GetResourceParams {
    /// URI of the resource to get
    pub uri: String,
}

/// Result of getting a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GetResourceResult {
    /// The resource metadata
    pub resource: Resource,
    /// The content of the resource
    pub content: String,
}

/// Parameters for creating a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CreateResourceParams {
    /// Human-readable name for the resource
    pub name: String,
    /// Content of the resource
    pub content: String,
    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Result of creating a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CreateResourceResult {
    /// The created resource
    pub resource: Resource,
}

/// Parameters for updating a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct UpdateResourceParams {
    /// URI of the resource to update
    pub uri: String,
    /// New content for the resource
    pub content: String,
}

/// Result of updating a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct UpdateResourceResult {
    /// The updated resource
    pub resource: Resource,
}

/// Parameters for deleting a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DeleteResourceParams {
    /// URI of the resource to delete
    pub uri: String,
}

/// Result of deleting a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DeleteResourceResult {
    /// Indicates whether the resource was successfully deleted
    pub success: bool,
}

/// A file-based resource that can be created from a local file
pub struct FileResource {
    /// The resource metadata
    pub resource: Resource,
    /// The local file path
    pub path: PathBuf,
    /// Whether the file should be read as binary
    pub is_binary: bool,
}

impl FileResource {
    /// Create a new file resource with the given parameters
    pub fn new<P: AsRef<Path>>(
        uri: Option<String>,
        name: String,
        path: P,
        description: Option<String>,
        mime_type: Option<String>,
        is_binary: bool
    ) -> Result<Self, io::Error> {
        let path = path.as_ref().to_path_buf();

        // Validate the path is absolute
        if !path.is_absolute() {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "FileResource path must be absolute")
            );
        }

        // Validate the file exists
        if !path.exists() {
            return Err(
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File not found: {}", path.display())
                )
            );
        }

        // Determine mime type if not provided
        let mime_type = mime_type.or_else(||
            mime_guess
                ::from_path(&path)
                .first()
                .map(|m| m.to_string())
        );

        // Generate a URI if not provided
        let uri = uri.unwrap_or_else(|| format!("file://{}", path.display()));

        Ok(Self {
            resource: Resource {
                uri,
                name,
                description,
                mime_type,
            },
            path,
            is_binary,
        })
    }

    /// Read the file content
    pub fn read(&self) -> Result<Vec<u8>, io::Error> {
        fs::read(&self.path)
    }

    /// Read the file as text
    pub fn read_text(&self) -> Result<String, io::Error> {
        fs::read_to_string(&self.path)
    }

    /// Get the resource metadata
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Convert to a CreateResourceParams for creating on the server
    pub fn to_create_params(&self) -> Result<CreateResourceParams, io::Error> {
        let content = if self.is_binary {
            // For binary files, read as bytes and convert to base64
            let bytes = self.read()?;
            BASE64_STANDARD.encode(&bytes)
        } else {
            // For text files, just read as string
            self.read_text()?
        };

        Ok(CreateResourceParams {
            name: self.resource.name.clone(),
            content,
            description: self.resource.description.clone(),
            mime_type: self.resource.mime_type.clone(),
        })
    }
}
