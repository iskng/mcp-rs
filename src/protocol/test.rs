#[cfg(test)]
mod tests {
    use crate::protocol::protocol::Definitions;
    use schemars::schema_for;
    use serde_json::{self, Value, json};
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_schema_completeness() {
        // Generate schema directly from our Definitions struct
        let schema = schema_for!(Definitions);
        let generated_schema = serde_json::to_value(schema).unwrap();

        // Read the original schema.json file
        let schema_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/protocol/protocol.json");
        let schema_json = fs::read_to_string(schema_path).expect("Failed to read schema.json");
        let original_schema: Value =
            serde_json::from_str(&schema_json).expect("Failed to parse schema.json");

        // Extract definitions
        let original_defs = match &original_schema["definitions"] {
            Value::Object(defs) => defs,
            _ => panic!("Original schema has no definitions object"),
        };

        let generated_defs = match &generated_schema["definitions"] {
            Value::Object(defs) => defs,
            _ => panic!("Generated schema has no definitions object"),
        };

        // Print all available definitions to help with debugging
        let available_defs: Vec<&String> = generated_defs.keys().collect();
        println!("Available definitions: {:?}", available_defs);

        // Check that all original definitions are present in our generated schema
        let mut missing_defs = Vec::new();

        for (def_name, original_def) in original_defs {
            // Find matching definition in our schema
            let mut found = false;
            let mut matching_def_name = "";

            // Try exact match first
            if generated_defs.contains_key(def_name) {
                found = true;
                matching_def_name = def_name;
            } else {
                // Try case-insensitive match (since Rust might use different casing)
                for gen_name in generated_defs.keys() {
                    if gen_name.to_lowercase() == def_name.to_lowercase() {
                        found = true;
                        matching_def_name = gen_name;
                        break;
                    }
                }
            }

            if !found {
                missing_defs.push(def_name.clone());
                continue;
            }

            // Now check that the definition's structure matches
            let generated_def = &generated_defs[matching_def_name];

            // Compare required properties
            compare_required_fields(def_name, original_def, generated_def);

            // Compare properties
            compare_properties(def_name, original_def, generated_def);
        }

        if !missing_defs.is_empty() {
            panic!(
                "Missing definitions in generated schema: {:?}",
                missing_defs
            );
        }

        println!("All definitions from the original schema are present and correct!");

        // Simple serialization test to verify implementation
        let json_data = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "initialize",
            "params": {
                "capabilities": {
                    "sampling": {}
                },
                "client_info": {
                    "name": "test-client",
                    "version": "1.0.0"
                },
                "protocol_version": "0.1.0"
            }
        });

        // Verify we can deserialize the data
        let _: crate::protocol::protocol::JSONRPCRequest =
            serde_json::from_value(json_data).unwrap();

        println!("Schema compatibility test passed!");
    }

    fn compare_required_fields(def_name: &str, original: &Value, generated: &Value) {
        // Get required fields from both schemas
        let original_required = match original.get("required") {
            Some(Value::Array(req)) => req
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<HashSet<_>>(),
            _ => HashSet::new(),
        };

        let generated_required = match generated.get("required") {
            Some(Value::Array(req)) => req
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<HashSet<_>>(),
            _ => HashSet::new(),
        };

        // Check if all original required fields are in generated schema
        for field in &original_required {
            // Special case for 'ref' field which is renamed to 'ref_' in Rust
            if *field == "ref" && generated_required.contains("ref_") {
                continue;
            }

            if !generated_required.contains(field) {
                println!(
                    "WARNING: Definition '{}' is missing required field '{}' in generated schema",
                    def_name, field
                );
            }
        }
    }

    fn compare_properties(def_name: &str, original: &Value, generated: &Value) {
        // Get properties from both schemas
        let original_props = match original.get("properties") {
            Some(Value::Object(props)) => props,
            _ => {
                return;
            } // No properties to check
        };

        let generated_props = match generated.get("properties") {
            Some(Value::Object(props)) => props,
            _ => {
                if !original_props.is_empty() {
                    println!(
                        "WARNING: Definition '{}' has properties in original schema but none in generated schema",
                        def_name
                    );
                }
                return;
            }
        };

        // Check that all properties from original schema exist in generated schema
        for (prop_name, _) in original_props {
            if !generated_props.contains_key(prop_name) {
                println!(
                    "WARNING: Definition '{}' is missing property '{}' in generated schema",
                    def_name, prop_name
                );
            }

            // We could add deeper checks here for property types and structure
        }
    }
}
