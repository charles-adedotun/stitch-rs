//! Canonical data types for Google Stitch API interactions.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Screen image asset representation with raw bytes and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenImage {
    /// Raw binary image bytes.
    #[serde(skip_serializing, skip_deserializing)]
    pub bytes: Vec<u8>,
    /// Base64-encoded image string.
    pub base64: String,
    /// MIME type (e.g., "image/png").
    pub mime_type: String,
    /// Optional image pixel width.
    pub width: Option<u32>,
    /// Optional image pixel height.
    pub height: Option<u32>,
}

/// Metadata record for downloaded screen assets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadedAsset {
    pub screen_id: String,
    pub title: String,
    pub html_path: Option<String>,
    pub image_path: Option<String>,
}

/// Supported device form factors for Stitch screen generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceType {
    DeviceTypeUnspecified,
    Mobile,
    Desktop,
    Tablet,
    Agnostic,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceTypeUnspecified => "DEVICE_TYPE_UNSPECIFIED",
            Self::Mobile => "MOBILE",
            Self::Desktop => "DESKTOP",
            Self::Tablet => "TABLET",
            Self::Agnostic => "AGNOSTIC",
        }
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DeviceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "MOBILE" => Ok(Self::Mobile),
            "DESKTOP" => Ok(Self::Desktop),
            "TABLET" => Ok(Self::Tablet),
            "AGNOSTIC" => Ok(Self::Agnostic),
            "DEVICE_TYPE_UNSPECIFIED" | "UNSPECIFIED" => Ok(Self::DeviceTypeUnspecified),
            other => Err(format!("Unknown device type: {other}")),
        }
    }
}

/// Generative AI models supported by Stitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelId {
    ModelIdUnspecified,
    Gemini38Flash,
    Gemini35FlashLite,
}

impl ModelId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelIdUnspecified => "MODEL_ID_UNSPECIFIED",
            Self::Gemini38Flash => "GEMINI_3_8_FLASH",
            Self::Gemini35FlashLite => "GEMINI_3_5_FLASH_LITE",
        }
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ModelId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_uppercase().replace(['-', '.'], "_");
        match normalized.as_str() {
            "GEMINI_3_8_FLASH" | "3_8_FLASH" => Ok(Self::Gemini38Flash),
            "GEMINI_3_5_FLASH_LITE" | "3_5_FLASH_LITE" | "3_5_LITE" => Ok(Self::Gemini35FlashLite),
            "MODEL_ID_UNSPECIFIED" | "UNSPECIFIED" => Ok(Self::ModelIdUnspecified),
            other => Err(format!("Unknown model ID: {other}")),
        }
    }
}

/// LLM Tool output export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolFormat {
    Gemini,
    OpenAi,
    Mcp,
}

impl FromStr for ToolFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gemini" => Ok(Self::Gemini),
            "openai" | "anthropic" | "claude" => Ok(Self::OpenAi),
            "mcp" => Ok(Self::Mcp),
            other => Err(format!(
                "Unknown tool format: {other}. Expected gemini, openai, or mcp"
            )),
        }
    }
}

impl fmt::Display for ToolFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gemini => write!(f, "gemini"),
            Self::OpenAi => write!(f, "openai"),
            Self::Mcp => write!(f, "mcp"),
        }
    }
}
