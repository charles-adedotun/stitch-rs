//! Recursive JSON Schema repair and dereferencer for Stitch tools.
//! Backend omits $defs for ScreenInstance, SelectedScreenInstance, File, etc.
//! Injects missing $defs and produces inlined schemas for Gemini and OpenAI tool calls.

use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

static REF_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_ref_regex() -> &'static Regex {
    REF_REGEX
        .get_or_init(|| Regex::new(r"^#/(?:\$defs|definitions)/(.+)$").expect("valid ref regex"))
}

/// Well-known schema definitions omitted by the Stitch backend in some MCP tool schemas.
pub fn get_well_known_defs() -> BTreeMap<String, Value> {
    let mut defs = BTreeMap::new();

    defs.insert(
        "ScreenInstance".to_string(),
        json!({
            "type": "object",
            "description": "An instance of a screen on the project.",
            "properties": {
                "groupId": { "type": "string" },
                "groupName": { "type": "string" },
                "height": { "type": "integer", "format": "int32" },
                "hidden": { "type": "boolean" },
                "id": { "type": "string" },
                "isFavourite": { "type": "boolean" },
                "label": { "type": "string" },
                "sourceAsset": { "type": "string" },
                "sourceScreen": { "type": "string" },
                "type": {
                    "type": "string",
                    "enum": [
                        "SCREEN_INSTANCE_TYPE_UNSPECIFIED",
                        "SCREEN_INSTANCE",
                        "DESIGN_SYSTEM_INSTANCE",
                        "GROUP_INSTANCE"
                    ]
                },
                "width": { "type": "integer", "format": "int32" },
                "x": { "type": "integer", "format": "int32" },
                "y": { "type": "integer", "format": "int32" }
            }
        }),
    );

    defs.insert(
        "SelectedScreenInstance".to_string(),
        json!({
            "type": "object",
            "description": "A screen instance reference to be edited or styled.",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The id of the screen instance."
                },
                "sourceScreen": {
                    "type": "string",
                    "description": "Resource name format: projects/{project}/screens/{screen}"
                },
                "screenId": { "type": "string" },
                "instanceId": { "type": "string" }
            },
            "required": ["id", "sourceScreen"]
        }),
    );

    defs.insert(
        "File".to_string(),
        json!({
            "type": "object",
            "description": "A File resource containing code, asset, or screenshot.",
            "properties": {
                "downloadUrl": { "type": "string" },
                "fileContentBase64": { "type": "string" },
                "mimeType": { "type": "string" },
                "name": { "type": "string" },
                "uploadBlobId": { "type": "string" }
            }
        }),
    );

    defs.insert(
        "VariantOptions".to_string(),
        json!({
            "type": "object",
            "description": "Configuration options for design variant generation.",
            "properties": {
                "aspects": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": [
                            "VARIANT_ASPECT_UNSPECIFIED",
                            "LAYOUT",
                            "COLOR_SCHEME",
                            "IMAGES",
                            "TEXT_FONT",
                            "TEXT_CONTENT"
                        ]
                    }
                },
                "creativeRange": {
                    "type": "string",
                    "enum": ["CREATIVE_RANGE_UNSPECIFIED", "REFINE", "EXPLORE", "REIMAGINE"]
                },
                "variantCount": {
                    "type": "integer",
                    "format": "int32"
                }
            }
        }),
    );

    defs
}

/// Recursively collects all definition targets referenced in `$ref` fields.
pub fn collect_ref_targets(node: &Value, refs: &mut BTreeSet<String>) {
    let re = get_ref_regex();
    match node {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get("$ref") {
                if let Some(caps) = re.captures(r) {
                    if let Some(matched) = caps.get(1) {
                        refs.insert(matched.as_str().to_string());
                    }
                }
            }
            for v in map.values() {
                collect_ref_targets(v, refs);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_ref_targets(v, refs);
            }
        }
        _ => {}
    }
}

/// Repairs a JSON schema by guaranteeing all referenced $defs exist.
pub fn repair_schema(schema: &mut Value) {
    if !schema.is_object() {
        return;
    }

    let mut referenced = BTreeSet::new();
    collect_ref_targets(schema, &mut referenced);

    if referenced.is_empty() {
        return;
    }

    let well_known = get_well_known_defs();
    let root = schema.as_object_mut().unwrap();

    let defs_entry = root
        .entry("$defs")
        .or_insert_with(|| Value::Object(Map::new()));

    if let Value::Object(ref mut defs_map) = defs_entry {
        for def_name in referenced {
            if !defs_map.contains_key(&def_name) {
                if let Some(canonical) = well_known.get(&def_name) {
                    defs_map.insert(def_name, canonical.clone());
                } else {
                    defs_map.insert(
                        def_name.clone(),
                        json!({
                            "type": "object",
                            "description": format!("Repaired definition for {def_name}"),
                            "additionalProperties": true
                        }),
                    );
                }
            }
        }
    }
}

/// Applies recursive schema repair across a slice of MCP tools.
pub fn repair_tool_schemas(tools: &mut [Value]) {
    for tool in tools {
        if let Some(input_schema) = tool.get_mut("inputSchema") {
            repair_schema(input_schema);
        }
        if let Some(parameters) = tool.get_mut("parameters") {
            repair_schema(parameters);
        }
        if let Some(output_schema) = tool.get_mut("outputSchema") {
            repair_schema(output_schema);
        }
    }
}

/// Recursively dereferences all $ref pointers into standalone inline schemas,
/// and strips Google-specific vendor extensions (e.g. `x-google-*`).
/// Required for Google Gemini tool declarations which forbid `$ref` and `$defs`.
pub fn dereference_schema(schema: &Value) -> Value {
    let mut cloned = schema.clone();
    repair_schema(&mut cloned);

    let mut defs = get_well_known_defs();
    if let Some(existing_defs) = cloned.get("$defs").and_then(|v| v.as_object()) {
        for (k, v) in existing_defs {
            defs.insert(k.clone(), v.clone());
        }
    }
    if let Some(existing_defs) = cloned.get("definitions").and_then(|v| v.as_object()) {
        for (k, v) in existing_defs {
            defs.insert(k.clone(), v.clone());
        }
    }

    walk_dereference(&cloned, &defs, 0)
}

fn walk_dereference(node: &Value, defs: &BTreeMap<String, Value>, depth: usize) -> Value {
    if depth > 20 {
        return node.clone();
    }

    match node {
        Value::Array(arr) => {
            let deref_arr = arr
                .iter()
                .map(|item| walk_dereference(item, defs, depth + 1))
                .collect();
            Value::Array(deref_arr)
        }
        Value::Object(map) => {
            let re = get_ref_regex();
            let mut result = Map::new();

            // Check if this object is a $ref pointer
            if let Some(Value::String(ref_val)) = map.get("$ref") {
                if let Some(caps) = re.captures(ref_val) {
                    if let Some(matched) = caps.get(1) {
                        let target_name = matched.as_str();
                        if let Some(target_def) = defs.get(target_name) {
                            let resolved = walk_dereference(target_def, defs, depth + 1);
                            if let Value::Object(resolved_map) = resolved {
                                for (k, v) in resolved_map {
                                    result.insert(k, v);
                                }
                            }
                        }
                    }
                }
            }

            for (k, v) in map {
                // Strip vendor extensions and definition containers
                if k.starts_with("x-google-") || k == "$defs" || k == "definitions" {
                    continue;
                }
                if k == "$ref" {
                    continue;
                }
                result.insert(k.clone(), walk_dereference(v, defs, depth + 1));
            }

            Value::Object(result)
        }
        other => other.clone(),
    }
}
