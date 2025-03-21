//! Tests for the MCP client core functionality
//!
//! These tests ensure compatibility with the Python SDK by
//! mirroring similar test patterns and expectations.

use std::time::Duration;
use tokio::sync::{ watch, RwLock };
use tokio::time::timeout;

use crate::client::client::{ Client, ClientConfig };
use crate::client::services::lifecycle::LifecycleState;
use crate::client::transport::state::{ TransportState, TransportStateChannel };
// use crate::client::handlers::handshake::HandshakeHandler;
use crate::protocol::{
    Error,
    JSONRPCMessage,
    JSONRPCNotification,
    JSONRPCRequest,
    JSONRPCResponse,
    Method,
    RequestId,
};

/// Mock transport for testing
struct MockTransport {
    inner: RwLock<MockTransportInner>,
    state: TransportStateChannel,
}

struct MockTransportInner {
    messages: Vec<JSONRPCMessage>,
    send_count: usize,
    receive_count: usize,
    connected: bool,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            inner: RwLock::new(MockTransportInner {
                messages: Vec::new(),
                send_count: 0,
                receive_count: 0,
                connected: false,
            }),
            state: TransportStateChannel::new(),
        }
    }

    /// Create a new transport with a prefilled response
    fn with_response(_method: &str, id: RequestId) -> Self {
        // Create a transport first
        let _transport = Self::new();

        // Create the response
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

        // Initialize the messages in the constructor directly
        let mut initial_messages = Vec::new();
        initial_messages.push(JSONRPCMessage::Response(response));

        // Create a new transport with the initialized messages
        Self {
            inner: RwLock::new(MockTransportInner {
                messages: initial_messages,
                send_count: 0,
                receive_count: 0,
                connected: false,
            }),
            state: TransportStateChannel::new(),
        }
    }

    /// Create a new transport with a prefilled notification
    fn with_notification(method: &Method) -> Self {
        // Create the notification
        let notification = JSONRPCNotification {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: Some(serde_json::json!({ "event": "test" })),
        };

        // Initialize the messages in the constructor directly
        let mut initial_messages = Vec::new();
        initial_messages.push(JSONRPCMessage::Notification(notification));

        // Create a new transport with the initialized messages
        Self {
            inner: RwLock::new(MockTransportInner {
                messages: initial_messages,
                send_count: 0,
                receive_count: 0,
                connected: false,
            }),
            state: TransportStateChannel::new(),
        }
    }
}

#[async_trait::async_trait]
impl crate::client::transport::Transport for MockTransport {
    async fn start(&self) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.connected = true;

        // Update the state
        self.state.update(|s| {
            s.has_endpoint = true;
            s.has_connected = true;
            s.endpoint_url = Some("mock://test/endpoint".to_string());
            s.session_id = Some("mock-session-id".to_string());
        });

        Ok(())
    }

    async fn close(&self) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.connected = false;

        // Update the state
        self.state.reset();

        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Use the value from the state channel directly
        self.state.current().has_connected && self.state.current().has_endpoint
    }

    fn subscribe_state(&self) -> watch::Receiver<TransportState> {
        self.state.receiver()
    }

    fn subscribe_status(
        &self
    ) -> tokio::sync::broadcast::Receiver<crate::client::transport::ConnectionStatus> {
        // Create a new channel each time since we don't need to track subscribers in tests
        let (_tx, rx) = tokio::sync::broadcast::channel(1);
        rx
    }

    async fn send(&self, message: &JSONRPCMessage) -> Result<(), Error> {
        let mut inner = self.inner.write().await;

        if let JSONRPCMessage::Request(request) = message {
            // Auto-respond to initialize requests for testing lifecycle
            if request.method == Method::Initialize {
                // Create and queue an initialize response
                let response = JSONRPCResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.clone(),
                    result: crate::protocol::Result {
                        _meta: None,
                        content: [
                            ("protocolVersion".to_string(), serde_json::json!("2024-11-05")),
                            (
                                "serverInfo".to_string(),
                                serde_json::json!({
                                "name": "MockServer",
                                "version": "1.0.0"
                            }),
                            ),
                            ("capabilities".to_string(), serde_json::json!({})),
                        ]
                            .iter()
                            .cloned()
                            .collect(),
                    },
                };
                inner.messages.push(JSONRPCMessage::Response(response));
            }

            // Store the sent request for potential response
            inner.messages.push(message.clone());
        }
        // For other message types (notifications, etc.), we don't add them back to the queue
        // to avoid recursive loops where the client receives its own sent notifications

        inner.send_count += 1;
        Ok(())
    }

    async fn receive(&self) -> Result<(Option<String>, JSONRPCMessage), Error> {
        // Get inner state under a lock
        let mut inner = self.inner.write().await;

        if inner.receive_count < inner.messages.len() {
            let message = inner.messages[inner.receive_count].clone();
            inner.receive_count += 1;
            Ok((None, message))
        } else {
            // Drop the lock before sleeping to avoid holding it across an await point
            drop(inner);

            // Wait for a bit to simulate blocking
            tokio::time::sleep(Duration::from_millis(100)).await;
            Err(Error::Transport("No more messages".to_string()))
        }
    }
}

#[tokio::test]
async fn test_client_initialization() {
    // Create a mock transport that will respond to initialize requests
    let transport = MockTransport::new();

    // Create client with default config
    let client = Client::new(Box::new(transport), ClientConfig::default());

    // Client should not be connected until started
    assert!(!client.is_connected());

    // Start the client
    client.start().await.expect("Failed to start client");

    // After starting, client should be connected
    assert!(client.is_connected());

    // Send initialize request - this should transition to Initializing state
    let _initialize_result: crate::protocol::Result = client
        .send_request(
            Method::Initialize,
            serde_json::json!({
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "TestClient",
                "version": "1.0.0"
            },
            "capabilities": {}
        })
        ).await
        .expect("Initialize request failed");

    // Send initialized notification - this should transition to Ready state
    client
        .send_notification(Method::NotificationsInitialized, serde_json::json!(null)).await
        .expect("Failed to send initialized notification");

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");

    // After shutting down, client should be disconnected
    assert!(!client.is_connected());

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Shutdown);
}

#[tokio::test]
async fn test_client_send_request() {
    // Create a mock transport with a prepared response
    let id = RequestId::Number(1);
    let transport = MockTransport::with_response("test.method", id.clone());

    // Create a client with the transport
    let client = Client::new(Box::new(transport), ClientConfig::default());
    client.start().await.expect("Failed to start client");

    // Manually initialize the client lifecycle state to Operation
    // This is needed because we can only transition from Operation to Shutdown
    client
        .lifecycle()
        .transition_to(LifecycleState::Operation).await
        .expect("Failed to transition state");

    // Send a request
    let request = JSONRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: id.clone(),
        method: Method::Initialize,
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

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Shutdown);
}

#[tokio::test]
async fn test_client_send_notification() {
    // Create a mock transport
    let transport = MockTransport::new();

    // Create a client with the transport
    let client = Client::new(Box::new(transport), ClientConfig::default());

    // Start the client
    client.start().await.expect("Failed to start client");

    // Manually initialize the client lifecycle state to Operation
    // This is a test-only shortcut to avoid the full initialization process
    client
        .lifecycle()
        .transition_to(LifecycleState::Operation).await
        .expect("Failed to transition state");

    // Now send the notification without going through the initialization flow
    let result = client.send_notification(
        Method::NotificationsResourcesUpdated,
        serde_json::json!({ "event": "test" })
    ).await;

    // Check result
    assert!(result.is_ok(), "Expected ok result, got {:?}", result);

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");

    // Validate final state
    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Shutdown);
}

#[tokio::test]
async fn test_client_notification_handler() {
    // Create a mock transport with a notification for resources updated (not Initialize)
    let transport = MockTransport::with_notification(&Method::NotificationsResourcesUpdated);

    // Create a client with the transport
    let client = Client::new(Box::new(transport), ClientConfig::default());

    // Create a notification receiver channel
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let tx_clone = tx.clone();

    // Register a notification handler
    client
        .register_notification_handler(Method::NotificationsResourcesUpdated, move |notification| {
            let tx = tx_clone.clone();
            async move {
                let _ = tx.send(notification).await;
                Ok(())
            }
        }).await
        .expect("Failed to register handler");

    // Start the client to process messages
    client.start().await.expect("Failed to start client");

    // Manually initialize the client lifecycle state to Operation
    // This is needed because we can only transition from Operation to Shutdown
    client
        .lifecycle()
        .transition_to(LifecycleState::Operation).await
        .expect("Failed to transition state");

    // Wait for notification
    let received = timeout(Duration::from_secs(1), rx.recv()).await;
    assert!(received.is_ok());

    if let Ok(Some(notification)) = received {
        assert_eq!(notification.method, Method::NotificationsResourcesUpdated);
    } else {
        panic!("Did not receive notification");
    }

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Shutdown);
}
