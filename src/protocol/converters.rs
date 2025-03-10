use crate::protocol::Error;
use crate::protocol::{
    BlobResourceContents, CallToolResult, Content, Cursor, EmbeddedResource, ImageContent,
    JSONRPCRequest, ListToolsResult, ResourceContentType, Result as ProtocolResult, TextContent,
    TextResourceContents, Tool,
};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde_json::Value;
use std::collections::HashMap;

/// Helper function to convert a JSON value to a HashMap
pub fn json_to_hashmap(value: Value) -> HashMap<String, Value> {
    match value {
        Value::Object(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                result.insert(k, v);
            }
            result
        }
        _ => HashMap::new(),
    }
}

/// Create a protocol result from a value
pub fn to_protocol_result(value: Value) -> ProtocolResult {
    ProtocolResult {
        _meta: None,
        content: json_to_hashmap(value),
    }
}

/// Helper to create a tools result with pagination
pub fn to_tools_result(tools: Vec<Tool>, next_cursor: Option<String>) -> ListToolsResult {
    ListToolsResult {
        tools,
        next_cursor: next_cursor.map(Cursor),
        _meta: None,
    }
}

/// Helper to extract params from a request
pub fn extract_params<T>(request: &JSONRPCRequest) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    if let Some(params) = &request.params {
        serde_json::from_value(params.clone())
            .map_err(|e| Error::InvalidParams(format!("Could not parse params: {}", e)))
    } else {
        Err(Error::InvalidParams("No params provided".to_string()))
    }
}

/// Helper function to create a CallToolResult from content
pub fn to_call_tool_result(content: Vec<Content>, is_error: bool) -> CallToolResult {
    CallToolResult {
        content,
        is_error: if is_error { Some(true) } else { None },
        _meta: None,
    }
}

/// Helper to create text content
pub fn to_text_content(text: &str) -> Content {
    Content::Text(TextContent {
        type_field: "text".to_string(),
        text: text.to_string(),
        annotations: None,
    })
}

/// Helper to create image content
pub fn to_image_content(data: &[u8], mime_type: &str) -> Content {
    Content::Image(ImageContent {
        type_field: "image".to_string(),
        data: BASE64_STANDARD.encode(data),
        mime_type: mime_type.to_string(),
        annotations: None,
    })
}

/// Helper to create embedded resource content
pub fn to_embedded_resource(_uri: &str, content_type: ResourceContentType) -> Content {
    Content::Resource(EmbeddedResource {
        type_field: "resource".to_string(),
        resource: content_type,
        annotations: None,
    })
}

/// Helper to create text resource content
pub fn to_text_resource_content(
    uri: &str,
    text: &str,
    mime_type: Option<&str>,
) -> ResourceContentType {
    ResourceContentType::Text(TextResourceContents {
        uri: uri.to_string(),
        text: text.to_string(),
        mime_type: mime_type.map(|s| s.to_string()),
    })
}

/// Helper to create blob resource content
pub fn to_blob_resource_content(
    uri: &str,
    data: &[u8],
    mime_type: Option<&str>,
) -> ResourceContentType {
    ResourceContentType::Blob(BlobResourceContents {
        uri: uri.to_string(),
        blob: BASE64_STANDARD.encode(data),
        mime_type: mime_type.map(|s| s.to_string()),
    })
}
