use crate::protocol::tools::{
    CallToolResultBuilder, ToolArgumentsBuilder, ToolBuilder, ToolParameters,
};
use crate::protocol::{CallToolParams, CallToolResult, Tool};

/// This file contains migration examples showing how to move from the old tools API to the new one

/// Example: Creating a simple tool
pub fn example_create_simple_tool() -> Tool {
    // New approach using our new tools module:
    let tool = ToolBuilder::new("calculator", "A simple calculator")
        .add_string_parameter("operation", "Operation to perform (+, -, *, /)", true)
        .add_number_parameter("a", "First operand", true)
        .add_number_parameter("b", "Second operand", true)
        .build();

    tool
}

/// Example: Alternative approach using ToolParameters
pub fn example_create_tool_with_parameters() -> Tool {
    let mut params = ToolParameters::new();

    // Add parameters
    params
        .add_string("operation", "Operation to perform (+, -, *, /)", true)
        .add_number("a", "First operand", true)
        .add_number("b", "Second operand", true);

    // Convert to a tool
    params.to_tool("calculator", "A simple calculator")
}

/// Example: Creating tool call parameters
pub fn example_create_call_params() -> CallToolParams {
    // Old approach:
    // let params = CallToolParams {
    //     name: "calculator".to_string(),
    //     arguments: {
    //         let mut args = HashMap::new();
    //         args.insert("operation".to_string(), Value::String("+".to_string()));
    //         args.insert("a".to_string(), Value::Number(serde_json::Number::from(5)));
    //         args.insert("b".to_string(), Value::Number(serde_json::Number::from(3)));
    //         args
    //     }
    // };

    // New approach using builder:
    let params = ToolArgumentsBuilder::new()
        .add_string("operation", "+")
        .add_number("a", 5.0)
        .add_number("b", 3.0)
        .to_call_params("calculator");

    params
}

/// Example: Creating tool results
pub fn example_create_tool_result() -> CallToolResult {
    // Old approach with CallToolResultBuilder:
    // let result = CallToolResultBuilder::new()
    //     .add_text("The result is: 8")
    //     .build();

    // New approach 1: using builder
    let result = CallToolResultBuilder::new()
        .add_text("The result is: 8")
        .build();

    // New approach 3: using static method
    let _result3 = CallToolResult::text("The result is: 8");

    result
}

/// Example: Creating error results
pub fn example_create_error_result() -> CallToolResult {
    // Old approach:
    // let error_result = CallToolResultBuilder::new()
    //     .add_text("Error: Division by zero")
    //     .with_error()
    //     .build();

    // New approach 1: using builder
    let error_result = CallToolResultBuilder::new()
        .add_text("Error: Division by zero")
        .with_error()
        .build();

    // New approach 3: using static method
    let _error_result3 = CallToolResult::error("Error: Division by zero");

    error_result
}

/// Example: Full implementation of a calculator tool
pub fn calculator_example(operation: &str, a: f64, b: f64) -> CallToolResult {
    match operation {
        "+" => CallToolResult::text(format!("The result is: {}", a + b)),
        "-" => CallToolResult::text(format!("The result is: {}", a - b)),
        "*" => CallToolResult::text(format!("The result is: {}", a * b)),
        "/" => {
            if b == 0.0 {
                CallToolResult::error("Error: Division by zero")
            } else {
                CallToolResult::text(format!("The result is: {}", a / b))
            }
        }
        _ => CallToolResult::error(format!("Unsupported operation: {}", operation)),
    }
}
