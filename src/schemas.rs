//! Canonical tool definitions catalog for Google Stitch MCP tools.

use serde_json::Value;
use std::sync::OnceLock;

static BUILTIN_TOOLS: OnceLock<Vec<Value>> = OnceLock::new();

/// Returns the complete list of built-in tool definitions for Google Stitch.
pub fn get_builtin_tools() -> &'static [Value] {
    BUILTIN_TOOLS.get_or_init(|| {
        let raw = include_str!("builtin_tools.json");
        serde_json::from_str(raw).expect("valid builtin_tools.json")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_tools_loaded() {
        let tools = get_builtin_tools();
        assert!(tools.len() >= 10);
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"create_project"));
        assert!(names.contains(&"generate_screen_from_text"));
        assert!(names.contains(&"generate_variants"));
    }
}
