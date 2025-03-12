use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

use mcp_rs::protocol::{ Error, PromptMessage, Role, Content, TextContent };
use mcp_rs::server::services::prompts::{
    Prompt,
    PromptArgument,
    PromptManager,
    create_assistant_text_message,
    create_user_text_message,
};
use mcp_rs::prompt;

struct GreetingPrompt;

#[async_trait]
impl Prompt for GreetingPrompt {
    async fn render(
        &self,
        arguments: Option<HashMap<String, Value>>
    ) -> Result<Vec<PromptMessage>, Error> {
        match arguments {
            Some(args) => {
                if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
                    Ok(
                        vec![
                            create_user_text_message("Hello, I'd like to be greeted."),
                            create_assistant_text_message(
                                &format!("Hello, {}! Welcome to MCP.", name)
                            )
                        ]
                    )
                } else {
                    Err(Error::MissingArgument("name".to_string()))
                }
            }
            None => Err(Error::MissingArguments),
        }
    }

    fn get_name(&self) -> &str {
        "greeting"
    }

    fn get_description(&self) -> Option<&str> {
        Some("A prompt that greets a user by name")
    }

    fn get_arguments(&self) -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "name".to_string(),
            description: Some("The name to greet".to_string()),
            required: true,
        }]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a prompt manager
    let manager = PromptManager::new(true);

    // Add the greeting prompt
    let greeting_prompt = Arc::new(GreetingPrompt);
    manager.add_prompt(greeting_prompt).await?;

    // Create another prompt using the macro
    let weather_prompt = prompt!(
        "weather",
        "A prompt that discusses the weather at a location",
        vec![PromptArgument {
            name: "location".to_string(),
            description: Some("The location to check weather for".to_string()),
            required: true,
        }],
        |arguments: Option<HashMap<String, Value>>| async move {
            let args = arguments.ok_or(Error::MissingArguments)?;
            let location = args
                .get("location")
                .ok_or_else(|| Error::MissingArgument("location".to_string()))?
                .as_str()
                .ok_or_else(|| Error::InvalidParams("location must be a string".to_string()))?;

            Ok(
                vec![
                    PromptMessage {
                        role: Role::User,
                        content: Content::Text(TextContent {
                            type_field: "text".to_string(),
                            text: format!("What's the weather like in {}?", location),
                            annotations: None,
                        }),
                    },
                    PromptMessage {
                        role: Role::Assistant,
                        content: Content::Text(TextContent {
                            type_field: "text".to_string(),
                            text: format!("I don't have real-time weather data for {}, but I can help you find that information.", location),
                            annotations: None,
                        }),
                    }
                ]
            )
        }
    );

    manager.add_prompt(weather_prompt).await?;

    // Create a simple static prompt using the new register_simple_prompt method
    manager.register_simple_prompt(
        "help",
        Some("A simple help prompt with static messages"),
        vec![],
        vec![
            create_user_text_message("Can you help me with this application?"),
            create_assistant_text_message(
                "Of course! This is an example MCP server that shows how to use the Prompt Manager. You can request different prompt templates to see how they work."
            )
        ]
    ).await?;

    // List all prompts
    let prompts = manager.list_prompts().await;
    println!("Registered prompts:");
    for prompt in prompts {
        println!(
            "- {}: {}",
            prompt.get_name(),
            prompt.get_description().unwrap_or("No description")
        );
        println!("  Arguments:");
        for arg in prompt.get_arguments() {
            println!(
                "  - {}{}: {}",
                arg.name,
                if arg.required {
                    " (required)"
                } else {
                    ""
                },
                arg.description.unwrap_or_else(|| "No description".to_string())
            );
        }
    }

    // Render the greeting prompt
    let mut args = HashMap::new();
    args.insert("name".to_string(), Value::String("Alice".to_string()));

    let messages = manager.render_prompt("greeting", Some(args)).await?;
    println!("\nRendered greeting prompt:");
    for message in messages {
        match message.content {
            Content::Text(text) => {
                println!("{:?}: {}", message.role, text.text);
            }
            _ => println!("Non-text message from {:?}", message.role),
        }
    }

    // Render the weather prompt
    let mut args = HashMap::new();
    args.insert("location".to_string(), Value::String("San Francisco".to_string()));

    let messages = manager.render_prompt("weather", Some(args)).await?;
    println!("\nRendered weather prompt:");
    for message in messages {
        match message.content {
            Content::Text(text) => {
                println!("{:?}: {}", message.role, text.text);
            }
            _ => println!("Non-text message from {:?}", message.role),
        }
    }

    // Render the help prompt (no arguments needed)
    let messages = manager.render_prompt("help", None).await?;
    println!("\nRendered help prompt:");
    for message in messages {
        match message.content {
            Content::Text(text) => {
                println!("{:?}: {}", message.role, text.text);
            }
            _ => println!("Non-text message from {:?}", message.role),
        }
    }

    // Try to render a non-existent prompt
    let result = manager.render_prompt("non_existent", None).await;
    println!("\nAttempted to render non-existent prompt:");
    println!("Result: {:?}", result);

    // Try to render with missing arguments
    let result = manager.render_prompt("greeting", None).await;
    println!("\nAttempted to render prompt with missing arguments:");
    println!("Result: {:?}", result);

    Ok(())
}
