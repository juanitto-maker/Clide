// ============================================
// pass_store.rs - GNU pass (password-store) integration
// ============================================
// Optional layer on top of secrets.yaml.
// Any secret value that starts with "pass:" is resolved by calling
// `pass show <path>` at runtime. The decrypted value is used in memory;
// it is never written back to disk.
//
// Example in ~/.clide/secrets.yaml:
//   GEMINI_API_KEY: "pass:clide/gemini"
//   VPS_ROOT_PW:    "pass:servers/prod/root"
//   GITHUB_TOKEN:   "pass:clide/github_token"
//
// Benefits:
//   - Secrets at rest are GPG-encrypted inside ~/.password-store/
//   - `pass` has its own git-based sync (optional)
//   - You can audit secrets with `pass ls` without decrypting
//   - Passphrase cached by gpg-agent so bot doesn't prompt every time
//
// Setup (one-time):
//   pkg install gnupg pass
//   gpg --full-generate-key        # create a key
//   pass init <your-gpg-key-id>    # initialise the store
//   pass insert clide/gemini       # add a secret
//
// FALLBACK BEHAVIOUR:
//   If `pass` is not installed, or the entry doesn't exist, or the GPG
//   agent is locked, the original "pass:..." value is left as-is and a
//   warning is printed. The bot will still start; the affected skill will
//   fail with a placeholder value rather than silently misbehave.
//
// DETECTION:
//   pass_available() → true if `pass` binary found in PATH.
//   is_pass_ref(v)   → true if value starts with "pass:".
//   resolve(path)    → Ok(decrypted_string) | Err(...)

use std::collections::HashMap;
use std::process::Command;

const PREFIX: &str = "pass:";

/// Returns true if the value is a pass reference (e.g. "pass:clide/token").
pub fn is_pass_ref(value: &str) -> bool {
    value.starts_with(PREFIX)
}

/// Returns true if `pass` is installed and executable.
pub fn pass_available() -> bool {
    Command::new("pass")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolve a pass path (without the "pass:" prefix) to its plaintext value.
/// Calls `pass show <path>` and returns the first non-empty line.
pub fn resolve(pass_path: &str) -> Result<String, String> {
    let output = Command::new("pass")
        .arg("show")
        .arg(pass_path)
        .output()
        .map_err(|e| format!("Failed to run `pass show {}`: {}", pass_path, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pass show {} failed: {}", pass_path, stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // `pass show` outputs the password on the first line, then optional metadata.
    let value = stdout.lines().next().unwrap_or("").trim().to_string();
    if value.is_empty() {
        return Err(format!("pass show {} returned empty value", pass_path));
    }
    Ok(value)
}

/// Walk a secrets HashMap and resolve any "pass:..." values in-place.
/// Values that fail to resolve are left as the raw "pass:..." string and
/// a warning is printed to stderr (not to the bot chat).
///
/// This is called once in Config::load() after secrets.yaml is parsed.
pub fn resolve_all(secrets: &mut HashMap<String, String>) {
    let pass_ok = pass_available();

    for (key, value) in secrets.iter_mut() {
        if !is_pass_ref(value) {
            continue;
        }
        let path = &value[PREFIX.len()..];
        if !pass_ok {
            eprintln!(
                "⚠️  Secret '{}' references pass ('{}') but `pass` is not installed. \
                 Install with: pkg install gnupg pass",
                key, path
            );
            continue;
        }
        match resolve(path) {
            Ok(plain) => {
                *value = plain;
            }
            Err(e) => {
                eprintln!("⚠️  Could not resolve pass secret '{}': {}", key, e);
                // leave as "pass:..." — skills will fail gracefully with a
                // placeholder value rather than crashing the bot.
            }
        }
    }
}

/// Insert a secret into the pass store interactively.
/// Calls `pass insert -e <path>` (echo mode, reads from stdin).
pub fn insert(pass_path: &str, value: &str) -> Result<(), String> {
    // `pass insert --echo <path>` reads the password from stdin (one line).
    // We pipe the value in non-interactively.
    let mut child = Command::new("pass")
        .args(["insert", "--echo", "--force", pass_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn `pass insert`: {}", e))?;

    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        stdin
            .write_all(format!("{}\n", value).as_bytes())
            .map_err(|e| format!("Failed to write to pass stdin: {}", e))?;
    }

    let out = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for pass: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("pass insert failed: {}", stderr.trim()));
    }
    Ok(())
}

/// List all entries under a prefix path (e.g. "clide").
/// Returns a list of pass paths (e.g. ["clide/gemini", "clide/github_token"]).
pub fn list_entries(prefix: &str) -> Result<Vec<String>, String> {
    let output = Command::new("pass")
        .arg("ls")
        .arg(prefix)
        .output()
        .map_err(|e| format!("Failed to run `pass ls {}`: {}", prefix, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // `pass ls` outputs a tree; extract leaf names with simple heuristics.
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let stripped = line
            .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '/')
            .trim();
        if !stripped.is_empty() && !stripped.contains("Password Store") {
            entries.push(format!("{}/{}", prefix, stripped));
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pass_ref() {
        assert!(is_pass_ref("pass:clide/token"));
        assert!(!is_pass_ref("plain_value_here"));
        assert!(!is_pass_ref(""));
    }

    #[test]
    fn test_resolve_all_no_pass() {
        // Without `pass` installed this should warn but not panic.
        let mut secrets = HashMap::new();
        secrets.insert("A".to_string(), "pass:clide/a".to_string());
        secrets.insert("B".to_string(), "plain_value".to_string());
        resolve_all(&mut secrets);
        // B must be unchanged
        assert_eq!(secrets.get("B").map(|s| s.as_str()), Some("plain_value"));
        // A stays as-is (pass not available in test env)
    }
}
