use dotenvy::from_path;
use std::env;
use std::path::PathBuf;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use colored::*;

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

    // Default: interactive REPL mode
    run_repl().await
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

/// Signal bot mode: poll signal-cli, reply via Gemini
async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    use clide::bot::Bot;
    use clide::config::Config;

    println!("{}", "Starting Clide Matrix bot...".bright_green());

    let config = Config::load().map_err(|e| {
        eprintln!("{} {}", "Config error:".red(), e);
        eprintln!(
            "Copy {} to {} and fill in Matrix credentials",
            "config.example.yaml".yellow(),
            "~/.clide/config.yaml".cyan()
        );
        e
    })?;

    let mut bot = Bot::new(config)?;
    bot.start().await?;

    Ok(())
}

/// Call Gemini API and return text response
async fn call_gemini(api_key: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.0-flash:generateContent?key={}",
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
