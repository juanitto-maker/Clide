use dotenvy::from_path;
use std::env;
use std::path::PathBuf;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use colored::*;

use clide::hosts::{self, HostEntry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env-based config (API key etc.) from ~/.config/clide/config.env
    let mut env_path = PathBuf::from(env::var("HOME").unwrap_or_default());
    env_path.push(".config/clide/config.env");
    let _ = from_path(&env_path);

    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("clide v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--bot" || a == "bot") {
        return run_bot().await;
    }

    // ── `clide host` subcommand ───────────────────────────────────────────────
    if args.get(1).map(|s| s.as_str()) == Some("host") {
        return run_host_cmd(&args[2..]).await;
    }

    // Default: interactive REPL mode
    run_repl().await
}

/// Handle `clide host <subcommand> [args]`
///
/// This runs LOCALLY only — host data never goes through the bot or AI.
///
/// Usage:
///   clide host list
///   clide host add <nickname> --ip <IP> --user <USER> [--key <PATH>] [--port <PORT>] [--notes <TEXT>]
///   clide host remove <nickname>
async fn run_host_cmd(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sub = args.get(0).map(|s| s.as_str()).unwrap_or("list");

    match sub {
        "list" => {
            let map = hosts::load()?;
            println!("{}", hosts::format_list(&map));
        }

        "add" => {
            let nickname = args.get(1).ok_or("Usage: clide host add <nickname> --ip <IP> --user <USER> [--key <PATH>] [--port <PORT>] [--notes <TEXT>]")?;
            let mut ip = String::new();
            let mut user = String::new();
            let mut key_path = format!(
                "{}/.ssh/id_ed25519",
                env::var("HOME").unwrap_or_default()
            );
            let mut port: u16 = 22;
            let mut notes = String::new();

            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--ip"    => { ip    = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                    "--user"  => { user  = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                    "--key"   => { key_path = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                    "--port"  => { port  = args.get(i+1).and_then(|p| p.parse().ok()).unwrap_or(22); i += 2; }
                    "--notes" => { notes = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                    _         => { i += 1; }
                }
            }

            if ip.is_empty() || user.is_empty() {
                eprintln!("{}", "Error: --ip and --user are required.".red());
                std::process::exit(1);
            }

            let entry = HostEntry { ip, user, key_path, port, notes };
            hosts::add(nickname, entry)?;
            println!("{}", format!("✅ Host '{}' saved to ~/.clide/hosts.yaml", nickname).green());
        }

        "remove" | "rm" => {
            let nickname = args.get(1).ok_or("Usage: clide host remove <nickname>")?;
            hosts::remove(nickname)?;
            println!("{}", format!("✅ Host '{}' removed.", nickname).green());
        }

        _ => {
            eprintln!("Unknown host subcommand: '{}'", sub);
            eprintln!("Usage: clide host list | add | remove");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Interactive REPL: type prompts, get Gemini replies
async fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        eprintln!("{}", "Error: GEMINI_API_KEY not set.".red());
        eprintln!("Add it to {}", "~/.config/clide/config.env".cyan());
        eprintln!("Or run the installer again.");
        std::process::exit(1);
    });

    println!("{}", "Clide ready. Type your message (or 'exit' to quit).".bright_green());

    loop {
        print!("{}", "you > ".bright_blue());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let prompt = input.trim();

        if prompt == "exit" || prompt == "quit" {
            break;
        }
        if prompt.is_empty() {
            continue;
        }

        match call_gemini(&api_key, prompt).await {
            Ok(response) => println!("\n{}\n", response.white()),
            Err(e) => eprintln!("{} {}", "Error:".red(), e),
        }
    }

    Ok(())
}

/// Bot mode: dispatch to Matrix, Telegram, or both based on config.platform
async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    use clide::bot::Bot;
    use clide::config::Config;
    use clide::telegram_bot::TelegramBot;

    let config = Config::load().map_err(|e| {
        eprintln!("{} {}", "Config error:".red(), e);
        eprintln!(
            "Copy {} to {} and fill in your credentials",
            "config.example.yaml".yellow(),
            "~/.clide/config.yaml".cyan()
        );
        e
    })?;

    match config.platform.as_str() {
        "telegram" => {
            println!("{}", "Starting Clide Telegram bot...".bright_green());
            let mut bot = TelegramBot::new(config)?;
            bot.start().await?;
        }
        "both" => {
            println!("{}", "Starting Clide on Matrix + Telegram...".bright_green());
            let config2 = config.clone();
            // Spawn Telegram in background task, run Matrix in foreground
            let tg_handle = tokio::spawn(async move {
                let mut bot = TelegramBot::new(config2)?;
                bot.start().await
            });
            let mut matrix_bot = Bot::new(config)?;
            tokio::select! {
                r = matrix_bot.start() => { r?; }
                r = tg_handle => { r??; }
            }
        }
        _ => {
            // Default: "matrix"
            println!("{}", "Starting Clide Matrix bot...".bright_green());
            let mut bot = Bot::new(config)?;
            bot.start().await?;
        }
    }

    Ok(())
}

/// Call Gemini API and return text response
async fn call_gemini(api_key: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    let body = json!({
        "contents": [{"parts": [{"text": prompt}]}]
    });

    let res = client.post(url).json(&body).send().await?;
    let json: serde_json::Value = res.json().await?;

    if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        Ok(text.to_string())
    } else if let Some(err) = json["error"]["message"].as_str() {
        Err(format!("Gemini API error: {}", err).into())
    } else {
        Err("No response from Gemini (check API key and network)".into())
    }
}
