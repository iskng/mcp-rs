use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal;

use mcp_rs::protocol::{ Error, Content, TextContent, PromptMessage, Role };
use mcp_rs::server::services::prompts::{ Prompt, PromptArgument };
use mcp_rs::server::transport::stdio::StdioTransport;
use mcp_rs::server::Server;
use mcp_rs::prompt;

/// A simple prompt that provides a greeting
struct GreetingPrompt;

#[async_trait]
impl Prompt for GreetingPrompt {
    async fn render(
        &self,
        arguments: Option<HashMap<String, Value>>
    ) -> Result<Vec<PromptMessage>, Error> {
        let name = match &arguments {
            Some(args) => {
                match args.get("name") {
                    Some(name_value) => {
                        match name_value.as_str() {
                            Some(name_str) => name_str,
                            None => "friend",
                        }
                    }
                    None => "friend",
                }
            }
            None => "friend",
        };

        Ok(
            vec![
                PromptMessage {
                    role: Role::User,
                    content: Content::Text(TextContent {
                        type_field: "text".to_string(),
                        text: format!("Hello! My name is {}.", name),
                        annotations: None,
                    }),
                },
                PromptMessage {
                    role: Role::Assistant,
                    content: Content::Text(TextContent {
                        type_field: "text".to_string(),
                        text: format!("Nice to meet you, {}! How can I help you today?", name),
                        annotations: None,
                    }),
                }
            ]
        )
    }

    fn get_name(&self) -> &str {
        "greeting"
    }

    fn get_description(&self) -> Option<&str> {
        Some("A prompt that greets the user by name")
    }

    fn get_arguments(&self) -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "name".to_string(),
            description: Some("The name of the user to greet".to_string()),
            required: false,
        }]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a server with a StdioTransport
    let _server = Server::builder()
        .with_server_name("Prompt Example Server")
        .with_transport(StdioTransport::new())
        // Register our greeting prompt during build
        .register_prompt(Arc::new(GreetingPrompt))
        // Register another prompt using the macro
        .register_prompt(
            prompt!("help", "A prompt that provides help information", vec![], |_args| async move {
                Ok(
                    vec![
                        PromptMessage {
                            role: Role::User,
                            content: Content::Text(TextContent {
                                type_field: "text".to_string(),
                                text: "Can you help me use this server?".to_string(),
                                annotations: None,
                            }),
                        },
                        PromptMessage {
                            role: Role::Assistant,
                            content: Content::Text(TextContent {
                                type_field: "text".to_string(),
                                text: "This server provides prompt templates. You can:\n\
                                  1. List available prompts with 'prompts/list'\n\
                                  2. Get a specific prompt with 'prompts/get'\n\
                                  Feel free to ask if you need more help!".to_string(),
                                annotations: None,
                            }),
                        }
                    ]
                )
            })
        )
        .build().await?;

    // Wait for ctrl-c signal
    signal::ctrl_c().await?;
    println!("Shutting down server...");

    Ok(())
}
