use std::sync::Arc;
use std::time::Duration;

use mcp_rs::client::client::{ Client, ClientConfig };
use mcp_rs::client::clientsession::{ ClientSession, ClientSessionConfig };
use mcp_rs::client::transport::sse::SseTransport;
use mcp_rs::protocol::{ Error, Implementation, ServerCapabilities };
use tracing::Level;
use mcp_rs::client::transport::ConnectionStatus;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize logging

    let subscriber = tracing_subscriber::fmt::Subscriber
        ::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    println!("Starting MCP client example...");

    // Create an SSE transport
    let transport = SseTransport::new("http://localhost:8090").await?;

    // Create a client session
    let session = ClientSession::builder(Box::new(transport))
        .name("Example Client".to_string())
        .version("1.0.0".to_string())
        .build();

    // Initialize the session (performs capability negotiation)
    println!("Initializing session with server...");
    let init_result = session.initialize().await?;

    // Subscribe to connection status updates
    let mut status_rx = session.subscribe_status();

    // Spawn a task to monitor connection status changes
    tokio::spawn(async move {
        println!("Monitoring connection status...");
        while let Ok(status) = status_rx.recv().await {
            println!("Connection status changed: {:?}", status);
        }
    });

    // Display server information
    println!(
        "Connected to server: {} {}",
        init_result.server_info.name,
        init_result.server_info.version
    );

    println!("Negotiated protocol version: {}", init_result.protocol_version);

    // Show server capabilities
    print_capabilities(&init_result.capabilities);

    // List resources if supported
    if session.has_capability("resources").await {
        println!("\nListing resources...");
        match session.list_resources(None).await {
            Ok(resources) => {
                println!("Found {} resources:", resources.resources.len());
                for resource in resources.resources {
                    println!("  - {}: {}", resource.uri, resource.name);
                }
            }
            Err(e) => println!("Error listing resources: {}", e),
        }
    }

    // List tools if supported
    if session.has_capability("tools").await {
        println!("\nListing tools...");
        match session.list_tools().await {
            Ok(tools) => {
                println!("Found {} tools:", tools.tools.len());
                for tool in tools.tools {
                    println!("  - {}: {:?}", tool.name, tool.description);
                }
            }
            Err(e) => println!("Error listing tools: {}", e),
        }
    }

    // Subscribe to notifications
    println!("\nSubscribing to notifications...");
    let mut subscription = session.subscribe_all().await;

    // Start a task to handle notifications
    let notification_task = tokio::spawn(async move {
        println!("Waiting for notifications...");

        // Listen for 10 seconds
        let timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    println!("Notification listening timeout reached");
                    break;
                }
                Some(notification) = subscription.next() => {
                    println!("Received notification: {}", notification.method);
                }
                else => break,
            }
        }
    });

    // Wait for notification task to complete
    notification_task.await.unwrap();

    // Perform proper shutdown
    println!("\nShutting down session...");
    session.shutdown().await?;

    println!("Client example completed successfully");
    Ok(())
}

fn print_capabilities(capabilities: &ServerCapabilities) {
    println!("\nServer capabilities:");

    if let Some(resources) = &capabilities.resources {
        println!("  - Resources: ");
        if let Some(subscribe) = resources.subscribe {
            println!("    - Subscribe: {}", subscribe);
        }
        if let Some(list_changed) = resources.list_changed {
            println!("    - List Changed: {}", list_changed);
        }
    }

    if let Some(prompts) = &capabilities.prompts {
        println!("  - Prompts: ");
        if let Some(list_changed) = prompts.list_changed {
            println!("    - List Changed: {}", list_changed);
        }
    }

    if let Some(tools) = &capabilities.tools {
        println!("  - Tools: ");
        if let Some(list_changed) = tools.list_changed {
            println!("    - List Changed: {}", list_changed);
        }
    }

    if capabilities.logging.is_some() {
        println!("  - Logging: supported");
    }

    if let Some(experimental) = &capabilities.experimental {
        println!("  - Experimental: {:?}", experimental);
    }
}
