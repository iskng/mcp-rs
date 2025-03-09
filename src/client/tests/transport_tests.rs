//! Tests for transport implementations
//!
//! These tests ensure our transport implementations are compatible
//! with the Python SDK by mirroring similar test patterns.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::protocol::{
    JSONRPCMessage,
    JSONRPCRequest,
    JSONRPCResponse,
    JSONRPCNotification,
    RequestId,
    Error,
};
use crate::transport::{ DirectIOTransport, Transport };

// Tests for transport behavior
// We won't test specific transport implementations (SSE, WebSocket, etc.)
// as they require external connections/systems, but we'll test the
// expected behavior of transports to ensure our transport trait
// matches the Python SDK's expectations.

/// Test interface compatibility with Python SDK's transport abstraction
#[test]
fn test_transport_interface() {
    // The Python SDK expects these methods to be available:
    // - connect() / disconnect()
    // - send(message) -> void
    // - recv() -> message
    // - is_connected() -> bool

    // Our Transport trait provides:
    // - start() / close() (equivalents to connect/disconnect)
    // - send(message) -> Result<(), Error>
    // - receive() -> Result<(Option<String>, JSONRPCMessage), Error>
    // - is_connected() -> bool

    // The main difference is that:
    // 1. Our methods return Results with proper error handling
    // 2. Our receive() includes a client_id for server transports

    // This test doesn't actually test functionality, it's just to document
    // that we've designed our transport API to be compatible with
    // the Python SDK's expectations.

    // No assertions needed as this is a documentation test
}

/// Test for transport message serialization/deserialization compatibility
#[test]
fn test_transport_message_format() {
    // Create a JSON-RPC request
    let request = JSONRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "test".to_string(),
        params: Some(serde_json::json!({ "test": "value" })),
    };

    // Convert to a JSON-RPC message
    let message = JSONRPCMessage::Request(request);

    // Serialize to JSON (this is what we send over transports)
    let json_str = serde_json::to_string(&message).unwrap();

    // This should match the format used by the Python SDK:
    // {"jsonrpc": "2.0", "id": 1, "method": "test", "params": {"test": "value"}}

    // Parse back to validate
    let parsed: JSONRPCMessage = serde_json::from_str(&json_str).unwrap();

    // We should be able to round-trip serialize/deserialize
    match parsed {
        JSONRPCMessage::Request(req) => {
            assert_eq!(req.method, "test");
            match req.id {
                RequestId::Number(num) => assert_eq!(num, 1),
                _ => panic!("Expected number request ID"),
            }

            // Check params
            if let Some(params) = req.params {
                assert_eq!(params.get("test").unwrap(), "value");
            } else {
                panic!("Expected params");
            }
        }
        _ => panic!("Expected request"),
    }
}

/// Test for notification message format
#[test]
fn test_notification_message_format() {
    // Create a JSON-RPC notification
    let notification = JSONRPCNotification {
        jsonrpc: "2.0".to_string(),
        method: "test/notification".to_string(),
        params: Some(serde_json::json!({ "event": "test" })),
    };

    // Convert to a JSON-RPC message
    let message = JSONRPCMessage::Notification(notification);

    // Serialize to JSON (this is what we send over transports)
    let json_str = serde_json::to_string(&message).unwrap();

    // This should match the format used by the Python SDK:
    // {"jsonrpc": "2.0", "method": "test/notification", "params": {"event": "test"}}

    // Parse back to validate
    let parsed: JSONRPCMessage = serde_json::from_str(&json_str).unwrap();

    // We should be able to round-trip serialize/deserialize
    match parsed {
        JSONRPCMessage::Notification(notif) => {
            assert_eq!(notif.method, "test/notification");

            // Check params
            if let Some(params) = notif.params {
                assert_eq!(params.get("event").unwrap(), "test");
            } else {
                panic!("Expected params");
            }
        }
        _ => panic!("Expected notification"),
    }
}

/// Test for response message format
#[test]
fn test_response_message_format() {
    // Create a JSON-RPC response
    let response = JSONRPCResponse {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        result: crate::protocol::Result {
            _meta: None,
            content: [("result".to_string(), serde_json::json!("success"))]
                .iter()
                .cloned()
                .collect(),
        },
    };

    // Convert to a JSON-RPC message
    let message = JSONRPCMessage::Response(response);

    // Serialize to JSON (this is what we receive over transports)
    let json_str = serde_json::to_string(&message).unwrap();

    // This should match the format used by the Python SDK:
    // {"jsonrpc": "2.0", "id": 1, "result": {"result": "success"}}

    // Parse back to validate
    let parsed: JSONRPCMessage = serde_json::from_str(&json_str).unwrap();

    // We should be able to round-trip serialize/deserialize
    match parsed {
        JSONRPCMessage::Response(resp) => {
            match resp.id {
                RequestId::Number(num) => assert_eq!(num, 1),
                _ => panic!("Expected number request ID"),
            }

            // Check result
            let result = resp.result.content.get("result").unwrap();
            assert_eq!(result, "success");
        }
        _ => panic!("Expected response"),
    }
}
