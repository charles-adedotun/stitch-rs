use serde_json::json;
use stitch_rs::{create_stitch_error, parse_tool_response, StitchError};

#[test]
fn test_precedence_hierarchy() {
    // 1. Structured content beats content array
    let res1 = json!({
        "structuredContent": { "id": "screen_101" },
        "content": [{ "type": "text", "text": "fallback text" }]
    });
    let parsed1 = parse_tool_response(&res1, Some("test")).unwrap();
    assert_eq!(parsed1["id"], "screen_101");

    // 2. content array with JSON string parsed to JSON
    let res2 = json!({
        "content": [{ "type": "text", "text": "{\"projects\": [\"p1\", \"p2\"]}" }]
    });
    let parsed2 = parse_tool_response(&res2, Some("test")).unwrap();
    assert!(parsed2["projects"].is_array());
    assert_eq!(parsed2["projects"][0], "p1");

    // 3. content array with raw string fallback
    let res3 = json!({
        "content": [{ "type": "text", "text": "Plain text output from tool" }]
    });
    let parsed3 = parse_tool_response(&res3, Some("test")).unwrap();
    assert_eq!(parsed3.as_str().unwrap(), "Plain text output from tool");
}

#[test]
fn test_error_categorization() {
    let err_rl = create_stitch_error("User has exceeded quota (429)", Some("gen"), Some(429));
    assert!(matches!(err_rl, StitchError::RateLimited { .. }));
    assert!(err_rl.is_recoverable());
    assert_eq!(err_rl.status_code(), Some(429));

    let err_nf = create_stitch_error("Project '123' was not found", Some("get_proj"), Some(404));
    assert!(matches!(err_nf, StitchError::NotFound { .. }));
    assert!(!err_nf.is_recoverable());
    assert_eq!(err_nf.status_code(), Some(404));

    let err_auth = create_stitch_error("Invalid API key provided", Some("call"), Some(401));
    assert!(matches!(err_auth, StitchError::AuthFailed { .. }));
    assert!(!err_auth.is_recoverable());
    assert_eq!(err_auth.status_code(), Some(401));

    let err_perm = create_stitch_error("Permission denied on resource", Some("delete"), Some(403));
    assert!(matches!(err_perm, StitchError::PermissionDenied { .. }));
    assert_eq!(err_perm.status_code(), Some(403));
}
