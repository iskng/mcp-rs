use crate::protocol::Error;
use crate::protocol::{
    CallToolParams,
    Cursor,
    GetPromptParams,
    ImageContent,
    InitializeParams,
    JSONRPCMessage,
    JSONRPCRequest,
    JSONRPCResponse,
    ListToolsResult,
    PaginatedRequestParams,
    ReadResourceParams,
    RequestId,
    Result as ProtocolResult,
    Tool,
};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::Value;
use std::collections::HashMap;

use super::response_from_typed;

/// Create a success response with the given result
pub fn create_success_response(id: RequestId, result: ProtocolResult) -> JSONRPCResponse {
    JSONRPCResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
    }
}

/// Create an empty success result
pub fn empty_result() -> ProtocolResult {
    ProtocolResult {
        _meta: None,
        content: HashMap::new(),
    }
}

/// Helper to convert from old i32 request ID to new RequestId enum
pub fn id_to_request_id(id: i32) -> RequestId {
    RequestId::Number(id as i64)
}

/// Helper to extract a numeric ID from a RequestId, defaulting to 0 if not a number
pub fn request_id_to_i32(id: &RequestId) -> i32 {
    match id {
        RequestId::Number(n) => *n as i32,
        RequestId::String(_) => 0,
    }
}

/// Create tools result with pagination
pub fn create_tools_result(
    tools: Vec<crate::protocol::Tool>,
    next_cursor: Option<String>
) -> ListToolsResult {
    ListToolsResult {
        tools,
        next_cursor: next_cursor.map(|s| crate::protocol::Cursor(s)),
        _meta: None,
    }
}

/// Helper function to extract the method from a JSONRPCRequest
pub fn extract_method(request: &JSONRPCRequest) -> &str {
    &request.method
}

/// Helper function to map a request to its type
pub fn map_request_to_type(request: &JSONRPCRequest) -> String {
    let method = extract_method(request);
    method.to_string()
}

/// Helper function to create a message for a tool list result
pub fn create_list_tools_message(
    id: RequestId,
    tools: Vec<Tool>,
    next_cursor: Option<String>
) -> JSONRPCMessage {
    let result = create_tools_result(tools, next_cursor);
    response_from_typed(id, result)
}

/// Helper to check if a method matches
pub fn method_matches(request: &JSONRPCRequest, method_name: &str) -> bool {
    request.method == method_name
}

/// Helper to convert JSON value to HashMap, or create a new HashMap if not an object
pub fn value_to_hashmap(value: &Value) -> HashMap<String, Value> {
    match value {
        Value::Object(map) => {
            let mut content = HashMap::new();
            for (k, v) in map {
                content.insert(k.clone(), v.clone());
            }
            content
        }
        _ => HashMap::new(),
    }
}

/// Helper to get a value from the HashMap or a default
pub fn get_value_or_default<T>(map: &HashMap<String, Value>, key: &str, default: T) -> T
    where T: serde::de::DeserializeOwned
{
    map.get(key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(default)
}

/// Combine multiple JSON objects into a single HashMap
pub fn combine_objects(values: &[Value]) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    for value in values {
        if let Value::Object(map) = value {
            for (k, v) in map {
                result.insert(k.clone(), v.clone());
            }
        }
    }
    result
}

/// Function to parse a ListToolsRequest and extract params
pub fn parse_list_tools_request(request: &JSONRPCRequest) -> Result<PaginatedRequestParams, Error> {
    // Extract the params object if present
    if let Some(params_value) = &request.params {
        if let Ok(params) = serde_json::from_value::<HashMap<String, Value>>(params_value.clone()) {
            let cursor = params.get("cursor").and_then(|v| {
                if v.is_string() { v.as_str().map(|s| s.to_string()) } else { None }
            });

            // Create a ListToolsParams with the cursor
            return Ok(PaginatedRequestParams {
                cursor: cursor.map(Cursor),
            });
        }
    }

    // Default to empty params
    Ok(PaginatedRequestParams { cursor: None })
}

/// Function to parse a CallToolRequest and extract params
pub fn parse_call_tool_request(request: &JSONRPCRequest) -> Result<CallToolParams, Error> {
    // Extract the params object if present
    if let Some(params_value) = &request.params {
        if let Ok(params) = serde_json::from_value::<CallToolParams>(params_value.clone()) {
            return Ok(params);
        } else {
            return Err(Error::InvalidParams("Invalid tool call parameters".to_string()));
        }
    }

    // No params provided
    Err(Error::InvalidParams("No parameters provided for tool call".to_string()))
}

/// Function to parse a ReadResourceRequest and extract params
pub fn parse_read_resource_request(request: &JSONRPCRequest) -> Result<ReadResourceParams, Error> {
    // Extract the params object if present
    if let Some(params_value) = &request.params {
        if let Ok(params) = serde_json::from_value::<ReadResourceParams>(params_value.clone()) {
            return Ok(params);
        } else {
            return Err(Error::InvalidParams("Invalid resource read parameters".to_string()));
        }
    }

    // No params provided
    Err(Error::InvalidParams("No parameters provided for resource read".to_string()))
}

/// Function to parse a ListPromptsRequest and extract params
pub fn parse_list_prompts_request(
    request: &JSONRPCRequest
) -> Result<PaginatedRequestParams, Error> {
    // Same as ListToolsRequest
    parse_list_tools_request(request)
}

/// Function to parse a GetPromptRequest and extract params
pub fn parse_get_prompt_request(request: &JSONRPCRequest) -> Result<GetPromptParams, Error> {
    // Extract the params object if present
    if let Some(params_value) = &request.params {
        if let Ok(params) = serde_json::from_value::<GetPromptParams>(params_value.clone()) {
            return Ok(params);
        } else {
            return Err(Error::InvalidParams("Invalid prompt parameters".to_string()));
        }
    }

    // No params provided
    Err(Error::InvalidParams("No parameters provided for get prompt".to_string()))
}

/// Function to parse a ListResourcesRequest and extract params
pub fn parse_list_resources_request(
    request: &JSONRPCRequest
) -> Result<PaginatedRequestParams, Error> {
    // Same as ListToolsRequest
    parse_list_tools_request(request)
}

/// Function to parse a InitializeRequest and extract params
pub fn parse_initialize_request(request: &JSONRPCRequest) -> Result<InitializeParams, Error> {
    // Extract the params object if present
    if let Some(params_value) = &request.params {
        if let Ok(params) = serde_json::from_value::<InitializeParams>(params_value.clone()) {
            return Ok(params);
        } else {
            return Err(Error::InvalidParams("Invalid initialize parameters".to_string()));
        }
    }

    // No params provided
    Err(Error::InvalidParams("No parameters provided for initialize".to_string()))
}
