use stitch_rs::{build_auth_headers, resolve_auth, AuthConfig, StitchError};

#[test]
fn test_auth_secret_redaction_in_debug() {
    let raw_key = "AIzaSySecretApiKey123456";
    let raw_token = "ya29.SecretBearerToken789";

    let auth_key = AuthConfig::from_api_key(raw_key);
    let debug_key = format!("{auth_key:?}");
    assert!(
        !debug_key.contains(raw_key),
        "Raw API key must not leak in Debug output!"
    );
    assert!(debug_key.contains("[REDACTED]"));

    let auth_token = AuthConfig::from_access_token(raw_token, Some("my-quota-project".to_string()));
    let debug_token = format!("{auth_token:?}");
    assert!(
        !debug_token.contains(raw_token),
        "Raw token must not leak in Debug output!"
    );
    assert!(debug_token.contains("[REDACTED]"));
    assert!(debug_token.contains("my-quota-project"));
}

#[test]
fn test_api_key_header_generation() {
    let auth = AuthConfig::from_api_key("test-key-abc");
    let headers = build_auth_headers(&auth).expect("headers generated");
    assert_eq!(headers.get("x-goog-api-key").unwrap(), "test-key-abc");
    assert!(headers.get("authorization").is_none());
}

#[test]
fn test_oauth_bearer_header_generation() {
    let auth = AuthConfig::from_access_token("test-bearer-xyz", Some("gcp-proj-1".to_string()));
    let headers = build_auth_headers(&auth).expect("headers generated");
    assert_eq!(
        headers.get("authorization").unwrap(),
        "Bearer test-bearer-xyz"
    );
    assert_eq!(headers.get("x-goog-user-project").unwrap(), "gcp-proj-1");
    assert!(headers.get("x-goog-api-key").is_none());
}

#[test]
fn test_missing_credentials_fails() {
    let auth = AuthConfig::default();
    let res = build_auth_headers(&auth);
    match res {
        Err(StitchError::AuthFailed { message, .. }) => {
            assert!(message.contains("No credentials"));
        }
        other => panic!("Expected AuthFailed error, got {other:?}"),
    }
}

#[test]
fn test_resolve_auth_explicit_priority() {
    let auth = AuthConfig {
        api_key: Some("explicit-key".to_string()),
        access_token: Some("explicit-token".to_string()),
        quota_project: None,
    };
    let resolved = resolve_auth(Some(auth)).expect("resolved");
    assert_eq!(resolved.api_key.as_deref(), Some("explicit-key"));
}
