//! MCP Resource Types
//!
//! This module defines types related to resources in the MCP protocol, such as
//! `Resource` and `ResourceTemplate`, which represent data sources or files that
//! can be accessed or manipulated by the AI application.

use async_trait::async_trait;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };

use std::fs;
use std::io;
use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use crate::errors::Error;
use crate::server::resources::ResourceProvider;
use crate::server::resources::TemplateResourceProvider;

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

/// Parameter for a resource template
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Parameter {
    /// Name of the parameter
    pub name: String,
    /// Optional description of the parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Type of the parameter (e.g., "string", "number")
    #[serde(rename = "type")]
    pub parameter_type: String,
}

/// Resource template for parameterized resources
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ResourceTemplate {
    /// URI template with parameters (e.g., "users/{user_id}/profile")
    pub uri_template: String,
    /// Human-readable name for the resource template
    pub name: String,
    /// Optional description of the resource template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Parameters that can be used in the URI template
    pub parameters: Vec<Parameter>,
    /// MIME type of the resource content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Resource content with support for both text and binary data
#[derive(Debug, Clone, JsonSchema)]
pub enum ResourceContent {
    /// Text content (e.g., JSON, HTML, etc.)
    Text {
        /// URI of the resource
        uri: String,
        /// Text content
        text: String,
        /// MIME type of the content
        mime_type: String,
    },
    /// Binary content (e.g., images, PDFs, etc.)
    Binary {
        /// URI of the resource
        uri: String,
        /// Binary content
        data: Vec<u8>,
        /// MIME type of the content
        mime_type: String,
    },
}

impl Serialize for ResourceContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        match self {
            ResourceContent::Text { uri, text, mime_type } => {
                let mut map = serde_json::Map::new();
                map.insert("uri".to_string(), serde_json::Value::String(uri.clone()));
                map.insert("text".to_string(), serde_json::Value::String(text.clone()));
                map.insert("mimeType".to_string(), serde_json::Value::String(mime_type.clone()));
                map.serialize(serializer)
            }
            ResourceContent::Binary { uri, data, mime_type } => {
                let mut map = serde_json::Map::new();
                map.insert("uri".to_string(), serde_json::Value::String(uri.clone()));
                map.insert(
                    "blob".to_string(),
                    serde_json::Value::String(BASE64_STANDARD.encode(data))
                );
                map.insert("mimeType".to_string(), serde_json::Value::String(mime_type.clone()));
                map.serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for ResourceContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let map = serde_json::Map::deserialize(deserializer)?;

        let uri = map
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("uri"))?
            .to_string();

        let mime_type = map
            .get("mimeType")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("mimeType"))?
            .to_string();

        if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
            Ok(ResourceContent::Text {
                uri,
                text: text.to_string(),
                mime_type,
            })
        } else if let Some(blob) = map.get("blob").and_then(|v| v.as_str()) {
            let data = BASE64_STANDARD.decode(blob).map_err(|e|
                serde::de::Error::custom(format!("Invalid base64: {}", e))
            )?;

            Ok(ResourceContent::Binary {
                uri,
                data,
                mime_type,
            })
        } else {
            Err(serde::de::Error::missing_field("text or blob"))
        }
    }
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

/// Parameters for listing resource templates
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListResourceTemplatesParams {
    /// Optional filter for resource template types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Result of listing resource templates
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ListResourceTemplatesResult {
    /// List of resource templates
    pub templates: Vec<ResourceTemplate>,
}

/// Parameters for reading a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ReadResourceParams {
    /// URI of the resource to read
    pub uri: String,
}

/// Result of reading a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ReadResourceResult {
    /// Content of the resource
    #[serde(flatten)]
    pub content: ResourceContent,
}

/// Parameters for subscribing to a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SubscribeResourceParams {
    /// URI of the resource to subscribe to
    pub uri: String,
}

/// Result of subscribing to a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SubscribeResourceResult {
    /// Success indicator
    pub success: bool,
}

/// Parameters for unsubscribing from a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct UnsubscribeResourceParams {
    /// URI of the resource to unsubscribe from
    pub uri: String,
}

/// Result of unsubscribing from a resource
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct UnsubscribeResourceResult {
    /// Success indicator
    pub success: bool,
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

    /// Get the resource content
    pub fn content(&self) -> Result<ResourceContent, io::Error> {
        if self.is_binary {
            let data = self.read()?;
            Ok(ResourceContent::Binary {
                uri: self.resource.uri.clone(),
                data,
                mime_type: self.resource.mime_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
            })
        } else {
            let text = self.read_text()?;
            Ok(ResourceContent::Text {
                uri: self.resource.uri.clone(),
                text,
                mime_type: self.resource.mime_type
                    .clone()
                    .unwrap_or_else(|| "text/plain".to_string()),
            })
        }
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

/// URI template utilities
pub struct UriTemplate {
    /// The template string
    template: String,
    /// Parsed segments
    segments: Vec<UriTemplateSegment>,
}

/// A segment in a URI template
enum UriTemplateSegment {
    /// Literal text
    Literal(String),
    /// Parameter placeholder
    Parameter(String),
}

impl UriTemplate {
    /// Create a new URI template
    pub fn new(template: &str) -> Self {
        let segments = Self::parse_template(template);
        Self {
            template: template.to_string(),
            segments,
        }
    }

    /// Parse a URI template into segments
    fn parse_template(template: &str) -> Vec<UriTemplateSegment> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut in_param = false;

        for c in template.chars() {
            if c == '{' && !in_param {
                if !current.is_empty() {
                    segments.push(UriTemplateSegment::Literal(current));
                    current = String::new();
                }
                in_param = true;
            } else if c == '}' && in_param {
                segments.push(UriTemplateSegment::Parameter(current));
                current = String::new();
                in_param = false;
            } else {
                current.push(c);
            }
        }

        if !current.is_empty() {
            if in_param {
                segments.push(UriTemplateSegment::Parameter(current));
            } else {
                segments.push(UriTemplateSegment::Literal(current));
            }
        }

        segments
    }

    /// Match a URI against this template and extract parameters
    pub fn match_uri(&self, uri: &str) -> Option<HashMap<String, String>> {
        let mut params = HashMap::new();
        let mut uri_pos = 0;

        for segment in &self.segments {
            match segment {
                UriTemplateSegment::Literal(lit) => {
                    // Must match literal exactly
                    if !uri[uri_pos..].starts_with(lit) {
                        return None;
                    }
                    uri_pos += lit.len();
                }
                UriTemplateSegment::Parameter(param_name) => {
                    // Find the end of this parameter
                    let end_pos = match
                        self.segments
                            .iter()
                            .skip_while(|s| !matches!(s, UriTemplateSegment::Literal(_)))
                            .next()
                    {
                        Some(UriTemplateSegment::Literal(next_lit)) => {
                            match uri[uri_pos..].find(next_lit) {
                                Some(pos) => uri_pos + pos,
                                None => {
                                    return None;
                                } // Cannot find next literal
                            }
                        }
                        _ => uri.len(), // End of URI
                    };

                    params.insert(param_name.clone(), uri[uri_pos..end_pos].to_string());
                    uri_pos = end_pos;
                }
            }
        }

        // Ensure we consumed the entire URI
        if uri_pos == uri.len() {
            Some(params)
        } else {
            None
        }
    }

    /// Generate a URI by substituting parameters
    pub fn generate_uri(&self, params: &HashMap<String, String>) -> Result<String, String> {
        let mut result = String::new();

        for segment in &self.segments {
            match segment {
                UriTemplateSegment::Literal(lit) => {
                    result.push_str(lit);
                }
                UriTemplateSegment::Parameter(param_name) => {
                    let value = params
                        .get(param_name)
                        .ok_or_else(|| format!("Missing parameter: {}", param_name))?;
                    result.push_str(value);
                }
            }
        }

        Ok(result)
    }
}

/// Simplified struct for static resources
pub struct StaticResource {
    resource: Resource,
    content_provider: Box<dyn (Fn() -> Result<ResourceContent, Error>) + Send + Sync>,
}

impl StaticResource {
    /// Create a new static resource
    pub fn new<F>(resource: Resource, content_provider: F) -> Self
        where F: Fn() -> Result<ResourceContent, Error> + Send + Sync + 'static
    {
        Self {
            resource,
            content_provider: Box::new(content_provider),
        }
    }

    /// Create a text resource
    pub fn text<F>(uri: String, name: String, mime_type: String, text_provider: F) -> Self
        where F: Fn() -> Result<String, Error> + Send + Sync + 'static
    {
        let resource = Resource {
            uri: uri.clone(),
            name,
            description: None,
            mime_type: Some(mime_type.clone()),
        };

        let uri_clone = uri.clone();
        let mime_type_clone = mime_type.clone();

        Self::new(resource, move || {
            let text = text_provider()?;
            Ok(ResourceContent::Text {
                uri: uri_clone.clone(),
                text,
                mime_type: mime_type_clone.clone(),
            })
        })
    }

    /// Create a binary resource
    pub fn binary<F>(uri: String, name: String, mime_type: String, data_provider: F) -> Self
        where F: Fn() -> Result<Vec<u8>, Error> + Send + Sync + 'static
    {
        let resource = Resource {
            uri: uri.clone(),
            name,
            description: None,
            mime_type: Some(mime_type.clone()),
        };

        let uri_clone = uri.clone();
        let mime_type_clone = mime_type.clone();

        Self::new(resource, move || {
            let data = data_provider()?;
            Ok(ResourceContent::Binary {
                uri: uri_clone.clone(),
                data,
                mime_type: mime_type_clone.clone(),
            })
        })
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.resource.description = Some(description);
        self
    }
}

#[async_trait]
impl ResourceProvider for StaticResource {
    fn metadata(&self) -> Resource {
        self.resource.clone()
    }

    async fn content(&self) -> Result<ResourceContent, Error> {
        (self.content_provider)()
    }
}

/// Simplified struct for template resources
pub struct TemplateResource {
    template: ResourceTemplate,
    content_provider: Box<
        dyn (Fn(HashMap<String, String>) -> Result<ResourceContent, Error>) + Send + Sync
    >,
}

impl TemplateResource {
    /// Create a new template resource
    pub fn new<F>(template: ResourceTemplate, content_provider: F) -> Self
        where
            F: Fn(HashMap<String, String>) -> Result<ResourceContent, Error> + Send + Sync + 'static
    {
        Self {
            template,
            content_provider: Box::new(content_provider),
        }
    }

    /// Create a text template resource
    pub fn text<F>(
        uri_template: String,
        name: String,
        mime_type: String,
        parameters: Vec<Parameter>,
        text_provider: F
    ) -> Self
        where F: Fn(HashMap<String, String>) -> Result<String, Error> + Send + Sync + 'static
    {
        let template = ResourceTemplate {
            uri_template: uri_template.clone(),
            name,
            description: None,
            parameters,
            mime_type: Some(mime_type.clone()),
        };

        // Clone strings to move into the closure
        let uri_template_clone = uri_template.clone();
        let mime_type_clone = mime_type.clone();

        Self::new(template, move |params| {
            // Clone params to avoid move issues
            let params_clone = params.clone();
            let text = text_provider(params)?;

            // Use the template to generate a URI
            let template_parser = UriTemplate::new(&uri_template_clone);
            let uri = match template_parser.generate_uri(&params_clone) {
                Ok(uri) => uri,
                Err(e) => {
                    return Err(Error::Resource(format!("Failed to generate URI: {}", e)));
                }
            };

            Ok(ResourceContent::Text {
                uri,
                text,
                mime_type: mime_type_clone.clone(),
            })
        })
    }

    /// Set template description
    pub fn with_description(mut self, description: String) -> Self {
        self.template.description = Some(description);
        self
    }
}

#[async_trait]
impl TemplateResourceProvider for TemplateResource {
    async fn content(&self, params: HashMap<String, String>) -> Result<ResourceContent, Error> {
        (self.content_provider)(params)
    }
}
