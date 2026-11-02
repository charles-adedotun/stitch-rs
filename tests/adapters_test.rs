use stitch_rs::{to_gemini_tools, to_mcp_tools, to_openai_tools};

#[test]
fn test_gemini_adapter_structure() {
    let gemini = to_gemini_tools(None, None);
    let arr = gemini.as_array().expect("array root");
    assert_eq!(arr.len(), 1);

    let decls = arr[0]["functionDeclarations"]
        .as_array()
        .expect("functionDeclarations array");
    assert!(!decls.is_empty());

    for decl in decls {
        assert!(decl.get("name").is_some());
        assert!(decl.get("description").is_some());
        let params = &decl["parameters"];
        assert_eq!(params["type"], "object");
        // Gemini function declarations forbid $defs and $ref
        assert!(params.get("$defs").is_none());
        assert!(params.get("$ref").is_none());
    }
}

#[test]
fn test_openai_adapter_structure() {
    let openai = to_openai_tools(None, Some(&["create_project", "generate_screen_from_text"]));
    let arr = openai.as_array().expect("array root");
    assert_eq!(arr.len(), 2);

    for tool in arr {
        assert_eq!(tool["type"], "function");
        assert!(tool["function"].get("name").is_some());
        assert!(tool["function"].get("description").is_some());
        assert!(tool["function"]["parameters"].is_object());
    }
}

#[test]
fn test_mcp_adapter_structure() {
    let mcp = to_mcp_tools(None, Some(&["download_file"]));
    let arr = mcp.as_array().expect("array root");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "download_file");
    assert!(arr[0].get("inputSchema").is_some());
}
