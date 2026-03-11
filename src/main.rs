use dotenvy::from_path;
use std::env;
use std::path::PathBuf;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use colored::*;

use clide::hosts::{self, HostEntry};
use clide::pass_store;

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

    // ── `clide secret` subcommand ─────────────────────────────────────────────
    if args.get(1).map(|s| s.as_str()) == Some("secret") {
        return run_secret_cmd(&args[2..]).await;
    }

    // Default: interactive REPL mode
    run_repl().await
}

// ── Prompt helpers ────────────────────────────────────────────────────────────

fn prompt_with_default(label: &str, default: &str) -> Result<String, Box<dyn std::error::Error>> {
    if default.is_empty() {
        print!("  {}: ", label.bright_white());
    } else {
        print!("  {} [{}]: ", label.bright_white(), default.dimmed());
    }
    io::stdout().flush()?;
    let mut val = String::new();
    io::stdin().read_line(&mut val)?;
    let val = val.trim().to_string();
    Ok(if val.is_empty() { default.to_string() } else { val })
}

fn prompt_required(label: &str) -> Result<String, Box<dyn std::error::Error>> {
    loop {
        print!("  {}: ", label.bright_white());
        io::stdout().flush()?;
        let mut val = String::new();
        io::stdin().read_line(&mut val)?;
        let val = val.trim().to_string();
        if !val.is_empty() { return Ok(val); }
        eprintln!("  {} This field is required.", "▶".red());
    }
}

fn prompt_secret(label: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Use rpassword-style approach: if it's a terminal, hide input; otherwise read normally.
    // We don't pull in rpassword crate — just print the prompt and read.
    print!("  {} (hidden): ", label.bright_white());
    io::stdout().flush()?;
    // Disable echo via termios if available; graceful fallback otherwise.
    let val = read_secret_line()?;
    println!(); // newline after hidden input
    Ok(val)
}

/// Read a line without echo on Unix (best-effort; falls back to plain read).
fn read_secret_line() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = io::stdin().as_raw_fd();
        let mut termios = libc_termios(fd)?;
        let orig = termios;
        termios.c_lflag &= !libc_echo_flag();
        set_termios(fd, &termios).ok();
        let mut val = String::new();
        let result = io::stdin().read_line(&mut val);
        // Always restore, even on error
        set_termios(fd, &orig).ok();
        result?;
        return Ok(val.trim().to_string());
    }
    #[cfg(not(unix))]
    {
        let mut val = String::new();
        io::stdin().read_line(&mut val)?;
        return Ok(val.trim().to_string());
    }
}

#[cfg(unix)]
fn libc_termios(fd: i32) -> Result<libc::termios, Box<dyn std::error::Error>> {
    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::tcgetattr(fd, &mut t) };
    if r != 0 { return Err("tcgetattr failed".into()); }
    Ok(t)
}

#[cfg(unix)]
fn libc_echo_flag() -> libc::tcflag_t { libc::ECHO }

#[cfg(unix)]
fn set_termios(fd: i32, t: &libc::termios) -> Result<(), ()> {
    let r = unsafe { libc::tcsetattr(fd, libc::TCSANOW, t) };
    if r == 0 { Ok(()) } else { Err(()) }
}

/// Handle `clide host <subcommand> [args]`
///
/// This runs LOCALLY only — host data never goes through the bot or AI.
///
/// Usage:
///   clide host list
///   clide host add [<nickname>] [--ip <IP>] [--user <USER>] [--key <PATH>] [--port <PORT>] [--notes <TEXT>]
///   clide host remove <nickname>
async fn run_host_cmd(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sub = args.get(0).map(|s| s.as_str()).unwrap_or("list");

    match sub {
        "list" => {
            let map = hosts::load()?;
            println!("{}", hosts::format_list(&map));
        }

        "add" => {
            let home = env::var("HOME").unwrap_or_default();
            let default_key = format!("{}/.ssh/id_ed25519", home);

            // Check if flags were supplied (non-interactive mode).
            // If the next arg starts with '--' or no nickname given, use wizard.
            let has_flags = args.iter().any(|a| a.starts_with("--ip") || a.starts_with("--user"));
            let nickname_arg = args.get(1).filter(|a| !a.starts_with('-'));

            let (nickname, ip, user, key_path, port, notes) = if has_flags && nickname_arg.is_some() {
                // ── Flag mode (scriptable) ────────────────────────────────────
                let nickname = nickname_arg.unwrap().clone();
                let mut ip = String::new();
                let mut user = String::new();
                let mut key_path = default_key.clone();
                let mut port: u16 = 22;
                let mut notes = String::new();

                let mut i = 2usize;
                while i < args.len() {
                    match args[i].as_str() {
                        "--ip"    => { ip       = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                        "--user"  => { user     = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                        "--key"   => { key_path = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                        "--port"  => { port     = args.get(i+1).and_then(|p| p.parse().ok()).unwrap_or(22); i += 2; }
                        "--notes" => { notes    = args.get(i+1).cloned().unwrap_or_default(); i += 2; }
                        _         => { i += 1; }
                    }
                }
                (nickname, ip, user, key_path, port, notes)
            } else {
                // ── Interactive wizard ────────────────────────────────────────
                println!("{}", "\nAdd a new SSH host\n".bright_cyan().bold());

                let nickname = prompt_with_default(
                    "Nickname (e.g. prod, pi, home)",
                    nickname_arg.map(|s| s.as_str()).unwrap_or(""),
                )?;

                let ip = prompt_required("IP or Tailscale address")?;
                let user = prompt_required("SSH user")?;

                let key_path = prompt_with_default("SSH key path", &default_key)?;

                let port_str = prompt_with_default("Port", "22")?;
                let port: u16 = port_str.parse().unwrap_or(22);

                let notes = prompt_with_default("Notes (optional)", "")?;

                (nickname, ip, user, key_path, port, notes)
            };

            if ip.is_empty() || user.is_empty() {
                eprintln!("{}", "Error: IP and user are required.".red());
                std::process::exit(1);
            }

            let entry = HostEntry { ip, user, key_path, port, notes };
            hosts::add(&nickname, entry)?;
            println!("{}", format!("\n✅ Host '{}' saved to ~/.clide/hosts.yaml", nickname).green());
            println!(
                "   Skills can reference it as: {}  {}  {}  {}",
                format!("${{HOST_{}_IP}}", nickname.to_uppercase()).cyan(),
                format!("${{HOST_{}_USER}}", nickname.to_uppercase()).cyan(),
                format!("${{HOST_{}_KEY_PATH}}", nickname.to_uppercase()).cyan(),
                format!("${{HOST_{}_PORT}}", nickname.to_uppercase()).cyan(),
            );
        }

        "remove" | "rm" => {
            let nickname = match args.get(1) {
                Some(n) => n.clone(),
                None => prompt_required("Nickname to remove")?,
            };
            hosts::remove(&nickname)?;
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

/// Handle `clide secret <subcommand> [args]`
///
/// Manages secrets in ~/.clide/secrets.yaml and optionally in GNU pass.
///
/// Subcommands:
///   list                 — show all key names (never values)
///   get <KEY>            — print a secret value (prompts for confirmation)
///   set <KEY>            — store a value (interactive, hidden input)
///   generate <KEY> [LEN] — generate a random value, store it, show it once
///   pass-init            — guided setup for GNU pass (optional GPG layer)
///   pass-set <KEY>       — move an existing secret from secrets.yaml into pass
async fn run_secret_cmd(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sub = args.get(0).map(|s| s.as_str()).unwrap_or("list");

    // Load secrets.yaml
    let secrets_path = {
        let home = env::var("HOME").unwrap_or_default();
        std::path::PathBuf::from(home).join(".clide/secrets.yaml")
    };

    match sub {
        // ── list ─────────────────────────────────────────────────────────────
        "list" => {
            println!("{}", "\nStored secrets (keys only):\n".bright_cyan().bold());
            if !secrets_path.exists() {
                println!("  (no secrets.yaml found at {:?})", secrets_path);
                return Ok(());
            }
            let raw = std::fs::read_to_string(&secrets_path)?;
            let map: std::collections::HashMap<String, serde_yaml::Value> =
                serde_yaml::from_str(&raw)?;
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in &keys {
                let v = map[*k].as_str().unwrap_or("");
                let hint = if v.starts_with("pass:") {
                    format!("  → {}", v.dimmed())
                } else if v.is_empty() {
                    "  (empty)".dimmed().to_string()
                } else {
                    "  [set]".green().to_string()
                };
                println!("  {}{}", k.bright_white(), hint);
            }
            println!();
        }

        // ── get ──────────────────────────────────────────────────────────────
        "get" => {
            let key = args.get(1).ok_or("Usage: clide secret get <KEY>")?;
            if !secrets_path.exists() {
                return Err("secrets.yaml not found".into());
            }
            let raw = std::fs::read_to_string(&secrets_path)?;
            let map: std::collections::HashMap<String, String> = serde_yaml::from_str(&raw)?;
            let value = map.get(key.as_str()).ok_or_else(|| format!("Key '{}' not found", key))?;

            // Resolve pass references
            let resolved = if pass_store::is_pass_ref(value) {
                let path = &value["pass:".len()..];
                match pass_store::resolve(path) {
                    Ok(v) => v,
                    Err(e) => return Err(e.into()),
                }
            } else {
                value.clone()
            };

            println!("{}", resolved);
        }

        // ── set ──────────────────────────────────────────────────────────────
        "set" => {
            let key = match args.get(1) {
                Some(k) => k.clone(),
                None => prompt_required("Secret key name")?,
            };

            // Ask where to store it
            println!("{}", "\nWhere to store this secret?".bright_cyan());
            println!("  1. secrets.yaml  (plain text, file-permission protected)");
            if pass_store::pass_available() {
                println!("  2. pass          (GPG-encrypted, recommended for sensitive values)");
            } else {
                println!("  2. pass          (not installed — run: clide secret pass-init)");
            }
            let choice = prompt_with_default("Choice", "1")?;

            let value = prompt_secret(&format!("Value for '{}'", key))?;
            if value.is_empty() {
                return Err("Value cannot be empty".into());
            }

            if choice == "2" && pass_store::pass_available() {
                let pass_path = format!("clide/{}", key.to_lowercase());
                pass_store::insert(&pass_path, &value)
                    .map_err(|e| format!("pass insert failed: {}", e))?;
                // Update secrets.yaml to reference pass
                update_secret_in_yaml(&secrets_path, &key, &format!("pass:{}", pass_path))?;
                println!("{}", format!("\n✅ '{}' stored in pass as '{}'", key, pass_path).green());
                println!("   secrets.yaml updated: {} → {}", key, format!("pass:{}", pass_path).dimmed());
            } else {
                update_secret_in_yaml(&secrets_path, &key, &value)?;
                println!("{}", format!("\n✅ '{}' saved to secrets.yaml", key).green());
            }
        }

        // ── generate ─────────────────────────────────────────────────────────
        "generate" => {
            let key = args.get(1).ok_or("Usage: clide secret generate <KEY> [length]")?;
            let len: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(32);

            let value = generate_secret(len);

            // Decide storage the same way as `set`
            let use_pass = pass_store::pass_available() && {
                println!("{}", "\nStore in:".bright_cyan());
                println!("  1. secrets.yaml");
                println!("  2. pass (GPG-encrypted)");
                prompt_with_default("Choice", "1")? == "2"
            };

            if use_pass {
                let pass_path = format!("clide/{}", key.to_lowercase());
                pass_store::insert(&pass_path, &value)
                    .map_err(|e| format!("pass insert failed: {}", e))?;
                update_secret_in_yaml(&secrets_path, key, &format!("pass:{}", pass_path))?;
                println!("{}", format!("\n✅ Generated and stored '{}' in pass", key).green());
            } else {
                update_secret_in_yaml(&secrets_path, key, &value)?;
                println!("{}", format!("\n✅ Generated and stored '{}' in secrets.yaml", key).green());
            }

            println!("   Value (shown once): {}", value.yellow().bold());
            println!("   Length: {} chars", len);
        }

        // ── pass-init ────────────────────────────────────────────────────────
        "pass-init" => {
            println!("{}", "\nGNU pass setup guide\n".bright_cyan().bold());

            if pass_store::pass_available() {
                println!("{}", "✅ pass is already installed.".green());
            } else {
                println!("Step 1: Install gnupg and pass");
                println!("  {}", "pkg install gnupg pass".cyan());
                println!();
                println!("Run that command, then come back and run: clide secret pass-init again");
                return Ok(());
            }

            println!("Step 2: List existing GPG keys");
            let _ = std::process::Command::new("gpg").args(["--list-keys"]).status();
            println!();

            let gpg_id = prompt_with_default(
                "GPG key ID or email to use (or press Enter to generate a new key)",
                "",
            )?;

            if gpg_id.is_empty() {
                println!("{}", "\nGenerating a new GPG key:".yellow());
                println!("  {}", "gpg --full-generate-key".cyan());
                println!("  Choose: RSA, 4096 bits, no expiry, enter your name/email");
                println!("  After creation, run: clide secret pass-init again");
                let _ = std::process::Command::new("gpg").args(["--full-generate-key"]).status();
                return Ok(());
            }

            // Initialise pass store
            let status = std::process::Command::new("pass")
                .args(["init", &gpg_id])
                .status()?;
            if status.success() {
                println!("{}", format!("\n✅ pass initialized with key: {}", gpg_id).green());
                println!("   Secrets can now be stored with: clide secret set <KEY>");
                println!("   Or mark any value in secrets.yaml as: pass:clide/<key>");
            } else {
                eprintln!("❌ pass init failed. Check that your GPG key ID is correct.");
            }
        }

        // ── pass-set ─────────────────────────────────────────────────────────
        "pass-set" => {
            // Move an existing secrets.yaml entry into pass.
            let key = args.get(1).ok_or("Usage: clide secret pass-set <KEY>")?;
            if !secrets_path.exists() {
                return Err("secrets.yaml not found".into());
            }
            if !pass_store::pass_available() {
                return Err("pass is not installed. Run: clide secret pass-init".into());
            }

            let raw = std::fs::read_to_string(&secrets_path)?;
            let map: std::collections::HashMap<String, String> = serde_yaml::from_str(&raw)?;
            let current = map.get(key.as_str()).ok_or_else(|| format!("Key '{}' not in secrets.yaml", key))?;

            if pass_store::is_pass_ref(current) {
                println!("'{}' is already stored in pass: {}", key, current);
                return Ok(());
            }

            let pass_path = format!("clide/{}", key.to_lowercase());
            pass_store::insert(&pass_path, current)
                .map_err(|e| format!("pass insert failed: {}", e))?;
            update_secret_in_yaml(&secrets_path, key, &format!("pass:{}", pass_path))?;

            println!("{}", format!("✅ '{}' moved to pass at '{}'", key, pass_path).green());
            println!("   Original value erased from secrets.yaml.");
        }

        _ => {
            eprintln!("Unknown subcommand: '{}'", sub);
            eprintln!("Usage: clide secret list | get | set | generate | pass-init | pass-set");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Update (or insert) a single key in secrets.yaml, preserving all other entries and comments.
fn update_secret_in_yaml(
    path: &std::path::Path,
    key: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure the file and its parent directory exist.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Read existing content line-by-line to preserve comments and ordering.
    let mut lines: Vec<String> = if path.exists() {
        std::fs::read_to_string(path)?
            .lines()
            .map(|l| l.to_string())
            .collect()
    } else {
        Vec::new()
    };

    // Quote value if it contains special YAML characters.
    let yaml_value = yaml_quote(value);
    let new_line = format!("{}: {}", key, yaml_value);
    let key_prefix = format!("{}:", key);

    // Replace in-place if the key already exists.
    let mut found = false;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&key_prefix) && !trimmed.starts_with('#') {
            *line = new_line.clone();
            found = true;
            break;
        }
    }

    if !found {
        lines.push(new_line);
    }

    std::fs::write(path, lines.join("\n") + "\n")?;

    // Tighten permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Minimal YAML quoting: wraps in double quotes if value contains special chars.
fn yaml_quote(v: &str) -> String {
    let needs_quotes = v.is_empty()
        || v.contains(':')
        || v.contains('#')
        || v.contains('"')
        || v.contains('\'')
        || v.starts_with('{')
        || v.starts_with('[')
        || v == "true" || v == "false" || v == "null";

    if needs_quotes {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

/// Generate a URL-safe random secret of `len` characters.
fn generate_secret(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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
