use dotenvy::from_path;
use std::env;
use std::path::PathBuf;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load config from ~/.config/clide/config.env
    let mut config_path = PathBuf::from(env::var("HOME").unwrap_or_default());
    config_path.push(".config/clide/config.env");
    let _ = from_path(config_path);

    // 2. Get API Key
    let api_key = env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        eprintln!("❌ Error: GEMINI_API_KEY not found in ~/.config/clide/config.env");
        std::process::exit(1);
    });

    println!("✨ Clide is ready! Type your prompt:");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let prompt = input.trim();

        if prompt == "exit" || prompt == "quit" { break; }
        if prompt.is_empty() { continue; }

        // 3. Call Gemini API
        let client = Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={}",
            api_key
        );

        let body = json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }]
        });

        let res = client.post(url)
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // 4. Print Response
        if let Some(text) = res["candidates"][0]["content"]["parts"][0]["text"].as_str() {
            println!("\n{}\n", text);
        } else {
            println!("❌ Error: Unexpected response format from API.");
        }
    }

    Ok(())
}
