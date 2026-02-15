// ============================================
// main.rs - Entry Point
// ============================================
// CLI interface and main function

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

    // Load config
    let config = match Config::load(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            eprintln!("\n💡 Try running: clide init");
            std::process::exit(1);
        }
    };

    // Setup logging
    let log_level = cli.log_level.unwrap_or_else(|| config.logging.level.clone());
    let log_config = clide::logger::LoggerConfig {
        level: log_level,
        file_path: config.log_file_path(),
        json_format: config.logging.json_format,
        with_timestamps: config.logging.timestamps,
        with_caller: config.logging.caller_info,
    };

    let _guard = clide::logger::init(log_config)?;

    info!("Clide v{} starting...", clide::VERSION);

    // Execute command
    match cli.command {
        Commands::Start => {
            let bot = Bot::new(config);
            bot.start().await?;
        }

        Commands::TestGemini { prompt } => {
            let client = clide::GeminiClient::new(
                config.gemini_api_key.clone(),
                config.gemini.model.clone(),
                config.gemini.temperature,
                config.gemini.max_tokens,
                config.gemini.system_prompt.clone(),
            );

            println!("🤖 Sending to Gemini: {}", prompt);
            
            match client.generate(&prompt).await {
                Ok(response) => {
                    println!("\n✅ Response:\n{}", response);
                }
                Err(e) => {
                    eprintln!("\n❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Ssh { host, command } => {
            let ssh_client = clide::SshClient::new(
                config.ssh_timeout,
                config.ssh_verify_host_keys,
            );

            println!("🔗 Connecting to {}...", host);

            // Parse user@host
            let (user, hostname) = if let Some((u, h)) = host.split_once('@') {
                (u.to_string(), h.to_string())
            } else {
                ("root".to_string(), host)
            };

            match ssh_client
                .execute(
                    &hostname,
                    &user,
                    &command,
                    config.ssh_key_path().as_deref(),
                )
                .await
            {
                Ok(output) => {
                    if output.success() {
                        println!("\n✅ Output:\n{}", output.stdout);
                    } else {
                        println!("\n⚠️  Exit code: {}", output.exit_code);
                        println!("{}", output.output());
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Status => {
            use sysinfo::{System, SystemExt};

            let mut sys = System::new_all();
            sys.refresh_all();

            let uptime = sys.uptime();
            let total_mem = sys.total_memory();
            let used_mem = sys.used_memory();

            println!("🖥️  System Status");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Uptime:     {}h {}m", uptime / 3600, (uptime % 3600) / 60);
            println!("CPU cores:  {}", sys.cpus().len());
            println!(
                "Memory:     {:.1} GB / {:.1} GB ({:.1}% used)",
                used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
                total_mem as f64 / 1024.0 / 1024.0 / 1024.0,
                (used_mem as f64 / total_mem as f64) * 100.0
            );
            println!("Clide:      v{} ✅", clide::VERSION);
        }

        Commands::Init => {
            init_config(&cli.config)?;
        }

        Commands::Config { show_secrets } => {
            println!("📋 Configuration");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("File: {:?}", cli.config);
            println!("\nSignal number: {}", config.signal_number);
            
            if show_secrets {
                println!("Gemini API key: {}", config.gemini_api_key);
            } else {
                println!("Gemini API key: ****...{}", &config.gemini_api_key[config.gemini_api_key.len().saturating_sub(4)..]);
            }
            
            println!("\nSecurity:");
            println!("  Allow commands: {}", config.allow_commands);
            println!("  Require confirmation: {}", config.require_confirmation);
            println!("  Dry run: {}", config.dry_run);
            
            println!("\nLogging:");
            println!("  Level: {}", config.logging.level);
            println!("  File: {:?}", config.log_file_path());
        }

        Commands::Update => {
            println!("🔄 Checking for updates...");
            println!("Current version: v{}", clide::VERSION);
            println!("\n💡 To update, run:");
            println!("  curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash");
        }
    }

    Ok(())
}

/// Initialize configuration
fn init_config(path: &PathBuf) -> Result<()> {
    let expanded_path = expand_path(path);
    
    if expanded_path.exists() {
        println!("⚠️  Config file already exists: {:?}", expanded_path);
        print!("Overwrite? (y/N): ");
        
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Create parent directory
    if let Some(parent) = expanded_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write example config
    let example_config = include_str!("../config.example.yaml");
    std::fs::write(&expanded_path, example_config)?;

    println!("✅ Configuration initialized: {:?}", expanded_path);
    println!("\n📝 Next steps:");
    println!("  1. Edit the config: nano {:?}", expanded_path);
    println!("  2. Add your Gemini API key");
    println!("  3. Add your Signal number");
    println!("  4. Run: clide start");

    Ok(())
}

/// Expand ~ in path
fn expand_path(path: &PathBuf) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            let path_str = path.to_string_lossy();
            let expanded = path_str.replacen("~", &home.to_string_lossy(), 1);
            return PathBuf::from(expanded);
        }
    }
    path.clone()
}
