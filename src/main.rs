// ============================================
// main.rs - Entry Point (CORRECTED)
// ============================================

use anyhow::Result;
use clap::{Parser, Subcommand};
use clide::{Bot, Config};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "clide")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "~/.clide/config.yaml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long)]
    log_level: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the bot
    Start,

    /// Test Gemini API connection
    TestGemini {
        /// Prompt to send
        prompt: String,
    },

    /// Execute SSH command
    Ssh {
        /// Host (user@host or user@host:port)
        host: String,
        /// Command to execute
        command: String,
    },

    /// Show system status
    Status,

    /// Initialize configuration
    Init,

    /// Show configuration
    Config {
        /// Show full config (including secrets)
        #[arg(long)]
        show_secrets: bool,
    },

    /// Update clide to latest version
    Update,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = cli.log_level.unwrap_or_else(|| "info".to_string());
    clide::logger::init(clide::logger::LoggerConfig {
        level: log_level,
        ..Default::default()
    })?;

    match cli.command {
        Commands::Start => {
            let config_path = expand_path(&cli.config);
            let config = Config::load(config_path)?;
            let bot = Bot::new(config).await?;
            bot.run().await?;
        }
        Commands::TestGemini { prompt } => {
            let config_path = expand_path(&cli.config);
            let config = Config::load(config_path)?;
            let client = clide::GeminiClient::new(
                config.gemini_api_key,
                "gemini-1.5-flash".to_string(),
                0.7,
                1024,
                "You are a helpful assistant.".to_string(),
            );
            println!("Sending prompt: {}", prompt);
            let response = client.generate(&prompt).await?;
            println!("\nGemini Response:\n{}", response);
        }
        Commands::Status => {
            // Re-using logic from bot.rs for the CLI status command
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            let load = sysinfo::System::load_average();
            println!("📊 System Status");
            println!("Load: {:.2}, {:.2}, {:.2}", load.one, load.five, load.fifteen);
            println!("Memory: {}/{} MB", sys.used_memory() / 1024 / 1024, sys.total_memory() / 1024 / 1024);
        }
        Commands::Init => {
            init_config(&cli.config)?;
        }
        _ => {
            println!("Command not yet implemented or use --help");
        }
    }

    Ok(())
}

fn init_config(path: &PathBuf) -> Result<()> {
    let expanded_path = expand_path(path);
    if expanded_path.exists() {
        println!("⚠️  Config file already exists: {:?}", expanded_path);
        return Ok(());
    }
    if let Some(parent) = expanded_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Simple placeholder for init
    let example = "gemini_api_key: \"YOUR_KEY\"\nsignal_number: \"+123456789\"\nauthorized_numbers: [\"+123456789\"]";
    std::fs::write(&expanded_path, example)?;
    println!("✅ Config initialized at {:?}", expanded_path);
    Ok(())
}

fn expand_path(path: &PathBuf) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            let path_str = path.to_string_lossy();
            return PathBuf::from(path_str.replacen("~", &home.to_string_lossy(), 1));
        }
    }
    path.clone()
}
