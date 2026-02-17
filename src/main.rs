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
                config.get_model(),
                0.7,
                1024,
                "You are a helpful assistant.".to_string(),
            );
            info!("Sending prompt to Gemini...");
            let response = client.generate(&prompt).await?;
            println!("\nGemini Response:\n{}", response);
        }
        Commands::Status => {
            // FIXED: sysinfo v0.31 uses static methods for load and uptime
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            
            let load = sysinfo::System::load_average();
            let uptime = sysinfo::System::uptime();
            let used_mem = sys.used_memory() / 1024 / 1024;
            let total_mem = sys.total_memory() / 1024 / 1024;

            println!("📊 **System Status**");
            println!("• Load: {:.2}, {:.2}, {:.2}", load.one, load.five, load.fifteen);
            println!("• Memory: {}/{} MB", used_mem, total_mem);
            println!("• Uptime: {}h {}m", uptime / 3600, (uptime % 3600) / 60);
        }
        Commands::Init => {
            init_config(&cli.config)?;
        }
        Commands::Config { show_secrets } => {
            let config_path = expand_path(&cli.config);
            let config = Config::load(config_path)?;
            if show_secrets {
                println!("{:#?}", config);
            } else {
                let mut safe_config = config.clone();
                safe_config.gemini_api_key = "********".to_string();
                println!("{:#?}", safe_config);
            }
        }
        _ => {
            println!("Command not yet implemented. Use --help for available commands.");
        }
    }

    Ok(())
}

fn init_config(path: &PathBuf) -> Result<()> {
    let expanded_path = expand_path(path);
    if expanded_path.exists() {
        println!("⚠️  Config file already exists at {:?}", expanded_path);
        return Ok(());
    }
    if let Some(parent) = expanded_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Create a basic config template
    let template = r#"gemini_api_key: "YOUR_API_KEY"
signal_number: "+123456789"
authorized_numbers: ["+123456789"]
logging:
  level: "info"
  json: false
"#;
    std::fs::write(&expanded_path, template)?;
    println!("✅ Configuration initialized at {:?}", expanded_path);
    Ok(())
}

fn expand_path(path: &PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return PathBuf::from(path_str.replacen('~', &home.to_string_lossy(), 1));
        }
    }
    path.clone()
}
