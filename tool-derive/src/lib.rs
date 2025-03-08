extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{ quote, format_ident };
use syn::{ parse_macro_input, DeriveInput, Data, DataStruct, Fields, Meta, Lit, Field, Type };

/// Derive macro for implementing a tool from a struct
///
/// # Example
///
/// ```
/// #[derive(Tool)]
/// #[tool(description = "A calculator tool")]
/// struct Calculator {
///     #[param(description = "First operand", required = true)]
///     a: f64,
///
///     #[param(description = "Second operand", required = true)]
///     b: f64,
///
///     #[param(description = "Operation to perform", required = true, enum_values = "Add,Subtract,Multiply,Divide")]
///     operation: Operation,
/// }
/// ```
#[proc_macro_derive(Tool, attributes(tool, param))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct
    let name = &input.ident;
    let name_str = name.to_string();

    // Get the tool description from the #[tool] attribute
    let mut tool_description = format!("Tool for {}", name_str);
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

    // Get the struct fields
    let fields = match &input.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields), .. }) => &fields.named,
        _ => panic!("Tool derive only works on structs with named fields"),
    };

    // Generate code for each parameter based on the struct fields
    let params = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().expect("Named field expected").to_string();

        // Default parameter description and required flag
        let mut description = format!("Parameter: {}", field_name);
        let mut required = false;
        let mut explicit_enum_values: Option<Vec<String>> = None;

        // Extract parameter attributes
        for attr in &field.attrs {
            if attr.path().is_ident("param") {
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
                            let values_str = s.value();
                            let parsed_values = values_str
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
                ::mcp_rs::typess::tools::ToolParameterBuilder::new(#field_name, #param_type)
                    .description(#description)
                    .required(#required)
                    .enum_values(vec![#(#enum_vals),*])
                    .build()
            }
        } else {
            // For all other types, just use the basic builder without enum values
            quote! {
                ::mcp_rs::typess::tools::ToolParameterBuilder::new(#field_name, #param_type)
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

    // Generate code for the Tool trait implementation
    let expanded =
        quote! {
        impl ::mcp_rs::typess::tools::ToTool for #name {
            fn to_tool(&self) -> ::mcp_rs::typess::tools::Tool {
                let mut builder = ::mcp_rs::typess::tools::ToolBuilder::new(#name_str, #tool_description);
                
                // Add all parameters
                #(#params)*
                
                builder.build()
            }
        }
        
        impl ::mcp_rs::typess::tools::EnumValues for #name {
            fn enum_values() -> Vec<String> {
                vec![]
            }
        }
    };

    // Return the generated impl
    expanded.into()
}

/// Helper function to determine parameter type information from a struct field
fn get_parameter_info_for_field(
    field: &Field
) -> (proc_macro2::TokenStream, bool, Option<syn::TypePath>) {
    match &field.ty {
        Type::Path(type_path) => {
            // Check the last segment to determine the type
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();

                if type_name == "String" || type_name == "str" {
                    (
                        quote! { ::mcp_rs::typess::tools::ToolParameterType::String },
                        false,
                        Some(type_path.clone()),
                    )
                } else if
                    type_name == "i8" ||
                    type_name == "i16" ||
                    type_name == "i32" ||
                    type_name == "i64" ||
                    type_name == "u8" ||
                    type_name == "u16" ||
                    type_name == "u32" ||
                    type_name == "u64" ||
                    type_name == "f32" ||
                    type_name == "f64"
                {
                    (
                        quote! { ::mcp_rs::typess::tools::ToolParameterType::Number },
                        false,
                        Some(type_path.clone()),
                    )
                } else if type_name == "bool" {
                    (
                        quote! { ::mcp_rs::typess::tools::ToolParameterType::Boolean },
                        false,
                        Some(type_path.clone()),
                    )
                } else if type_name == "Vec" {
                    (
                        quote! { ::mcp_rs::typess::tools::ToolParameterType::Array },
                        false,
                        Some(type_path.clone()),
                    )
                } else if type_name == "HashMap" || type_name == "BTreeMap" {
                    (
                        quote! { ::mcp_rs::typess::tools::ToolParameterType::Object },
                        false,
                        Some(type_path.clone()),
                    )
                } else {
                    // Assume it's an enum (or other object type)
                    (
                        quote! { ::mcp_rs::typess::tools::ToolParameterType::String },
                        true,
                        Some(type_path.clone()),
                    )
                }
            } else {
                // Default to Object if we can't determine
                (quote! { ::mcp_rs::typess::tools::ToolParameterType::Object }, false, None)
            }
        }
        _ => {
            // Default to Object for complex types
            (quote! { ::mcp_rs::typess::tools::ToolParameterType::Object }, false, None)
        }
    }
}

/// Convert a string from CamelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if i > 0 && c.is_uppercase() {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // Tests would be added here
    }
}
