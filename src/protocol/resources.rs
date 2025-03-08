use crate::protocol::Error;
use crate::protocol::{
    Annotations, BlobResourceContents, Content, Cursor, EmbeddedResource, ImageContent,
    ListResourcesResult, PaginatedRequestParams, ReadResourceParams, ReadResourceResult, Resource,
    ResourceContentType, ResourceTemplate, SubscribeParams, TextContent, TextResourceContents,
    UnsubscribeParams,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

/// Helper to create a resource from a path
pub fn resource_from_path(
    uri: &str,
    name: &str,
    path: &Path,
    description: Option<&str>,
    mime_type: Option<&str>,
) -> Result<Resource, Error> {
    let metadata = fs::metadata(path).map_err(|e| {
        Error::Resource(format!(
            "Failed to read metadata for path {}: {}",
            path.display(),
            e
        ))
    })?;

    let size = metadata.len() as i64;

    Ok(Resource {
        uri: uri.to_string(),
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        mime_type: mime_type.map(|s| s.to_string()),
        size: Some(size),
        annotations: None,
    })
}

/// Helper to create a resource template
pub fn create_resource_template(
    uri_template: &str,
    name: &str,
    description: Option<&str>,
    mime_type: Option<&str>,
) -> ResourceTemplate {
    ResourceTemplate {
        uri_template: uri_template.to_string(),
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        mime_type: mime_type.map(|s| s.to_string()),
        annotations: None,
    }
}

/// Helper to create text resource contents from a file
pub fn text_resource_contents_from_file(
    uri: &str,
    path: &Path,
    mime_type: Option<&str>,
) -> Result<ResourceContentType, Error> {
    let content = fs::read_to_string(path).map_err(|e| {
        Error::Resource(format!(
            "Failed to read text file {}: {}",
            path.display(),
            e
        ))
    })?;

    Ok(ResourceContentType::Text(TextResourceContents {
        uri: uri.to_string(),
        text: content,
        mime_type: mime_type.map(|s| s.to_string()),
    }))
}

/// Helper to create blob resource contents from a file
pub fn blob_resource_contents_from_file(
    uri: &str,
    path: &Path,
    mime_type: Option<&str>,
) -> Result<ResourceContentType, Error> {
    let content = fs::read(path).map_err(|e| {
        Error::Resource(format!(
            "Failed to read binary file {}: {}",
            path.display(),
            e
        ))
    })?;

    Ok(ResourceContentType::Blob(BlobResourceContents {
        uri: uri.to_string(),
        blob: base64::encode(&content),
        mime_type: mime_type.map(|s| s.to_string()),
    }))
}

/// Convert a file path to a resource URI
pub fn path_to_uri(path: &Path) -> String {
    if path.is_absolute() {
        Url::from_file_path(path)
            .map(|url| url.to_string())
            .unwrap_or_else(|_| format!("file://{}", path.display()))
    } else {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let absolute_path = current_dir.join(path);

        Url::from_file_path(absolute_path)
            .map(|url| url.to_string())
            .unwrap_or_else(|_| format!("file://{}", path.display()))
    }
}

/// Create text content from a string
pub fn create_text_content(text: &str) -> Content {
    Content::Text(TextContent {
        type_field: "text".to_string(),
        text: text.to_string(),
        annotations: None,
    })
}

/// Create image content from binary data
pub fn create_image_content(data: &[u8], mime_type: &str) -> Content {
    Content::Image(ImageContent {
        type_field: "image".to_string(),
        data: base64::encode(data),
        mime_type: mime_type.to_string(),
        annotations: None,
    })
}

/// Create embedded resource content
pub fn create_embedded_resource(resource_content: ResourceContentType) -> Content {
    Content::Resource(EmbeddedResource {
        type_field: "resource".to_string(),
        resource: resource_content,
        annotations: None,
    })
}

/// Create a list resources result
pub fn create_resources_list_result(
    resources: Vec<Resource>,
    next_cursor: Option<String>,
) -> ListResourcesResult {
    ListResourcesResult {
        resources,
        next_cursor: next_cursor.map(Cursor),
        _meta: None,
    }
}

/// Create a read resource result
pub fn create_read_resource_result(contents: Vec<ResourceContentType>) -> ReadResourceResult {
    ReadResourceResult {
        contents,
        _meta: None,
    }
}

/// Create read resource params
pub fn create_read_resource_params(uri: &str) -> ReadResourceParams {
    ReadResourceParams {
        uri: uri.to_string(),
    }
}

/// Create subscribe params
pub fn create_subscribe_params(uri: &str) -> SubscribeParams {
    SubscribeParams {
        uri: uri.to_string(),
    }
}

/// Create unsubscribe params
pub fn create_unsubscribe_params(uri: &str) -> UnsubscribeParams {
    UnsubscribeParams {
        uri: uri.to_string(),
    }
}

/// Create paginated request params
pub fn create_paginated_params(cursor: Option<String>) -> PaginatedRequestParams {
    PaginatedRequestParams {
        cursor: cursor.map(Cursor),
    }
}

/// Represents a local file resource that can be shared via MCP
pub struct FileResource {
    /// The resource metadata
    pub resource: Resource,
    /// The local file path
    pub path: PathBuf,
    /// Whether the file should be read as binary
    pub is_binary: bool,
}

impl FileResource {
    /// Create a new file resource
    pub fn new<P: AsRef<Path>>(
        uri: Option<String>,
        name: String,
        path: P,
        description: Option<String>,
        mime_type: Option<String>,
        is_binary: bool,
    ) -> Result<Self, io::Error> {
        let path_buf = path.as_ref().to_path_buf();

        // Verify the file exists
        if !path_buf.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", path_buf.display()),
            ));
        }

        // Generate a URI if none provided
        let uri = uri.unwrap_or_else(|| path_to_uri(&path_buf));

        // Create resource metadata
        let metadata = fs::metadata(&path_buf)?;
        let size = metadata.len() as i64;

        let resource = Resource {
            uri,
            name,
            description,
            mime_type,
            size: Some(size),
            annotations: None,
        };

        Ok(Self {
            resource,
            path: path_buf,
            is_binary,
        })
    }

    /// Read the file contents as bytes
    pub fn read(&self) -> Result<Vec<u8>, io::Error> {
        fs::read(&self.path)
    }

    /// Read the file contents as text
    pub fn read_text(&self) -> Result<String, io::Error> {
        fs::read_to_string(&self.path)
    }

    /// Get the resource metadata
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Get the resource content
    pub fn content(&self) -> Result<ResourceContentType, io::Error> {
        if self.is_binary {
            let data = self.read()?;
            Ok(ResourceContentType::Blob(BlobResourceContents {
                uri: self.resource.uri.clone(),
                blob: base64::encode(data),
                mime_type: self.resource.mime_type.clone(),
            }))
        } else {
            let text = self.read_text()?;
            Ok(ResourceContentType::Text(TextResourceContents {
                uri: self.resource.uri.clone(),
                text,
                mime_type: self.resource.mime_type.clone(),
            }))
        }
    }

    /// Convert to embedded resource content
    pub fn to_embedded_resource(&self) -> Result<Content, io::Error> {
        let content = self.content()?;
        Ok(Content::Resource(EmbeddedResource {
            type_field: "resource".to_string(),
            resource: content,
            annotations: None,
        }))
    }
}

/// A URI template for parameterized resources
pub struct UriTemplate {
    /// The template string
    template: String,
    /// Parsed segments
    segments: Vec<UriTemplateSegment>,
}

#[derive(PartialEq, Eq, Debug)]
/// Segment of a URI template
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

    /// Parse a template string into segments
    fn parse_template(template: &str) -> Vec<UriTemplateSegment> {
        let mut segments = Vec::new();
        let mut current_pos = 0;

        while current_pos < template.len() {
            // Find the next parameter start
            if let Some(param_start) = template[current_pos..].find('{') {
                let param_start = current_pos + param_start;

                // Add any literal before the parameter
                if param_start > current_pos {
                    let literal = &template[current_pos..param_start];
                    segments.push(UriTemplateSegment::Literal(literal.to_string()));
                }

                // Find the parameter end
                if let Some(param_end) = template[param_start..].find('}') {
                    let param_end = param_start + param_end;
                    let param_name = &template[param_start + 1..param_end];
                    segments.push(UriTemplateSegment::Parameter(param_name.to_string()));
                    current_pos = param_end + 1;
                } else {
                    // No closing brace, treat the rest as literal
                    let literal = &template[current_pos..];
                    segments.push(UriTemplateSegment::Literal(literal.to_string()));
                    current_pos = template.len();
                }
            } else {
                // No more parameters, add the rest as literal
                let literal = &template[current_pos..];
                segments.push(UriTemplateSegment::Literal(literal.to_string()));
                current_pos = template.len();
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
                UriTemplateSegment::Literal(literal) => {
                    // The literal should match exactly at the current position
                    if !uri[uri_pos..].starts_with(literal) {
                        return None;
                    }
                    uri_pos += literal.len();
                }
                UriTemplateSegment::Parameter(param_name) => {
                    // Find the next segment that is a literal
                    let next_segment_pos = self
                        .segments
                        .iter()
                        .position(|s| s == segment)
                        .and_then(|p| {
                            // Find the next literal segment
                            self.segments[p + 1..]
                                .iter()
                                .position(|s| matches!(s, UriTemplateSegment::Literal(_)))
                                .map(|offset| p + 1 + offset)
                        });

                    // If we have a next literal segment, find where it starts in the URI
                    let next_literal_pos = (if let Some(next_segment_idx) = next_segment_pos {
                        if let UriTemplateSegment::Literal(next_literal) =
                            &self.segments[next_segment_idx]
                        {
                            uri[uri_pos..].find(next_literal).map(|p| p + uri_pos)
                        } else {
                            None
                        }
                    } else {
                        None
                    })
                    .unwrap_or(uri.len());

                    // Extract the parameter value
                    let value = &uri[uri_pos..next_literal_pos];
                    params.insert(param_name.clone(), value.to_string());

                    uri_pos = next_literal_pos;
                }
            }
        }

        // If we consumed the entire URI, the match is successful
        if uri_pos == uri.len() {
            Some(params)
        } else {
            None
        }
    }

    /// Generate a URI by applying parameter values
    pub fn generate_uri(&self, params: &HashMap<String, String>) -> Result<String, String> {
        let mut result = String::new();

        for segment in &self.segments {
            match segment {
                UriTemplateSegment::Literal(literal) => {
                    result.push_str(literal);
                }
                UriTemplateSegment::Parameter(param_name) => {
                    if let Some(value) = params.get(param_name) {
                        // URL encode the parameter value
                        let encoded = urlencoding::encode(value);
                        result.push_str(&encoded);
                    } else {
                        return Err(format!("Missing parameter: {}", param_name));
                    }
                }
            }
        }

        Ok(result)
    }
}

/// A trait for providing resource content
pub trait ResourceProvider: Send + Sync {
    /// Get the resource metadata
    fn metadata(&self) -> Resource;

    /// Get the resource content
    async fn content(&self) -> Result<ResourceContentType, Error>;
}

/// A trait for providing parameterized resource content
pub trait TemplateResourceProvider: Send + Sync {
    /// Get the resource template metadata
    fn template(&self) -> ResourceTemplate;

    /// Get the resource content for the given parameters
    async fn content(&self, params: HashMap<String, String>) -> Result<ResourceContentType, Error>;
}

/// A static resource with content that doesn't change
pub struct StaticResource {
    resource: Resource,
    content_provider: Box<dyn (Fn() -> Result<ResourceContentType, Error>) + Send + Sync>,
}

impl StaticResource {
    /// Create a new static resource
    pub fn new<F>(resource: Resource, content_provider: F) -> Self
    where
        F: Fn() -> Result<ResourceContentType, Error> + Send + Sync + 'static,
    {
        Self {
            resource,
            content_provider: Box::new(content_provider),
        }
    }

    /// Create a static text resource
    pub fn text<F>(uri: String, name: String, mime_type: String, text_provider: F) -> Self
    where
        F: Fn() -> Result<String, Error> + Send + Sync + 'static,
    {
        let resource = Resource {
            uri: uri.clone(),
            name,
            description: None,
            mime_type: Some(mime_type.clone()),
            size: None,
            annotations: None,
        };

        let content_provider = move || {
            let text = text_provider()?;
            Ok(ResourceContentType::Text(TextResourceContents {
                uri: uri.clone(),
                text,
                mime_type: Some(mime_type.clone()),
            }))
        };

        Self {
            resource,
            content_provider: Box::new(content_provider),
        }
    }

    /// Create a static binary resource
    pub fn binary<F>(uri: String, name: String, mime_type: String, data_provider: F) -> Self
    where
        F: Fn() -> Result<Vec<u8>, Error> + Send + Sync + 'static,
    {
        let resource = Resource {
            uri: uri.clone(),
            name,
            description: None,
            mime_type: Some(mime_type.clone()),
            size: None,
            annotations: None,
        };

        let content_provider = move || {
            let data = data_provider()?;
            Ok(ResourceContentType::Blob(BlobResourceContents {
                uri: uri.clone(),
                blob: base64::encode(&data),
                mime_type: Some(mime_type.clone()),
            }))
        };

        Self {
            resource,
            content_provider: Box::new(content_provider),
        }
    }

    /// Add a description to this resource
    pub fn with_description(mut self, description: String) -> Self {
        self.resource.description = Some(description);
        self
    }
}

impl ResourceProvider for StaticResource {
    fn metadata(&self) -> Resource {
        self.resource.clone()
    }

    async fn content(&self) -> Result<ResourceContentType, Error> {
        (self.content_provider)()
    }
}

/// A dynamic resource with content based on URI parameters
pub struct TemplateResource {
    template: ResourceTemplate,
    content_provider:
        Box<dyn (Fn(HashMap<String, String>) -> Result<ResourceContentType, Error>) + Send + Sync>,
}

impl TemplateResource {
    /// Create a new template resource
    pub fn new<F>(template: ResourceTemplate, content_provider: F) -> Self
    where
        F: Fn(HashMap<String, String>) -> Result<ResourceContentType, Error>
            + Send
            + Sync
            + 'static,
    {
        Self {
            template,
            content_provider: Box::new(content_provider),
        }
    }

    /// Create a template resource with text content
    pub fn text<F>(uri_template: String, name: String, mime_type: String, text_provider: F) -> Self
    where
        F: Fn(HashMap<String, String>) -> Result<String, Error> + Send + Sync + 'static,
    {
        let template = ResourceTemplate {
            uri_template: uri_template.clone(),
            name,
            description: None,
            mime_type: Some(mime_type.clone()),
            annotations: None,
        };

        let uri_template_obj = Arc::new(UriTemplate::new(&uri_template));
        let content_provider = move |params: HashMap<String, String>| {
            // Generate the actual URI from the template and parameters
            let uri = uri_template_obj
                .generate_uri(&params)
                .map_err(|e| Error::Resource(format!("Failed to generate URI: {}", e)))?;

            let text = text_provider(params)?;
            Ok(ResourceContentType::Text(TextResourceContents {
                uri,
                text,
                mime_type: Some(mime_type.clone()),
            }))
        };

        Self {
            template,
            content_provider: Box::new(content_provider),
        }
    }

    /// Add a description to this template
    pub fn with_description(mut self, description: String) -> Self {
        self.template.description = Some(description);
        self
    }
}

impl TemplateResourceProvider for TemplateResource {
    fn template(&self) -> ResourceTemplate {
        self.template.clone()
    }

    async fn content(&self, params: HashMap<String, String>) -> Result<ResourceContentType, Error> {
        (self.content_provider)(params)
    }
}

/// ResourceBuilder for easy creation of resources
pub struct ResourceBuilder {
    uri: String,
    name: String,
    description: Option<String>,
    mime_type: Option<String>,
    size: Option<i64>,
    annotations: Option<Annotations>,
}

impl ResourceBuilder {
    /// Create a new resource builder
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
            size: None,
            annotations: None,
        }
    }

    /// Add a description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a MIME type
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Add a size
    pub fn size(mut self, size: i64) -> Self {
        self.size = Some(size);
        self
    }

    /// Add annotations
    pub fn annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Build the resource
    pub fn build(self) -> Resource {
        Resource {
            uri: self.uri,
            name: self.name,
            description: self.description,
            mime_type: self.mime_type,
            size: self.size,
            annotations: self.annotations,
        }
    }
}

/// ResourceTemplateBuilder for easy creation of resource templates
pub struct ResourceTemplateBuilder {
    uri_template: String,
    name: String,
    description: Option<String>,
    mime_type: Option<String>,
    annotations: Option<Annotations>,
}

impl ResourceTemplateBuilder {
    /// Create a new resource template builder
    pub fn new(uri_template: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri_template: uri_template.into(),
            name: name.into(),
            description: None,
            mime_type: None,
            annotations: None,
        }
    }

    /// Add a description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a MIME type
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Add annotations
    pub fn annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Build the resource template
    pub fn build(self) -> ResourceTemplate {
        ResourceTemplate {
            uri_template: self.uri_template,
            name: self.name,
            description: self.description,
            mime_type: self.mime_type,
            annotations: self.annotations,
        }
    }
}
