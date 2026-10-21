//! High-performance, safe, idiomatic Rust client library for Google Stitch.
#![forbid(unsafe_code)]
//!
//! # Architecture
//! Follows the modular composition root pattern with single-responsibility submodules:
//! - [`agent::StitchAgent`]: Primary high-level facade for agents.
//! - [`rpc::StitchRpcClient`]: JSON-RPC 2.0 transport client.
//! - [`auth::AuthConfig`]: Secure credential resolution with secret redaction.
//! - [`schema_repair`]: Recursive schema repair and dereferencing for LLM function calling.
//! - [`adapters`]: Formatters for Google Gemini, OpenAI/Claude, and MCP.
//! - [`fife`]: Google Frontend Image Format Engine sizing.

pub mod adapters;
pub mod agent;
pub mod assets;
pub mod auth;
pub mod errors;
pub mod fife;
pub mod parser;
pub mod rpc;
pub mod schema_repair;
pub mod schemas;
pub mod types;

// Re-exports for clean library consumption
pub use adapters::{to_gemini_tools, to_mcp_tools, to_openai_tools};
pub use agent::StitchAgent;
pub use assets::{download_assets, get_screen_code, get_screen_image, sanitize_filename};
pub use auth::{build_auth_headers, resolve_auth, AuthConfig};
pub use errors::StitchError;
pub use fife::{apply_fife_sizing, build_fife_suffix};
pub use parser::{create_stitch_error, parse_tool_response};
pub use rpc::StitchRpcClient;
pub use schema_repair::{dereference_schema, repair_schema, repair_tool_schemas};
pub use schemas::get_builtin_tools;
pub use types::{DeviceType, DownloadedAsset, ModelId, ScreenImage, ToolFormat};
