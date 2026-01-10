//! rat-squad: Multi-agent AI squad manager for ratterm
//!
//! This is the main entry point for the rat-squad ratterm extension.

use clap::{Parser, Subcommand};
use rat_squad::config::{default_config_path, Config, ConfigManager};
use rat_squad::error::Result;
use rat_squad::ratterm_client::{RattermClient, RattermClientConfig};
use rat_squad::state::create_shared_state;
use rat_squad::ui::{UiCommand, UiHandler};
use std::io::{BufRead, Write};
use tracing::info;

/// Maximum initialization retries (reserved for future use)
#[allow(dead_code)]
const MAX_INIT_RETRIES: usize = 3;

/// CLI arguments
#[derive(Parser)]
#[command(name = "rat-squad")]
#[command(author, version, about = "Multi-agent AI squad manager for ratterm")]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Subcommand to run
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand)]
enum Commands {
    /// Create a new agent session
    New {
        /// Session name
        name: String,
        /// Agent type (claude, aider, codex, gemini)
        #[arg(default_value = "claude")]
        agent: String,
        /// Enable yolo mode (auto-accept changes)
        #[arg(short, long)]
        yolo: bool,
    },
    /// Start a session
    Start {
        /// Session ID or name
        session: String,
    },
    /// Stop a session
    Stop {
        /// Session ID or name
        session: String,
    },
    /// Remove a session
    Remove {
        /// Session ID or name
        session: String,
    },
    /// List all sessions
    List,
    /// Run in interactive mode (default when run as extension)
    Interactive,
}

/// Initialize logging
fn init_logging() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rat_squad=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber).ok();
}

/// Load configuration from file or defaults
fn load_config(config_path: Option<&str>) -> Result<Config> {
    let path = config_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_config_path);

    let manager = ConfigManager::new(&path)?;
    manager.load()
}

/// Initialize the ratterm client from environment
fn init_client() -> Result<RattermClient> {
    let config = RattermClientConfig::from_env()?;
    RattermClient::new(config)
}

/// Run a single command
async fn run_command(handler: &UiHandler, command: UiCommand) -> Result<bool> {
    let is_quit = matches!(command, UiCommand::Quit);
    let output = handler.process(command).await?;
    println!("{output}");
    Ok(is_quit)
}

/// Run in interactive mode (REPL)
async fn run_interactive(handler: &UiHandler) -> Result<()> {
    println!("rat-squad v{} - Multi-agent AI squad manager", rat_squad::VERSION);
    println!("Type 'help' for available commands, 'quit' to exit.\n");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for _line_num in 0..usize::MAX {
        print!("rat-squad> ");
        stdout.flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match UiCommand::parse(input) {
            Ok(command) => {
                let should_quit = run_command(handler, command).await?;
                if should_quit {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }

    Ok(())
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let cli = Cli::parse();

    let config = load_config(cli.config.as_deref())?;
    info!("Loaded configuration from {:?}", default_config_path());

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let state = create_shared_state(config, &cwd)?;

    if let Ok(client) = init_client() {
        let mut state_lock = state.write().await;
        state_lock.init_client(client);
        drop(state_lock);

        if let Some(client) = state.read().await.client() {
            let _ = client.set_status("rat-squad: initialized").await;
            let _ = client
                .register_command("squad-new", "Create a new agent session")
                .await;
            let _ = client
                .register_command("squad-list", "List all agent sessions")
                .await;
        }
    }

    let handler = UiHandler::new(state);

    match cli.command {
        Some(Commands::New { name, agent, yolo }) => {
            let cmd = UiCommand::NewSession { name, agent, yolo };
            run_command(&handler, cmd).await?;
        }
        Some(Commands::Start { session }) => {
            let cmd = UiCommand::StartSession {
                session_id: session,
            };
            run_command(&handler, cmd).await?;
        }
        Some(Commands::Stop { session }) => {
            let cmd = UiCommand::StopSession {
                session_id: session,
            };
            run_command(&handler, cmd).await?;
        }
        Some(Commands::Remove { session }) => {
            let cmd = UiCommand::RemoveSession {
                session_id: session,
            };
            run_command(&handler, cmd).await?;
        }
        Some(Commands::List) => {
            run_command(&handler, UiCommand::ListSessions).await?;
        }
        Some(Commands::Interactive) | None => {
            run_interactive(&handler).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(["rat-squad", "list"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_new_command() {
        let cli = Cli::try_parse_from(["rat-squad", "new", "test-session", "claude", "--yolo"]);
        assert!(cli.is_ok());
    }
}
