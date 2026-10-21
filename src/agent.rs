//! High-level StitchAgent facade for autonomous AI agents and applications.

use crate::adapters::{to_gemini_tools, to_mcp_tools, to_openai_tools};
use crate::assets::{download_assets, get_screen_code, get_screen_image};
use crate::auth::AuthConfig;
use crate::errors::StitchError;
use crate::rpc::StitchRpcClient;
use crate::types::{DownloadedAsset, ScreenImage};
use serde_json::{json, Value};
use std::path::Path;

/// Primary programmatic facade for Google Stitch operations.
#[derive(Debug, Clone)]
pub struct StitchAgent {
    pub rpc: StitchRpcClient,
}

impl StitchAgent {
    /// Creates a new StitchAgent with default or resolved credentials.
    pub fn new(auth: Option<AuthConfig>) -> Result<Self, StitchError> {
        let rpc = StitchRpcClient::new(auth)?;
        Ok(Self { rpc })
    }

    /// Wraps an existing StitchRpcClient instance.
    pub fn with_client(rpc: StitchRpcClient) -> Self {
        Self { rpc }
    }

    // --- High-Level Project Operations ---

    /// Creates a new Stitch project.
    pub async fn create_project(&self, title: Option<&str>) -> Result<Value, StitchError> {
        let args = match title {
            Some(t) => json!({ "title": t }),
            None => json!({}),
        };
        self.rpc.call_tool("create_project", args).await
    }

    /// Lists all accessible Stitch projects with an optional view filter.
    pub async fn list_projects(&self, filter: Option<&str>) -> Result<Value, StitchError> {
        let args = match filter {
            Some(f) => json!({ "filter": f }),
            None => json!({}),
        };
        self.rpc.call_tool("list_projects", args).await
    }

    /// Retrieves project details by resource name ("projects/{id}") or ID.
    pub async fn get_project(&self, name_or_id: &str) -> Result<Value, StitchError> {
        let name = if name_or_id.starts_with("projects/") {
            name_or_id.to_string()
        } else {
            format!("projects/{name_or_id}")
        };
        self.rpc
            .call_tool("get_project", json!({ "name": name }))
            .await
    }

    // --- Screen Operations ---

    /// Lists all screens in a given project.
    pub async fn list_screens(&self, project_id: &str) -> Result<Value, StitchError> {
        let clean = project_id.trim_start_matches("projects/");
        self.rpc
            .call_tool("list_screens", json!({ "projectId": clean }))
            .await
    }

    /// Retrieves screen details by project ID and screen ID, or full resource name.
    pub async fn get_screen(
        &self,
        project_id: &str,
        screen_id: Option<&str>,
    ) -> Result<Value, StitchError> {
        let resource_name = match screen_id {
            Some(sid) => {
                let clean_p = project_id.trim_start_matches("projects/");
                let clean_s = sid.trim_start_matches("screens/");
                format!("projects/{clean_p}/screens/{clean_s}")
            }
            None => project_id.to_string(),
        };
        self.rpc
            .call_tool("get_screen", json!({ "name": resource_name }))
            .await
    }

    /// Generates a new screen within a project from a text prompt.
    pub async fn generate_screen(
        &self,
        project_id: &str,
        prompt: &str,
        device_type: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<Value, StitchError> {
        let clean = project_id.trim_start_matches("projects/");
        let mut args = json!({
            "projectId": clean,
            "prompt": prompt,
        });

        if let Some(dev) = device_type {
            args["deviceType"] = Value::String(dev.to_uppercase());
        }
        if let Some(model) = model_id {
            args["modelId"] = Value::String(model.to_string());
        }

        self.rpc.call_tool("generate_screen_from_text", args).await
    }

    /// Edits existing screens within a project using an instruction prompt.
    pub async fn edit_screens(
        &self,
        project_id: &str,
        screen_ids: &[&str],
        prompt: &str,
        device_type: Option<&str>,
    ) -> Result<Value, StitchError> {
        let clean_p = project_id.trim_start_matches("projects/");
        let clean_screens: Vec<&str> = screen_ids
            .iter()
            .map(|s| s.trim_start_matches("screens/"))
            .collect();

        let mut args = json!({
            "projectId": clean_p,
            "selectedScreenIds": clean_screens,
            "prompt": prompt,
        });

        if let Some(dev) = device_type {
            args["deviceType"] = Value::String(dev.to_uppercase());
        }

        self.rpc.call_tool("edit_screens", args).await
    }

    /// Generates design variants of existing screens within a project.
    pub async fn generate_variants(
        &self,
        project_id: &str,
        screen_ids: &[&str],
        prompt: &str,
        variant_options: &Value,
    ) -> Result<Value, StitchError> {
        let clean_p = project_id.trim_start_matches("projects/");
        let clean_screens: Vec<&str> = screen_ids
            .iter()
            .map(|s| s.trim_start_matches("screens/"))
            .collect();

        let args = json!({
            "projectId": clean_p,
            "selectedScreenIds": clean_screens,
            "prompt": prompt,
            "variantOptions": variant_options,
        });

        self.rpc.call_tool("generate_variants", args).await
    }

    /// Retrieves raw HTML code for a specific screen.
    pub async fn get_screen_code(
        &self,
        project_id: &str,
        screen_id: &str,
    ) -> Result<String, StitchError> {
        get_screen_code(&self.rpc, project_id, screen_id).await
    }

    /// Retrieves screenshot image data with optional dynamic FIFE sizing.
    pub async fn get_screen_image(
        &self,
        project_id: &str,
        screen_id: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<ScreenImage, StitchError> {
        get_screen_image(&self.rpc, project_id, screen_id, width, height).await
    }

    /// Downloads all project screens and screenshots to a local directory.
    pub async fn download_assets(
        &self,
        project_id: &str,
        output_dir: &Path,
    ) -> Result<Vec<DownloadedAsset>, StitchError> {
        download_assets(&self.rpc, project_id, output_dir).await
    }

    // --- Tool Calling & LLM Adapters ---

    /// Raw tool call escape hatch via JSON-RPC `tools/call`.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, StitchError> {
        self.rpc.call_tool(name, args).await
    }

    /// Fetches all available tools with repaired schemas.
    pub async fn list_tools(&self) -> Result<Vec<Value>, StitchError> {
        self.rpc.list_tools().await
    }

    /// Exports tools in Google Gemini function declarations format.
    pub fn to_gemini_tools(&self, include: Option<&[&str]>) -> Value {
        to_gemini_tools(None, include)
    }

    /// Exports tools in OpenAI / Claude function calling format.
    pub fn to_openai_tools(&self, include: Option<&[&str]>) -> Value {
        to_openai_tools(None, include)
    }

    /// Exports tools in Model Context Protocol (MCP) format.
    pub fn to_mcp_tools(&self, include: Option<&[&str]>) -> Value {
        to_mcp_tools(None, include)
    }
}
