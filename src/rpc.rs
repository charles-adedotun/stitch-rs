//! JSON-RPC 2.0 transport client for Google Stitch MCP Server.

use crate::auth::{build_auth_headers, resolve_auth, AuthConfig};
use crate::errors::StitchError;
use crate::parser::{create_stitch_error, parse_tool_response};
use crate::schema_repair::repair_tool_schemas;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use serde_json::{json, Value};
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

pub const DEFAULT_MCP_URL: &str = "https://stitch.googleapis.com/mcp";
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Low-level JSON-RPC 2.0 client for Google Stitch.
#[derive(Clone)]
pub struct StitchRpcClient {
    pub base_url: String,
    pub auth: AuthConfig,
    pub timeout: Duration,
    pub client: reqwest::Client,
}

impl fmt::Debug for StitchRpcClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StitchRpcClient")
            .field("base_url", &self.base_url)
            .field("auth", &self.auth)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl StitchRpcClient {
    /// Creates a new StitchRpcClient with automatic credential resolution.
    pub fn new(auth: Option<AuthConfig>) -> Result<Self, StitchError> {
        let resolved = resolve_auth(auth)?;
        Self::with_options(
            DEFAULT_MCP_URL,
            resolved,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )
    }

    /// Creates a StitchRpcClient with explicit URL, authentication, and timeout.
    pub fn with_options(
        base_url: impl Into<String>,
        auth: AuthConfig,
        timeout: Duration,
    ) -> Result<Self, StitchError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| StitchError::Network(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            base_url: base_url.into(),
            auth,
            timeout,
            client,
        })
    }

    /// Builds common and authenticated request headers.
    pub fn build_headers(&self) -> Result<HeaderMap, StitchError> {
        let mut headers = build_auth_headers(&self.auth)?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        Ok(headers)
    }

    /// Sends a JSON-RPC 2.0 request to the Stitch MCP server.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, StitchError> {
        let id = Uuid::new_v4().to_string();
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let headers = self.build_headers()?;

        let response = self
            .client
            .post(&self.base_url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(create_stitch_error(
                &error_text,
                Some(method),
                Some(status.as_u16()),
            ));
        }

        let body: Value = response.json().await?;

        if let Some(err_obj) = body.get("error") {
            let code = err_obj.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let msg = err_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("JSON-RPC error")
                .to_string();
            let data = err_obj.get("data").cloned();

            return Err(StitchError::Rpc {
                code,
                message: msg,
                data,
            });
        }

        Ok(body.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Executes a tool via `tools/call` and parses the response.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, StitchError> {
        let params = json!({
            "name": name,
            "arguments": args,
        });

        let raw_result = self.request("tools/call", params).await?;
        parse_tool_response(&raw_result, Some(name))
    }

    /// Fetches all available tools from the server with schemas repaired.
    pub async fn list_tools(&self) -> Result<Vec<Value>, StitchError> {
        let result = self.request("tools/list", json!({})).await?;
        let mut tools = match result.get("tools") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };

        repair_tool_schemas(&mut tools);
        Ok(tools)
    }
}
