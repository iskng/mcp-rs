use crate::protocol::{ Content, PromptMessage, Role, TextContent };

/// Create a new prompt message with text content
pub fn create_text_prompt_message(role: Role, text: &str) -> PromptMessage {
    PromptMessage {
        role,
        content: Content::Text(TextContent {
            type_field: "text".to_string(),
            text: text.to_string(),
            annotations: None,
        }),
    }
}

/// Create a new user prompt message with text content
pub fn create_user_text_message(text: &str) -> PromptMessage {
    create_text_prompt_message(Role::User, text)
}

/// Create a new assistant prompt message with text content
pub fn create_assistant_text_message(text: &str) -> PromptMessage {
    create_text_prompt_message(Role::Assistant, text)
}
