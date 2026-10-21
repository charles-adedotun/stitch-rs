//! LLM Tool format adapters for Google Gemini, OpenAI/Claude, and Model Context Protocol (MCP).

use crate::schema_repair::{dereference_schema, repair_schema};
use crate::schemas::get_builtin_tools;
use serde_json::{json, Value};
use std::collections::HashSet;

fn filter_tools<'a>(tools: &'a [Value], include: Option<&[&str]>) -> Vec<&'a Value> {
    if let Some(inc) = include {
        let set: HashSet<&str> = inc.iter().copied().collect();
        tools
            .iter()
            .filter(|t| {
                t.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| set.contains(n))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        tools.iter().collect()
    }
}

/// Converts tool definitions into Google Gemini SDK function declarations.
/// Gemini functionDeclarations strictly require schemas without `$ref`, `$defs`,
/// and without `x-google-*` vendor extensions.
pub fn to_gemini_tools(tools: Option<&[Value]>, include: Option<&[&str]>) -> Value {
    let source: &[Value] = match tools {
        Some(t) => t,
        None => get_builtin_tools(),
    };
    let filtered = filter_tools(source, include);

    let function_declarations: Vec<Value> = filtered
        .into_iter()
        .map(|tool| {
            let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let desc = tool
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let raw_params = tool
                .get("parameters")
                .or_else(|| tool.get("inputSchema"))
                .cloned()
                .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));

            let clean_params = dereference_schema(&raw_params);

            json!({
                "name": name,
                "description": desc,
                "parameters": clean_params
            })
        })
        .collect();

    json!([
        {
            "functionDeclarations": function_declarations
        }
    ])
}

/// Converts tool definitions into OpenAI and Anthropic Claude tool calling format.
pub fn to_openai_tools(tools: Option<&[Value]>, include: Option<&[&str]>) -> Value {
    let source: &[Value] = match tools {
        Some(t) => t,
        None => get_builtin_tools(),
    };
    let filtered = filter_tools(source, include);

    let list: Vec<Value> = filtered
        .into_iter()
        .map(|tool| {
            let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let desc = tool
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mut raw_params = tool
                .get("parameters")
                .or_else(|| tool.get("inputSchema"))
                .cloned()
                .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));

            repair_schema(&mut raw_params);

            json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": desc,
                    "parameters": raw_params
                }
            })
        })
        .collect();

    json!(list)
}

/// Converts tool definitions into standard Model Context Protocol (MCP) tool format.
pub fn to_mcp_tools(tools: Option<&[Value]>, include: Option<&[&str]>) -> Value {
    let source: &[Value] = match tools {
        Some(t) => t,
        None => get_builtin_tools(),
    };
    let filtered = filter_tools(source, include);

    let list: Vec<Value> = filtered
        .into_iter()
        .map(|tool| {
            let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let desc = tool
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mut raw_schema = tool
                .get("inputSchema")
                .or_else(|| tool.get("parameters"))
                .cloned()
                .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));

            repair_schema(&mut raw_schema);

            json!({
                "name": name,
                "description": desc,
                "inputSchema": raw_schema
            })
        })
        .collect();

    json!(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_gemini_format() {
        let gemini = to_gemini_tools(None, Some(&["create_project"]));
        let decls = &gemini[0]["functionDeclarations"];
        assert_eq!(decls.as_array().unwrap().len(), 1);
        assert_eq!(decls[0]["name"], "create_project");
    }

    #[test]
    fn test_to_openai_format() {
        let openai = to_openai_tools(None, Some(&["generate_variants"]));
        let arr = openai.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["type"], "function");
        assert_eq!(arr[0]["function"]["name"], "generate_variants");
    }

    #[test]
    fn test_to_mcp_format() {
        let mcp = to_mcp_tools(None, Some(&["list_screens"]));
        let arr = mcp.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "list_screens");
        assert!(arr[0].get("inputSchema").is_some());
    }
}
