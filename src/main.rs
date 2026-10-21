//! CLI entrypoint for Google Stitch (`stitch-cli`).
#![forbid(unsafe_code)]

use clap::{Args, Parser, Subcommand};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use stitch_rs::{AuthConfig, StitchAgent, ToolFormat};

#[derive(Parser, Debug)]
#[command(
    name = "stitch-cli",
    about = "Google Stitch High-Performance CLI & Agent Toolkit"
)]
struct Cli {
    /// Explicit Stitch API key (overrides STITCH_API_KEY)
    #[arg(long, global = true)]
    api_key: Option<String>,

    /// Explicit OAuth 2.0 access token (overrides STITCH_ACCESS_TOKEN)
    #[arg(long, global = true)]
    access_token: Option<String>,

    /// Google Cloud project ID for quota attribution
    #[arg(long, global = true)]
    quota_project: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Export tool declarations formatted for LLMs (Gemini, OpenAI, or MCP)
    Tools {
        /// Output schema format [default: mcp]
        #[arg(long, default_value = "mcp")]
        format: String,
    },

    /// Project management operations
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    /// Screen operations: generate, inspect, extract code, and screenshots
    Screen {
        #[command(subcommand)]
        command: ScreenCommands,
    },

    /// Raw JSON-RPC tool invocation escape hatch
    Call {
        /// Tool name (e.g. create_project, list_screens)
        tool: String,
        /// JSON arguments string
        #[arg(long, default_value = "{}")]
        args: String,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCommands {
    /// Create a new Stitch project
    Create {
        /// Title of the project
        title: String,
    },
    /// List Stitch projects
    List {
        /// Optional filter query (e.g. view=owned)
        #[arg(long)]
        filter: Option<String>,
    },
    /// Get details of a project by name or ID
    Get {
        /// Resource name (projects/{id}) or project ID
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum ScreenCommands {
    /// Generate a new screen from a text prompt
    Generate(ScreenGenerateArgs),
    /// Retrieve raw HTML code for a screen
    Code {
        /// Project ID
        #[arg(long)]
        project: String,
        /// Screen ID
        #[arg(long)]
        screen: String,
    },
    /// Retrieve screenshot image with optional FIFE sizing
    Image(ScreenImageArgs),
    /// List all screens within a project
    List {
        /// Project ID
        #[arg(long)]
        project: String,
    },
}

#[derive(Args, Debug)]
struct ScreenGenerateArgs {
    /// Project ID
    #[arg(long)]
    project: String,
    /// Prompt describing the screen
    #[arg(long)]
    prompt: String,
    /// Target device type (mobile, desktop, tablet, agnostic)
    #[arg(long)]
    device: Option<String>,
    /// Model to use for generation
    #[arg(long)]
    model: Option<String>,
}

#[derive(Args, Debug)]
struct ScreenImageArgs {
    /// Project ID
    #[arg(long)]
    project: String,
    /// Screen ID
    #[arg(long)]
    screen: String,
    /// Optional dynamic FIFE image width
    #[arg(long)]
    width: Option<u32>,
    /// Optional dynamic FIFE image height
    #[arg(long)]
    height: Option<u32>,
    /// Optional local path to write image file
    #[arg(long)]
    out: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let auth = if cli.api_key.is_some() || cli.access_token.is_some() || cli.quota_project.is_some()
    {
        Some(AuthConfig {
            api_key: cli.api_key,
            access_token: cli.access_token,
            quota_project: cli.quota_project,
        })
    } else {
        None
    };

    match run_command(cli.command, auth).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run_command(
    command: Commands,
    auth: Option<AuthConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Tools { format } => {
            let fmt = format.parse::<ToolFormat>()?;
            let agent = StitchAgent::new(auth.or_else(|| Some(AuthConfig::default()))).ok();
            let tools = match fmt {
                ToolFormat::Gemini => match agent {
                    Some(a) => a.to_gemini_tools(None),
                    None => stitch_rs::to_gemini_tools(None, None),
                },
                ToolFormat::OpenAi => match agent {
                    Some(a) => a.to_openai_tools(None),
                    None => stitch_rs::to_openai_tools(None, None),
                },
                ToolFormat::Mcp => match agent {
                    Some(a) => a.to_mcp_tools(None),
                    None => stitch_rs::to_mcp_tools(None, None),
                },
            };
            println!("{}", serde_json::to_string_pretty(&tools)?);
        }

        Commands::Project { command } => {
            let agent = StitchAgent::new(auth)?;
            match command {
                ProjectCommands::Create { title } => {
                    let res = agent.create_project(Some(&title)).await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
                ProjectCommands::List { filter } => {
                    let res = agent.list_projects(filter.as_deref()).await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
                ProjectCommands::Get { name } => {
                    let res = agent.get_project(&name).await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
            }
        }

        Commands::Screen { command } => {
            let agent = StitchAgent::new(auth)?;
            match command {
                ScreenCommands::Generate(args) => {
                    let res = agent
                        .generate_screen(
                            &args.project,
                            &args.prompt,
                            args.device.as_deref(),
                            args.model.as_deref(),
                        )
                        .await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
                ScreenCommands::Code { project, screen } => {
                    let code = agent.get_screen_code(&project, &screen).await?;
                    println!("{code}");
                }
                ScreenCommands::Image(args) => {
                    let img = agent
                        .get_screen_image(&args.project, &args.screen, args.width, args.height)
                        .await?;
                    if let Some(out_path) = args.out {
                        fs::write(&out_path, &img.bytes)?;
                        eprintln!("Screenshot saved to {}", out_path.display());
                    } else {
                        println!("{}", img.base64);
                    }
                }
                ScreenCommands::List { project } => {
                    let res = agent.list_screens(&project).await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
            }
        }

        Commands::Call { tool, args } => {
            let agent = StitchAgent::new(auth)?;
            let parsed_args: Value = serde_json::from_str(&args)?;
            let res = agent.call_tool(&tool, parsed_args).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
    }

    Ok(())
}
