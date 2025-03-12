pub mod base;
pub mod manager;
pub mod utils;

pub use base::{ Prompt, PromptArgument };
pub use manager::PromptManager;
pub use utils::{
    create_text_prompt_message,
    create_user_text_message,
    create_assistant_text_message,
};

/// Macro for creating a prompt
///
/// # Example
/// ```
/// let greet_prompt = prompt!(
///     "greet",
///     "Greets a user",
///     vec![PromptArgument {
///         name: "name".to_string(),
///         description: Some("The name to greet".to_string()),
///         required: true,
///     }],
///     |args| async move {
///         let name = args.unwrap().get("name").unwrap().as_str().unwrap();
///         Ok(vec![create_assistant_text_message(&format!("Hello, {}!", name))])
///     }
/// );
/// ```
#[macro_export]
macro_rules! prompt {
    ($name:expr, $description:expr, $args:expr, $render:expr) => {
        {
            struct GeneratedPrompt;
            #[async_trait::async_trait]
            impl $crate::server::services::prompts::Prompt for GeneratedPrompt {
                async fn render(
                    &self, 
                    arguments: Option<std::collections::HashMap<String, serde_json::Value>>
                ) -> Result<Vec<$crate::protocol::PromptMessage>, $crate::protocol::Error> {
                    ($render)(arguments).await
                }
                
                fn get_name(&self) -> &str { $name }
                
                fn get_description(&self) -> Option<&str> { Some($description) }
                
                fn get_arguments(&self) -> Vec<$crate::server::services::prompts::PromptArgument> { $args }
            }
            std::sync::Arc::new(GeneratedPrompt)
        }
    };
}
