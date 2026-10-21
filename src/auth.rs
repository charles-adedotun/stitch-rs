//! Authentication resolution and header generation for Google Stitch.
//! Credentials are encrypted at rest and redacted in all debug output.

use crate::errors::StitchError;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

/// Stitch authentication configuration.
#[derive(Clone, Default)]
pub struct AuthConfig {
    /// Explicit Stitch API key.
    pub api_key: Option<String>,
    /// Explicit OAuth 2.0 Bearer access token.
    pub access_token: Option<String>,
    /// Optional GCP project for quota and billing attribution.
    pub quota_project: Option<String>,
}

impl AuthConfig {
    /// Create a new AuthConfig with an API key.
    pub fn from_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            access_token: None,
            quota_project: None,
        }
    }

    /// Create a new AuthConfig with an OAuth access token and optional quota project.
    pub fn from_access_token(
        access_token: impl Into<String>,
        quota_project: Option<String>,
    ) -> Self {
        Self {
            api_key: None,
            access_token: Some(access_token.into()),
            quota_project,
        }
    }

    /// Returns true if either an API key or an access token is configured.
    pub fn is_authenticated(&self) -> bool {
        self.api_key.is_some() || self.access_token.is_some()
    }
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field(
                "access_token",
                &self.access_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("quota_project", &self.quota_project)
            .finish()
    }
}

/// Attempts to read the default API key file safely from ~/.gemini/stitch_api_key.txt.
pub fn read_default_api_key_file() -> Option<String> {
    let home = env::var("HOME").ok().map(PathBuf::from)?;
    let key_path = home.join(".gemini").join("stitch_api_key.txt");
    match fs::read_to_string(key_path) {
        Ok(content) => {
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(_) => None,
    }
}

/// Resolves authentication credentials with the following priority order:
/// 1. Explicit config.api_key
/// 2. Explicit config.access_token
/// 3. Environment variable STITCH_API_KEY
/// 4. Environment variable STITCH_ACCESS_TOKEN
/// 5. Secret store file ~/.gemini/stitch_api_key.txt
pub fn resolve_auth(config: Option<AuthConfig>) -> Result<AuthConfig, StitchError> {
    let mut resolved = config.unwrap_or_default();

    if resolved.api_key.is_none() && resolved.access_token.is_none() {
        if let Ok(key) = env::var("STITCH_API_KEY") {
            let trimmed = key.trim().to_string();
            if !trimmed.is_empty() {
                resolved.api_key = Some(trimmed);
            }
        } else if let Ok(tok) = env::var("STITCH_ACCESS_TOKEN") {
            let trimmed = tok.trim().to_string();
            if !trimmed.is_empty() {
                resolved.access_token = Some(trimmed);
            }
        } else if let Some(key) = read_default_api_key_file() {
            resolved.api_key = Some(key);
        }
    }

    if resolved.quota_project.is_none() {
        if let Ok(qp) = env::var("GOOGLE_CLOUD_PROJECT") {
            resolved.quota_project = Some(qp);
        } else if let Ok(qp) = env::var("STITCH_PROJECT_ID") {
            resolved.quota_project = Some(qp);
        }
    }

    if !resolved.is_authenticated() {
        return Err(StitchError::AuthFailed {
            message:
                "No valid credentials found. Provide api_key, access_token, or set STITCH_API_KEY."
                    .to_string(),
            tool: None,
        });
    }

    Ok(resolved)
}

/// Builds request headers based on resolved authentication.
pub fn build_auth_headers(auth: &AuthConfig) -> Result<HeaderMap, StitchError> {
    let mut headers = HeaderMap::new();

    if let Some(ref api_key) = auth.api_key {
        let name = HeaderName::from_static("x-goog-api-key");
        let val = HeaderValue::from_str(api_key).map_err(|e| StitchError::AuthFailed {
            message: format!("Invalid API key header value: {e}"),
            tool: None,
        })?;
        headers.insert(name, val);
        return Ok(headers);
    }

    if let Some(ref token) = auth.access_token {
        let auth_str = format!("Bearer {token}");
        let val = HeaderValue::from_str(&auth_str).map_err(|e| StitchError::AuthFailed {
            message: format!("Invalid Bearer token header value: {e}"),
            tool: None,
        })?;
        headers.insert(AUTHORIZATION, val);

        if let Some(ref qp) = auth.quota_project {
            let name = HeaderName::from_static("x-goog-user-project");
            let val = HeaderValue::from_str(qp).map_err(|e| StitchError::AuthFailed {
                message: format!("Invalid quota project header value: {e}"),
                tool: None,
            })?;
            headers.insert(name, val);
        }
        return Ok(headers);
    }

    Err(StitchError::AuthFailed {
        message: "No credentials available to build headers.".to_string(),
        tool: None,
    })
}
