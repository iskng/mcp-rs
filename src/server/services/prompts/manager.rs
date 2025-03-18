use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde_json::Value;
use tracing::{ debug, warn };

use crate::protocol::{ Error, PromptMessage };
use super::base::{ Prompt, PromptArgument };

/// Manager for prompt templates
pub struct PromptManager {
    /// Thread-safe collection of prompts
    prompts: RwLock<HashMap<String, Arc<dyn Prompt + Send + Sync>>>,

    /// Whether to log warnings when a duplicate prompt is registered
    warn_on_duplicate_prompts: bool,
}

impl PromptManager {
    /// Create a new prompt manager
    pub fn new(warn_on_duplicate_prompts: bool) -> Self {
        Self {
            prompts: RwLock::new(HashMap::new()),
            warn_on_duplicate_prompts,
        }
    }

    /// Add a prompt to the manager
    pub async fn add_prompt(&self, prompt: Arc<dyn Prompt + Send + Sync>) -> Result<(), Error> {
        let name = prompt.get_name().to_string();
        let mut prompts = self.prompts.write().await;

        if prompts.contains_key(&name) && self.warn_on_duplicate_prompts {
            warn!("Prompt already exists: {}", name);
            return Ok(());
        }

        prompts.insert(name, prompt.clone());
        debug!("Added prompt: {}", prompt.get_name());
        Ok(())
    }

    /// List all registered prompts
    pub async fn list_prompts(&self) -> Vec<Arc<dyn Prompt + Send + Sync>> {
        let prompts = self.prompts.read().await;
        prompts.values().cloned().collect()
    }

    /// Render a prompt by name with the provided arguments
    pub async fn render_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, Value>>
    ) -> Result<Vec<PromptMessage>, Error> {
        let prompts = self.prompts.read().await;
        let prompt = prompts.get(name).ok_or_else(|| Error::PromptNotFound(name.to_string()))?;

        // Validate required arguments
        let expected_args = prompt.get_arguments();
        if let Some(args) = &arguments {
            for arg in &expected_args {
                if arg.required && !args.contains_key(&arg.name) {
                    return Err(Error::MissingArgument(arg.name.clone()));
                }
            }
        } else if expected_args.iter().any(|arg| arg.required) {
            return Err(Error::MissingArguments);
        }

        // Render the prompt
        debug!("Rendering prompt: {}", name);
        prompt.render(arguments).await
    }

    /// Get a prompt by name
    pub async fn get_prompt(&self, name: &str) -> Option<Arc<dyn Prompt + Send + Sync>> {
        let prompts = self.prompts.read().await;
        prompts.get(name).cloned()
    }

    /// Create and register a simple prompt
    pub async fn register_simple_prompt(
        &self,
        name: &str,
        description: Option<&str>,
        arguments: Vec<PromptArgument>,
        messages: Vec<PromptMessage>
    ) -> Result<(), Error> {
        // Clone messages for ownership in the closure
        let messages_clone = messages.clone();
        let name_str = name.to_string();
        let desc_str = description.map(|s| s.to_string()).unwrap_or_default();

        // Create a struct that implements Prompt
        struct SimplePrompt {
            name: String,
            description: String,
            arguments: Vec<PromptArgument>,
            messages: Vec<PromptMessage>,
        }

        #[async_trait::async_trait]
        impl Prompt for SimplePrompt {
            async fn render(
                &self,
                _arguments: Option<HashMap<String, Value>>
            ) -> Result<Vec<PromptMessage>, Error> {
                Ok(self.messages.clone())
            }

            fn get_name(&self) -> &str {
                &self.name
            }

            fn get_description(&self) -> Option<&str> {
                Some(&self.description)
            }

            fn get_arguments(&self) -> Vec<PromptArgument> {
                self.arguments.clone()
            }
        }

        // Create and register the prompt
        let prompt = Arc::new(SimplePrompt {
            name: name_str,
            description: desc_str,
            arguments,
            messages: messages_clone,
        });

        self.add_prompt(prompt).await
    }
}
