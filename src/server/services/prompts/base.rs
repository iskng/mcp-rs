use async_trait::async_trait;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use schemars::JsonSchema;
use std::collections::HashMap;
use crate::protocol::{ Error, PromptMessage };

#[async_trait]
pub trait Prompt: Send + Sync {
    /// Renders the prompt with the provided arguments
    async fn render(
        &self,
        arguments: Option<HashMap<String, Value>>
    ) -> Result<Vec<PromptMessage>, Error>;

    /// Returns the name of the prompt
    fn get_name(&self) -> &str;

    /// Returns an optional description of the prompt
    fn get_description(&self) -> Option<&str>;

    /// Returns a list of arguments expected by the prompt
    fn get_arguments(&self) -> Vec<PromptArgument>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptArgument {
    /// Name of the argument
    pub name: String,

    /// Optional description of the argument
    pub description: Option<String>,

    /// Whether the argument is required
    pub required: bool,
}
