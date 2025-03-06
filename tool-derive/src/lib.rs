use proc_macro::TokenStream;
use quote::{ quote, format_ident };
use syn::{
    parse_macro_input,
    DeriveInput,
    Data,
    Fields,
    Lit,
    Attribute,
    Field,
    Type,
    Expr,
    ExprLit,
    Ident,
    Path,
    Meta,
    meta::ParseNestedMeta,
};
use syn::spanned::Spanned;
use std::collections::HashMap;

/// Implements the #[derive(Tool)] macro
///
/// This macro automatically generates an implementation of the ToTool trait for a struct,
/// allowing it to be converted to a Tool with minimal code.
///
/// # Example
/// ```
/// use mcp_rs_tool_derive::Tool;
///
/// // Define an enum for operation
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub enum Operation {
///     Add,
///     Subtract,
///     Multiply,
///     Divide,
/// }
///
/// // Implement EnumValues for the enum to provide its variants
/// impl EnumValues for Operation {
///    fn enum_values() -> Vec<String> {
///        vec!["add".to_string(), "subtract".to_string(), "multiply".to_string(), "divide".to_string()]
///    }
/// }
///
/// // Define a struct for your tool
/// #[derive(Tool, Debug, Clone)]
/// #[tool(description = "Performs basic arithmetic")]
/// pub struct Calculator {
///     #[param(description = "First operand", required = true)]
///     a: i64,
///
///     #[param(description = "Second operand", required = true)]
///     b: i64,
///
///     #[param(description = "Operation to perform", required = true)]
///     operation: Operation,
/// }
///
/// // Later in your code:
/// let calc = Calculator { a: 5, b: 10, operation: Operation::Add };
/// let tool = calc.to_tool(); // Automatically creates a Tool with proper parameters
/// ```
///
/// The macro will:
/// 1. Use the struct name to generate the tool name (converted to snake_case)
/// 2. Extract parameter definitions from struct fields
/// 3. Automatically detect enum types and use their variants as enum values if they implement EnumValues
/// 4. Generate an implementation of ToTool that builds the tool
#[proc_macro_derive(Tool, attributes(tool, param))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct name
    let struct_name = &input.ident;

    // Generate a snake case name from the struct name
    let tool_name = to_snake_case(&struct_name.to_string());

    // Default description (can be overridden by attributes)
    let mut tool_description = format!("Tool generated from {}", struct_name);

    // Parse the tool attributes if present
    for attr in &input.attrs {
        if attr.path().is_ident("tool") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        tool_description = s.value();
                    }
                }
                Ok(())
            }).unwrap_or_else(|_| {});
        }
    }

    // Only proceed with struct data
    let fields = if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            fields
        } else {
            panic!("Unnamed fields are not supported");
        }
    } else {
        panic!("Tool can only be derived for structs");
    };

    // Extract the field identifiers for later use in match statement
    let field_identifiers = fields.named.iter().map(|f| f.ident.as_ref().unwrap());

    // Process all parameter fields
    let parameter_additions = fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_ident = field.ident.as_ref().unwrap();

        // Default parameter attributes
        let mut description = format!("Parameter {}", field_name);
        let mut required = false;
        let mut explicit_enum_values = None;

        // Process all param attributes for this field
        for attr in &field.attrs {
            if attr.path().is_ident("param") {
                // Try to extract values using the standard approach
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("description") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            description = s.value();
                        }
                    } else if meta.path.is_ident("required") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Bool(b) = lit {
                            required = b.value();
                        }
                    } else if meta.path.is_ident("enum_values") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            // Parse enum values from string
                            let enum_vals_str = s.value().replace(['[', ']', '"', '\''], "");
                            let parsed_values = enum_vals_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect::<Vec<_>>();

                            if !parsed_values.is_empty() {
                                explicit_enum_values = Some(parsed_values);
                            }
                        }
                    }
                    Ok(())
                }).unwrap_or_else(|_| {});
            }
        }

        // Determine parameter type
        let (param_type, is_enum_type, _) = get_parameter_info_for_field(field);

        // Generate code to build this parameter
        let param_builder = if let Some(enum_vals) = explicit_enum_values {
            // If explicit enum values are provided, use them
            quote! {
                ToolParameterBuilder::new(#field_name, #param_type)
                    .description(#description)
                    .required(#required)
                    .enum_values(vec![#(#enum_vals),*])
                    .build()
            }
        } else {
            // For all other types, just use the basic builder without enum values
            quote! {
                ToolParameterBuilder::new(#field_name, #param_type)
                    .description(#description)
                    .required(#required)
                    .build()
            }
        };

        // Return the code to add this parameter
        quote! {
            builder = builder.add_parameter(#param_builder);
        }
    });

    // Generate the implementation
    let output =
        quote! {
        impl ToTool for #struct_name {
            fn to_tool(&self) -> Tool {
                let mut builder = ToolBuilder::new(#tool_name, #tool_description);
                
                // Add parameters based on fields
                #(#parameter_additions)*
                
                builder.build()
            }
        }
    };

    output.into()
}

// Helper function to determine the ToolParameterType for a field
fn get_parameter_info_for_field(
    field: &Field
) -> (proc_macro2::TokenStream, bool, Option<syn::TypePath>) {
    match &field.ty {
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().unwrap();
            let type_name = last_segment.ident.to_string();

            match type_name.as_str() {
                "String" | "str" =>
                    (quote! { ToolParameterType::String }, false, Some(type_path.clone())),
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64" => (quote! { ToolParameterType::Number }, false, Some(type_path.clone())),
                "bool" => (quote! { ToolParameterType::Boolean }, false, Some(type_path.clone())),
                // Handle Option<T> types
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            if let Type::Path(inner_path) = inner_type {
                                if let Some(inner_segment) = inner_path.path.segments.last() {
                                    let inner_type_name = inner_segment.ident.to_string();
                                    match inner_type_name.as_str() {
                                        "String" | "str" =>
                                            (
                                                quote! { ToolParameterType::String },
                                                false,
                                                Some(inner_path.clone()),
                                            ),
                                        | "i8"
                                        | "i16"
                                        | "i32"
                                        | "i64"
                                        | "i128"
                                        | "isize"
                                        | "u8"
                                        | "u16"
                                        | "u32"
                                        | "u64"
                                        | "u128"
                                        | "usize"
                                        | "f32"
                                        | "f64" =>
                                            (
                                                quote! { ToolParameterType::Number },
                                                false,
                                                Some(inner_path.clone()),
                                            ),
                                        "bool" =>
                                            (
                                                quote! { ToolParameterType::Boolean },
                                                false,
                                                Some(inner_path.clone()),
                                            ),
                                        // Assume any other type is an enum
                                        _ =>
                                            (
                                                quote! { ToolParameterType::String },
                                                true,
                                                Some(inner_path.clone()),
                                            ),
                                    }
                                } else {
                                    (quote! { ToolParameterType::String }, false, None)
                                }
                            } else {
                                (quote! { ToolParameterType::String }, false, None)
                            }
                        } else {
                            (quote! { ToolParameterType::String }, false, None)
                        }
                    } else {
                        (quote! { ToolParameterType::String }, false, None)
                    }
                }
                // Assume any other type is an enum
                _ => (quote! { ToolParameterType::String }, true, Some(type_path.clone())),
            }
        }
        _ => (quote! { ToolParameterType::String }, false, None), // Default to string for complex types
    }
}

// Helper function to convert CamelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // Tests for the macro implementation would go here
        assert_eq!(super::to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(super::to_snake_case("Calculator"), "calculator");
    }
}
