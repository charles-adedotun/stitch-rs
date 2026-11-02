use serde_json::json;
use stitch_rs::{dereference_schema, repair_schema};

#[test]
fn test_repair_omitted_defs() {
    let mut broken_schema = json!({
        "type": "object",
        "properties": {
            "screen": {
                "$ref": "#/$defs/ScreenInstance"
            },
            "file": {
                "$ref": "#/$defs/File"
            },
            "selected": {
                "$ref": "#/$defs/SelectedScreenInstance"
            }
        }
    });

    repair_schema(&mut broken_schema);

    let defs = broken_schema.get("$defs").expect("$defs must be inserted");
    assert!(defs.get("ScreenInstance").is_some());
    assert!(defs.get("File").is_some());
    assert!(defs.get("SelectedScreenInstance").is_some());

    // Verify ScreenInstance properties
    assert_eq!(defs["ScreenInstance"]["properties"]["id"]["type"], "string");
    assert_eq!(defs["File"]["properties"]["downloadUrl"]["type"], "string");
}

#[test]
fn test_dereference_for_gemini() {
    let schema_with_refs = json!({
        "type": "object",
        "properties": {
            "options": {
                "$ref": "#/$defs/VariantOptions"
            }
        },
        "x-google-internal": true
    });

    let dereferenced = dereference_schema(&schema_with_refs);

    // Assert $defs and definitions are removed
    assert!(dereferenced.get("$defs").is_none());
    assert!(dereferenced.get("definitions").is_none());

    // Assert vendor extensions are stripped
    assert!(dereferenced.get("x-google-internal").is_none());

    // Assert options has inlined properties instead of $ref
    let options = &dereferenced["properties"]["options"];
    assert!(options.get("$ref").is_none());
    assert_eq!(options["type"], "object");
    assert!(options["properties"]["creativeRange"].is_object());
}

#[test]
fn test_repair_unknown_ref_fallback() {
    let mut custom_schema = json!({
        "type": "object",
        "properties": {
            "custom": {
                "$ref": "#/$defs/CustomUnknownType"
            }
        }
    });

    repair_schema(&mut custom_schema);

    let defs = custom_schema.get("$defs").expect("$defs created");
    assert!(defs.get("CustomUnknownType").is_some());
    assert_eq!(defs["CustomUnknownType"]["type"], "object");
    assert_eq!(defs["CustomUnknownType"]["additionalProperties"], true);
}
