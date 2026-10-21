//! Asset extraction and downloading utilities for screen code and screenshots.

use crate::errors::StitchError;
use crate::fife::apply_fife_sizing;
use crate::rpc::StitchRpcClient;
use crate::types::{DownloadedAsset, ScreenImage};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Sanitizes a title string for safe filesystem usage.
pub fn sanitize_filename(input: &str) -> String {
    let sanitized: String = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let non_consecutive = sanitized
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if non_consecutive.is_empty() {
        "untitled".to_string()
    } else {
        non_consecutive.chars().take(60).collect()
    }
}

/// Fetches the raw HTML code for a specific screen.
pub async fn get_screen_code(
    rpc: &StitchRpcClient,
    project_id: &str,
    screen_id: &str,
) -> Result<String, StitchError> {
    let clean_project = project_id.trim_start_matches("projects/");
    let clean_screen = screen_id.trim_start_matches("screens/");
    let screen_name = format!("projects/{clean_project}/screens/{clean_screen}");

    let screen = rpc
        .call_tool("get_screen", json!({ "name": screen_name }))
        .await?;

    let html_resource = screen
        .get("htmlCode")
        .or_else(|| screen.get("html_code"))
        .ok_or_else(|| StitchError::NotFound {
            message: format!("No HTML code resource found for screen {clean_screen}"),
            tool: Some("get_screen".to_string()),
        })?;

    if let Some(resource_name) = html_resource.get("name").and_then(|n| n.as_str()) {
        let file_res = rpc
            .call_tool("download_file", json!({ "name": resource_name }))
            .await?;

        if let Some(html) = file_res.get("html").and_then(|h| h.as_str()) {
            return Ok(html.to_string());
        }
        if let Some(text) = file_res.as_str() {
            return Ok(text.to_string());
        }
    }

    if let Some(download_url) = html_resource.get("downloadUrl").and_then(|u| u.as_str()) {
        let res = rpc.client.get(download_url).send().await?;
        if res.status().is_success() {
            return Ok(res.text().await?);
        }
    }

    Err(StitchError::General(format!(
        "Failed to retrieve HTML content for screen {clean_screen}"
    )))
}

/// Fetches screenshot image data for a specific screen with optional dynamic FIFE sizing.
pub async fn get_screen_image(
    rpc: &StitchRpcClient,
    project_id: &str,
    screen_id: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<ScreenImage, StitchError> {
    let clean_project = project_id.trim_start_matches("projects/");
    let clean_screen = screen_id.trim_start_matches("screens/");
    let screen_name = format!("projects/{clean_project}/screens/{clean_screen}");

    let screen = rpc
        .call_tool("get_screen", json!({ "name": screen_name }))
        .await?;

    let screenshot = screen
        .get("screenshot")
        .ok_or_else(|| StitchError::NotFound {
            message: format!("No screenshot found for screen {clean_screen}"),
            tool: Some("get_screen".to_string()),
        })?;

    // If FIFE sizing is requested and downloadUrl exists, fetch directly with dynamic size suffix
    if (width.is_some() || height.is_some()) && screenshot.get("downloadUrl").is_some() {
        if let Some(download_url) = screenshot.get("downloadUrl").and_then(|u| u.as_str()) {
            let sized_url = apply_fife_sizing(download_url, width, height);
            let res = rpc.client.get(&sized_url).send().await?;
            if res.status().is_success() {
                let mime = res
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("image/png")
                    .to_string();
                let bytes = res.bytes().await?.to_vec();
                let base64 = BASE64.encode(&bytes);

                return Ok(ScreenImage {
                    bytes,
                    base64,
                    mime_type: mime,
                    width,
                    height,
                });
            }
        }
    }

    // Default: fetch using download_file tool
    if let Some(resource_name) = screenshot.get("name").and_then(|n| n.as_str()) {
        let file_res = rpc
            .call_tool("download_file", json!({ "name": resource_name }))
            .await?;

        let base64_str = file_res
            .get("image")
            .and_then(|i| i.as_str())
            .or_else(|| file_res.as_str())
            .ok_or_else(|| {
                StitchError::General("Screenshot payload missing image data".to_string())
            })?;

        let bytes = BASE64.decode(base64_str).map_err(|e| {
            StitchError::General(format!("Failed to decode screenshot base64: {e}"))
        })?;

        let mime = file_res
            .get("image_metadata")
            .and_then(|m| m.get("mime_type"))
            .and_then(|m| m.as_str())
            .or_else(|| file_res.get("mimeType").and_then(|m| m.as_str()))
            .unwrap_or("image/png")
            .to_string();

        let w = file_res
            .get("image_metadata")
            .and_then(|m| m.get("width"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or(width);

        let h = file_res
            .get("image_metadata")
            .and_then(|m| m.get("height"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or(height);

        return Ok(ScreenImage {
            bytes,
            base64: base64_str.to_string(),
            mime_type: mime,
            width: w,
            height: h,
        });
    }

    Err(StitchError::General(format!(
        "Failed to retrieve screenshot for screen {clean_screen}"
    )))
}

/// Downloads all screen assets (HTML and screenshots) into a local destination directory.
pub async fn download_assets(
    rpc: &StitchRpcClient,
    project_id: &str,
    output_dir: &Path,
) -> Result<Vec<DownloadedAsset>, StitchError> {
    let clean_project = project_id.trim_start_matches("projects/");
    fs::create_dir_all(output_dir)?;

    let list_res = rpc
        .call_tool("list_screens", json!({ "projectId": clean_project }))
        .await?;

    let screens = match list_res.get("screens") {
        Some(Value::Array(arr)) => arr.clone(),
        _ => Vec::new(),
    };

    let mut manifest = Vec::new();

    for (idx, screen) in screens.iter().enumerate() {
        let name_str = screen.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let screen_id = name_str.rsplit('/').next().unwrap_or("unknown");
        let title = screen
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or(screen_id);

        let slug = sanitize_filename(title);
        let prefix = format!("{:02}_{slug}", idx + 1);

        let mut html_path = None;
        let mut image_path = None;

        // Download HTML
        if let Ok(code) = get_screen_code(rpc, clean_project, screen_id).await {
            let out_file = output_dir.join(format!("{prefix}.html"));
            if fs::write(&out_file, code).is_ok() {
                html_path = Some(out_file.to_string_lossy().to_string());
            }
        }

        // Download Screenshot
        if let Ok(img) = get_screen_image(rpc, clean_project, screen_id, None, None).await {
            let ext = if img.mime_type.contains("jpeg") || img.mime_type.contains("jpg") {
                "jpg"
            } else {
                "png"
            };
            let out_file = output_dir.join(format!("{prefix}.{ext}"));
            if fs::write(&out_file, &img.bytes).is_ok() {
                image_path = Some(out_file.to_string_lossy().to_string());
            }
        }

        manifest.push(DownloadedAsset {
            screen_id: screen_id.to_string(),
            title: title.to_string(),
            html_path,
            image_path,
        });
    }

    Ok(manifest)
}
