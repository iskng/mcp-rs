//! Tests for the MCP client core functionality
//!
//! These tests ensure compatibility with the Python SDK by
//! mirroring similar test patterns and expectations.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use crate::protocol::{
    JSONRPCMessage,
    JSONRPCRequest,
    JSONRPCResponse,
    JSONRPCNotification,
    RequestId,
    Error,
};
use crate::client::client::{ Client, ClientConfig };
use crate::transport::DirectIOTransport;
use crate::transport::BoxedDirectIOTransport;

/// Mock transport for testing
struct MockTransport {
    messages: Vec<JSONRPCMessage>,
    send_count: usize,
    receive_count: usize,
    connected: bool,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            send_count: 0,
            receive_count: 0,
            connected: true,
        }
    }

    fn with_response(method: &str, id: RequestId) -> Self {
        let mut transport = Self::new();
        let response = JSONRPCResponse {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            result: crate::protocol::Result {
                _meta: None,
                content: [("success".to_string(), serde_json::json!(true))]
                    .iter()
                    .cloned()
                    .collect(),
            },
        };

        transport.messages.push(JSONRPCMessage::Response(response));
        transport
    }

    fn with_notification(method: &str) -> Self {
        let mut transport = Self::new();
        let notification = JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(serde_json::json!({ "event": "test" })),
        };

        transport.messages.push(JSONRPCMessage::Notification(notification));
        transport
    }
}

#[async_trait::async_trait]
impl crate::transport::Transport for MockTransport {
    async fn start(&mut self) -> Result<(), Error> {
        self.connected = true;
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send_to(&mut self, client_id: &str, message: &JSONRPCMessage) -> Result<(), Error> {
        self.messages.push(message.clone());
        self.send_count += 1;
        Ok(())
    }

    async fn set_app_state(&mut self, app_state: Arc<crate::server::server::AppState>) {
        // Not needed for client tests
    }
}

#[async_trait::async_trait]
impl DirectIOTransport for MockTransport {
    async fn receive(&mut self) -> Result<(Option<String>, JSONRPCMessage), Error> {
        if self.receive_count < self.messages.len() {
            let message = self.messages[self.receive_count].clone();
            self.receive_count += 1;
            Ok((None, message))
        } else {
            // Wait for a bit to simulate blocking
            tokio::time::sleep(Duration::from_millis(100)).await;
            Err(Error::Transport("No more messages".to_string()))
        }
    }

    async fn send(&mut self, message: &JSONRPCMessage) -> Result<(), Error> {
        self.messages.push(message.clone());
        self.send_count += 1;
        Ok(())
    }
}

#[tokio::test]
async fn test_client_initialization() {
    // Create a mock transport
    let transport = MockTransport::new();

    // Create client with default config
    let client = Client::new(transport, ClientConfig::default());

    // Client should not be connected until started
    assert!(!client.is_connected().await);

    // Start the client
    client.start().await.expect("Failed to start client");

    // After starting, client should be connected
    assert!(client.is_connected().await);

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");

    // After shutting down, client should be disconnected
    assert!(!client.is_connected().await);
}

#[tokio::test]
async fn test_client_send_request() {
    // Create a mock transport with a prepared response
    let id = RequestId::Number(1);
    let transport = MockTransport::with_response("test", id.clone());
    let boxed_transport = BoxedDirectIOTransport(Box::new(transport));

    // Create and start client
    let client = Client::new(boxed_transport, ClientConfig::default());
    client.start().await.expect("Failed to start client");

    // Send a request
    let request = JSONRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: id.clone(),
        method: "test".to_string(),
        params: Some(serde_json::json!({})),
    };

    // Use a timeout to ensure we don't hang
    let result = timeout(
        Duration::from_secs(1),
        client.send_raw_message(JSONRPCMessage::Request(request))
    ).await;

    // Request should complete successfully
    assert!(result.is_ok());

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");
}

#[tokio::test]
async fn test_client_send_notification() {
    // Create a mock transport
    let transport = MockTransport::new();
    let boxed_transport = BoxedDirectIOTransport(Box::new(transport));

    // Create and start client
    let client = Client::new(boxed_transport, ClientConfig::default());
    client.start().await.expect("Failed to start client");

    // Send a notification
    let notification = JSONRPCNotification {
        jsonrpc: "2.0".to_string(),
        method: "test/notification".to_string(),
        params: Some(serde_json::json!({ "event": "test" })),
    };

    let result = client.send_notification(
        "test/notification",
        serde_json::json!({ "event": "test" })
    ).await;
    assert!(result.is_ok());

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");
}

#[tokio::test]
async fn test_client_notification_handler() {
    // Create a mock transport with a notification
    let transport = MockTransport::with_notification("test/notification");
    let boxed_transport = BoxedDirectIOTransport(Box::new(transport));

    // Create and start client
    let client = Client::new(boxed_transport, ClientConfig::default());

    // Create a notification receiver channel
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let tx_clone = tx.clone();

    // Register a notification handler
    client
        .register_notification_handler("test/notification", move |notification| {
            let tx = tx_clone.clone();
            async move {
                let _ = tx.send(notification).await;
                Ok(())
            }
        }).await
        .expect("Failed to register handler");

    // Start the client to process messages
    client.start().await.expect("Failed to start client");

    // Wait for notification
    let received = timeout(Duration::from_secs(1), rx.recv()).await;
    assert!(received.is_ok());

    if let Ok(Some(notification)) = received {
        assert_eq!(notification.method, "test/notification");
    } else {
        panic!("Did not receive notification");
    }

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");
}

/// Integration-style test that simulates a typical client workflow
#[tokio::test]
async fn test_client_workflow() {
    // This test demonstrates a typical workflow similar to the Python SDK:
    // 1. Create and start a client
    // 2. Send initialization request
    // 3. Register for notifications
    // 4. Process responses and notifications
    // 5. Shutdown cleanly

    // TODO: Implement this test when client session is more complete
}
