use dotenvy::from_path;
use std::env;
use std::path::PathBuf;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config_path = PathBuf::from(env::var("HOME").unwrap_or_default());
    config_path.push(".config/clide/config.env");
    let _ = from_path(config_path);

    let api_key = env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        eprintln!("{}", "❌ Error: API Key not found!".red());
        eprintln!("Please add: {} to {}", "GEMINI_API_KEY=your_key".yellow(), "~/.config/clide/config.env".cyan());
        std::process::exit(1);
    });

    println!("{}", "✨ Clide is active. Type 'exit' to quit.".bright_green());

    loop {
        print!("{}", "user > ".bright_blue());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let prompt = input.trim();

        if prompt == "exit" || prompt == "quit" { break; }
        if prompt.is_empty() { continue; }

        let client = Client::new();
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}", api_key);

        let body = json!({"contents": [{"parts": [{"text": prompt}]}]});

        let res = client.post(url).json(&body).send().await?;
        let json: serde_json::Value = res.json().await?;

        if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
            println!("\n{}\n", text.white());
        } else {
            println!("{}", "❌ Could not get a response. Check your API key or connection.".red());
        }
    }
    Ok(())
}
