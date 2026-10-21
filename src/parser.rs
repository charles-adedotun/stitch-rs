//! Response parsing and typed error categorization for Stitch JSON-RPC tool calls.

use crate::errors::StitchError;
use serde_json::Value;

/// Categorizes an error message and optional HTTP status into a typed StitchError.
pub fn create_stitch_error(
    error_text: &str,
    tool_name: Option<&str>,
    status: Option<u16>,
) -> StitchError {
    let lower = error_text.to_lowercase();
    let tool_owned = tool_name.map(|s| s.to_string());

    if lower.contains("rate limit") || lower.contains("quota exceeded") || status == Some(429) {
        return StitchError::RateLimited {
            message: error_text.to_string(),
            tool: tool_owned,
        };
    }

    if lower.contains("not found") || status == Some(404) {
        return StitchError::NotFound {
            message: error_text.to_string(),
            tool: tool_owned,
        };
    }

    if lower.contains("unauthorized")
        || lower.contains("unauthenticated")
        || lower.contains("invalid api key")
        || status == Some(401)
    {
        return StitchError::AuthFailed {
            message: error_text.to_string(),
            tool: tool_owned,
        };
    }

    if lower.contains("permission") || lower.contains("forbidden") || status == Some(403) {
        return StitchError::PermissionDenied {
            message: error_text.to_string(),
            tool: tool_owned,
        };
    }

    if let Some(st) = status {
        return StitchError::Http {
            status: st,
            message: error_text.to_string(),
        };
    }

    StitchError::General(error_text.to_string())
}

/// Parses the payload returned by a Stitch MCP tool execution.
/// Follows precedence:
/// 1. MCP isError flag detection
/// 2. structuredContent (preferred high-fidelity object)
/// 3. content[0].text parsed as JSON (fallback to raw text string)
/// 4. Direct html/image fields or raw result object
pub fn parse_tool_response(result: &Value, tool_name: Option<&str>) -> Result<Value, StitchError> {
    if !result.is_object() {
        return Ok(result.clone());
    }

    // Check MCP tool error flag
    if let Some(is_error) = result.get("isError").and_then(|v| v.as_bool()) {
        if is_error {
            let error_text = extract_content_text(result)
                .or_else(|| {
                    result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .or_else(|| {
                    result
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| "Unknown MCP tool execution failure".to_string());

            return Err(create_stitch_error(&error_text, tool_name, None));
        }
    }

    // 1. Structured content takes highest precedence
    if let Some(structured) = result.get("structuredContent") {
        if !structured.is_null() {
            return Ok(structured.clone());
        }
    }

    // 2. Parse text content from content array
    if let Some(content_array) = result.get("content").and_then(|v| v.as_array()) {
        for entry in content_array {
            if let Some(text) = entry.get("text").and_then(|t| t.as_str()) {
                let trimmed = text.trim();
                if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                    || (trimmed.starts_with('[') && trimmed.ends_with(']'))
                {
                    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                        return Ok(parsed);
                    }
                }
                return Ok(Value::String(text.to_string()));
            }
        }
    }

    // 3. Fallback to direct properties or result itself
    Ok(result.clone())
}

fn extract_content_text(result: &Value) -> Option<String> {
    let arr = result.get("content")?.as_array()?;
    let mut texts = Vec::new();
    for item in arr {
        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
            texts.push(text);
        }
    }
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_structured_content() {
        let input = json!({
            "structuredContent": { "name": "projects/123" }
        });
        let res = parse_tool_response(&input, Some("create_project")).unwrap();
        assert_eq!(res["name"], "projects/123");
    }

    #[test]
    fn test_parse_content_json() {
        let input = json!({
            "content": [{ "type": "text", "text": "{\"screens\": [\"s1\", \"s2\"]}" }]
        });
        let res = parse_tool_response(&input, Some("list_screens")).unwrap();
        assert_eq!(res["screens"][0], "s1");
    }

    #[test]
    fn test_parse_error_rate_limit() {
        let input = json!({
            "isError": true,
            "content": [{ "type": "text", "text": "Rate limit exceeded (429)" }]
        });
        let err = parse_tool_response(&input, Some("generate_screen")).unwrap_err();
        match err {
            StitchError::RateLimited { message, tool } => {
                assert!(message.contains("Rate limit"));
                assert_eq!(tool.as_deref(), Some("generate_screen"));
            }
            other => panic!("Unexpected error type: {other:?}"),
        }
    }
}
