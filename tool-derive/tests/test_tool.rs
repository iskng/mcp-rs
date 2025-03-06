use mcp_rs::types::tools::ToTool;
use tool_derive::Tool;
use serde::{ Serialize, Deserialize };
use mcp_rs::types::Tool;
use mcp_rs::types::tools::{ ToolBuilder, ToolParameterBuilder, ToolParameterType };

// Define an operation enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

// Output format enum for Calculator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Format {
    Json,
    Text,
    Html,
}

// Temperature unit enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

// Define nested struct for complex parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub timestamp: i64,
    pub tags: Vec<String>,
}

// Define a Calculator struct that will be converted to a Tool
#[derive(Tool, Debug, Clone)]
#[tool(description = "Performs basic arithmetic")]
pub struct Calculator {
    #[param(description = "First operand", required = true)]
    a: i64,

    #[param(description = "Second operand", required = true)]
    b: i64,

    #[param(
        description = "Operation to perform",
        required = true,
        enum_values = "add,subtract,multiply,divide"
    )]
    operation: Operation,
}

// Define a WeatherConverter struct that will be converted to a Tool
#[derive(Tool, Debug, Clone)]
#[tool(description = "Converts temperature between units")]
pub struct WeatherConverter {
    #[param(description = "Temperature value", required = true)]
    temperature: f64,

    #[param(description = "Temperature unit", required = true, enum_values = "celsius,fahrenheit")]
    unit: TemperatureUnit,
}

// Define a FormatCalculator struct that will be converted to a Tool
#[derive(Tool, Debug, Clone)]
#[tool(description = "Performs basic arithmetic with formatting")]
pub struct FormatCalculator {
    #[param(description = "First operand", required = true)]
    a: i64,

    #[param(description = "Second operand", required = true)]
    b: i64,

    #[param(
        description = "Operation to perform",
        required = true,
        enum_values = "add,subtract,multiply,divide"
    )]
    operation: Operation,

    #[param(description = "Output format", required = true, enum_values = "json,text,html")]
    format: Format,
}

// A struct with optional parameters
#[derive(Tool, Debug, Clone)]
#[tool(description = "Formats a greeting")]
pub struct Greeter {
    #[param(description = "Name to greet", required = true)]
    name: String,

    #[param(description = "Include timestamp")]
    include_timestamp: Option<bool>,

    #[param(description = "Custom greeting phrase")]
    greeting: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test Calculator conversion
    #[test]
    fn test_calculator_to_tool() {
        let calc = Calculator {
            a: 5,
            b: 10,
            operation: Operation::Add,
        };

        let tool = calc.to_tool();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, "Performs basic arithmetic");

        // Check parameters - need to unwrap the Option<Vec<ToolParameter>>
        let params = tool.parameters.as_ref().expect("Should have parameters");
        assert_eq!(params.len(), 3);

        // Check a parameter
        let a_param = params
            .iter()
            .find(|p| p.name == "a")
            .unwrap();
        assert_eq!(a_param.description.as_ref().unwrap(), "First operand");
        assert!(matches!(a_param.type_name, ToolParameterType::Number));
        assert!(a_param.required.unwrap_or(false));

        // Check enum parameter
        let op_param = params
            .iter()
            .find(|p| p.name == "operation")
            .unwrap();
        assert_eq!(op_param.description.as_ref().unwrap(), "Operation to perform");
        assert!(matches!(op_param.type_name, ToolParameterType::String));
        assert!(op_param.required.unwrap_or(false));
        assert_eq!(
            op_param.enum_values.as_ref(),
            Some(
                &vec![
                    "add".to_string(),
                    "subtract".to_string(),
                    "multiply".to_string(),
                    "divide".to_string()
                ]
            )
        );
    }

    // Test WeatherConverter conversion
    #[test]
    fn test_weather_converter_to_tool() {
        let converter = WeatherConverter {
            temperature: 25.0,
            unit: TemperatureUnit::Celsius,
        };

        let tool = converter.to_tool();

        assert_eq!(tool.name, "weather_converter");
        assert_eq!(tool.description, "Converts temperature between units");

        // Check parameters - need to unwrap the Option<Vec<ToolParameter>>
        let params = tool.parameters.as_ref().expect("Should have parameters");
        assert_eq!(params.len(), 2);

        // Check enum parameter
        let unit_param = params
            .iter()
            .find(|p| p.name == "unit")
            .unwrap();
        assert_eq!(unit_param.description.as_ref().unwrap(), "Temperature unit");
        assert!(matches!(unit_param.type_name, ToolParameterType::String));
        assert!(unit_param.required.unwrap_or(false));
        assert_eq!(
            unit_param.enum_values.as_ref(),
            Some(&vec!["celsius".to_string(), "fahrenheit".to_string()])
        );
    }
}
