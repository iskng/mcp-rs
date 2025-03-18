extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ Data, DataStruct, DeriveInput, Field, Fields, Lit, Type, parse_macro_input };

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
        let field_ident = field.ident.as_ref().expect("Named field expected");
        let field_name = field_ident.to_string();

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
                            // Parse the enum values and ensure first letter is capitalized
                            let parsed_values = values_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .map(|s| capitalize_first_letter(&s))
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

        // Determine parameter type and build the appropriate parameter
        let (param_type, _, _) = get_parameter_info_for_field(field);

        if let Some(enum_vals) = explicit_enum_values {
            // For use in deserializing, we need only capitalized values in the schema
            let deserialization_values: Vec<String> = enum_vals.clone();

            // For enum parameters, use add_enum_parameter with capitalized values only
            quote! {
                builder = builder.add_enum_parameter(
                    #field_name,
                    #description,
                    &[#(#deserialization_values),*],
                    #required
                );
            }
        } else if param_type.to_string().contains("String") {
            // For string parameters
            quote! {
                builder = builder.add_string_parameter(
                    #field_name,
                    #description,
                    #required
                );
            }
        } else if param_type.to_string().contains("Number") {
            // For number parameters
            quote! {
                builder = builder.add_number_parameter(
                    #field_name,
                    #description,
                    #required
                );
            }
        } else if param_type.to_string().contains("Boolean") {
            // For boolean parameters
            quote! {
                builder = builder.add_boolean_parameter(
                    #field_name,
                    #description,
                    #required
                );
            }
        } else if param_type.to_string().contains("Array") {
            // For array parameters (using string as default item type)
            quote! {
                builder = builder.add_array_parameter(
                    #field_name,
                    #description,
                    "string",
                    #required
                );
            }
        } else {
            // For other types, default to string
            quote! {
                builder = builder.add_string_parameter(
                    #field_name,
                    #description,
                    #required
                );
            }
        }
    });

    // Generate code for deserializer that will handle both lowercase and capitalized enum values
    let enum_field_deserializers = quote! {};

    // Generate code for the ToToolSchema trait implementation
    let expanded =
        quote! {
        impl ::mcp_rs::protocol::tools::ToToolSchema for #name {
            fn to_tool_schema(&self) -> ::mcp_rs::protocol::Tool {
                let mut builder = ::mcp_rs::protocol::tools::ToolBuilder::new(#name_str, #tool_description);

                // Add all parameters
                #(#params)*

                builder.build()
            }
        }

        #enum_field_deserializers
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
                    (quote! { ::mcp_rs::protocol::tools::String }, false, Some(type_path.clone()))
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
                    (quote! { ::mcp_rs::protocol::tools::Number }, false, Some(type_path.clone()))
                } else if type_name == "bool" {
                    (quote! { ::mcp_rs::protocol::tools::Boolean }, false, Some(type_path.clone()))
                } else if type_name == "Vec" {
                    (quote! { ::mcp_rs::protocol::tools::Array }, false, Some(type_path.clone()))
                } else if type_name == "HashMap" || type_name == "BTreeMap" {
                    (quote! { ::mcp_rs::protocol::tools::Object }, false, Some(type_path.clone()))
                } else {
                    // Assume it's an enum (or other object type)
                    (quote! { ::mcp_rs::protocol::tools::String }, true, Some(type_path.clone()))
                }
            } else {
                // Default to Object if we can't determine
                (quote! { ::mcp_rs::protocol::tools::Object }, false, None)
            }
        }
        _ => {
            // Default to Object for complex types
            (quote! { ::mcp_rs::protocol::tools::Object }, false, None)
        }
    }
}

/// Helper function to capitalize the first letter of a string
fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // Tests would be added here
    }
}
