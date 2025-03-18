use mcp_rs::protocol::tools::ToToolSchema;
use serde::{ Deserialize, Serialize };
use tool_derive::Tool;

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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct WeatherConverter {
    #[param(description = "Temperature value", required = true)]
    temperature: f64,

    #[param(description = "Temperature unit", required = true, enum_values = "celsius,fahrenheit")]
    unit: TemperatureUnit,
}

// Define a FormatCalculator struct that will be converted to a Tool
#[derive(Tool, Debug, Clone)]
#[tool(description = "Performs basic arithmetic with formatting")]
#[allow(dead_code)]
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
#[allow(dead_code)]
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

        let tool = calc.to_tool_schema();

        assert_eq!(tool.name, "Calculator");
        assert_eq!(tool.description.unwrap(), "Performs basic arithmetic");

        let input_schema = tool.input_schema;

        let properties = input_schema.properties.expect("Should have properties");
        assert_eq!(properties.len(), 3);

        let a_param = properties.get("a").expect("Should have 'a' parameter");
        assert_eq!(
            a_param.get("description").and_then(|v| v.as_str()),
            Some("First operand")
        );
        assert_eq!(
            a_param.get("type").and_then(|v| v.as_str()),
            Some("number")
        );

        let op_param = properties.get("operation").expect("Should have 'operation' parameter");
        assert_eq!(
            op_param.get("description").and_then(|v| v.as_str()),
            Some("Operation to perform")
        );
        assert_eq!(
            op_param.get("type").and_then(|v| v.as_str()),
            Some("string")
        );

        let enum_values = op_param
            .get("enum")
            .expect("Should have enum values")
            .as_array()
            .expect("enum should be an array");
        assert_eq!(enum_values.len(), 4);
        assert!(enum_values.iter().any(|v| v.as_str() == Some("Add")));
        assert!(enum_values.iter().any(|v| v.as_str() == Some("Subtract")));
        assert!(enum_values.iter().any(|v| v.as_str() == Some("Multiply")));
        assert!(enum_values.iter().any(|v| v.as_str() == Some("Divide")));

        let required = input_schema.required.expect("Should have required fields");
        assert_eq!(required.len(), 3);
        assert!(required.contains(&"a".to_string()));
        assert!(required.contains(&"b".to_string()));
        assert!(required.contains(&"operation".to_string()));
    }

    // Test WeatherConverter conversion
    #[test]
    fn test_weather_converter_to_tool() {
        let converter = WeatherConverter {
            temperature: 25.0,
            unit: TemperatureUnit::Celsius,
        };

        let tool = converter.to_tool_schema();

        assert_eq!(tool.name, "WeatherConverter");
        assert_eq!(tool.description.unwrap(), "Converts temperature between units");

        let input_schema = tool.input_schema;

        let properties = input_schema.properties.expect("Should have properties");
        assert_eq!(properties.len(), 2);

        let unit_param = properties.get("unit").expect("Should have 'unit' parameter");
        assert_eq!(
            unit_param.get("description").and_then(|v| v.as_str()),
            Some("Temperature unit")
        );
        assert_eq!(
            unit_param.get("type").and_then(|v| v.as_str()),
            Some("string")
        );

        let enum_values = unit_param
            .get("enum")
            .expect("Should have enum values")
            .as_array()
            .expect("enum should be an array");
        assert_eq!(enum_values.len(), 2);
        assert!(enum_values.iter().any(|v| v.as_str() == Some("celsius")));
        assert!(enum_values.iter().any(|v| v.as_str() == Some("fahrenheit")));

        let required = input_schema.required.expect("Should have required fields");
        assert_eq!(required.len(), 2);
        assert!(required.contains(&"temperature".to_string()));
        assert!(required.contains(&"unit".to_string()));
    }
}
