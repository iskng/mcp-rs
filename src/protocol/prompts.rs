use base64::{Engine, prelude::BASE64_STANDARD};

use crate::protocol::{
    Content, ImageContent, Prompt, PromptArgument, PromptMessage, PromptReference, Role,
    TextContent,
};
use std::collections::HashMap;

/// Helper to create a prompt
pub fn create_prompt(
    name: &str,
    description: Option<&str>,
    arguments: Option<Vec<PromptArgument>>,
) -> Prompt {
    Prompt {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        arguments,
    }
}

/// Helper to create a prompt argument
pub fn create_prompt_argument(
    name: &str,
    description: Option<&str>,
    required: Option<bool>,
) -> PromptArgument {
    PromptArgument {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        required,
    }
}

/// Helper to create a prompt reference
pub fn create_prompt_reference(name: &str) -> PromptReference {
    PromptReference {
        type_field: "ref/prompt".to_string(),
        name: name.to_string(),
    }
}

/// Helper to create a prompt message
pub fn create_prompt_message(role: Role, content: Content) -> PromptMessage {
    PromptMessage { role, content }
}

/// Helper to create a user prompt message with text
pub fn create_user_text_message(text: &str) -> PromptMessage {
    create_prompt_message(
        Role::User,
        Content::Text(TextContent {
            type_field: "text".to_string(),
            text: text.to_string(),
            annotations: None,
        }),
    )
}

/// Helper to create an assistant prompt message with text
pub fn create_assistant_text_message(text: &str) -> PromptMessage {
    create_prompt_message(
        Role::Assistant,
        Content::Text(TextContent {
            type_field: "text".to_string(),
            text: text.to_string(),
            annotations: None,
        }),
    )
}

/// Helper to create a prompt message with image
pub fn create_image_message(role: Role, data: &[u8], mime_type: &str) -> PromptMessage {
    create_prompt_message(
        role,
        Content::Image(ImageContent {
            type_field: "image".to_string(),
            data: BASE64_STANDARD.encode(data),
            mime_type: mime_type.to_string(),
            annotations: None,
        }),
    )
}

/// Replace template variables in a string
pub fn apply_template_variables(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = template.to_string();

    for (key, value) in variables {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

/// Apply template variables to a prompt message
pub fn apply_variables_to_message(
    message: &PromptMessage,
    variables: &HashMap<String, String>,
) -> PromptMessage {
    match &message.content {
        Content::Text(text_content) => {
            let new_text = apply_template_variables(&text_content.text, variables);
            PromptMessage {
                role: message.role.clone(),
                content: Content::Text(TextContent {
                    type_field: text_content.type_field.clone(),
                    text: new_text,
                    annotations: text_content.annotations.clone(),
                }),
            }
        }
        _ => message.clone(), // For other content types, just return as is
    }
}
