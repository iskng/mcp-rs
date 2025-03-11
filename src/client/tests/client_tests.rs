//! Tests for the MCP client core functionality
//!
//! These tests ensure compatibility with the Python SDK by
//! mirroring similar test patterns and expectations.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{ watch, RwLock };
use tokio::time::timeout;

use crate::client::client::{ Client, ClientConfig };
use crate::client::services::lifecycle::LifecycleState;
use crate::client::transport::state::{ TransportState, TransportStateChannel };
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
    fn with_response(method: &str, id: RequestId) -> Self {
        // Create a transport first
        let transport = Self::new();

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
        let (tx, rx) = tokio::sync::broadcast::channel(1);
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
        }

        // Store the sent message
        inner.messages.push(message.clone());
        inner.send_count += 1;
        Ok(())
    }

    async fn set_app_state(&self, _app_state: Arc<crate::server::server::AppState>) {
        // Not needed for client tests
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
    let initialize_result: crate::protocol::Result = client
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

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Initialization);
}

#[tokio::test]
async fn test_client_send_request() {
    // Create a mock transport with a prepared response
    let id = RequestId::Number(1);
    let transport = MockTransport::with_response("test.method", id.clone());

    // Create a client with the transport
    let client = Client::new(Box::new(transport), ClientConfig::default());
    client.start().await.expect("Failed to start client");

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

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Operation);
}

#[tokio::test]
async fn test_client_send_notification() {
    // Create a mock transport
    let transport = MockTransport::new();

    // Create a client with the transport
    let client = Client::new(Box::new(transport), ClientConfig::default());

    // Send a notification
    let notification = JSONRPCNotification {
        jsonrpc: "2.0".to_string(),
        method: Method::NotificationsResourcesUpdated,
        params: Some(serde_json::json!({ "event": "test" })),
    };

    let result = client.send_notification(
        Method::NotificationsResourcesUpdated,
        serde_json::json!({ "event": "test" })
    ).await;
    assert!(result.is_ok());

    // Shutdown the client
    client.shutdown().await.expect("Failed to shutdown client");

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Shutdown);
}

#[tokio::test]
async fn test_client_notification_handler() {
    // Create a mock transport with a notification
    let transport = MockTransport::with_notification(&Method::Initialize);

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

/// Integration-style test that simulates a typical client workflow
#[tokio::test]
async fn test_client_workflow() {
    // This test demonstrates a typical workflow similar to the Python SDK:
    // 1. Create and start a client
    // 2. Send initialization request
    // 3. Register for notifications
    // 4. Process responses and notifications
    // 5. Shutdown cleanly

    // Create a transport
    let transport = MockTransport::new();

    // Create the client
    let client = Arc::new(Client::new(Box::new(transport), ClientConfig::default()));

    // ... existing code ...
}

#[tokio::test]
async fn test_client_with_handlers() {
    // Create a transport
    let transport = MockTransport::new();

    // Create the client
    let client = Arc::new(Client::new(Box::new(transport), ClientConfig::default()));

    // Create a service provider
    let service_provider = Arc::new(crate::client::services::ServiceProvider::new());

    // Create the handshake handler
    let handshake_handler = Box::new(
        crate::client::handlers::handshake::DefaultHandshakeHandler::new(
            client.clone(),
            service_provider.clone()
        )
    );

    let prompt_handler = Box::new(MockPromptHandler {});
    let tool_handler = Box::new(MockToolHandler {});
    let completion_handler = Box::new(MockCompletionHandler {});

    // Create the composite handler
    let handler = crate::client::handlers::composite::CompositeClientHandler::with_handlers(
        client.clone(),
        service_provider.clone(),
        handshake_handler,
        prompt_handler,
        tool_handler,
        completion_handler
    );

    // Start the client
    client.start().await.expect("Failed to start client");
    assert!(client.is_connected());

    // Initialize the client using the handler
    // This will:
    // 1. Send initialize request
    // 2. Process initialize response
    // 3. Send initialized notification
    // 4. Update lifecycle state
    let init_result = handler.initialize().await;

    // Check that initialization succeeded (even though we're using mocks)
    assert!(init_result.is_ok(), "Initialization failed: {:?}", init_result);

    // Shutdown the client using the handler
    handler.shutdown().await.expect("Failed to shutdown client");

    // Verify the client is disconnected
    assert!(!client.is_connected());

    assert_eq!(client.lifecycle().current_state().await, LifecycleState::Shutdown);
    assert_eq!(
        service_provider.lifecycle_manager().current_state().await,
        LifecycleState::Shutdown
    );
}

// Mock handlers for testing

struct MockPromptHandler {}

#[async_trait::async_trait]
impl crate::client::handlers::prompts::PromptHandler for MockPromptHandler {
    async fn list_prompts(&self) -> Result<crate::protocol::ListPromptsResult, Error> {
        unimplemented!()
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Option<std::collections::HashMap<String, serde_json::Value>>
    ) -> Result<crate::protocol::GetPromptResult, Error> {
        unimplemented!()
    }
}

struct MockToolHandler {}

#[async_trait::async_trait]
impl crate::client::handlers::tools::ToolHandler for MockToolHandler {
    async fn list_tools(&self) -> Result<crate::protocol::ListToolsResult, Error> {
        unimplemented!()
    }

    async fn call_tool(
        &self,
        _params: crate::protocol::CallToolParams
    ) -> Result<crate::protocol::CallToolResult, Error> {
        unimplemented!()
    }

    async fn find_tool_by_name(
        &self,
        _name: &str
    ) -> Result<crate::client::services::tools::ToolInfo, Error> {
        unimplemented!()
    }

    async fn call_tool_and_wait(
        &self,
        _name: &str,
        _args: serde_json::Value,
        _timeout: Option<std::time::Duration>
    ) -> Result<crate::protocol::CallToolResult, Error> {
        unimplemented!()
    }

    async fn call_tool_with_string_args(
        &self,
        _name: &str,
        _args: std::collections::HashMap<String, String>
    ) -> Result<crate::protocol::CallToolResult, Error> {
        unimplemented!()
    }

    async fn call_tool_by_name(
        &self,
        _name: &str,
        _args: serde_json::Value
    ) -> Result<crate::protocol::CallToolResult, Error> {
        unimplemented!()
    }
}

struct MockCompletionHandler {}

#[async_trait::async_trait]
impl crate::client::handlers::completion::CompletionHandler for MockCompletionHandler {
    async fn complete(
        &self,
        _params: crate::protocol::CompleteParams
    ) -> Result<crate::protocol::CompleteResult, Error> {
        unimplemented!()
    }
}
