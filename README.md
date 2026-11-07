# stitch-rs

> Rust SDK, CLI, and MCP client for Google Stitch UI generation.

[![CI](https://github.com/charles-adedotun/stitch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/charles-adedotun/stitch-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

---

## Highlights

- **Dual Auth & Secret Redaction**: Resolves `X-Goog-Api-Key` (config, `STITCH_API_KEY`, or `~/.gemini/stitch_api_key.txt`) or `Authorization: Bearer <token>` with optional `X-Goog-User-Project`. Custom `Debug` implementations redact keys and tokens.
- **Recursive Schema Cleanup**: Injects missing `$defs` (`ScreenInstance`, `SelectedScreenInstance`, `File`, `VariantOptions`) and dereferences inline schemas for Gemini function declarations.
- **Dynamic FIFE Image Sizing**: Rewrites Google Frontend Image Format Engine sizing suffixes (`=w{width}-h{height}`).
- **Structured Tool Parsing & Typed Errors**: Prioritizes `structuredContent`, falls back to parsing `content[0].text` as JSON, and maps errors into typed enums (`RateLimited`, `NotFound`, `AuthFailed`, `PermissionDenied`, `Rpc`, `Http`).
- **LLM Tool Adapters**: Exports tool definitions for Google Gemini (`functionDeclarations`), OpenAI/Claude (`tools`), and Model Context Protocol (`inputSchema`).

---

## Modules

| Module | Role |
|---|---|
| `src/lib.rs` | Public library exports |
| `src/agent.rs` | High-level `StitchAgent` client |
| `src/rpc.rs` | JSON-RPC 2.0 client for `https://stitch.googleapis.com/mcp` |
| `src/auth.rs` | API key and OAuth bearer token resolver with redacted `Debug` output |
| `src/errors.rs` | Error types and retry classification |
| `src/schema_repair.rs` | Recursive `$defs` injection and `$ref` flattening |
| `src/schemas.rs` | Built-in Stitch tool definitions |
| `src/adapters.rs` | Tool schema exporters for Gemini, OpenAI, and MCP |
| `src/assets.rs` | HTML extraction, screenshots, and asset downloads |
| `src/fife.rs` | FIFE image URL suffix parser and rewriter |
| `src/types.rs` | Data types (`ScreenImage`, `DeviceType`, `ModelId`) |
| `src/main.rs` | CLI binary (`stitch-cli`) |

---

## Library Usage

### 1. Initialization

```rust
use stitch_rs::{AuthConfig, StitchAgent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Automatically resolves credentials from environment or ~/.gemini/stitch_api_key.txt
    let agent = StitchAgent::new(None)?;

    // Or with explicit API key:
    // let agent = StitchAgent::new(Some(AuthConfig::from_api_key("API_KEY")))?;

    // Or with OAuth token and quota project:
    // let agent = StitchAgent::new(Some(AuthConfig::from_access_token("BEARER_TOKEN", Some("my-gcp-project".to_string()))))?;

    Ok(())
}
```

### 2. High-Level Agent Operations

```rust
use stitch_rs::StitchAgent;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = StitchAgent::new(None)?;

    // 1. Create a project
    let project = agent.create_project(Some("Executive Analytics")).await?;
    let project_id = project["name"].as_str().unwrap();

    // 2. Generate screen from text
    let screen = agent.generate_screen(
        project_id,
        "Modern dark-mode fintech dashboard with order book and trade stream",
        Some("DESKTOP"),
        Some("GEMINI_2_5_FLASH"),
    ).await?;

    // 3. Edit existing screens
    agent.edit_screens(
        project_id,
        &["screen_01"],
        "Change primary accent color to violet and increase chart resolution",
        Some("DESKTOP"),
    ).await?;

    // 4. Generate variants
    let variants = agent.generate_variants(
        project_id,
        &["screen_01"],
        "Explore minimal, high-density, and cards layouts",
        &json!({
            "creativeRange": "EXPLORE",
            "variantCount": 3
        }),
    ).await?;

    // 5. Retrieve HTML code
    let html = agent.get_screen_code(project_id, "screen_01").await?;

    // 6. Retrieve screenshot with dynamic FIFE sizing
    let image = agent.get_screen_image(project_id, "screen_01", Some(1280), Some(800)).await?;
    println!("Image bytes length: {}, MIME: {}", image.bytes.len(), image.mime_type);

    // 7. Call any underlying MCP tool directly
    let raw = agent.call_tool("list_design_systems", json!({ "projectId": project_id })).await?;

    Ok(())
}
```

### 3. LLM Tool Export Adapters

```rust
use stitch_rs::StitchAgent;

let agent = StitchAgent::new(None)?;

// Google Gemini SDK: dereferenced schemas (no $ref, no $defs, stripped vendor tags)
let gemini_tools = agent.to_gemini_tools(Some(&["create_project", "generate_screen_from_text"]));

// OpenAI / Anthropic Claude tool calling format
let openai_tools = agent.to_openai_tools(None);

// Standard MCP JSON schemas
let mcp_tools = agent.to_mcp_tools(None);
```

---

## CLI Binary (`stitch-cli`)

Compile or install the binary:

```bash
cargo build --release
./target/release/stitch-cli --help
```

### Commands

```bash
# 1. Export tool definitions for LLMs
stitch-cli tools --format gemini
stitch-cli tools --format openai
stitch-cli tools --format mcp

# 2. Project management
stitch-cli project create "Fintech Portal"
stitch-cli project list --filter "view=owned"
stitch-cli project get projects/<PROJECT_ID>

# 3. Screen operations
stitch-cli screen generate \
  --project <PROJECT_ID> \
  --prompt "Crypto exchange terminal with order book" \
  --device desktop

# 4. Fetch HTML code
stitch-cli screen code --project <PROJECT_ID> --screen screen_01

# 5. Fetch screenshot with dynamic FIFE sizing and save to disk
stitch-cli screen image \
  --project <PROJECT_ID> \
  --screen screen_01 \
  --width 1280 \
  --height 800 \
  --out /tmp/screen_01.png
```

---

## Verification & Testing

Run all unit, integration, and doc tests:

```bash
cargo check
cargo test
cargo build --release
```
